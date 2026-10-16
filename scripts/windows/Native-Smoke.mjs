// Native WebView2 smoke for the installed Azeroth Navigator (task G2b).
//
// Launches the installed executable with a WebView2 remote debugging port (via the documented
// AdditionalBrowserArguments policy) and attaches
// Playwright over CDP. This observes the real UI under Tauri's actual CSP over real IPC.
// It is hosted-runner REGRESSION EVIDENCE ONLY: it cannot satisfy any WIN-* acceptance row
// (release/v1/README.md). It never touches the game, adds no socket bridge to the addon,
// and drives only the application's own window through the browser debugging protocol.
//
// Modes: smoke | populate | verify | check-download | db-summary
import { spawn, spawnSync } from "node:child_process";
import { createHash } from "node:crypto";
import { createRequire } from "node:module";
import { mkdirSync, readFileSync, writeFileSync, existsSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import {
  SCREEN_LABELS, MODES, parseArgs, isCspViolationText, isNonNativeAlert, parseSha256Sums,
  verifyAgainstSums, cspFromHeaders, parseCsp, Recorder, policyAddArgs, policyDeleteArgs,
} from "./native-smoke-lib.mjs";

const here = dirname(fileURLToPath(import.meta.url));
const repoRoot = resolve(here, "..", "..");
const OPTIONS = ["mode", "exe", "port", "out", "label", "file", "sums", "name", "db"];
const args = parseArgs(process.argv.slice(2), OPTIONS);
const mode = args.mode ?? "smoke";
const label = args.label ?? mode;
if (![...MODES, "check-download", "db-summary"].includes(mode)) throw new Error(`Unknown mode ${mode}`);

const rec = new Recorder();
const evidence = { label, mode, platform: process.platform, node: process.version, startedAt: new Date().toISOString() };

function finish() {
  evidence.finishedAt = new Date().toISOString();
  evidence.summary = rec.summary();
  evidence.steps = rec.steps;
  const text = JSON.stringify(evidence, null, 2);
  if (args.out) { mkdirSync(dirname(resolve(args.out)), { recursive: true }); writeFileSync(args.out, `${text}\n`); }
  for (const s of rec.steps) console.log(`${s.status.toUpperCase().padEnd(7)} ${s.name}${s.error ? ` :: ${s.error}` : ""}${s.reason ? ` :: ${s.reason}` : ""}`);
  console.log(`native-smoke ${label}: ${JSON.stringify(evidence.summary)} (regression evidence only; not WIN-* acceptance)`);
  process.exit(evidence.summary.ok ? 0 : 1);
}

function sha256(path) { return createHash("sha256").update(readFileSync(path)).digest("hex"); }

// ----- non-browser modes -------------------------------------------------------------------
if (mode === "check-download") {
  await rec.run(`sha256 of ${args.name} matches SHA256SUMS.txt`, async () => {
    const sums = parseSha256Sums(readFileSync(args.sums, "utf8"));
    const actual = sha256(args.file);
    verifyAgainstSums(sums, args.name, actual);
    return { sha256: actual };
  });
  finish();
}

async function dbSummary(db) {
  const { DatabaseSync } = await import("node:sqlite");
  const conn = new DatabaseSync(db, { readOnly: true });
  try {
    const m = conn.prepare("SELECT COUNT(*) AS n, MAX(version) AS v FROM schema_migrations").get();
    const tables = conn.prepare("SELECT name FROM sqlite_master WHERE type='table' ORDER BY name").all().map((r) => r.name);
    return { migrations: Number(m.n), max_migration_version: Number(m.v), tables };
  } finally { conn.close(); }
}

if (mode === "db-summary") {
  await rec.run("read schema_migrations from the app database", async () => {
    if (!existsSync(args.db)) throw new Error(`No database at ${args.db}`);
    const summary = await dbSummary(args.db);
    evidence.db = summary;
    return summary;
  });
  finish();
}

// ----- browser modes -----------------------------------------------------------------------
if (!args.exe) throw new Error("--exe is required");
const port = Number(args.port ?? 9222);
const outDir = resolve(args.out ? dirname(args.out) : join(process.cwd(), "native-smoke"));
mkdirSync(outDir, { recursive: true });
const requirePw = createRequire(join(repoRoot, "apps", "desktop", "package.json"));
const { chromium } = requirePw("@playwright/test");
const snapshotText = readFileSync(join(repoRoot, "fixtures", "transport", "anav1-character-snapshot.txt"), "utf8").trim();

const sleep = (ms) => new Promise((r) => setTimeout(r, ms));
let child = null;
let childExit = null;

// Spike result (G2b run 38949898107): WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS is ignored by this app on
// the hosted runner (Tauri sets browser arguments through the API); the documented
// Edge\WebView2\AdditionalBrowserArguments machine policy for this exe name is honoured.
// This changes only the ephemeral runner, is removed in `finally`, and never touches the game.
function setDebugPolicy() {
  if (process.platform !== "win32") return;
  const exeName = args.exe.split(/[\\/]/).pop();
  const r = spawnSync("reg", policyAddArgs(exeName, port), { encoding: "utf8" });
  if (r.status !== 0) throw new Error(`Could not write the WebView2 debugging policy (administrator required): ${r.stdout}${r.stderr}`);
  evidence.debug_policy = { key: "Edge\\WebView2\\AdditionalBrowserArguments", value_name: exeName, port };
}

function clearDebugPolicy() {
  if (process.platform === "win32") spawnSync("reg", policyDeleteArgs(), { encoding: "utf8" });
}

function launch() {
  setDebugPolicy();
  child = spawn(args.exe, [], { stdio: "ignore", windowsHide: false });
  child.on("exit", (code, signal) => { childExit = { code, signal }; });
}

function killTree() {
  if (!child || childExit) return;
  if (process.platform === "win32") spawnSync("taskkill", ["/PID", String(child.pid), "/T", "/F"], { stdio: "ignore" });
  else child.kill("SIGKILL");
}

async function waitForEndpoint(timeoutMs) {
  const deadline = Date.now() + timeoutMs;
  let last = "no response";
  while (Date.now() < deadline) {
    if (childExit) throw new Error(`Application exited before CDP attached: ${JSON.stringify(childExit)}`);
    try {
      const response = await fetch(`http://127.0.0.1:${port}/json/version`);
      if (response.ok) return await response.json();
      last = `HTTP ${response.status}`;
    } catch (error) { last = String(error?.cause?.code ?? error?.message ?? error); }
    await sleep(500);
  }
  evidence.diagnostics = diagnoseCdp();
  throw new Error(`CDP endpoint 127.0.0.1:${port} did not answer within ${timeoutMs} ms (${last})`);
}

/** Best-effort facts for a failed attach; printed and stored, never used to pass. */
function diagnoseCdp() {
  const out = {};
  const ps = (name, script) => {
    if (process.platform !== "win32") return;
    const r = spawnSync("powershell.exe", ["-NoProfile", "-NonInteractive", "-Command", script], { encoding: "utf8", timeout: 60000 });
    out[name] = `${r.stdout ?? ""}${r.stderr ?? ""}`.trim().slice(0, 12000);
    console.log(`--- diagnostic: ${name}\n${out[name]}`);
  };
  ps("processes", "Get-CimInstance Win32_Process | Where-Object { $_.Name -match 'azeroth|msedgewebview2' } | Select-Object ProcessId,ParentProcessId,Name,CommandLine | Format-List | Out-String -Width 400");
  ps("listening_tcp", "Get-NetTCPConnection -State Listen -ErrorAction SilentlyContinue | Select-Object LocalAddress,LocalPort,OwningProcess | Sort-Object LocalPort | Format-Table -AutoSize | Out-String -Width 200");
  ps("webview_folders", "Get-ChildItem -Recurse -Depth 2 -Force -ErrorAction SilentlyContinue (Join-Path $env:LOCALAPPDATA 'io.github.laymer.azeroth-navigator') | Select-Object -First 80 FullName,Length | Format-Table -AutoSize | Out-String -Width 300; Get-ChildItem -Recurse -Filter DevToolsActivePort -Force -ErrorAction SilentlyContinue $env:LOCALAPPDATA | ForEach-Object { $_.FullName; Get-Content $_.FullName }");
  ps("webview_runtime", "Get-ItemProperty 'HKLM:\\SOFTWARE\\WOW6432Node\\Microsoft\\EdgeUpdate\\Clients\\*' -ErrorAction SilentlyContinue | Where-Object { $_.name -match 'WebView2' } | Select-Object name,pv | Format-List | Out-String");
  ps("env", "[Environment]::GetEnvironmentVariable('WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS'); Get-Process -Name azeroth-desktop -ErrorAction SilentlyContinue | Select-Object Id,Responding,MainWindowHandle,MainWindowTitle,SessionId | Format-List | Out-String");
  return out;
}

async function findMainPage(browser, timeoutMs) {
  const deadline = Date.now() + timeoutMs;
  while (Date.now() < deadline) {
    for (const context of browser.contexts()) {
      for (const page of context.pages()) {
        const url = page.url();
        if (url && url !== "about:blank" && !url.includes("#reference")) return { context, page };
      }
    }
    await sleep(500);
  }
  throw new Error("No application page appeared over CDP");
}

const violations = [];
const consoleErrors = [];
const pageErrors = [];

async function settle(page) {
  await page.waitForLoadState("load");
  await page.getByRole("status").filter({ hasText: /^Loading/ }).first().waitFor({ state: "detached", timeout: 15000 }).catch(() => {});
  await sleep(600);
}

async function alertTexts(page) {
  return (await page.getByRole("alert").allInnerTexts()).map((t) => t.trim()).filter(Boolean);
}

async function go(page, name) {
  await page.getByRole("navigation", { name: "Primary" }).getByRole("button", { name, exact: true }).click();
  await settle(page);
}

async function importSnapshot(page) {
  await go(page, "Imports");
  await page.getByLabel("Snapshot export text").fill(snapshotText);
  await page.getByRole("button", { name: "Import text", exact: true }).click();
  await page.getByText("Stored as the current snapshot.").waitFor({ timeout: 30000 });
}

async function stageAndActivate(page) {
  await go(page, "Content");
  await page.getByRole("button", { name: "Load synthetic planning example" }).click();
  await page.getByRole("button", { name: "Validate and stage content" }).click();
  await page.getByRole("button", { name: "Activate staged revision" }).waitFor({ timeout: 30000 });
  await page.getByRole("button", { name: "Activate staged revision" }).click();
  await page.getByText(/Content activated/).waitFor({ timeout: 30000 });
}

async function run() {
  launch();
  const version = await waitForEndpoint(90000);
  evidence.cdp = { browser: version.Browser, protocol: version["Protocol-Version"], attach: "WebView2 AdditionalBrowserArguments machine policy (remote-debugging-port) + playwright connectOverCDP" };
  rec.add("attach over CDP (WebView2 remote debugging)", "passed", evidence.cdp);
  const browser = await chromium.connectOverCDP(`http://127.0.0.1:${port}`);
  try {
    const { context, page } = await findMainPage(browser, 60000);
    await context.addInitScript(() => {
      window.__anavCsp = window.__anavCsp ?? [];
      document.addEventListener("securitypolicyviolation", (e) => window.__anavCsp.push(`${e.violatedDirective} blocked ${e.blockedURI}`));
    });
    page.on("console", (m) => {
      const text = m.text();
      if (isCspViolationText(text)) violations.push(`console: ${text}`);
      else if (m.type() === "error") consoleErrors.push(text);
    });
    page.on("pageerror", (e) => pageErrors.push(String(e?.message ?? e)));

    // Reload with listeners in place; the response headers carry Tauri's rewritten CSP.
    const response = await page.reload({ waitUntil: "load" });
    await settle(page);
    evidence.url = page.url();

    if (mode === "smoke") await smoke(page, response);
    else if (mode === "populate") await populate(page);
    else await verify(page);

    const inPage = await page.evaluate(() => window.__anavCsp ?? []);
    for (const v of inPage) violations.push(`securitypolicyviolation: ${v}`);
    await rec.run("no CSP violations, uncaught page errors or PROVIDER_UNAVAILABLE alerts were observed", async () => {
      evidence.consoleErrors = consoleErrors;
      if (violations.length > 0) throw new Error(`CSP violations: ${violations.join(" | ")}`);
      if (pageErrors.length > 0) throw new Error(`Page errors: ${pageErrors.join(" | ")}`);
      return { csp_violations: 0, page_errors: 0, console_errors_recorded: consoleErrors.length };
    });
  } finally {
    await browser.close().catch(() => {});
  }
}

async function smoke(page, response) {
  await rec.run("actual Tauri CSP header is present and has no data: or remote sources", async () => {
    const csp = cspFromHeaders(response ? response.headers() : {});
    const meta = await page.evaluate(() => document.querySelector('meta[http-equiv="Content-Security-Policy"]')?.getAttribute("content") ?? null);
    evidence.csp = { header: csp, meta };
    const effective = csp ?? meta;
    if (!effective) throw new Error("Neither a CSP response header nor a CSP meta tag was observable");
    const parsed = parseCsp(effective);
    const sources = Object.values(parsed).flat();
    const bad = sources.filter((s) => s === "data:" || /^https?:\/\/(?!ipc\.localhost|atlas\.localhost|tauri\.localhost)/.test(s) || s === "*");
    if (bad.length > 0) throw new Error(`Unexpected CSP sources: ${bad.join(" ")}`);
    return { directives: Object.keys(parsed), observed_in: csp ? "response header" : "meta tag only" };
  });

  await rec.run("real native IPC: the renderer has Tauri internals and shows no demo label", async () => {
    const internals = await page.evaluate(() => typeof window.__TAURI_INTERNALS__?.invoke);
    if (internals !== "function") throw new Error(`__TAURI_INTERNALS__.invoke is ${internals}`);
    const demo = await page.getByText(/^DEMO — /).count();
    if (demo !== 0) throw new Error(`${demo} DEMO labels visible; the page is not using the native adapter`);
    const h1 = await page.locator("h1").allInnerTexts();
    if (h1.length !== 1 || h1[0].trim() !== "Azeroth Navigator") throw new Error(`Unexpected h1: ${JSON.stringify(h1)}`);
    return { tauri_invoke: internals };
  });

  await rec.run("exactly 14 primary navigation buttons, and no profiles yet on an empty database", async () => {
    const nav = page.getByRole("navigation", { name: "Primary" }).getByRole("button");
    const names = (await nav.allInnerTexts()).map((t) => t.trim());
    if (JSON.stringify(names.slice().sort()) !== JSON.stringify(SCREEN_LABELS.slice().sort())) throw new Error(`Navigation buttons: ${JSON.stringify(names)}`);
    await go(page, "Imports");
    await page.getByText("No stored profiles yet.").waitFor({ timeout: 15000 });
    return { buttons: names.length };
  });

  const screens = [];
  evidence.screens = screens;
  for (const name of SCREEN_LABELS) {
    await rec.run(`screen "${name}" renders over real IPC (empty database)`, async () => {
      await go(page, name);
      const current = await page.getByRole("navigation", { name: "Primary" }).locator('[aria-current="page"]').allInnerTexts();
      if (current.length !== 1 || current[0].trim() !== name) throw new Error(`aria-current is ${JSON.stringify(current)}`);
      const alerts = await alertTexts(page);
      const headings = (await page.locator("main h2").allInnerTexts()).map((t) => t.trim());
      await page.screenshot({ path: join(outDir, `screen-${name.replace(/[^A-Za-z0-9]+/g, "-")}.png`) }).catch(() => {});
      screens.push({ screen: name, alerts, headings });
      const nonNative = alerts.filter(isNonNativeAlert);
      if (nonNative.length > 0) throw new Error(`Non-native alert: ${nonNative.join(" | ")}`);
      if ((await page.locator("h1").count()) !== 1) throw new Error("h1 missing after navigation");
      // Alerts are recorded verbatim, not assumed absent: an empty database may legitimately report some.
      return { alerts, headings };
    });
  }

  await rec.run("synthetic ANAV1 snapshot (fixtures/transport) imports and becomes current", async () => {
    await importSnapshot(page);
    await page.getByText(/SYNTHETIC — fixture/).first().waitFor({ timeout: 15000 });
    const alerts = await alertTexts(page);
    if (alerts.length > 0) throw new Error(`Alerts after import: ${alerts.join(" | ")}`);
    return { bytes: snapshotText.length };
  });

  await rec.run("synthetic planning content stages and activates", async () => {
    await stageAndActivate(page);
    return {};
  });

  await rec.run("plan, inspect route and export produce ANAV1:", async () => {
    await go(page, "Planner");
    await page.getByRole("combobox", { name: "Origin", exact: true }).selectOption("demo-node-1");
    await page.getByRole("combobox", { name: "Stopping node", exact: true }).selectOption("demo-node-1");
    await page.getByLabel("Consider objective").check();
    await page.getByRole("combobox", { name: "Objective node", exact: true }).selectOption("demo-node-2");
    await page.getByLabel("utility", { exact: true }).fill("10");
    await page.getByLabel("completion_ms", { exact: true }).fill("1000");
    await page.getByRole("button", { name: "Plan session", exact: true }).click();
    const result = page.getByRole("region", { name: "Planning result" });
    await result.waitFor({ timeout: 60000 });
    const resultText = (await result.innerText()).trim();
    await page.getByRole("button", { name: "Inspect route", exact: true }).click();
    await page.getByRole("region", { name: "Route inspection", exact: true }).waitFor({ timeout: 30000 });
    await page.getByRole("button", { name: "Export plan", exact: true }).click();
    const text = await page.getByLabel("Plan export text").inputValue();
    if (!text.startsWith("ANAV1:")) throw new Error(`Export does not start with ANAV1: ${text.slice(0, 20)}`);
    return { planning_result: resultText.slice(0, 300), export_bytes: text.length, alerts: await alertTexts(page) };
  });

  await rec.run("reference window state query works and reports the window closed", async () => {
    await go(page, "Today");
    await page.getByText("Optional reference window", { exact: true }).click();
    await page.getByRole("button", { name: "Check reference window state", exact: true }).click();
    const observed = page.getByText(/^Last observed window:/);
    await observed.waitFor({ timeout: 15000 });
    const text = (await observed.innerText()).trim();
    if ((await page.getByText(/Reference window unavailable/).count()) !== 0) throw new Error("Reference window state query reported unavailable");
    return { observed: text };
  });
}

async function populate(page) {
  await rec.run("populate: import synthetic snapshot", () => importSnapshot(page).then(() => ({})));
  await rec.run("populate: stage and activate synthetic planning content", () => stageAndActivate(page).then(() => ({})));
}

async function verify(page) {
  await rec.run("verify: imported profile survived and is selectable", async () => {
    await go(page, "Imports");
    if ((await page.getByText("No stored profiles yet.").count()) !== 0) throw new Error("Imports reports no stored profiles; data was lost");
    const options = await page.getByLabel("Selected profile").locator("option").allInnerTexts();
    if (options.length < 1) throw new Error("Selected profile list is empty");
    return { profiles: options };
  });
  await rec.run("verify: active content revision survived", async () => {
    await go(page, "Content");
    if ((await page.getByText("No active content for this profile.").count()) !== 0) throw new Error("Active content was lost");
    return { alerts: await alertTexts(page) };
  });
  await rec.run("verify: Today shows the imported synthetic character", async () => {
    await go(page, "Today");
    await page.getByText(/SYNTHETIC — fixture/).first().waitFor({ timeout: 15000 }).catch(() => { throw new Error("Imported synthetic snapshot is not shown"); });
    return {};
  });
}

try {
  await run();
} catch (error) {
  rec.add("native smoke run", "failed", { error: String(error?.stack ?? error) });
} finally {
  killTree();
  clearDebugPolicy();
  await sleep(1500);
  if (args.db && existsSync(args.db)) {
    await rec.run("database schema_migrations summary after run", async () => {
      const summary = await dbSummary(args.db);
      evidence.db = summary;
      return summary;
    });
  }
}
finish();
