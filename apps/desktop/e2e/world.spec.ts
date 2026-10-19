import AxeBuilder from "@axe-core/playwright";
import { expect, test } from "@playwright/test";
import pages from "../../../fixtures/desktop/world/pages.json" with { type: "json" };
import type { WorldPage } from "../src/commands/world";
import { installMock } from "./support/mockInvoke";

// The world browser over the Rust-written synthetic world pages (fixtures/desktop/world,
// UPDATE_WORLD_BROWSER_FIXTURES). Search and paging are the backend's; this mock only
// returns the golden page for the requested view and query.
const golden = pages as unknown as Record<"entities" | "entities_hog" | "vendors" | "trainers" | "quests" | "loot", WorldPage>;

test("the world browser finds Hogger and marks him as 1.12 data", async ({ page }) => {
  const requests: { view: string; query: string }[] = [];
  await installMock(page, {
    get_app_status: () => ({ version: "synthetic-world", data_mode: "stored", active_profile: null, capabilities: {} }),
    get_snapshot_watch: () => ({ active: false, status: null, last_import: null, error: null }),
    list_profiles: () => [{ id: "synthetic-world-profile", display_name: "Synthetic world profile", data_mode: "stored" }],
    get_world_page: ({ text }) => {
      const request = JSON.parse(text) as { view: keyof typeof golden; query: string };
      requests.push(request);
      return request.view === "entities" && request.query ? golden.entities_hog : golden[request.view];
    },
  });
  await page.goto("/");
  await page.getByRole("button", { name: "World", exact: true }).click();
  await expect(page.getByRole("heading", { name: "World browser", exact: true })).toBeVisible();
  await expect(page.getByRole("status")).toHaveText("NPCs and objects: 1–4 of 4.");
  await page.getByLabel("Search NPCs and objects", { exact: true }).fill("Hogger");
  await page.getByRole("button", { name: "Search", exact: true }).click();
  const hogger = page.getByRole("heading", { level: 3, name: /^Hogger/ });
  await expect(hogger).toBeVisible();
  await expect(hogger.locator(".ui-origin")).toHaveText(/^◆From 1\.12 data$/);
  await expect(page.getByText("map demo-map-1, floor 0, at 27, 87 (node world-node-hogger)")).toBeVisible();
  expect(requests.at(-1)).toMatchObject({ view: "entities", query: "Hogger", offset: 0 });
  await page.getByRole("button", { name: "Vendors (1)", exact: true }).click();
  await expect(page.getByText(/item demo-item-1 \(base\); cost 250 copper/)).toContainText("From Classic Era data");
  const scan = await new AxeBuilder({ page }).analyze();
  expect(scan.violations.filter((v) => v.impact === "serious" || v.impact === "critical").map((v) => v.id)).toEqual([]);
  await page.setViewportSize({ width: 320, height: 800 });
  const width = await page.locator(".world-screen").evaluate((e) => ({ width: e.clientWidth, scroll: e.scrollWidth }));
  expect(width.scroll).toBeLessThanOrEqual(width.width);
});
