// Generates schemas/v1/exchange.ts from the committed Rust-generated schema.
// Usage: pnpm contracts:types   (CI fails if the committed output differs)
import { readFile, writeFile } from "node:fs/promises";
import { compile } from "json-schema-to-typescript";

const root = new URL("../schemas/v1/", import.meta.url);
for (const [file, type] of [["exchange", "AzerothNavigatorExchangeV1"], ["progression-api", "ProgressionApi"]]) {
const schema = JSON.parse(await readFile(new URL(`${file}.schema.json`, root), "utf8"));
const banner = `/* Generated from ${file}.schema.json by scripts/generate-contract-types.mjs.
 * Do not edit. Structural types only: Rust validation enforces semantic
 * invariants, and unknown members are omitted from these types. */`;
const ts = await compile(schema, type, {
  bannerComment: banner,
  additionalProperties: false,
  unreachableDefinitions: true,
  strictIndexSignatures: true,
  format: true,
  style: { endOfLine: "lf", printWidth: 100 },
});
await writeFile(new URL(`${file}.ts`, root), ts.replace(/\r\n/g, "\n"));
console.log(`Wrote schemas/v1/${file}.ts`);
}
