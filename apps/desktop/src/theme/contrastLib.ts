import { readFileSync } from "node:fs";
import { join } from "node:path";

export const css = readFileSync(join(__dirname, "tokens.css"), "utf8");

/** Returns the body of the first rule whose selector starts with `selector`, brace matched. */
export function block(source: string, selector: string): string {
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

export function declarations(body: string): Record<string, string> {
  const result: Record<string, string> = {};
  for (const m of body.matchAll(/(--[\w-]+)\s*:\s*([^;]+);/g)) result[m[1] ?? ""] = (m[2] ?? "").trim();
  return result;
}

export const stripped = css.replace(/\/\*[\s\S]*?\*\//g, "");
export const base = declarations(block(stripped, ":root {"));
export const high = { ...base, ...declarations(block(stripped, ':root[data-contrast="high"]')) };
export const forcedBlock = block(stripped, "@media (forced-colors: active)");
export const forced = { ...base, ...declarations(block(forcedBlock, ":root")) };

export function resolve(tokens: Record<string, string>, name: string): string {
  let value = tokens[name];
  for (let hops = 0; value !== undefined && hops < 8; hops++) {
    const ref = /^var\((--[\w-]+)\)$/.exec(value);
    if (ref === null) return value;
    value = tokens[ref[1] ?? ""];
  }
  throw new Error(`cannot resolve ${name}`);
}

export function luminance(hex: string): number {
  const v = /^#([0-9a-f]{6})$/i.exec(hex);
  if (v === null) throw new Error(`not a six digit hex colour: ${hex}`);
  const channels = [0, 2, 4].map((offset) => {
    const c = parseInt((v[1] ?? "").slice(offset, offset + 2), 16) / 255;
    return c <= 0.03928 ? c / 12.92 : ((c + 0.055) / 1.055) ** 2.4;
  });
  return 0.2126 * (channels[0] ?? 0) + 0.7152 * (channels[1] ?? 0) + 0.0722 * (channels[2] ?? 0);
}

export function ratio(a: string, b: string): number {
  const [hi, lo] = [luminance(a), luminance(b)].sort((x, y) => y - x);
  return ((hi ?? 0) + 0.05) / ((lo ?? 0) + 0.05);
}


export type Rgb = readonly [number, number, number];

export function parseHex(hex: string): Rgb {
  const v = /^#([0-9a-f]{6})$/i.exec(hex);
  if (v === null) throw new Error(`not a six digit hex colour: ${hex}`);
  const n = parseInt(v[1] ?? "", 16);
  return [(n >> 16) & 255, (n >> 8) & 255, n & 255];
}

export function toHex(rgb: Rgb): string {
  return "#" + rgb.map((c) => Math.round(c).toString(16).padStart(2, "0")).join("");
}

/** `top` at `opacity` over an opaque `bottom`, in sRGB like the browser. */
export function over(top: string, opacity: number, bottom: string): string {
  const [a, b] = [parseHex(top), parseHex(bottom)];
  return toHex([0, 1, 2].map((i) => (a[i] ?? 0) * opacity + (b[i] ?? 0) * (1 - opacity)) as unknown as Rgb);
}
