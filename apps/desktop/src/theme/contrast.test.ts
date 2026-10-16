import { describe, expect, it } from "vitest";
import { readFileSync } from "node:fs";
import { join } from "node:path";

// Parses tokens.css and checks the WCAG 2.x contrast ratio of every pair declared in
// its "contrast:" comments, in default, high-contrast and forced-colours mode.

const css = readFileSync(join(__dirname, "tokens.css"), "utf8");

interface Pair {
  fg: string;
  bg: string;
  min: number;
}

const pairs: Pair[] = [...css.matchAll(/contrast:\s*(--[\w-]+)\s+on\s+(--[\w-]+)\s+([\d.]+)/g)].map((m) => ({
  fg: m[1] ?? "",
  bg: m[2] ?? "",
  min: Number(m[3]),
}));

/** Returns the body of the first rule whose selector starts with `selector`, brace matched. */
function block(source: string, selector: string): string {
  const start = source.indexOf(selector);
  if (start < 0) throw new Error(`tokens.css has no ${selector} rule`);
  const open = source.indexOf("{", start);
  let depth = 0;
  for (let index = open; index < source.length; index++) {
    if (source[index] === "{") depth++;
    if (source[index] === "}" && --depth === 0) return source.slice(open + 1, index);
  }
  throw new Error(`unbalanced braces after ${selector}`);
}

function declarations(body: string): Record<string, string> {
  const result: Record<string, string> = {};
  for (const m of body.matchAll(/(--[\w-]+)\s*:\s*([^;]+);/g)) result[m[1] ?? ""] = (m[2] ?? "").trim();
  return result;
}

const stripped = css.replace(/\/\*[\s\S]*?\*\//g, "");
const base = declarations(block(stripped, ":root {"));
const high = { ...base, ...declarations(block(stripped, ':root[data-contrast="high"]')) };
const forcedBlock = block(stripped, "@media (forced-colors: active)");
const forced = { ...base, ...declarations(block(forcedBlock, ":root")) };

function resolve(tokens: Record<string, string>, name: string): string {
  let value = tokens[name];
  for (let hops = 0; value !== undefined && hops < 8; hops++) {
    const ref = /^var\((--[\w-]+)\)$/.exec(value);
    if (ref === null) return value;
    value = tokens[ref[1] ?? ""];
  }
  throw new Error(`cannot resolve ${name}`);
}

function luminance(hex: string): number {
  const v = /^#([0-9a-f]{6})$/i.exec(hex);
  if (v === null) throw new Error(`not a six digit hex colour: ${hex}`);
  const channels = [0, 2, 4].map((offset) => {
    const c = parseInt((v[1] ?? "").slice(offset, offset + 2), 16) / 255;
    return c <= 0.03928 ? c / 12.92 : ((c + 0.055) / 1.055) ** 2.4;
  });
  return 0.2126 * (channels[0] ?? 0) + 0.7152 * (channels[1] ?? 0) + 0.0722 * (channels[2] ?? 0);
}

function ratio(a: string, b: string): number {
  const [hi, lo] = [luminance(a), luminance(b)].sort((x, y) => y - x);
  return ((hi ?? 0) + 0.05) / ((lo ?? 0) + 0.05);
}

/** The system-colour pairs that forced-colours mode guarantees are legible together. */
const SYSTEM_PAIRS = new Set([
  "CanvasText/Canvas",
  "LinkText/Canvas",
  "VisitedText/Canvas",
  "ButtonText/ButtonFace",
  "HighlightText/Highlight",
  "Highlight/Canvas",
  "GrayText/Canvas",
  "FieldText/Field",
]);

describe("contrast declarations", () => {
  it("declares a meaningful set of pairs", () => {
    expect(pairs.length).toBeGreaterThanOrEqual(30);
    expect(pairs.some((p) => p.min === 3)).toBe(true);
  });

  it("covers every colour token that carries text or a boundary", () => {
    for (const name of ["--text", "--text-muted", "--accent", "--gold-hi", "--on-ember", "--control-edge", "--focus"]) {
      expect(pairs.some((p) => p.fg === name), name).toBe(true);
    }
  });

  it("resolves every declared token in every mode", () => {
    for (const tokens of [base, high, forced]) {
      for (const { fg, bg } of pairs) {
        expect(() => resolve(tokens, fg), fg).not.toThrow();
        expect(() => resolve(tokens, bg), bg).not.toThrow();
      }
    }
  });

  for (const [mode, tokens] of [["default", base], ["high contrast", high]] as const) {
    describe(`${mode} mode`, () => {
      for (const { fg, bg, min } of pairs) {
        it(`${fg} on ${bg} is at least ${min}:1`, () => {
          expect(ratio(resolve(tokens, fg), resolve(tokens, bg))).toBeGreaterThanOrEqual(min);
        });
      }
    });
  }

  describe("forced-colours mode", () => {
    for (const { fg, bg } of pairs) {
      it(`${fg} on ${bg} uses a guaranteed system-colour pair`, () => {
        const pair = `${resolve(forced, fg)}/${resolve(forced, bg)}`;
        expect(SYSTEM_PAIRS.has(pair), pair).toBe(true);
      });
    }
  });

  it("holds high contrast to at least 7:1 for text pairs", () => {
    for (const { fg, bg, min } of pairs) {
      if (min < 4.5) continue;
      expect(ratio(resolve(high, fg), resolve(high, bg)), `${fg} on ${bg}`).toBeGreaterThanOrEqual(7);
    }
  });
});
