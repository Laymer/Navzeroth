// Pure helpers for Native-Smoke.mjs. No I/O so they can be tested on Linux.
// Everything built on these is regression evidence only and never WIN-* acceptance.

export const SCREEN_LABELS = [
  "Today", "Settings", "Compatibility and coverage", "Local diagnostics",
  "Backup and recovery", "Atlas", "Imports", "Party", "Dungeons", "Content",
  "Planner", "Gear", "Sessions", "Reminders", "World",
];

export const MODES = ["smoke", "populate", "verify", "capture"];

/** Parse `--name value` pairs. Unknown names and missing values are errors. */
export function parseArgs(argv, allowed) {
  const out = {};
  for (let i = 0; i < argv.length; i += 1) {
    const arg = argv[i];
    if (!arg.startsWith("--")) throw new Error(`Unexpected argument: ${arg}`);
    const name = arg.slice(2);
    if (!allowed.includes(name)) throw new Error(`Unknown option: ${arg}`);
    const value = argv[i + 1];
    if (value === undefined || value.startsWith("--")) throw new Error(`Option ${arg} needs a value`);
    out[name] = value;
    i += 1;
  }
  return out;
}

/** Text of a console message or violation that indicates a CSP block. */
export function isCspViolationText(text) {
  return /content security policy|refused to (load|apply|connect|execute|frame|create a worker)|violated the following/i.test(text);
}

/** Alerts that show the page is not running over real native IPC. */
export function isNonNativeAlert(text) {
  return /PROVIDER_UNAVAILABLE/.test(text);
}

export function parseSha256Sums(text) {
  const map = new Map();
  for (const raw of text.split(/\r?\n/)) {
    const line = raw.trim();
    if (line === "") continue;
    const match = /^([0-9a-fA-F]{64}) [ *](.+)$/.exec(line);
    if (!match) throw new Error(`Malformed SHA256SUMS line: ${line}`);
    const name = match[2];
    if (name.includes("/") || name.includes("\\")) throw new Error(`SHA256SUMS path is not a bare name: ${name}`);
    if (map.has(name)) throw new Error(`Duplicate SHA256SUMS entry: ${name}`);
    map.set(name, match[1].toLowerCase());
  }
  return map;
}

/** Throws unless `name` is listed and `actualSha256` matches the listed digest. */
export function verifyAgainstSums(sums, name, actualSha256) {
  const expected = sums.get(name);
  if (expected === undefined) throw new Error(`${name} is not listed in SHA256SUMS.txt`);
  if (expected !== String(actualSha256).toLowerCase()) {
    throw new Error(`${name} digest ${actualSha256} does not match SHA256SUMS.txt (${expected})`);
  }
  return expected;
}

/** Strict SemVer precedence. Decimal strings avoid rounding large numeric identifiers. */
export function compareVersions(a, b) {
  const parse = (value) => {
    const match = typeof value === "string" && /^(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)(?:-([0-9A-Za-z-]+(?:\.[0-9A-Za-z-]+)*))?(?:\+([0-9A-Za-z-]+(?:\.[0-9A-Za-z-]+)*))?$/.exec(value);
    if (!match || match[0] !== value) throw new Error("Malformed SemVer version");
    const pre = match[4]?.split(".") ?? [];
    if (pre.some(id => /^0[0-9]+$/.test(id))) throw new Error("Malformed SemVer numeric prerelease identifier");
    return { core: match.slice(1, 4), pre };
  };
  const decimal = (x, y) => x.length !== y.length ? (x.length < y.length ? -1 : 1) : x === y ? 0 : x < y ? -1 : 1;
  const x = parse(a), y = parse(b);
  for (let i = 0; i < 3; i += 1) {
    const order = decimal(x.core[i], y.core[i]);
    if (order) return order;
  }
  if (!x.pre.length || !y.pre.length) return x.pre.length === y.pre.length ? 0 : x.pre.length ? -1 : 1;
  for (let i = 0; i < Math.min(x.pre.length, y.pre.length); i += 1) {
    const left = x.pre[i], right = y.pre[i];
    if (left === right) continue;
    const ln = /^[0-9]+$/.test(left), rn = /^[0-9]+$/.test(right);
    if (ln && rn) return decimal(left, right);
    if (ln !== rn) return ln ? -1 : 1;
    return left < right ? -1 : 1;
  }
  return x.pre.length === y.pre.length ? 0 : x.pre.length < y.pre.length ? -1 : 1;
}

/** Describe what an upgrade run actually exercises, so same-version runs are never called upgrades. */
export function classifyUpgrade(baseline, candidate) {
  const c = compareVersions(baseline, candidate);
  if (c < 0) return { kind: "upgrade", downgradeApplicable: true };
  if (c === 0) return { kind: "reinstall_same_version", downgradeApplicable: false };
  return { kind: "baseline_newer_than_candidate", downgradeApplicable: false };
}

