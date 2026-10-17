import { expect, test, type Page } from "@playwright/test";
import fixture from "../../../fixtures/desktop/today/synthetic-view.json" with { type: "json" };

// G5b O5: while an import or a save runs, the surrounding <fieldset disabled> disables the form
// controls, but the Help triggers (span role=button, tabindex 0) stay in the tab order and open.
// The pending command is held until the test releases it.

async function install(page: Page, hold: string) {
  await page.addInitScript(({ fixture, hold }) => {
    let release: () => void = () => undefined;
    Object.defineProperty(window, "__RELEASE__", { value: () => release() });
    (window as unknown as { __TAURI_INTERNALS__: unknown }).__TAURI_INTERNALS__ = { invoke: async (command: string) => {
      if (command === hold) {
        await new Promise<void>((done) => { release = done; });
        throw { code: "INVALID_INPUT", message: "Synthetic held command released", retryable: false };
      }
      switch (command) {
        case "get_app_status": return { version: "synthetic", data_mode: "stored", active_profile: fixture.profiles[0]?.id, capabilities: {} };
        case "get_app_settings": return fixture.settings;
        case "get_today_view": return fixture.today;
        case "list_profiles": return fixture.profiles;
        case "list_parties": return [];
        case "get_character_snapshot": return fixture.snapshot;
        case "get_stored_snapshot": return { envelope: fixture.snapshot, profile_id: fixture.profiles[0]?.id, data_mode: "stored", freshness: fixture.today.members[0]?.freshness };
        case "get_snapshot_history": return [];
        case "get_snapshot_watch": return { active: false, status: null, last_import: null, error: null };
        default: throw { code: "PROVIDER_UNAVAILABLE", message: "Synthetic command unavailable", retryable: false };
      }
    } };
  }, { fixture, hold });
}

async function expectHelpUsable(page: Page, control: string) {
  await expect(page.getByLabel(control, { exact: true })).toBeDisabled();
  const help = page.getByRole("button", { name: `Help: ${control}`, exact: true });
  await expect(help).toHaveAttribute("tabindex", "0");
  await expect(help).not.toBeDisabled();
  // Reach it with the keyboard from the trigger before it, then open it with Enter.
  await help.focus();
  await expect(help).toBeFocused();
  await page.keyboard.press("Shift+Tab");
  await expect(help).not.toBeFocused();
  await page.keyboard.press("Tab");
  await expect(help).toBeFocused();
  await page.keyboard.press("Enter");
  await expect(help).toHaveAttribute("aria-expanded", "true");
  await expect(page.getByRole("tooltip").filter({ visible: true })).toHaveCount(1);
  await page.keyboard.press("Escape");
}

test("Imports: Help inside the busy file fieldset stays focusable while an import runs", async ({ page }) => {
  await install(page, "import_snapshot_text");
  await page.goto("/");
  await page.getByRole("button", { name: "Imports", exact: true }).click();
  await expect(page.getByLabel("Export file path", { exact: true })).toBeEnabled();
  await page.getByLabel("Snapshot export text", { exact: true }).fill("ANAV1:held");
  await page.getByRole("button", { name: "Import text", exact: true }).click();
  await expect(page.getByRole("button", { name: "Import text", exact: true })).toBeDisabled();
  await expectHelpUsable(page, "Export file path");
  await page.evaluate(() => (window as unknown as { __RELEASE__: () => void }).__RELEASE__());
  await expect(page.getByLabel("Export file path", { exact: true })).toBeEnabled();
});

test("Settings: Help inside the busy settings fieldset stays focusable while a save runs", async ({ page }) => {
  await install(page, "update_app_settings");
  await page.goto("/");
  await page.getByRole("button", { name: "Settings", exact: true }).click();
  const path = page.getByLabel("Chosen game installation path", { exact: true });
  await expect(path).toBeEnabled();
  await path.fill("/local/held/path");
  await page.getByRole("button", { name: "Save local settings", exact: true }).click();
  await expectHelpUsable(page, "Chosen game installation path");
  await page.evaluate(() => (window as unknown as { __RELEASE__: () => void }).__RELEASE__());
  await expect(path).toBeEnabled();
});
