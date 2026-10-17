import { describe, expect, it } from "vitest";
import { base, css, forced, high, ratio, resolve } from "./contrastLib";

// Parses tokens.css and checks the WCAG 2.x contrast ratio of every pair declared in
// its "contrast:" comments, in default, high-contrast and forced-colours mode.
// Compositions that tokens alone cannot show (gradients, ::selection, text over the
// backdrop) are checked in composite.test.ts.

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
