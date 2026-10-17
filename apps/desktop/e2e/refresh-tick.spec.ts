import { expect, test, type Page } from "@playwright/test";

// App.tsx bumps a revision every 60 s so backend freshness is requeried. Plan 12.6
// says unsaved user input must survive that. Each case types into one field, lets
// the 60 s tick fire on a controlled clock, and expects the text to be untouched.
// A case may be marked fixme only after its reset was reproduced (none is now: G5b fixed Settings).
interface Case { screen: string; field: string; value: string; open?: string; fixme?: string }
const CASES: readonly Case[] = [
  { screen: "Settings", field: "Manual goal identifier", value: "unsaved-goal" },
  { screen: "Local diagnostics", field: "New diagnostic file path", value: "C:\\unsaved\\diagnostic.json" },
  { screen: "Backup and recovery", field: "New backup file path", value: "C:\\unsaved\\backup.sqlite3" },
  { screen: "Atlas", field: "Search reference pins", value: "unsaved search" },
  { screen: "Imports", field: "Snapshot export text", value: "ANAV1:unsaved draft" },
  { screen: "Dungeons", field: "Local preparation package JSON", value: "{\"unsaved\":true}", open: "Advanced local reference editor" },
  { screen: "Content", field: "Content revision JSON", value: "{\"unsaved\":true}" },
  { screen: "Planner", field: "Versioned goal identifier", value: "unsaved-goal-v1" },
  { screen: "Gear", field: "Route goal", value: "unsaved-route" },
  { screen: "Sessions", field: "Session export text", value: "ANAVSESSION1:unsaved" },
];

/** Fires one 60 s revision tick and proves it ran: the app re-applies the saved text scale. */
async function tick(page: Page) {
  await page.evaluate(() => { document.documentElement.style.fontSize = "77%"; });
  await page.clock.fastForward(61_000);
  await expect.poll(() => page.evaluate(() => document.documentElement.style.fontSize)).toBe("100%");
}

for (const { screen, field, value, open, fixme } of CASES) {
  test(`${screen} keeps unsaved "${field}" across the 60 s refresh tick`, async ({ page }) => {
    test.fixme(fixme !== undefined, fixme ?? "");
    await page.clock.install();
    await page.goto("/");
    await page.getByRole("navigation", { name: "Primary" }).getByRole("button", { name: screen, exact: true }).click();
    if (open !== undefined) await page.getByText(open, { exact: true }).click();
    // Not exact: a label wrapping a textarea includes the typed text in its name.
    const input = page.getByLabel(field);
    await input.fill(value);
    await tick(page);
    await expect(input).toHaveValue(value);
  });
}

test("the tick never changes the screen the person is on", async ({ page }) => {
  await page.clock.install();
  await page.goto("/");
  const nav = page.getByRole("navigation", { name: "Primary" });
  await nav.getByRole("button", { name: "Sessions", exact: true }).click();
  await tick(page);
  await expect(nav.getByRole("button", { name: "Sessions", exact: true })).toHaveAttribute("aria-current", "page");
});
