import { test, expect, type Page } from "@playwright/test";
import snapshot from "../../../fixtures/contracts/v1/character-snapshot.json" with { type: "json" };
import view from "../../../fixtures/desktop/atlas/query-atlas.json" with { type: "json" };
import markers from "../../../fixtures/desktop/atlas/snapshot-markers-uncertain.json" with { type: "json" };

/** Mock the existing native adapter at its command boundary; no product test hook. */
async function mockCommands(page: Page) {
  await page.addInitScript(({ snapshot, view, markers }) => {
    let selected: string | null = null;
    const imported = new Set<string>();
    const calls: { command: string; args: Record<string, unknown> }[] = [];
    let watching = false;
    let query = 0;
    const freshness = { status: "uncertain", reason: "missing_or_untrusted_time" };
    const receipt = (id: string, disposition: object) => ({ profile_id: id, snapshot_id: `current-${id}`, disposition, fixture: true, namespace: "demo", data_mode: "stored", freshness });
    const current = (id: string) => ({ envelope: { ...snapshot, payload: { ...snapshot.payload, id: `current-${id}` } }, profile_id: id, freshness, data_mode: "stored" });
    const watch = () => ({ active: watching, status: watching ? "waiting_for_stable_file" : "cancelled", last_import: null, error: null });
    Object.defineProperty(window, "__MOCK_CALLS__", { value: calls });
    Object.defineProperty(window, "__TAURI_INTERNALS__", { value: {
      invoke: async (command: string, args: Record<string, unknown>) => {
        calls.push({ command, args });
        switch (command) {
          case "get_app_status": return { version: "test", data_mode: "stored", active_profile: selected, capabilities: {} };
          case "list_profiles": return [...imported].map((id) => ({ id, display_name: id, data_mode: "stored", context: snapshot.payload.context, fixture: true, snapshot_count: 1 }));
          case "get_stored_snapshot": return current(String(args.profile_id));
          case "get_snapshot_history": return [{ snapshot_id: `current-${String(args.profile_id)}`, is_current: true, freshness, observed_at_utc_ms: null, received_at_utc_ms: null }];
          case "select_profile": selected = String(args.profile_id); return null;
          case "import_snapshot_text": {
            const text = String(args.text);
            if (!text.startsWith("ANAV1:valid")) throw { code: "INVALID_INPUT", message: "Invalid snapshot export", retryable: false };
            const id = text.endsWith("two") ? "profile-two" : "profile-one";
            const disposition = text.endsWith("older") ? { outcome: "history_only", reason: "older" }
              : imported.has(id) ? { outcome: "duplicate" } : { outcome: "stored", previous_current: null };
            imported.add(id); selected = id; return receipt(id, disposition);
          }
          case "import_snapshot_file": imported.add("profile-one"); selected = "profile-one"; return receipt("profile-one", { outcome: "stored", previous_current: null });
          case "start_snapshot_watch": watching = true; return watch();
          case "stop_snapshot_watch": watching = false; return watch();
          case "get_snapshot_watch": return watch();
          case "get_snapshot_markers": return selected === null ? [] : markers;
          case "query_atlas": {
            query += 1;
            return { ...view, revision: `mock-atlas-${query}`, images: view.images.map((image) => ({ ...image, pins: Array.from({ length: 150 }, (_, i) => ({ ...image.pins[0], id: `pin-${i}`, label: `Reference ${String(i).padStart(3, "0")}`, kind: { kind: i % 2 === 0 ? "landmark" : "settlement" }, pixel: { x: i % image.width_px, y: i % image.height_px } })) })) };
          }
          default: throw { code: "INVALID_INPUT", message: "Unexpected mock command", retryable: false };
        }
      },
    } });
  }, { snapshot, view, markers });
}

test.beforeEach(async ({ page }) => { await page.clock.install(); await mockCommands(page); await page.goto("/"); });

test("imports recover from failure and preserve duplicate/history-only current state", async ({ page }) => {
  await expect(page.getByText("No character snapshot has been imported.")).toBeVisible();
  await page.getByRole("button", { name: "Imports", exact: true }).click();
  await page.getByLabel("Snapshot export text").fill("ANAV1:{");
  await page.getByRole("button", { name: "Import text", exact: true }).click();
  await expect(page.getByRole("alert")).toContainText("INVALID_INPUT");
  await expect(page.getByText("No stored profiles yet.")).toBeVisible();
  await page.getByLabel("Snapshot export text").fill("ANAV1:valid-one");
  await page.getByRole("button", { name: "Import text", exact: true }).click();
  await expect(page.getByText("Stored as the current snapshot.")).toBeVisible();
  await expect(page.getByRole("alert")).toHaveCount(0);
  await expect(page.getByText(/SYNTHETIC — fixture/)).toBeVisible();
  await page.getByRole("button", { name: "Import text", exact: true }).click();
  await expect(page.getByText("Already stored: the current snapshot is unchanged.")).toBeVisible();
  await page.getByLabel("Snapshot export text").fill("ANAV1:valid-older");
  await page.getByRole("button", { name: "Import text", exact: true }).click();
  await expect(page.getByText(/Stored in history only.*older/)).toBeVisible();
  await page.getByRole("button", { name: "Today", exact: true }).click();
  await expect(page.getByText("current-profile-one", { exact: true })).toBeVisible();
  await expect(page.getByText("Last imported: age uncertain")).toBeVisible();
});