export function cspFromHeaders(headers) {
  for (const [name, value] of Object.entries(headers ?? {})) {
    if (name.toLowerCase() === "content-security-policy") return value;
  }
  return null;
}

/** Split a CSP into directive name -> source list. */
export function parseCsp(csp) {
  const out = {};
  for (const part of csp.split(";")) {
    const tokens = part.trim().split(/\s+/).filter(Boolean);
    if (tokens.length === 0) continue;
    out[tokens[0].toLowerCase()] = tokens.slice(1);
  }
  return out;
}

/** Records steps. `skipped` and `failed` stay distinct from `passed`. */
export class Recorder {
  constructor() { this.steps = []; }
  add(name, status, detail = {}) {
    if (!["passed", "failed", "skipped"].includes(status)) throw new Error(`Bad status ${status}`);
    const step = { name, status, ...detail };
    this.steps.push(step);
    return step;
  }
  async run(name, fn) {
    try {
      const detail = (await fn()) ?? {};
      return this.add(name, "passed", detail);
    } catch (error) {
      return this.add(name, "failed", { error: String(error?.message ?? error) });
    }
  }
  summary() {
    const count = (s) => this.steps.filter((x) => x.status === s).length;
    return { passed: count("passed"), failed: count("failed"), skipped: count("skipped"), ok: count("failed") === 0 };
  }
}

/** Registry location of the documented WebView2 AdditionalBrowserArguments policy. */
export const WEBVIEW2_POLICY_KEY = "HKLM\\SOFTWARE\\Policies\\Microsoft\\Edge\\WebView2\\AdditionalBrowserArguments";

/** Arguments for one exe name. The WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS variable is ignored by this app (spike, G2b). */
export function debugPolicyValue(port) {
  if (!Number.isInteger(port) || port < 1024 || port > 65535) throw new Error(`Bad debugging port ${port}`);
  return `--remote-debugging-port=${port} --remote-allow-origins=*`;
}

export function policyAddArgs(exeName, port) {
  return ["add", WEBVIEW2_POLICY_KEY, "/v", exeName, "/t", "REG_SZ", "/d", debugPolicyValue(port), "/f"];
}

export function policyDeleteArgs() {
  return ["delete", WEBVIEW2_POLICY_KEY, "/f"];
}

/**
 * Bounded IPC observation log. Entries carry a monotonically increasing sequence number so callers
 * can ask "was X seen after this point" even after old entries were dropped from the window.
 */
export class IpcLog {
  constructor(limit = 600) { this.limit = limit; this.entries = []; this.next = 0; }
  push(line) {
    this.entries.push({ seq: this.next, line });
    this.next += 1;
    if (this.entries.length > this.limit) this.entries.shift();
  }
  /** Sequence number the next pushed entry will get. */
  mark() { return this.next; }
  /** True when a line pushed at or after `mark` contains `needle`. */
  seenSince(mark, needle) { return this.entries.some((e) => e.seq >= mark && e.line.includes(needle)); }
  lines() { return this.entries.map((e) => e.line); }
}

const CAPTURE_TIME_KEYS = new Set(["observed_at_utc_ms", "received_at_utc_ms", "created_at_utc_ms"]);

/**
 * Re-time a synthetic ANAV1 snapshot so its capture is recent on the runner's trusted clock.
 * Only the three capture-time fields change (observed 10 s, received 5 s, created now, before
 * `nowMs`); everything else, including `fixture:true` and the synthetic labels, is kept, so the
 * planner sees a fresh, clearly synthetic capture rather than a pretended live one.
 */
export function freshenSnapshotText(text, nowMs) {
  const prefix = "ANAV1:";
  if (!text.startsWith(prefix)) throw new Error("Not an ANAV1 export");
  if (!Number.isSafeInteger(nowMs) || nowMs <= 0) throw new Error("Bad current time");
  const offsets = { observed_at_utc_ms: 10000, received_at_utc_ms: 5000, created_at_utc_ms: 0 };
  let replaced = 0;
  const walk = (value) => {
    if (Array.isArray(value)) value.forEach(walk);
    else if (value !== null && typeof value === "object") {
      for (const key of Object.keys(value)) {
        if (CAPTURE_TIME_KEYS.has(key) && typeof value[key] === "number") { value[key] = nowMs - offsets[key]; replaced += 1; }
        else walk(value[key]);
      }
    }
  };
  const doc = JSON.parse(text.slice(prefix.length));
  walk(doc);
  if (replaced === 0) throw new Error("No capture times found to freshen");
  return prefix + JSON.stringify(doc);
}

