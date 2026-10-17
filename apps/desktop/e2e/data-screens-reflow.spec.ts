import { expect, test } from "@playwright/test";

// Review finding I4: Field controls must not widen the page at 320 px and 200 % text.
const SCREENS = ["Local diagnostics", "Backup and recovery", "Compatibility and coverage", "Settings", "Imports", "Content"] as const;

for (const screen of SCREENS) {
  test(`${screen} does not overflow horizontally at 320px and 200% text`, async ({ page }) => {
    await page.setViewportSize({ width: 320, height: 800 });
    await page.addInitScript(() => {
      Object.defineProperty(window, "__TAURI_INTERNALS__", { value: { invoke: async (command: string) => {
        switch (command) {
          case "get_app_status": return { version: "synthetic-reflow", data_mode: "stored", active_profile: null, capabilities: {} };
          case "get_snapshot_watch": return { active: false, status: null, last_import: null, error: null };
          case "get_database_status": return { state: "ready", schema_version: 1, restore_limit_bytes: 1048576, preserved_recovery_directory: null, error: null };
          case "list_profiles": case "list_parties": return [];
          case "get_app_settings": return { revision: 1, settings: { schema_version: 1, selected_subject: null, selected_installation: null, selected_export: null, manual_planning_defaults: { goal_profile: null, budget_ms: 600000, stopping_service_ms: 30000 }, accessibility: { text_scale_percent: 100, contrast: "system", motion: "system" }, reference_always_on_top: false } };
          default: throw { code: "PROVIDER_UNAVAILABLE", message: "Synthetic unavailable fixture", retryable: false };
        }
      } } });
    });
    await page.goto("/");
    await page.evaluate(() => { document.documentElement.style.fontSize = "200%"; });
    await page.getByRole("navigation", { name: "Primary" }).getByRole("button", { name: screen, exact: true }).click();
    await expect(page.getByRole("main").getByRole("heading").first()).toBeVisible();
    await page.waitForTimeout(200);
    const widths = await page.evaluate(() => ({ scroll: document.documentElement.scrollWidth, client: document.documentElement.clientWidth }));
    expect(widths.scroll).toBeLessThanOrEqual(widths.client);
    // G5b O7: every control fits too (the Compatibility "Extraction resources" select once measured 798 px).
    const controls = await page.evaluate(() => Array.from(document.querySelectorAll("main select, main input, main textarea")).map((el) => {
      const r = el.getBoundingClientRect();
      return { name: el.getAttribute("id") ?? el.tagName, left: r.left, right: r.right, width: r.width };
    }).filter((c) => c.width > 0));
    for (const control of controls) {
      expect(control.left, control.name).toBeGreaterThanOrEqual(0);
      expect(control.right, control.name).toBeLessThanOrEqual(widths.client);
    }
  });
}
