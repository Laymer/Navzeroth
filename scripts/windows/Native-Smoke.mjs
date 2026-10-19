// Native WebView2 smoke for the installed Azeroth Navigator (task G2b).
//
// Launches the installed executable with a WebView2 remote debugging port (via the documented
// AdditionalBrowserArguments policy) and attaches
// Playwright over CDP. This observes the real UI under Tauri's actual CSP over real IPC.
// It is hosted-runner REGRESSION EVIDENCE ONLY: it cannot satisfy any WIN-* acceptance row
// (release/v1/README.md). It never touches the game, adds no socket bridge to the addon,
// and drives only the application's own window through the browser debugging protocol.
//
// Modes: smoke | populate | verify | capture | check-download | db-summary
import { spawn, spawnSync } from "node:child_process";
import { createHash } from "node:crypto";
import { createRequire } from "node:module";
import { mkdirSync, readFileSync, writeFileSync, existsSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { COMMAND_BOUNDARIES, validateRefusal } from "./native-command-boundaries.mjs";
import {
  SCREEN_LABELS, MODES, parseArgs, isCspViolationText, isNonNativeAlert, parseSha256Sums,
  verifyAgainstSums, cspFromHeaders, parseCsp, Recorder, policyAddArgs, policyDeleteArgs,
  IpcLog, freshenSnapshotText, migrationSummary, gracefulCloseArgs,
  invokeNative, waitForDatabaseReady,
} from "./native-smoke-lib.mjs";

const here = dirname(fileURLToPath(import.meta.url));
const repoRoot = resolve(here, "..", "..");
const OPTIONS = ["mode", "exe", "port", "out", "label", "file", "sums", "name", "db", "shutdown", "release-commit", "release-tag", "installer-sha256"];
const args = parseArgs(process.argv.slice(2), OPTIONS);
const mode = args.mode ?? "smoke";
if (args.shutdown !== undefined && args.shutdown !== "graceful") throw new Error("Unknown shutdown mode");
const label = args.label ?? mode;
if (![...MODES, "check-download", "db-summary"].includes(mode)) throw new Error(`Unknown mode ${mode}`);

if (mode === "capture" && (process.env.GITHUB_ACTIONS !== "true" || process.env.RUNNER_ENVIRONMENT !== "github-hosted" ||
    args["release-tag"] !== "v1.0.0-rc.1" || !/^[a-f0-9]{40}$/.test(args["release-commit"] ?? "") || !/^[a-f0-9]{64}$/.test(args["installer-sha256"] ?? ""))) {
  throw new Error("Release capture requires a bound public RC on a disposable hosted runner");
}
const rec = new Recorder();
const evidence = { label, mode, platform: process.platform, node: process.version, startedAt: new Date().toISOString() };

function finish() {
  evidence.finishedAt = new Date().toISOString();
  evidence.summary = rec.summary();
  evidence.steps = rec.steps;
  if (mode === "capture" && evidence.summary.ok && evidence.capture) {
    writeFileSync(join(dirname(resolve(args.out)), "provenance.json"), JSON.stringify(evidence.capture, null, 2) + "\n");
  }
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
    const summary = migrationSummary(m.n, m.v);
    if (summary.max_migration_version === null) throw new Error("schema_migrations is empty: the migration version is unknown");
    return { ...summary, tables };
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
const fixtureText = readFileSync(join(repoRoot, "fixtures", "transport", "anav1-character-snapshot.txt"), "utf8").trim();
// The committed fixture has fixed capture times, so the backend correctly calls it stale and refuses to
// plan it. The driver re-times only the three capture-time fields to the runner's clock (still
// fixture:true, still labelled synthetic) so planning, route inspection and export run natively.
const snapshotRetimedAt = Date.now();
const snapshotText = freshenSnapshotText(fixtureText, snapshotRetimedAt);

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

async function gracefulShutdown() {
  if (process.platform !== "win32" || !child || childExit) throw new Error("Instrumented shutdown requires the running owned Windows process");
  const requestedPid = child.pid;
  const outcome = spawnSync("powershell.exe", gracefulCloseArgs(requestedPid), { encoding: "utf8", timeout: 15000, windowsHide: true });
  if (outcome.error || outcome.status !== 0) throw new Error(`Graceful application close failed (${outcome.status ?? "timeout"}); force cleanup is not coverage evidence`);
  const deadline = Date.now() + 2000;
  while (!childExit && Date.now() < deadline) await sleep(50);
  if (!childExit || childExit.code !== 0 || childExit.signal !== null) throw new Error("Application did not confirm a successful normal exit");
  return { pid: requestedPid, exit_code: childExit.code, normal_exit: true, force_termination: false };
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

const ipcLog = new IpcLog(600);
const nonNativeAlerts = [];
/** Observes native IPC from outside the page (http://ipc.localhost/<command>); the page is never patched. */
function watchIpc(page) {
  const t0 = Date.now();
  const commandOf = (url) => { try { return new URL(url).pathname.replace(/^\//, ""); } catch { return url; } };
  const push = (line) => ipcLog.push(line);
  page.on("request", (request) => { if (request.url().includes("ipc.localhost")) push(`${Date.now() - t0}ms -> ${commandOf(request.url())} (${request.method()})`); });
  page.on("response", async (response) => {
    if (!response.url().includes("ipc.localhost")) return;
    const command = commandOf(response.url());
    let body = "";
    // Bodies only for the planning job protocol (synthetic fixture data); never for imports or settings.
    if (/^(start_planning_job|get_planning_job|take_planning_job_result|cancel_planning_job)$/.test(command)) {
      body = await response.text().then((t) => ` body=${t.slice(0, 240)}`).catch(() => " body=<unreadable>");
    }
    push(`${Date.now() - t0}ms <- ${command} HTTP ${response.status()}${body}`);
  });
  page.on("requestfailed", (request) => { if (request.url().includes("ipc.localhost")) push(`${Date.now() - t0}ms !! ${commandOf(request.url())} ${request.failure()?.errorText ?? ""}`); });
}

async function settle(page) {
  await page.waitForLoadState("load");
  await page.getByRole("status").filter({ hasText: /^Loading/ }).first().waitFor({ state: "detached", timeout: 15000 }).catch(() => {});
  await sleep(600);
}

async function alertTexts(page) {
  const texts = (await page.getByRole("alert").allInnerTexts()).map((t) => t.trim()).filter(Boolean);
  for (const t of texts) if (isNonNativeAlert(t) && !nonNativeAlerts.includes(t)) nonNativeAlerts.push(t);
  return texts;
}

/** Run fn; on failure attach the visible alerts, status text, main text and a screenshot to the error. */
async function withDiagnostics(page, label, fn) {
  try { return await fn(); } catch (error) {
    let extra = "";
    let extra0 = "";
    try {
      const alerts = await alertTexts(page);
      const statuses = (await page.getByRole("status").allInnerTexts()).map((t) => t.trim()).filter(Boolean);
      const main = (await page.locator("main").innerText()).replace(/\s+/g, " ").slice(0, 2500);
      const controls = await page.evaluate(() => [...document.querySelectorAll("main select, main input, main button")].slice(0, 400)
        .filter((e) => e.matches("select, input") || /^(Plan session|Inspect route|Export plan)$/.test(e.textContent.trim()))
        .map((e) => `${e.tagName}:${(e.labels?.[0]?.textContent ?? e.textContent).trim().slice(0, 40)}=${e.value ?? ""}${e.disabled ? " [disabled]" : ""}`));
      const ipc = ipcLog.lines().filter((l) => !l.includes("get_snapshot_watch")).slice(-80);
      extra0 = `\ncontrols=${JSON.stringify(controls)}\nipc_tail=${JSON.stringify(ipc)}`;
      await page.screenshot({ path: join(outDir, `failure-${label}.png`) }).catch(() => {});
      extra = `${extra0}\nalerts=${JSON.stringify(alerts)}\nstatuses=${JSON.stringify(statuses)}\nmain=${main}`;
    } catch (inner) { extra = `\n(diagnostics unavailable: ${inner?.message})`; }
    throw new Error(`${error?.message ?? error}${extra}`);
  }
}

/** Click a control that may sit at the window edge; if the app issues no IPC call, fall back to keyboard activation. */
async function activate(locator, ipcName) {
  await locator.scrollIntoViewIfNeeded();
  // Sequence marks, not array indices: the log is a bounded window and old entries are dropped.
  const seen = () => ipcLog.seenSince(before, ` ${ipcName} `);
  let before = ipcLog.mark();
  await locator.click();
  for (let i = 0; i < 20 && !seen(); i += 1) await sleep(250);
  if (seen()) return "click";
  // Fixes review finding 1: restart the observation window with a sequence mark (not an array length).
  before = ipcLog.mark();
  await locator.focus();
  await locator.press("Enter");
  for (let i = 0; i < 20 && !seen(); i += 1) await sleep(250);
  if (seen()) return "focus+Enter fallback (the click issued no IPC call)";
  throw new Error(`Neither click nor Enter on the control issued ${ipcName}`);
}

async function go(page, name) {
  await page.getByRole("navigation", { name: "Primary" }).getByRole("button", { name, exact: true }).click();
  await settle(page);
}

async function importSnapshot(page) {
  await go(page, "Imports");
  await page.getByLabel("Snapshot export text", { exact: true }).fill(snapshotText);
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
      if (m.type() === "warning") consoleErrors.push(`warning: ${text}`);
      if (isCspViolationText(text)) violations.push(`console: ${text}`);
      else if (m.type() === "error") consoleErrors.push(text);
    });
    watchIpc(page);
    page.on("pageerror", (e) => pageErrors.push(String(e?.message ?? e)));

    // Reload with listeners in place; the response headers carry Tauri's rewritten CSP.
    const response = await page.reload({ waitUntil: "load" });
    await settle(page);
    evidence.url = page.url();

    const startup = await rec.run("native database becomes ready within deadline", () =>
      waitForDatabaseReady(() => invokeNative(page, "get_database_status")));
    if (startup.status !== "passed") return;

    if (mode === "capture") {
      await smoke(page, response);
      if (!rec.summary().ok) throw new Error("Native synthetic workflow failed before capture");
      await captureRelease(page);
    }
    else if (mode === "smoke") await smoke(page, response);
    else if (mode === "populate") await populate(page);
    else await verify(page);

    evidence.ipc_tail = ipcLog.lines().filter((l) => !l.includes("get_snapshot_watch")).slice(-300);
    const inPage = await page.evaluate(() => window.__anavCsp ?? []);
    for (const v of inPage) violations.push(`securitypolicyviolation: ${v}`);
    await alertTexts(page).catch(() => []);
    await rec.run("no CSP violations, uncaught page errors or PROVIDER_UNAVAILABLE alerts were observed", async () => {
      evidence.consoleErrors = consoleErrors;
      if (nonNativeAlerts.length > 0) throw new Error(`Non-native alerts observed: ${nonNativeAlerts.join(" | ")}`);
      if (violations.length > 0) throw new Error(`CSP violations: ${violations.join(" | ")}`);
      if (pageErrors.length > 0) throw new Error(`Page errors: ${pageErrors.join(" | ")}`);
      return { csp_violations: 0, page_errors: 0, non_native_alerts: 0, console_errors_recorded: consoleErrors.length };
    });
  } finally {
    await browser.close().catch(() => {});
  }
}

async function smoke(page, response) {
  await rec.run("invalid native requests preserve structured refusals and current snapshot", async () => {
    const before = await invokeNative(page, "get_reference_view");
    for (const test of COMMAND_BOUNDARIES) {
      const outcome = await page.evaluate(async ({command,args}) => {
        try { await window.__TAURI_INTERNALS__.invoke(command,args); return {ok:true}; }
        catch(error) { return {ok:false,error}; }
      },test);
      validateRefusal(test,outcome);
    }
    const after = await invokeNative(page, "get_reference_view");
    if (JSON.stringify(before) !== JSON.stringify(after)) throw new Error("Invalid requests changed the current imported reference");
    return { requests: COMMAND_BOUNDARIES.length, current_reference_unchanged: true };
  });
  for (const [command, commandArgs] of [["select_local_installation", {}], ["select_local_material", {kind:"snapshot"}], ["select_local_material", {kind:"authored_pack"}]]) {
    await rec.run(`native ${command} cancellation preserves selections`, async () => {
      const selections = () => Promise.all(
        ["get_local_workflow", "list_local_installations", "list_local_materials"].map(command => invokeNative(page, command)),
      );
      const before = await selections();
      await page.evaluate(({command,args}) => {
        window.__navDialogOutcome = null;
        window.__TAURI_INTERNALS__.invoke(command,args).then(
          value => { window.__navDialogOutcome = {ok:true,value}; },
          error => { window.__navDialogOutcome = {ok:false,error: {
            code: typeof error?.code === "string" && /^[A-Z][A-Z0-9_]{0,63}$/.test(error.code)
              ? error.code : "UNSTRUCTURED_IPC_ERROR",
          }}; },
        );
      }, {command,args:commandArgs});
      const cancelled = spawnSync("powershell.exe", ["-NoLogo", "-NoProfile", "-NonInteractive", "-File",
        join(repoRoot,"scripts/windows/Cancel-OwnedDialog.ps1"), "-TargetProcessId", String(child.pid),
        "-ExpectedExecutable", resolve(args.exe), "-TimeoutSeconds", "20"], {encoding:"utf8",timeout:30000});
      if (cancelled.status !== 0) throw new Error(`Owned dialog cancellation failed: ${cancelled.stdout ?? ""}${cancelled.stderr ?? ""}`);
      await page.waitForFunction(() => window.__navDialogOutcome !== null, undefined, {timeout:10000});
      const outcome = await page.evaluate(() => {
        const outcome = window.__navDialogOutcome;
        delete window.__navDialogOutcome;
        return outcome;
      });
      if (!outcome.ok || outcome.value !== null) throw new Error(`Dialog cancellation did not return null: ${JSON.stringify(outcome)}`);
      const after = await selections();
      if (JSON.stringify(before) !== JSON.stringify(after)) throw new Error("Cancelled picker changed persisted workflow choices or selections");
      return {cancelled:true,workflow_and_selections_unchanged:true};
    });
  }
  await rec.run("registered atlas protocol refuses unbound assets in all namespaces", async () => {
    const observed=[];
    for (const path of ["/research/missing-owned-fixture", "/stored/missing-owned-fixture", "/missing-owned-fixture"]) {
      const url=`http://atlas.localhost${path}`;
      const response = page.waitForResponse(reply => reply.url()===url, {timeout:15000});
      const imageOutcome = page.evaluate(url => new Promise(resolve => {
        const image=new Image();
        const timer=setTimeout(() => { image.src=""; resolve("timeout"); },10000);
        image.onload=() => {clearTimeout(timer); resolve("loaded");};
        image.onerror=() => {clearTimeout(timer); resolve("refused");};
        image.src=url;
      }),url);
      const [outcome,reply]=await Promise.all([imageOutcome,response]);
      if (outcome!=="refused" || reply.status()!==404 || reply.headers()["cache-control"]!=="no-store") throw new Error(`Unbound native asset was not a noncacheable refusal: ${url} ${outcome} ${reply.status()}`);
      observed.push({path,status:reply.status()});
    }
    return {observed};
  });
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
    await nav.nth(SCREEN_LABELS.length - 1).waitFor({ timeout: 20000 }).catch(async () => {
      throw new Error(`Navigation did not reach 14 buttons; alerts=${JSON.stringify(await alertTexts(page))}`);
    });
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

  await rec.run("synthetic planning content stages and activates", () => withDiagnostics(page, "stage", async () => {
    await stageAndActivate(page);
    return {};
  }));

  let planned = false;
  let planRefusal = null;
  await rec.run("plan session over real IPC plans the re-timed synthetic snapshot", () => withDiagnostics(page, "plan", async () => {
    // The app requeries imported views every 60 s by design and that discards an in-flight planner
    // result. Reload so the planner steps start right after a fresh interval (state is in the database).
    await page.reload({ waitUntil: "load" });
    await settle(page);
    await go(page, "Planner");
    await page.getByRole("combobox", { name: "Origin", exact: true }).selectOption("demo-node-1");
    await page.getByRole("combobox", { name: "Stopping node", exact: true }).selectOption("demo-node-1");
    await page.getByLabel("Consider objective").check();
    await page.getByRole("combobox", { name: "Objective node", exact: true }).selectOption("demo-node-2");
    await page.getByLabel("utility", { exact: true }).fill("10");
    await page.getByLabel("completion_ms", { exact: true }).fill("1000");
    const activation = await activate(page.getByRole("button", { name: "Plan session", exact: true }), "start_planning_job");
    const result = page.getByRole("region", { name: "Planning result" });
    const deadline = Date.now() + 40000;
    while (Date.now() < deadline) {
      if ((await result.count()) > 0) {
        planned = true;
        return { activation, outcome: "planned", planning_result: (await result.innerText()).trim().slice(0, 300) };
      }
      const alerts = await alertTexts(page);
      if (alerts.length > 0) {
        // The snapshot was re-timed to the runner clock, so a capture-age refusal is a failure, not a pass.
        if (alerts.every((a) => /fresh capture with trusted age/.test(a))) {
          planRefusal = alerts;
          throw new Error(`Backend refused the freshly re-timed synthetic snapshot: ${JSON.stringify(alerts)}`);
        }
        throw new Error(`Unexpected planning alert: ${JSON.stringify(alerts)}`);
      }
      await sleep(250);
    }
    throw new Error("Planning produced neither a result nor an alert within 40 s");
  }));

  if (planned) {
    await rec.run("inspect route and export plan produce ANAV1:", () => withDiagnostics(page, "export", async () => {
      await activate(page.getByRole("button", { name: "Inspect route", exact: true }), "inspect_plan_route");
      await page.getByRole("region", { name: "Route inspection", exact: true }).waitFor({ timeout: 30000 });
      await activate(page.getByRole("button", { name: "Export plan", exact: true }), "export_plan");
      const text = await page.getByLabel("Plan export text").inputValue();
      if (!text.startsWith("ANAV1:")) throw new Error(`Export does not start with ANAV1: ${text.slice(0, 20)}`);
      return { export_bytes: text.length, alerts: await alertTexts(page) };
    }));
  } else if (planRefusal !== null) {
    rec.add("inspect route and export plan produce ANAV1:", "skipped", {
      reason: "Not run: planning did not produce a result.",
      backend_alerts: planRefusal,
    });
  }

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

async function captureRelease(page) {
  await rec.run("capture populated release application with synthetic provenance", async () => {
    await page.setViewportSize({ width: 1280, height: 900 });
    await go(page, "Today");
    const later = page.getByRole("button", { name: "Choose later", exact: true });
    if (await later.isVisible()) await later.click();
    const reference = page.locator("details").filter({ has: page.locator("summary").filter({ hasText: /^Optional reference window$/ }) });
    if (await reference.getAttribute("open") !== null) await reference.locator("summary").click();
    const screenshots = [];
    for (const name of ["Today", "Atlas", "Content"]) {
      await go(page, name);
      await page.evaluate(() => document.fonts.ready);
      await page.evaluate(() => scrollTo(0, 0));
      if (name === "Today") await page.getByText(/SYNTHETIC — fixture/).first().waitFor({ timeout: 15000 });
      const file = `capture-${name}.png`;
      const bytes = await page.screenshot({ path: join(outDir, file) });
      screenshots.push({ file, sha256: createHash("sha256").update(bytes).digest("hex"), width: bytes.readUInt32BE(16), height: bytes.readUInt32BE(20), captured_at: new Date().toISOString(), screen: name });
    }
    evidence.capture = {
      schema_version: 1, synthetic: true, release_tag: args["release-tag"], source_commit: args["release-commit"],
      installer_sha256: args["installer-sha256"], application_sha256: sha256(args.exe),
      workflow_run_id: process.env.GITHUB_RUN_ID, workflow_run_attempt: process.env.GITHUB_RUN_ATTEMPT,
      capture_driver_commit: process.env.GITHUB_SHA,
      fixture_path: "fixtures/transport/anav1-character-snapshot.txt",
      fixture_sha256: sha256(join(repoRoot, "fixtures/transport/anav1-character-snapshot.txt")),
      imported_snapshot_file: "imported-synthetic-snapshot.txt", retimed_at_utc_ms: snapshotRetimedAt,
      imported_snapshot_sha256: createHash("sha256").update(snapshotText).digest("hex"),
      retiming: "Only observed_at_utc_ms, received_at_utc_ms and created_at_utc_ms re-timed to capture-run clock; fixture remains true",
      content: "Built-in Load synthetic planning example, validated and activated through real IPC",
      presentation: "Chose later in onboarding, collapsed optional reference controls and scrolled each view to the top through ordinary UI actions; no content or styles replaced",
      screenshots, cdp: evidence.cdp, evidence_scope: "Actual verified release app; synthetic fixtures; hosted regression, not live acceptance",
    };
    writeFileSync(join(outDir, "imported-synthetic-snapshot.txt"), snapshotText);
    return { screenshots: screenshots.length };
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
    const options = await page.getByLabel("Selected profile", { exact: true }).locator("option").allInnerTexts();
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
  if (args.shutdown === "graceful") await rec.run("ordinary application close flushes instrumented profiles", gracefulShutdown);
  // A failed graceful close remains a failed smoke result; kill only for cleanup.
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
