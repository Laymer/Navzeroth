import test from "node:test";
import assert from "node:assert/strict";
import {
  SCREEN_LABELS, parseArgs, isCspViolationText, isNonNativeAlert, parseSha256Sums,
  verifyAgainstSums, compareVersions, classifyUpgrade, cspFromHeaders, parseCsp, Recorder,
} from "./native-smoke-lib.mjs";

const H = "a".repeat(64);

test("there are exactly 14 distinct screen labels", () => {
  assert.equal(SCREEN_LABELS.length, 14);
  assert.equal(new Set(SCREEN_LABELS).size, 14);
});

test("parseArgs accepts known pairs and rejects the rest", () => {
  assert.deepEqual(parseArgs(["--mode", "smoke", "--port", "9222"], ["mode", "port"]), { mode: "smoke", port: "9222" });
  assert.throws(() => parseArgs(["--nope", "1"], ["mode"]), /Unknown option/);
  assert.throws(() => parseArgs(["--mode"], ["mode"]), /needs a value/);
  assert.throws(() => parseArgs(["--mode", "--port"], ["mode", "port"]), /needs a value/);
  assert.throws(() => parseArgs(["stray"], ["mode"]), /Unexpected argument/);
});

test("CSP violation text is recognised, ordinary logs are not", () => {
  assert.ok(isCspViolationText("Refused to load the image 'data:image/png' because it violates the following Content Security Policy directive"));
  assert.ok(isCspViolationText("Refused to apply inline style"));
  assert.ok(!isCspViolationText("Loaded 14 screens"));
});

test("only PROVIDER_UNAVAILABLE marks a non-native page", () => {
  assert.ok(isNonNativeAlert("Error PROVIDER_UNAVAILABLE: demo"));
  assert.ok(!isNonNativeAlert("Error INVALID_INPUT: Invalid snapshot export"));
});

test("SHA256SUMS parsing and verification", () => {
  const sums = parseSha256Sums(`${H}  a.exe\n${"B".repeat(64)} *b.zip\n\n`);
  assert.equal(sums.get("a.exe"), H);
  assert.equal(sums.get("b.zip"), "b".repeat(64));
  assert.equal(verifyAgainstSums(sums, "a.exe", H.toUpperCase()), H);
  assert.throws(() => verifyAgainstSums(sums, "a.exe", "c".repeat(64)), /does not match/);
  assert.throws(() => verifyAgainstSums(sums, "missing.exe", H), /not listed/);
  assert.throws(() => parseSha256Sums("nothex  a.exe"), /Malformed/);
  assert.throws(() => parseSha256Sums(`${H}  ../a.exe`), /bare name/);
  assert.throws(() => parseSha256Sums(`${H}  a.exe\n${H}  a.exe`), /Duplicate/);
});

test("version comparison and upgrade classification", () => {
  assert.equal(compareVersions("0.9.1", "0.9.2"), -1);
  assert.equal(compareVersions("0.10.0", "0.9.9"), 1);
  assert.equal(compareVersions("1.0.0", "1.0.0"), 0);
  assert.throws(() => compareVersions("1.0.0-rc1", "1.0.0"), /plain/);
  assert.deepEqual(classifyUpgrade("0.9.1", "0.9.2"), { kind: "upgrade", downgradeApplicable: true });
  assert.deepEqual(classifyUpgrade("0.9.1", "0.9.1"), { kind: "reinstall_same_version", downgradeApplicable: false });
  assert.equal(classifyUpgrade("0.9.2", "0.9.1").kind, "baseline_newer_than_candidate");
});

test("CSP header lookup is case-insensitive and parsing keeps directives", () => {
  assert.equal(cspFromHeaders({ "Content-Security-Policy": "default-src 'self'" }), "default-src 'self'");
  assert.equal(cspFromHeaders({ other: "x" }), null);
  assert.equal(cspFromHeaders(undefined), null);
  const csp = parseCsp("default-src 'self'; img-src 'self' atlas: http://atlas.localhost;");
  assert.deepEqual(csp["img-src"], ["'self'", "atlas:", "http://atlas.localhost"]);
  assert.ok(!csp["img-src"].includes("data:"));
});

test("Recorder keeps failed and skipped distinct from passed", async () => {
  const r = new Recorder();
  await r.run("ok", async () => ({ n: 1 }));
  await r.run("bad", async () => { throw new Error("boom"); });
  r.add("later", "skipped", { reason: "no seed" });
  assert.deepEqual(r.summary(), { passed: 1, failed: 1, skipped: 1, ok: false });
  assert.equal(r.steps[1].error, "boom");
  assert.throws(() => r.add("x", "maybe"), /Bad status/);
});
