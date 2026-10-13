import { expect, test } from "@playwright/test";
import example from "../../../fixtures/desktop/sessions/analytics.synthetic.json" with { type: "json" };
import snapshot from "../../../fixtures/contracts/v1/character-snapshot.json" with { type: "json" };
test("Sessions manual import, duplicate, analytics and deletion at command boundary", async ({ page }) => {
  await page.addInitScript(({ example, snapshot }) => {
    let stored = false;
    Object.defineProperty(window, "__TAURI_INTERNALS__", { value: { invoke: async (command: string, args: Record<string, unknown>) => {
      switch (command) {
        case "get_app_status": return { version: "test", data_mode: "stored", active_profile: "profile", capabilities: {} };
        case "list_profiles": return [{ id: "profile", display_name: "Synthetic character", fixture: true, context: example.summary.context, snapshot_count: 1, data_mode: "stored" }];
        case "get_stored_snapshot": return { envelope: snapshot, profile_id: "profile", freshness: { status: "uncertain", reason: "missing_or_untrusted_time" }, data_mode: "stored" };
        case "list_sessions": return stored ? [{ session_id: example.summary.session, analytics: example }] : [];
        case "get_session_detail": return { fixture: true, batch: { character: example.summary.character, context: example.summary.context, events: [] }, analytics: example };
        case "delete_session": stored = false; return true;
        case "import_session_text":
          if (!String(args.text).startsWith("ANAVSESSION1:")) throw { code: "INVALID_INPUT", message: "Invalid session framing", retryable: false };
          { const duplicate = stored; stored = true; return { profile_id: "profile", inserted: duplicate ? 0 : 1, duplicates: duplicate ? 1 : 0, session_ids: [example.summary.session] }; }
        default: throw { code: "INVALID_INPUT", message: "Unexpected mock command", retryable: false };
      }
    } } });
  }, { example, snapshot });
  await page.goto("/");
  await page.getByRole("button", { name: "Sessions", exact: true }).click();
  await expect(page.getByText(/SYNTHETIC — engineering examples/)).toBeVisible();
  await page.getByLabel("Session export text").fill("invalid");
  await page.getByRole("button", { name: "Import session text" }).click();
  await expect(page.getByRole("alert")).toContainText("Invalid session framing");
  await page.getByLabel("Session export text").fill("ANAVSESSION1:synthetic-command-mock");
  await page.getByRole("button", { name: "Import session text" }).click();
  await expect(page.getByText("Imported 1 session(s); 0 already stored.")).toBeVisible();
  await page.getByRole("button", { name: "Inspect synthetic-session" }).click();
  await expect(page.getByText(/Unclassified intervals: 10.0 s/)).toContainText("Gaps: 5.0 s");
  await expect(page.getByText(/UTC mapping: uncertain/)).toBeVisible();
  await page.getByRole("button", { name: "Import session text" }).click();
  await expect(page.getByText("Imported 0 session(s); 1 already stored.")).toBeVisible();
  await expect(page.getByRole("button", { name: "Delete synthetic-session" })).toHaveCount(1);
  await page.getByRole("button", { name: "Delete synthetic-session" }).click();
  await expect(page.getByText("Session deleted.")).toBeVisible();
  await expect(page.getByText("No stored sessions for this profile.")).toBeVisible();
});