test("profile selection, explicit file import and watch cancellation use command DTOs", async ({ page }) => {
  await page.getByRole("button", { name: "Imports", exact: true }).click();
  await page.getByLabel("Export file path").fill("/selected/synthetic-export.txt");
  await page.getByRole("button", { name: "Import selected file", exact: true }).click();
  await expect(page.getByText("Stored as the current snapshot.")).toBeVisible();
  await page.getByRole("button", { name: "Watch selected file", exact: true }).click();
  await expect(page.getByText("Waiting for a complete, stable save.")).toBeVisible();
  await page.getByRole("button", { name: "Stop watching", exact: true }).click();
  await expect(page.getByText("Watching stopped.")).toBeVisible();
  await page.getByLabel("Snapshot export text").fill("ANAV1:valid-two");
  await page.getByRole("button", { name: "Import text", exact: true }).click();
  await expect(page.getByLabel("Selected profile")).toHaveValue("profile-two");
  await page.getByLabel("Selected profile").selectOption("profile-one");
  await expect(page.getByLabel("Selected profile")).toHaveValue("profile-one");
  await page.getByRole("button", { name: "Today", exact: true }).click();
  await expect(page.getByText("current-profile-one", { exact: true })).toBeVisible();
  const calls = await page.evaluate(() => (window as unknown as { __MOCK_CALLS__: { command: string; args: object }[] }).__MOCK_CALLS__);
  expect(calls).toContainEqual({ command: "import_snapshot_file", args: { path: "/selected/synthetic-export.txt", format: { kind: "anav1_text" } } });
  expect(calls).toContainEqual({ command: "select_profile", args: { profile_id: "profile-one" } });
});

test("dense atlas filters and keyboard navigation retain selection after completed refresh", async ({ page }) => {
  await page.getByRole("button", { name: "Atlas", exact: true }).click();
  await expect(page.getByText("150 of 150 reference pins shown.")).toBeVisible();
  await page.getByLabel("Point type").selectOption("landmark");
  await expect(page.getByText("75 of 150 reference pins shown.")).toBeVisible();
  await page.getByLabel("Search reference pins").fill("Reference 148");
  const pin = page.getByRole("button", { name: "Show Reference 148 on the map" });
  await pin.focus(); await pin.press("Enter");
  await expect(pin).toHaveAttribute("aria-pressed", "true");
  const dataset = page.getByText(/Dataset mock-atlas-/);
  const before = await dataset.textContent();
  await page.getByRole("button", { name: "Refresh atlas" }).click();
  await expect(dataset).not.toHaveText(before ?? "");
  await expect(pin).toHaveAttribute("aria-pressed", "true");
  await expect(page.getByLabel("Search reference pins")).toHaveValue("Reference 148");
  const map = page.getByRole("group", { name: /Atlas map/ });
  const frame = map.locator("rect.frame");
  const initialWidth = await frame.getAttribute("width");
  await map.focus(); await map.press("+");
  await expect(frame).not.toHaveAttribute("width", initialWidth ?? "");
  const initialY = await frame.getAttribute("y");
  await map.press("ArrowDown");
  await expect(frame).not.toHaveAttribute("y", initialY ?? "");
  await expect(map).toBeFocused();
  await page.getByLabel("Search reference pins").fill("nothing matches");
  await expect(page.getByText("No reference pins match these filters.")).toBeVisible();
});


test("open imported views requery backend freshness without local age calculations", async ({ page }) => {
  await page.getByRole("button", { name: "Imports", exact: true }).click();
  await page.getByLabel("Snapshot export text").fill("ANAV1:valid-one");
  await page.getByRole("button", { name: "Import text", exact: true }).click();
  await expect(page.getByText("Stored as the current snapshot.")).toBeVisible();
  await page.getByRole("button", { name: "Today", exact: true }).click();
  const queries = () => page.evaluate(() => (window as unknown as { __MOCK_CALLS__: { command: string }[] }).__MOCK_CALLS__.filter((call) => call.command === "get_stored_snapshot").length);
  const before = await queries();
  await page.clock.fastForward(60_000);
  await expect.poll(queries).toBeGreaterThan(before);
  await expect(page.getByText("Last imported: age uncertain")).toBeVisible();
});
