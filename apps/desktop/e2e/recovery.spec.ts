import { expect, test } from "@playwright/test";
test("startup failure permits explicit inspected recovery and preserves normal navigation afterward", async ({ page }) => {
  await page.addInitScript(() => {
    let ready = false;
    const calls: { command: string; args: Record<string, unknown> }[] = [];
    Object.defineProperty(window, "__RECOVERY_CALLS__", { value: calls });
    Object.defineProperty(window, "__TAURI_INTERNALS__", { value: { invoke: async (command: string, args: Record<string, unknown>) => {
      calls.push({ command, args });
      switch (command) {
        case "get_app_status": if (!ready) throw { code: "STORAGE_ERROR", message: "Database recovery required", retryable: false }; return { version: "test", data_mode: "stored", active_profile: null, capabilities: {} };
        case "get_database_status": return { state: ready ? "ready" : "recovery_required", error: null, schema_version: 8, restore_limit_bytes: 536870912 };
        case "inspect_database_backup": return { candidate_id: "staged-only", inspection: { source_schema_version: 5, restored_schema_version: 8, rows: { profiles: 2, sessions: 1 } } };
        case "restore_database_backup": if (args.candidate_id !== "staged-only") throw { code: "INVALID_INPUT", message: "Inspection changed", retryable: false }; ready = true; return { original_files_retained: true, recovery_directory: "preserved-originals" };
        case "get_snapshot_watch": return { active: false, status: "cancelled", last_import: null, error: null };
        default: throw { code: "INVALID_INPUT", message: "Unexpected mock command", retryable: false };
      }
    } } });
  });
  await page.goto("/");
  await expect(page.getByRole("heading", { name: "Database backup and recovery" })).toBeVisible();
  await expect(page.getByRole("button", { name: "Create consistent backup" })).toBeDisabled();
  await page.getByLabel("Backup file to restore").fill("selected-backup.sqlite3");
  await page.getByRole("button", { name: "Inspect backup" }).click();
  await expect(page.getByText("Character profiles: 2")).toBeVisible();
  await expect(page.getByRole("button", { name: "Restore inspected backup" })).toBeDisabled();
  await page.getByLabel("Replace all local database state with this inspected backup").check();
  await page.getByRole("button", { name: "Restore inspected backup" }).click();
  await expect(page.getByRole("navigation", { name: "Primary" })).toBeVisible();
  await page.getByRole("button", { name: "Backup and recovery", exact: true }).click();
  await expect(page.getByText(/Database ready/)).toBeVisible();
  const calls = await page.evaluate(() => (window as unknown as { __RECOVERY_CALLS__: { command: string; args: Record<string, unknown> }[] }).__RECOVERY_CALLS__);
  expect(calls.filter((call) => call.command === "restore_database_backup")).toEqual([{ command: "restore_database_backup", args: { candidate_id: "staged-only" } }]);
});
