// Offline tests for the build watch parser (synthetic tables in Blizzard's format).
import test from "node:test";
import assert from "node:assert/strict";
import { KNOWN_WOW_PRODUCTS, WATCHED, buildIssueTitle, newProducts, parseTable, productIssueTitle, summarizeVersions } from "./lib.mjs";

const HEADER = "Region!STRING:0|BuildConfig!HEX:16|CDNConfig!HEX:16|KeyRing!HEX:16|BuildId!DEC:4|VersionsName!String:0|ProductConfig!HEX:16";
const row = (region, config, build, version) => `${region}|${config}|${"c".repeat(32)}||${build}|${version}|${"d".repeat(32)}`;
const A = "a".repeat(32);
const B = "b".repeat(32);

test("a consistent product reports one build with all its regions", () => {
  const text = [HEADER, "## seqn = 42", row("us", A, "70450", "1.60.1.70450"), row("eu", A, "70450", "1.60.1.70450"), ""].join("\n");
  const summary = summarizeVersions("wow_classic_beta", text);
  assert.equal(summary.seqn, 42);
  assert.equal(summary.regions_agree, true);
  assert.deepEqual(summary.builds, [{ version: "1.60.1.70450", build: 70450, build_config: A, regions: ["eu", "us"] }]);
  assert.equal(buildIssueTitle("wow_classic_beta", summary.builds[0]), "Build watch: wow_classic_beta is 1.60.1.70450 (build 70450)");
});

test("regions on different builds are reported separately, newest first", () => {
  const text = [HEADER, row("us", B, "70451", "1.60.1.70451"), row("eu", A, "70450", "1.60.1.70450")].join("\r\n");
  const summary = summarizeVersions("wowf", text);
  assert.equal(summary.regions_agree, false);
  assert.deepEqual(summary.builds.map((b) => b.build), [70451, 70450]);
  assert.equal(summary.seqn, null);
});

test("malformed tables and rows are refused instead of guessed", () => {
  assert.throws(() => parseTable(""), /no header/);
  assert.throws(() => parseTable("<html>404</html>"), /no header/);
  assert.throws(() => parseTable("A!STRING:0|A!STRING:0\nx|y"), /header is invalid/);
  assert.throws(() => parseTable(`${HEADER}\nus|only|three`), /wrong column count/);
  assert.throws(() => parseTable("x".repeat(256 * 1024 + 1)), /too large/);
  assert.throws(() => summarizeVersions("p", [HEADER, row("us", A, "70450", "not-a-version")].join("\n")), /Unexpected version row/);
  assert.throws(() => summarizeVersions("p", [HEADER, row("us", "short", "70450", "1.60.1.70450")].join("\n")), /Unexpected version row/);
});

test("only unknown wow product codes are reported as new", () => {
  const summary = ["Product!STRING:0|Seqn!DEC:4|Flags!STRING:0", "## seqn = 1", "agent|1|cdn", "wowf|2|", "wowf|3|cdn", "wow_forever|4|", "wowx-bad|5|"].join("\n");
  assert.deepEqual(newProducts(summary), ["wow_forever"]);
  assert.equal(productIssueTitle("wow_forever"), "Build watch: new WoW product code wow_forever");
});

test("every watched product is in the known list", () => {
  for (const product of WATCHED) assert.ok(KNOWN_WOW_PRODUCTS.includes(product), product);
});