/** Migration summary from raw SQL results; an empty table is unknown (null), never version 0. */
export function migrationSummary(count, maxVersion) {
  const n = Number(count);
  return { migrations: n, max_migration_version: n > 0 && maxVersion !== null && maxVersion !== undefined ? Number(maxVersion) : null };
}

/** Request ordinary WM_CLOSE for one owned process, then await its normal exit.
 * No force-termination counts as a successful instrumented shutdown. */
export function gracefulCloseArgs(pid) {
  if (!Number.isSafeInteger(pid) || pid <= 0 || pid > 0xffffffff) throw new Error("Invalid owned process ID");
  return ["-NoLogo", "-NoProfile", "-NonInteractive", "-Command",
    `$ErrorActionPreference='Stop'; $ownedProcess=[System.Diagnostics.Process]::GetProcessById(${pid}); if(-not $ownedProcess.CloseMainWindow()){exit 2}; if(-not $ownedProcess.WaitForExit(10000)){exit 3}; exit $ownedProcess.ExitCode`];
}

/** Serialize only a bounded code, never backend paths, messages, stacks or arguments.
 * Self-contained so Playwright can serialize this function into the app page. */
export async function invokeInPage({ command, args }) {
  try {
    return { ok: true, value: await window.__TAURI_INTERNALS__.invoke(command, args) };
  } catch (error) {
    const code = typeof error?.code === "string" && /^[A-Z][A-Z0-9_]{0,63}$/.test(error.code)
      ? error.code : "UNSTRUCTURED_IPC_ERROR";
    return { ok: false, error: { code } };
  }
}

export async function invokeNative(page, command, args) {
  const result = await page.evaluate(invokeInPage, { command, args });
  if (!result.ok) throw new Error(`Native IPC ${command}: ${JSON.stringify(result.error)}`);
  return result.value;
}

/** One deadline also bounds a hung status request. No recovery paths enter receipts. */
export async function waitForDatabaseReady(readStatus, timeoutMs = 30000, intervalMs = 100) {
  if (!Number.isFinite(timeoutMs) || timeoutMs <= 0 || !Number.isFinite(intervalMs) || intervalMs <= 0) {
    throw new Error("Invalid database readiness deadline");
  }
  let timer;
  let expired = false;
  const timeout = new Promise((_, reject) => {
    timer = setTimeout(() => {
      expired = true;
      reject(new Error("Database readiness timed out"));
    }, timeoutMs);
  });
  try {
    const poll = async () => {
      while (!expired) {
        const status = await readStatus();
        if (expired) return;
        if (status?.state === "ready") return { state: "ready" };
        if (status?.state === "recovery_required") throw new Error("Database recovery required");
        if (status?.state !== "checking") throw new Error("Unknown database startup state");
        await new Promise(resolve => setTimeout(resolve, intervalMs));
      }
    };
    return await Promise.race([poll(), timeout]);
  } finally {
    expired = true;
    clearTimeout(timer);
  }
}

/** Verify the exact synthetic baseline through issued IPC, then its rendered inspection.
 * One deadline includes hung IPC and UI; absence and mismatches are never retried as success.
 */
export async function verifyPreservedSyntheticContent({ readProfiles, readActive, waitForInspection, expected, timeoutMs = 30000 }) {
  if (!Number.isFinite(timeoutMs) || timeoutMs <= 0) throw new Error("Invalid content verification deadline");
  let timer;
  const sameContext = (a, b) => a?.namespace === b?.namespace &&
    ["edition", "build", "interface"].every((key) => a?.build?.[key] === b?.build?.[key]);
  const diagnostic = { profile_found: false, active_present: false };
  try {
    return await Promise.race([
      (async () => {
        const profiles = await readProfiles();
        const matches = Array.isArray(profiles) ? profiles.filter((p) => p.id === expected.profile_id) : [];
        if (matches.length !== 1 || matches[0].fixture !== true || !sameContext(matches[0].context, expected.scope.context)) {
          throw new Error("Expected synthetic profile identity was not preserved");
        }
        diagnostic.profile_found = true;
        const active = await readActive(matches[0].id);
        diagnostic.active_present = active !== null && active !== undefined;
        if (!active || active.key?.revision !== expected.revision || active.key?.scope?.fixture !== true ||
            !sameContext(active.key.scope.context, expected.scope.context)) {
          throw new Error("Expected synthetic active revision or scope was not preserved");
        }
        await waitForInspection(expected);
        return { profile_id: expected.profile_id, revision: expected.revision, scope: expected.scope };
      })(),
      new Promise((_, reject) => { timer = setTimeout(() => reject(new Error("Synthetic content verification timed out")), timeoutMs); }),
    ]);
  } catch (error) {
    throw new Error(`${String(error?.message ?? error).slice(0, 300)}; content diagnostic=${JSON.stringify(diagnostic)}`);
  } finally { clearTimeout(timer); }
}
