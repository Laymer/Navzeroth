import { expect, test } from "@playwright/test";
import fixture from "../../../fixtures/desktop/planning/synthetic-revision.json" with { type: "json" };
test("planning progress and cancellation keep navigation available and preserve only current advice", async ({ page }) => {
  await page.addInitScript(({ fixture }) => {
    let mode = "success"; let serial = 0;
    const requests = new Map<string, unknown>(); const calls: { command: string; args: Record<string, unknown> }[] = [];
    Object.defineProperty(window, "__PLANNING_CALLS__", { value: calls });
    Object.defineProperty(window, "__PLANNING_MODE__", { value: (value: string) => { mode = value; } });
    Object.defineProperty(window, "__TAURI_INTERNALS__", { value: { invoke: async (command: string, args: Record<string, unknown>) => {
      calls.push({ command, args });
      switch (command) {
        case "get_app_status": return { version: "test", data_mode: "stored", active_profile: null, capabilities: {} };
        case "list_profiles": return [{ id: "p", display_name: "Synthetic" }];
        case "get_planning_catalog": return { snapshot_id: "snapshot-p", revision: "r1", fixture: true, nodes: fixture.envelope.payload.nodes, objectives: [] };
        case "get_snapshot_watch": return { active: false, status: "cancelled", last_import: null, error: null };
        case "start_planning_job": {
          if (args.kind !== "solo") throw new Error("Wrong mock kind");const job_id = `job-${++serial}`;requests.set(job_id, JSON.parse(String(args.text)));
          if (mode === "deferred") return new Promise((resolve) => { Object.defineProperty(window, "__COMPLETE_REGISTRATION__", { configurable: true, value: () => resolve({ job_schema_version: 1, job_id }) }); });
          return { job_schema_version: 1, job_id };
        }
        case "get_planning_job": {
          const state = mode === "success" ? "succeeded" : mode === "stale" ? "failed" : mode === "queued" ? "queued" : "running";
          return { job_schema_version: 1, job_id: args.job_id, kind: "solo", state, cancel_requested: false, phase: state === "queued" ? null : "route_expansion", work_units: state === "queued" ? 0 : 13, result_available: state === "succeeded", error: state === "failed" ? { code: "SNAPSHOT_OUTDATED", message: "Imported snapshot changed", retryable: false } : null };
        }
        case "take_planning_job_result": return { job_schema_version: 1, job_id: args.job_id, kind: "solo", report_json: JSON.stringify({ request: requests.get(String(args.job_id)), result: { selected: [], stopping: { total_ms: 0 }, expected_total_ms: 0 }, assumptions: ["Previous native-command mock estimate"], filtered: [] }) };
        case "cancel_planning_job": return null;
        default: throw { code: "INVALID_INPUT", message: "Unexpected mock command", retryable: false };
      }
    } } });
  }, { fixture });
  await page.goto("/");await page.getByRole("button", { name: "Planner", exact: true }).click();await page.getByRole("combobox", { name: "Origin", exact: true }).selectOption("demo-node-1");await page.getByRole("combobox", { name: "Stopping node", exact: true }).selectOption("demo-node-1");await page.getByRole("button", { name: "Plan session" }).click();await expect(page.getByText("Previous native-command mock estimate")).toBeVisible();
  await page.evaluate(() => (window as unknown as { __PLANNING_MODE__: (value: string) => void }).__PLANNING_MODE__("queued"));await page.getByRole("button", { name: "Plan session" }).click();await expect(page.getByText("queued; awaiting worker; 0 work units checked")).toBeVisible();await page.evaluate(() => (window as unknown as { __PLANNING_MODE__: (value: string) => void }).__PLANNING_MODE__("running"));await expect(page.getByText("running; route expansion; 13 work units checked")).toBeVisible();await expect(page.getByRole("button", { name: "Today", exact: true })).toBeEnabled();await page.getByRole("button", { name: "Cancel planning" }).click();await expect(page.getByText("Previous native-command mock estimate")).toBeVisible();
  await page.evaluate(() => (window as unknown as { __PLANNING_MODE__: (value: string) => void }).__PLANNING_MODE__("stale"));await page.getByRole("button", { name: "Plan session" }).click();await expect(page.getByText("Imported snapshot changed")).toBeVisible();await expect(page.getByRole("region", { name: "Planning result" })).toHaveCount(0);
  await page.evaluate(() => (window as unknown as { __PLANNING_MODE__: (value: string) => void }).__PLANNING_MODE__("deferred"));await page.getByRole("button", { name: "Plan session" }).click();await expect(page.getByText("Registering planning job…")).toBeVisible();await page.getByRole("button", { name: "Today", exact: true }).click();await page.evaluate(() => (window as unknown as { __COMPLETE_REGISTRATION__: () => void }).__COMPLETE_REGISTRATION__());
  await expect.poll(() => page.evaluate(() => (window as unknown as { __PLANNING_CALLS__: { command: string; args: Record<string, unknown> }[] }).__PLANNING_CALLS__.filter((call) => call.command === "cancel_planning_job").map((call) => call.args.job_id))).toEqual(["job-2", "job-4"]);
  await expect(page.getByLabel("Planning job progress")).toHaveCount(0);
});
