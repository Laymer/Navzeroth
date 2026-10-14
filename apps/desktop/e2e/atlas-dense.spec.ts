import { test, expect, type Page } from "@playwright/test";
import atlas from "../../../fixtures/desktop/atlas/query-atlas.json" with { type: "json" };

async function denseAtlas(page: Page, count: number) {
  await page.addInitScript(({ atlas, count }) => {
    const base = atlas.images[0];
    if (!base) throw new Error("Missing synthetic atlas image");
    const pins = Array.from({ length: count }, (_, i) => ({ ...base.pins[0], id: `dense-${i}`, label: `Dense ${String(i).padStart(5, "0")}`, kind: { kind: i % 2 ? "settlement" : "landmark" }, pixel: { x: (i % 100) * base.width_px / 100, y: Math.floor(i / 100) * base.height_px / Math.ceil(count / 100) } }));
    Object.defineProperty(window, "__TAURI_INTERNALS__", { value: { invoke: async (command: string) => {
      switch (command) {
        case "get_app_status": return { version: "synthetic-dense", data_mode: "stored", active_profile: null, capabilities: {} };
        case "get_snapshot_watch": return { active: false, status: null, last_import: null, error: null };
        case "get_snapshot_markers": return [];
        case "query_atlas": return { ...atlas, revision: "synthetic-dense", images: [{ ...base, layers: [], pins }] };
        default: throw new Error(`Unexpected mock command: ${command}`);
      }
    } } });
  }, { atlas, count });
  await page.goto("/");
}

for (const count of [1000, 5000, 10000]) {
  test(`synthetic ${count}-pin rendering measurement`, async ({ page }) => {
    await denseAtlas(page, count);
    const samples = [];
    for (let run = 0; run < 3; run += 1) {
      // A full reload isolates each sample from previous React trees.
      await page.reload();
      await expect(page.getByRole("button", { name: "Atlas", exact: true })).toBeVisible();
      const start = await page.evaluate(() => performance.now());
      await page.getByRole("button", { name: "Atlas", exact: true }).click();
      await expect(page.getByText(`${count} of ${count} reference pins shown.`)).toBeVisible();
      samples.push(await page.evaluate(() => performance.now()) - start);
      await expect(page.locator(".pins button")).toHaveCount(200);
      await expect(page.locator("g.pin")).toHaveCount(200);
      console.log(JSON.stringify({ count, run, elapsed_ms: samples[run], list_buttons: await page.locator(".pins button").count(), svg_pins: await page.locator("g.pin").count() }));
      await page.getByRole("button", { name: "Today", exact: true }).click();
    }
  });
}

for (const width of [320, 640]) {
  test(`dense keyboard and schematic reflow at ${width}px and 200% text`, async ({ page }) => {
    await page.setViewportSize({ width, height: 900 });
    await page.emulateMedia({ forcedColors: "active" });
    await denseAtlas(page, 10000);
    await page.addStyleTag({ content: ":root { font-size: 32px; }" });
    await page.getByRole("button", { name: "Atlas", exact: true }).click();
    await expect(page.locator(".pins button")).toHaveCount(200);
    await expect(page.locator("g.pin")).toHaveCount(200);
    await expect(page.getByTestId("schematic")).toBeVisible();
    const first = page.getByRole("button", { name: "Show Dense 00000 on the map" });
    await first.focus(); await first.press("End");
    const last = page.getByRole("button", { name: "Show Dense 09999 on the map" });
    await expect(last).toBeFocused();
    await expect(page.getByText("Page 50 of 50", { exact: true })).toBeVisible();
    await last.press("Enter");
    await expect(last).toHaveAttribute("aria-pressed", "true");
    await last.press("ArrowDown");
    await expect(first).toBeFocused();
    await expect(page.getByText("The selected pin is on another page. Change pages or search to show it.")).toBeVisible();
    await first.press("ArrowUp"); await expect(last).toBeFocused();
    await page.getByLabel("Search reference pins").fill("09999");
    await expect(last).toHaveAttribute("aria-pressed", "true");
    await expect(page.locator("g.pin")).toHaveCount(1);
    await page.getByLabel("Point type").selectOption("landmark");
    await expect(page.getByText("The selected pin is hidden by the current filters.")).toBeVisible();
    await expect(page.getByText("No reference pins match these filters.")).toBeVisible();
    await page.getByLabel("Point type").selectOption("all");
    const map = page.getByRole("group", { name: /Atlas map/ });
    await map.focus();
    const frame = map.locator("rect.frame");
    const fitted = await frame.getAttribute("width");
    await map.press("+");
    await expect(frame).not.toHaveAttribute("width", fitted ?? "");
    await map.press("0"); await expect(frame).toHaveAttribute("width", fitted ?? "");
    await expect(map).toBeFocused();
    expect(await page.locator(".atlas").evaluate((el) => el.scrollWidth <= el.clientWidth)).toBe(true);
    const box = await map.boundingBox();
    expect(box?.width).toBeLessThanOrEqual(width);
    await expect(page.getByText("Reference geography; these pins do not indicate player unlocks.")).toBeVisible();
    await expect(page.getByText("No imported snapshot: no position to show")).toBeVisible();
  });
}
