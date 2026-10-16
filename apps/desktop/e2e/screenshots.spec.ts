import { expect, test } from "@playwright/test";
import { SCREENS } from "./support/screens";

// Review artifact only. These are not pixel assertions: font rasterisation differs
// between WSL and CI, so baselines would flake. CI uploads the folder for a human to
// look at; the spec just proves every screen rendered something to capture.
for (const label of SCREENS) {
  test(`capture ${label}`, async ({ page }) => {
    await page.setViewportSize({ width: 1100, height: 760 });
    await page.goto("/");
    const nav = page.getByRole("navigation", { name: "Primary" });
    await nav.getByRole("button", { name: label, exact: true }).click();
    await expect(nav.getByRole("button", { name: label, exact: true })).toHaveAttribute("aria-current", "page");
    const slug = label.toLowerCase().replace(/[^a-z0-9]+/g, "-");
    await page.screenshot({ path: `screenshots/${slug}.png`, fullPage: true });
  });
}
