import test from "node:test";
import assert from "node:assert/strict";
import {
  invokeInPage, invokeNative, waitForDatabaseReady, SCREEN_LABELS, parseArgs, isCspViolationText, isNonNativeAlert, parseSha256Sums,
  verifyAgainstSums, compareVersions, classifyUpgrade, cspFromHeaders, parseCsp, Recorder,
  gracefulCloseArgs, debugPolicyValue, IpcLog, freshenSnapshotText, migrationSummary, policyAddArgs, policyDeleteArgs, WEBVIEW2_POLICY_KEY,
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

test("debugging policy arguments are bounded and target one exe name", () => {
  assert.equal(debugPolicyValue(9222), "--remote-debugging-port=9222 --remote-allow-origins=*");
  assert.throws(() => debugPolicyValue(80), /Bad debugging port/);
  assert.throws(() => debugPolicyValue(99999), /Bad debugging port/);
  assert.throws(() => debugPolicyValue("9222"), /Bad debugging port/);
  const add = policyAddArgs("azeroth-desktop.exe", 9223);
  assert.deepEqual(add.slice(0, 4), ["add", WEBVIEW2_POLICY_KEY, "/v", "azeroth-desktop.exe"]);
  assert.ok(add.includes("--remote-debugging-port=9223 --remote-allow-origins=*"));
  assert.deepEqual(policyDeleteArgs(), ["delete", WEBVIEW2_POLICY_KEY, "/f"]);
  assert.match(WEBVIEW2_POLICY_KEY, /^HKLM\\SOFTWARE\\Policies\\Microsoft\\Edge\\WebView2\\AdditionalBrowserArguments$/);
});

test("IpcLog sequence marks keep working after the window is full", () => {
  const log = new IpcLog(3);
  for (let i = 0; i < 10; i += 1) log.push(`noise ${i}`);
  const mark = log.mark();
  assert.equal(log.seenSince(mark, "start_planning_job"), false);
  for (let i = 0; i < 5; i += 1) log.push(`more ${i}`);
  log.push("1ms -> start_planning_job (POST)");
  assert.equal(log.entries.length, 3);
  assert.equal(log.seenSince(mark, " start_planning_job "), true);
  assert.equal(log.seenSince(log.mark(), " start_planning_job "), false);
});

test("freshenSnapshotText re-times capture fields only and stays synthetic", () => {
  const text = `ANAV1:${JSON.stringify({ fixture: true, provenance: { created_at_utc_ms: 1 }, payload: { capture: { observed_at_utc_ms: 2, received_at_utc_ms: 3, clock_trust: "trusted" }, list: [{ observed_at_utc_ms: 2 }] } })}`;
  const now = 1_800_000_000_000;
  const out = JSON.parse(freshenSnapshotText(text, now).slice(6));
  assert.equal(out.fixture, true);
  assert.equal(out.provenance.created_at_utc_ms, now);
  assert.equal(out.payload.capture.observed_at_utc_ms, now - 10000);
  assert.equal(out.payload.capture.received_at_utc_ms, now - 5000);
  assert.equal(out.payload.capture.clock_trust, "trusted");
  assert.equal(out.payload.list[0].observed_at_utc_ms, now - 10000);
  assert.throws(() => freshenSnapshotText("{}", now), /Not an ANAV1/);
  assert.throws(() => freshenSnapshotText("ANAV1:{}", now), /No capture times/);
  assert.throws(() => freshenSnapshotText(text, 0), /Bad current time/);
});

test("migrationSummary reports an empty table as unknown, not zero", () => {
  assert.deepEqual(migrationSummary(0, null), { migrations: 0, max_migration_version: null });
  assert.deepEqual(migrationSummary(12, 12), { migrations: 12, max_migration_version: 12 });
});

test("instrumented shutdown accepts only a bounded owned PID and requests a normal window close", () => {
  for (const pid of [0, -1, 1.5, Infinity, NaN, 0x100000000, "42; exit 0", null, undefined]) assert.throws(() => gracefulCloseArgs(pid));
  const args = gracefulCloseArgs(1042);
  assert.deepEqual(args.slice(0, 4), ["-NoLogo", "-NoProfile", "-NonInteractive", "-Command"]);
  assert.match(args[4], /GetProcessById\(1042\)/);
  assert.match(args[4], /CloseMainWindow\(\)/);
  assert.match(args[4], /WaitForExit\(10000\)/);
  assert.match(args[4], /exit \$ownedProcess.ExitCode/);
  assert.doesNotMatch(args[4], /Stop-Process|Kill\(|taskkill/);
});


test("readiness waits for delayed startup and rejects recovery, unknown, error and timeout", async () => {
  let calls = 0;
  assert.deepEqual(await waitForDatabaseReady(async () => ({state: ++calls < 3 ? "checking" : "ready"}), 1000, 1), {state:"ready"});
  assert.equal(calls, 3);
  await assert.rejects(waitForDatabaseReady(async () => ({state:"recovery_required", preserved_recovery_directory:"private"})), /recovery required/);
  await assert.rejects(waitForDatabaseReady(async () => ({state:"unexpected"})), /Unknown/);
  await assert.rejects(waitForDatabaseReady(async () => { throw new Error("IPC failed"); }), /IPC failed/);
  await assert.rejects(waitForDatabaseReady(async () => ({state:"checking"}), 10, 1), /timed out/);
  await assert.rejects(waitForDatabaseReady(() => new Promise(() => {}), 10, 1), /timed out/);
  await assert.rejects(waitForDatabaseReady(async () => ({}), 0), /Invalid/);
});

test("native IPC errors preserve safe codes and omit private fields", async () => {
  const previous = globalThis.window;
  try {
    globalThis.window = {__TAURI_INTERNALS__: {invoke: async () => { throw {code:"DATABASE_ERROR", message:"C:/private/export", stack:"secret", details:{private:true}}; }}};
    assert.deepEqual(await invokeInPage({command:"get_reference_view"}), {ok:false,error:{code:"DATABASE_ERROR"}});
    const page = {evaluate: (fn, args) => fn(args)};
    await assert.rejects(invokeNative(page, "get_reference_view"), /^Error: Native IPC get_reference_view: \{"code":"DATABASE_ERROR"\}$/);
    window.__TAURI_INTERNALS__.invoke = async () => { throw "private unstructured text"; };
    assert.deepEqual(await invokeInPage({command:"test"}), {ok:false,error:{code:"UNSTRUCTURED_IPC_ERROR"}});
    window.__TAURI_INTERNALS__.invoke = async () => false;
    assert.equal(await invokeNative(page, "test"), false);
  } finally {
    globalThis.window = previous;
  }
});
