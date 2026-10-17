import { expect, test } from "@playwright/test";
import content from "../../../fixtures/desktop/content/demo-revision.json" with { type: "json" };
import snapshot from "../../../fixtures/contracts/v1/character-snapshot.json" with { type: "json" };

// G5a #11 / G5b notes: a Panel inside a Panel is drawn flat with no horizontal padding, and the
// screens that nest panels still do not overflow at 320 px and 200 % text.
const SYNTHETIC_DATA_SCREENS = ["Compatibility and coverage", "Content"] as const;
const DEMO_PLAY_SCREENS = ["Atlas", "Gear", "Planner", "Sessions", "Dungeons"] as const;

async function measure(page: import("@playwright/test").Page) {
  return page.evaluate(() => {
    const nested = Array.from(document.querySelectorAll(".ui-panel--nested")).map((el) => {
      const style = getComputedStyle(el);
      const outer = el.parentElement?.closest(".ui-panel");
      const outerWidth = outer?.getBoundingClientRect().width ?? 0;
      return { left: style.paddingLeft, right: style.paddingRight, shadow: style.boxShadow, width: el.getBoundingClientRect().width, outerWidth };
    });
    const outer = Array.from(document.querySelectorAll(".ui-panel:not(.ui-panel--nested)")).map((el) => getComputedStyle(el).paddingLeft);
    return { nested, outer, scroll: document.documentElement.scrollWidth, client: document.documentElement.clientWidth };
  });
}

for (const screen of SYNTHETIC_DATA_SCREENS) {
  test(`${screen}: nested panels are flat and the page does not overflow at 320px and 200% text`, async ({ page }) => {
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
    const result = await measure(page);
    expect(result.scroll).toBeLessThanOrEqual(result.client);
    for (const panel of result.nested) {
      expect(panel.left).toBe("0px");
      expect(panel.right).toBe("0px");
      expect(panel.shadow).toBe("none");
    }
  });
}

for (const screen of DEMO_PLAY_SCREENS) {
  test(`${screen}: play screen panels reflow at 320px and 200% text`, async ({ page }) => {
    await page.setViewportSize({ width: 320, height: 800 });
    await page.goto("/");
    await page.evaluate(() => { document.documentElement.style.fontSize = "200%"; });
    await page.getByRole("navigation", { name: "Primary" }).getByRole("button", { name: screen, exact: true }).click();
    await expect(page.getByRole("main").getByRole("heading").first()).toBeVisible();
    await page.waitForTimeout(200);
    const result = await measure(page);
    expect(result.scroll).toBeLessThanOrEqual(result.client);
    for (const panel of result.nested) {
      expect(panel.left).toBe("0px");
      expect(panel.right).toBe("0px");
      expect(panel.shadow).toBe("none");
      // A nested panel is as wide as the body of the panel around it, not narrower by a frame.
      expect(panel.width).toBeGreaterThan(panel.outerWidth * 0.6);
    }
  });
}

test("Party with two participants: joint planning panels reflow at 320px and 200% text", async ({ page }) => {
  await page.setViewportSize({ width: 320, height: 800 });
  await page.addInitScript(({ content, snapshot }) => {
    const profiles = ["one", "two"];
    const members = () => profiles.map((character) => ({ character, profile_id: character, snapshot_id: `snapshot-${character}`, freshness: { status: "fresh", age_ms: 0 } }));
    Object.defineProperty(window, "__TAURI_INTERNALS__", { value: { invoke: async (command: string) => {
      switch (command) {
        case "get_app_status": return { version: "test", data_mode: "stored", active_profile: null, capabilities: {} };
        case "list_profiles": return profiles.map((id) => ({ id, display_name: id, data_mode: "stored", context: snapshot.payload.context, fixture: true }));
        case "list_parties": return ["duo"];
        case "get_party": return { id: "duo", data_mode: "stored", participants: profiles.map((id) => ({ character: id, profile_id: id, snapshot: { envelope: { ...snapshot, payload: { ...snapshot.payload, id: `snapshot-${id}`, character: id } }, profile_id: id, data_mode: "stored", freshness: { status: "fresh", age_ms: 0 } } })) };
        case "get_party_plan_proposal": return null;
        case "get_party_quests": return { context: snapshot.payload.context, fixture: true, revision: null, quests: [] };
        case "get_joint_planning_catalog": return { context: snapshot.payload.context, fixture: true, revision: "demo-r1", participants: members(), nodes: content.envelope.payload.nodes, objectives: content.envelope.payload.objectives };
        default: throw { code: "INVALID_INPUT", message: "Unexpected mock command", retryable: false };
      }
    } } });
  }, { content, snapshot });
  await page.goto("/");
  await page.evaluate(() => { document.documentElement.style.fontSize = "200%"; });
  await page.getByRole("button", { name: "Party", exact: true }).click();
  await expect(page.getByRole("heading", { name: "Manual joint session planning" })).toBeVisible();
  await page.waitForTimeout(300);
  const result = await measure(page);
  expect(result.nested.length, "expected a Panel nested in a Panel on Party").toBeGreaterThan(0);
  expect(result.scroll).toBeLessThanOrEqual(result.client);
  for (const panel of result.nested) {
    expect(panel.left).toBe("0px");
    expect(panel.right).toBe("0px");
    expect(panel.shadow).toBe("none");
  }
});
