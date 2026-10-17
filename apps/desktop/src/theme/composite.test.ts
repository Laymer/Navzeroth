import { describe, expect, it } from "vitest";
import { readdirSync, readFileSync, statSync } from "node:fs";
import { join } from "node:path";
import { base, high, over, ratio, resolve } from "./contrastLib";

// Token-on-token pairs live in contrast.test.ts. These tests check compositions that a
// pair cannot show: every point along a gradient, ::selection as written in base.css, and
// text over the translucent shell sheet above the worst-case backdrop colours.

const read = (...parts: string[]) => readFileSync(join(__dirname, "..", ...parts), "utf8");
const uiCss = read("ui", "ui.css");
const baseCss = read("theme", "base.css");

/** First rule body for an exact selector at the start of a line. */
function rule(source: string, selector: string): string {
  const escaped = selector.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
  const m = new RegExp(`^${escaped}\\s*\\{([^}]*)\\}`, "m").exec(source);
  if (m === null) throw new Error(`no rule for ${selector}`);
  return m[1] ?? "";
}

const varName = (text: string) => /var\((--[\w-]+)\)/.exec(text)?.[1] ?? "";

function gradientStops(body: string): string[] {
  const m = /linear-gradient\(([^;]*)\)\s*[;,]/.exec(body);
  if (m === null) throw new Error("no linear-gradient");
  return [...(m[1] ?? "").matchAll(/var\((--[\w-]+)\)/g)].map((x) => x[1] ?? "");
}

describe("gradient buttons", () => {
  const defaultColor = varName(/color:\s*(var\(--[\w-]+\))/.exec(rule(uiCss, ".ui-button"))?.[1] ?? "");
  const cases = [
    { selector: ".ui-button--primary", expectedStops: ["--ember", "--ember-shade"], expectedText: "--on-ember" },
    { selector: ".ui-button--danger", expectedStops: ["--danger", "--danger-shade"], expectedText: "--on-danger" },
    { selector: ".ui-button--secondary", expectedStops: ["--surface-raised", "--surface"], expectedText: defaultColor },
  ];
  for (const { selector, expectedStops, expectedText } of cases) {
    it(`${selector} text holds 4.5:1 at every point of its gradient`, () => {
      const body = rule(uiCss, selector);
      const stops = gradientStops(body);
      expect(stops).toEqual(expectedStops);
      const text = /(?:^|;|\s)color:\s*var\((--[\w-]+)\)/.exec(body)?.[1] ?? defaultColor;
      expect(text).toBe(expectedText);
      for (const tokens of [base, high]) {
        const [from, to] = stops.map((name) => resolve(tokens, name)) as [string, string];
        for (let step = 0; step <= 10; step++) {
          const point = over(to, step / 10, from);
          expect(ratio(resolve(tokens, text), point), `${selector} at ${step * 10}% (${point})`).toBeGreaterThanOrEqual(4.5);
        }
      }
    });
  }

  it("fails for a gradient that starts at --ember-hi (the G4 review finding)", () => {
    // Guards the helper itself: white on the old first stop was 3.30:1.
    expect(ratio(resolve(base, "--on-ember"), resolve(base, "--ember-hi"))).toBeLessThan(4.5);
  });
});

describe("::selection", () => {
  it("is written with the declared selection tokens", () => {
    const body = rule(baseCss, "::selection");
    expect(varName(/background:\s*(var\(--[\w-]+\))/.exec(body)?.[1] ?? "")).toBe("--select-bg");
    expect(varName(/color:\s*(var\(--[\w-]+\))/.exec(body)?.[1] ?? "")).toBe("--select-fg");
  });

  it("is readable in default and high contrast", () => {
    expect(ratio(resolve(base, "--select-fg"), resolve(base, "--select-bg"))).toBeGreaterThanOrEqual(4.5);
    expect(ratio(resolve(high, "--select-fg"), resolve(high, "--select-bg"))).toBeGreaterThanOrEqual(7);
  });
});

describe("text over the backdrop", () => {
  const sheet = /^\.app-shell \{[^}]*background:\s*color-mix\(in srgb, var\((--[\w-]+)\) (\d+)%, transparent\)/m.exec(baseCss);
  const backdrop = read("theme", "Backdrop.tsx");
  // Every colour the SVG draws, taken as fully opaque: a conservative bound on any pixel under the sheet.
  const colours = [...new Set([...backdrop.matchAll(/#[0-9a-fA-F]{6}\b/g)].map((m) => m[0].toLowerCase()))];

  it("puts the reference window on the same translucent token sheet", () => {
    const ref = /^\.reference-shell \{[^}]*background:\s*color-mix\(in srgb, var\((--[\w-]+)\) (\d+)%, transparent\)/m.exec(baseCss);
    expect(ref?.[1]).toBe(sheet?.[1]);
    expect(ref?.[2]).toBe(sheet?.[2]);
  });

  it("puts the app shell on a translucent token sheet", () => {
    expect(sheet).not.toBeNull();
    expect(Number(sheet?.[2])).toBeGreaterThanOrEqual(85);
    expect(colours.length).toBeGreaterThan(8);
  });

  const textTokens = ["--text", "--text-muted", "--accent", "--gold-hi", "--tone-ok", "--tone-warn", "--tone-danger", "--tone-info"];
  it("keeps body text at 4.5:1 over the sheet above any backdrop colour", () => {
    const surface = resolve(base, sheet?.[1] ?? "--surface");
    const opacity = Number(sheet?.[2]) / 100;
    for (const colour of colours) {
      const composite = over(surface, opacity, colour);
      for (const name of textTokens) {
        expect(ratio(resolve(base, name), composite), `${name} over ${colour} -> ${composite}`).toBeGreaterThanOrEqual(4.5);
      }
    }
  });
});

describe("--text-muted and --mist", () => {
  // --text-muted on --mist is 4.31:1, so the pair is deliberately not declared and must not be used.
  it("never places muted text on a --mist background", () => {
    const files: string[] = [];
    const walk = (dir: string) => {
      for (const entry of readdirSync(dir)) {
        const path = join(dir, entry);
        if (statSync(path).isDirectory()) walk(path);
        else if (path.endsWith(".css")) files.push(path);
      }
    };
    walk(join(__dirname, ".."));
    expect(files.length).toBeGreaterThan(5);
    for (const file of files) {
      for (const m of readFileSync(file, "utf8").matchAll(/\{([^{}]*)\}/g)) {
        const body = m[1] ?? "";
        if (/background[^;]*var\(--mist\)/.test(body)) expect(body, file).not.toMatch(/(?<![-\w])color:\s*var\(--text-muted\)/);
      }
    }
  });

  it("is a real ratio below 4.5:1 so the missing declaration is intentional", () => {
    expect(ratio(resolve(base, "--text-muted"), resolve(base, "--mist"))).toBeLessThan(4.5);
  });
});
