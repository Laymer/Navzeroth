import { expect, test } from "@playwright/test";
test("compatibility inspection labels historical presence and unavailable active scope", async ({ page }) => {
  await page.addInitScript(() => {
    const calls: { command: string; args: Record<string, unknown> }[] = [];
    Object.defineProperty(window, "__COMPAT_CALLS__", { value: calls });
    Object.defineProperty(window, "__TAURI_INTERNALS__", { value: { invoke: async (command: string, args: Record<string, unknown>) => {
      calls.push({ command, args });
      if (command === "get_app_status") return { version: "test", data_mode: "stored", active_profile: null, capabilities: {} };
      if (command === "get_watch") return { running: false, last_import: null };
      if (command === "get_compatibility_report") return { report_version: 1, app_version: "test", selected: null, active: null, recorded_builds: [
        { build: { edition: "wow_forever", build: "70338", interface: "16001" }, version: "1.60.1", basis: "reported_flow", reported_on: "2026-10-13", addon_version: "0.1.0", matches_selected_build: null, limitations: "One manual sample only", capabilities: [{ capability: "C_Map.GetPlayerMapPosition", state: "function_present_only", statement: "Position value untested", source: "sanitized fixture" }] },
        { build: { edition: "wow_forever", build: "1.60.1.70205", interface: "16001" }, version: "1.60.1", basis: "assumed_reference", reported_on: "2026-10-12", addon_version: null, matches_selected_build: null, limitations: "No client session for this tuple", capabilities: [] },
      ], unverified_checks: [{ check: "live_plan_and_party", required_evidence: "Recorded actual-client results" }], official_api: "Enablement unverified; this does not establish official API absence", limitations: "Historical finite evidence only" };
      throw { code: "INVALID_INPUT", message: "Unexpected mock command", retryable: false };
    } } });
  });
  await page.goto("/");
  await page.getByRole("button", { name: "Compatibility and coverage", exact: true }).click();
  await page.getByRole("button", { name: "Inspect compatibility and coverage" }).click();
  await expect(page.getByText(/No selected profile: active dataset coverage is unavailable, not zero/)).toBeVisible();
  await expect(page.getByText(/Function present; usability untested/)).toBeVisible();
  await expect(page.getByText(/Reference assumption; no client session/)).toBeVisible();
  await expect(page.getByText(/live plan and party: Recorded actual-client results/)).toBeVisible();
  const calls = await page.evaluate(() => (window as unknown as { __COMPAT_CALLS__: { command: string; args: Record<string, unknown> }[] }).__COMPAT_CALLS__);
  expect(calls.find((c) => c.command === "get_compatibility_report")?.args).toEqual({ profile_id: null });
});
