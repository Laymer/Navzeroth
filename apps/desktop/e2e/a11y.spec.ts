import AxeBuilder from "@axe-core/playwright";
import { expect, test } from "@playwright/test";
import type { ReferenceViewData } from "../src/commands/reference";
import report from "../../../fixtures/desktop/reference/imported.json" with { type: "json" };
import { installMock } from "./support/mockInvoke";
import { SCREENS } from "./support/screens";

// Automated accessibility scan (axe-core). Serious and critical violations fail the
// run unless they are listed here with the reason. The allowlist only ever shrinks:
// the design-system and screen reskin work (G4, G5) drives it to empty. It is empty
// today: the scan runs on the labelled demo adapter, which leaves several screens in
// their empty or unavailable state, so populated states are not scanned here.
// axe cannot judge everything (focus order, meaning, forced-colour legibility), so a
// pass here is a floor, not an accessibility sign-off.
interface Allowed {
  rule: string;
  screen: string;
  reason: string;
}
const ALLOWLIST: Allowed[] = [];

const MODES = ["default", "high-contrast", "forced-colors"] as const;
const WIDTHS = [320, 1100] as const;
const REFERENCE = "Imported reference (#reference)";

const key = (rule: string, screen: string) => `${rule}|${screen}`;
const allowed = new Set(ALLOWLIST.map((entry) => key(entry.rule, entry.screen)));
const matched = new Set<string>();

for (const width of WIDTHS) {
  for (const mode of MODES) {
    for (const screen of [...SCREENS, REFERENCE]) {
      test(`${screen} at ${width}px in ${mode} mode has no serious or critical violations`, async ({ page }) => {
        await page.setViewportSize({ width, height: 900 });
        if (mode === "forced-colors") await page.emulateMedia({ forcedColors: "active" });
        if (screen === REFERENCE) {
          await installMock(page, { get_reference_view: () => report as unknown as ReferenceViewData });
          await page.goto("/#reference");
          await expect(page.getByRole("heading", { name: "Imported reference", exact: true })).toBeVisible();
        } else {
          await page.goto("/");
          const nav = page.getByRole("navigation", { name: "Primary" });
          await nav.getByRole("button", { name: screen, exact: true }).click();
          await expect(nav.getByRole("button", { name: screen, exact: true })).toHaveAttribute("aria-current", "page");
        }
        if (mode === "high-contrast") await page.evaluate(() => { document.documentElement.dataset.contrast = "high"; });
        const { violations } = await new AxeBuilder({ page }).analyze();
        const serious = violations.filter((violation) => violation.impact === "serious" || violation.impact === "critical");
        const unexpected = serious.filter((violation) => {
          const id = key(violation.id, screen);
          if (allowed.has(id)) { matched.add(id); return false; }
          return true;
        });
        expect(
          unexpected.map((violation) => ({ rule: violation.id, impact: violation.impact, help: violation.help, nodes: violation.nodes.slice(0, 3).map((node) => node.target) })),
        ).toEqual([]);
      });
    }
  }
}

test.afterAll(() => {
  // Informational: an entry is only provably stale after every combination ran.
  for (const entry of ALLOWLIST) if (!matched.has(key(entry.rule, entry.screen))) console.log(`stale allowlist entry: ${entry.rule} on ${entry.screen}`);
});
