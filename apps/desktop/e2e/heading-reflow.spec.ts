import { expect, test } from "@playwright/test";

for (const fontSize of ["100%", "200%"]) {
  test(`main headings reflow at 320px and ${fontSize} text`, async ({ page }) => {
    await page.setViewportSize({ width: 320, height: 1000 });
    await page.addInitScript(() => {
      Object.defineProperty(window, "__TAURI_INTERNALS__", { value: { invoke: async (command: string) => {
        if (command === "get_app_status") return { version: "synthetic-reflow", data_mode: "stored", active_profile: null, capabilities: {} };
        if (command === "get_snapshot_watch") return { active: false, status: null, last_import: null, error: null };
        throw { code: "PROVIDER_UNAVAILABLE", message: "Synthetic unavailable fixture", retryable: false };
      } } });
    });
    await page.goto("/");
    await expect(page.getByRole("heading", { name: "Capabilities", exact: true })).toBeVisible();
    await page.evaluate(size => { document.documentElement.style.fontSize = size; }, fontSize);
    for (const name of ["Azeroth Navigator", "Imported character reference", "Capabilities"]) {
      const heading = page.getByRole("heading", { name, exact: true });
      await expect(heading).toBeVisible();
      expect(await heading.evaluate(element => element.scrollWidth <= element.clientWidth + 1)).toBe(true);
      expect(await heading.evaluate(element => element.getBoundingClientRect().right <= window.innerWidth + 1)).toBe(true);
    }
  });
}
