// Generates schemas/v1/exchange.ts from the committed Rust-generated schema.
// Usage: pnpm contracts:types   (CI fails if the committed output differs)
import { readFile, writeFile } from "node:fs/promises";
import { compile } from "json-schema-to-typescript";

const root = new URL("../schemas/v1/", import.meta.url);
const schema = JSON.parse(await readFile(new URL("exchange.schema.json", root), "utf8"));
const banner = `/* Generated from exchange.schema.json by scripts/generate-contract-types.mjs.
 * Do not edit. Structural types only: Rust validation enforces semantic
 * invariants, and unknown members are omitted from these types. */`;
const ts = await compile(schema, "AzerothNavigatorExchangeV1", {
  bannerComment: banner,
  additionalProperties: false,
  unreachableDefinitions: true,
  strictIndexSignatures: true,
  format: true,
  style: { endOfLine: "lf", printWidth: 100 },
});
await writeFile(new URL("exchange.ts", root), ts.replace(/\r\n/g, "\n"));
console.log("Wrote schemas/v1/exchange.ts");
