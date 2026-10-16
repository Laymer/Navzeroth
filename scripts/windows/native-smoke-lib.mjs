// Pure helpers for Native-Smoke.mjs. No I/O so they can be tested on Linux.
// Everything built on these is regression evidence only and never WIN-* acceptance.

export const SCREEN_LABELS = [
  "Today", "Settings", "Compatibility and coverage", "Local diagnostics",
  "Backup and recovery", "Atlas", "Imports", "Party", "Dungeons", "Content",
  "Planner", "Gear", "Sessions", "Reminders",
];

export const MODES = ["smoke", "populate", "verify"];

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

/** Numeric semver triple comparison; prerelease suffixes are rejected, not ignored. */
export function compareVersions(a, b) {
  const parse = (v) => {
    const m = /^(\d+)\.(\d+)\.(\d+)$/.exec(v);
    if (!m) throw new Error(`Not a plain MAJOR.MINOR.PATCH version: ${v}`);
    return m.slice(1).map(Number);
  };
  const [x, y] = [parse(a), parse(b)];
  for (let i = 0; i < 3; i += 1) if (x[i] !== y[i]) return x[i] < y[i] ? -1 : 1;
  return 0;
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
