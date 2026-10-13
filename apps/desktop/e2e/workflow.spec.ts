import { test, expect, type Page } from "@playwright/test";
import planning from "../../../fixtures/desktop/planning/synthetic-revision.json" with { type: "json" };
import snapshot from "../../../fixtures/contracts/v1/character-snapshot.json" with { type: "json" };
import view from "../../../fixtures/desktop/atlas/query-atlas.json" with { type: "json" };
import markers from "../../../fixtures/desktop/atlas/snapshot-markers-uncertain.json" with { type: "json" };

/** Mock the existing native adapter at its command boundary; no product test hook. */
async function mockCommands(page: Page) {
  await page.addInitScript(({ snapshot, view, markers, planning }) => {
    let selected: string | null = null;
    const imported = new Set<string>();
    const calls: { command: string; args: Record<string, unknown> }[] = [];
    let watching = false;
    let activeContent = false;
    let promoted = false;
    const inspection = { key: { scope: { context: snapshot.payload.context, fixture: true }, revision: planning.envelope.payload.revision }, report: { passes: true, gaps: [], errors: [], coverage: null }, sources: [], conflicts: [] };
    let query = 0;
    const parties = new Map<string, string[]>();
    const freshness = { status: "uncertain", reason: "missing_or_untrusted_time" };
    const receipt = (id: string, disposition: object) => ({ profile_id: id, snapshot_id: promoted ? "historical" : `current-${id}`, disposition, fixture: true, namespace: "demo", data_mode: "stored", freshness });
    const current = (id: string) => ({ envelope: { ...snapshot, payload: { ...snapshot.payload, id: `current-${id}` } }, profile_id: id, freshness, data_mode: "stored" });
    const watch = () => ({ active: watching, status: watching ? "waiting_for_stable_file" : "cancelled", last_import: null, error: null });
    Object.defineProperty(window, "__MOCK_CALLS__", { value: calls });
    Object.defineProperty(window, "__TAURI_INTERNALS__", { value: {
      invoke: async (command: string, args: Record<string, unknown>) => {
        calls.push({ command, args });
        switch (command) {
          case "get_app_status": return { version: "test", data_mode: "stored", active_profile: selected, capabilities: {} };
          case "list_profiles": return [...imported].map((id) => ({ id, display_name: id, data_mode: "stored", context: snapshot.payload.context, fixture: true, snapshot_count: 1 }));
          case "get_party_quests": return { context: snapshot.payload.context, fixture: true, revision: null, quests: [] };
          case "list_parties": return [...parties.keys()].sort();
          case "save_party": parties.set(String(args.party_id), args.profile_ids as string[]); return null;
          case "delete_party": return parties.delete(String(args.party_id));
          case "get_party": return parties.has(String(args.party_id)) ? { id: args.party_id, data_mode: "stored", participants: (parties.get(String(args.party_id)) ?? []).map((id) => ({ profile_id: id, character: id, snapshot: id === "profile-one" ? { ...current(id), freshness: { status: "stale", age_ms: 3600000 } } : null })) } : null;
          case "get_stored_snapshot": return current(String(args.profile_id));
          case "get_snapshot_history": return [
            { snapshot_id: `current-${String(args.profile_id)}`, is_current: !promoted, freshness, observed_at_utc_ms: null, received_at_utc_ms: null },
            { snapshot_id: "historical", is_current: promoted, freshness, observed_at_utc_ms: null, received_at_utc_ms: null },
          ];
          case "get_historical_snapshot": return { ...current(String(args.profile_id)), envelope: { ...snapshot, payload: { ...snapshot.payload, id: "historical" } } };
          case "promote_snapshot": if (args.expected_current !== "current-profile-one") throw new Error("Wrong expected snapshot"); promoted = true; return null;
          case "stage_content": return inspection;
          case "activate_content": activeContent = true; return inspection;
          case "get_active_content": return activeContent ? inspection : null;
          case "get_planning_catalog": return { snapshot_id: "current-profile-one", revision: activeContent ? planning.envelope.payload.revision : null, fixture: true, nodes: planning.envelope.payload.nodes, objectives: planning.envelope.payload.objectives };
          case "inspect_plan_route": return { context: snapshot.payload.context, fixture: true, frames: [], hops: [{ leg: 0, stopping: false, hop: { from: { id: "demo-node-1" }, to: { id: "demo-node-2" }, mode: "walking", cost_ms: 30000, status: "estimated" }, from: { outcome: "gap", gap: "no_image_for_map" }, to: { outcome: "gap", gap: "no_image_for_map" }, requirements: { status: "not_applicable" }, schematic_connection: false, geometry_gap: "Exact dataset projection unavailable; inspect endpoint gaps" }] };
          case "export_plan": return { text: "ANAV1:synthetic-command-mock", byte_count: 28, plan_id: "mock-plan", expires_at_utc_ms: 1 };
          case "plan_session": return { request: JSON.parse(String(args.text)), fixture: true, context: snapshot.payload.context, assumptions: ["Synthetic manual estimates"], filtered: [], result: { selected: [{ candidate: "synthetic-service", utility: 10, incremental_ms: 31000, completion_ms: 1000, service_ms: 0, travel: { total_ms: 30000 }, expected_session_ms: 61000 }], stopping: { total_ms: 30000 }, expected_total_ms: 61000, estimated: true, alternatives: [], trace: [], termination: "no_feasible_candidate" } };
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
  }, { snapshot, view, markers, planning });
}

test.beforeEach(async ({ page }) => { await page.clock.install(); await mockCommands(page); await page.goto("/"); });

test("manual party keeps stale and missing participants separate across navigation", async ({ page }) => {
  await page.getByRole("button", { name: "Imports", exact: true }).click();
  for (const name of ["one", "two"]) {
    await page.getByLabel("Snapshot export text").fill(`ANAV1:valid-${name}`);
    await page.getByRole("button", { name: "Import text", exact: true }).click();
    await expect(page.getByText("Stored as the current snapshot.")).toBeVisible();
  }
  await page.getByRole("button", { name: "Party", exact: true }).click();
  await page.getByRole("button", { name: "New party", exact: true }).click();
  await page.getByLabel("Party name").fill("Duo");
  await page.getByLabel(/^profile-one —/).check();
  await page.getByLabel(/^profile-two —/).check();
  await page.getByRole("button", { name: "Save membership", exact: true }).click();
  await expect(page.getByText("Last imported: captured 1 h ago (stale)")).toBeVisible();
  await expect(page.getByText(/Missing imported snapshot/)).toBeVisible();
  await page.clock.fastForward(60000);
  await expect(page.getByLabel("Selected party")).toHaveValue("Duo");
  await page.getByRole("button", { name: "Today", exact: true }).click();
  await page.getByRole("button", { name: "Party", exact: true }).click();
  await expect(page.getByText(/Missing imported snapshot/)).toBeVisible();
  await page.getByRole("button", { name: "Remove local party", exact: true }).click();
  await expect(page.getByText("No local parties yet.")).toBeVisible();
  await page.getByRole("button", { name: "Imports", exact: true }).click();
  await expect(page.getByLabel("Selected profile").locator("option")).toHaveCount(2);
  const calls = await page.evaluate(() => (window as unknown as { __MOCK_CALLS__: { command: string; args: object }[] }).__MOCK_CALLS__);
  expect(calls).toContainEqual({ command: "save_party", args: { party_id: "Duo", profile_ids: ["profile-one", "profile-two"] } });
  expect(calls).toContainEqual({ command: "delete_party", args: { party_id: "Duo" } });
});

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


test("explicit promotion previews and confirms the expected current snapshot", async ({ page }) => {
  await page.getByRole("button", { name: "Imports", exact: true }).click();
  await page.getByLabel("Snapshot export text").fill("ANAV1:valid-one");
  await page.getByRole("button", { name: "Import text", exact: true }).click();
  await page.getByRole("button", { name: "Preview historical", exact: true }).click();
  await expect(page.getByRole("region", { name: "Promotion preview" })).toBeVisible();
  await page.getByLabel("Selection reason").fill("New session manually selected");
  await page.getByRole("button", { name: "Confirm promotion" }).click();
  await expect(page.getByText(/historical \(current\)/)).toBeVisible();
});

test("content activation enables explicit planning and editing invalidates the result", async ({ page }) => {
  await page.getByRole("button", { name: "Imports", exact: true }).click();
  await page.getByLabel("Snapshot export text").fill("ANAV1:valid-one");
  await page.getByRole("button", { name: "Import text", exact: true }).click();
  await page.getByRole("button", { name: "Content", exact: true }).click();
  await page.getByRole("button", { name: "Load synthetic planning example" }).click();
  await page.getByRole("button", { name: "Validate and stage content" }).click();
  await page.getByRole("button", { name: "Activate staged revision" }).click();
  await expect(page.getByText(/Content activated/)).toBeVisible();
  await page.getByRole("button", { name: "Planner", exact: true }).click();
  await page.getByRole("combobox", { name: "Origin", exact: true }).selectOption("demo-node-1");
  await page.getByRole("combobox", { name: "Stopping node", exact: true }).selectOption("demo-node-1");
  await page.getByLabel("Consider objective").check();
  await page.getByRole("combobox", { name: "Objective node", exact: true }).selectOption("demo-node-2");
  await page.getByLabel("utility", { exact: true }).fill("10");
  await page.getByLabel("completion_ms", { exact: true }).fill("1000");
  await page.getByRole("button", { name: "Plan session", exact: true }).click();
  await expect(page.getByRole("region", { name: "Planning result" })).toContainText("Total 61000 ms");
  await page.getByRole("button", { name: "Inspect route", exact: true }).click();
  await expect(page.getByRole("region", { name: "Route inspection", exact: true })).toBeVisible();
  await expect(page.getByText("Exact dataset projection unavailable; inspect endpoint gaps", { exact: true })).toBeVisible();
  await page.getByRole("button", { name: "Export plan", exact: true }).click();
  await expect(page.getByLabel("Plan export text")).toHaveValue("ANAV1:synthetic-command-mock");
  await page.getByRole("button", { name: "Copy plan", exact: true }).click();
  await expect(page.getByRole("region", { name: "Plan export", exact: true }).getByRole("status")).not.toBeEmpty();
  await page.getByLabel("Budget (ms)").fill("100");
  await expect(page.getByLabel("Plan export text")).toHaveCount(0);
  await expect(page.getByRole("region", { name: "Planning result" })).toHaveCount(0);
});
