import { expect, test, type Locator, type Page } from "@playwright/test";
import atlas from "../../../fixtures/desktop/atlas/query-atlas.json" with { type: "json" };

// G6 O4 and O5: a pressed Button (aria-pressed="true") stays distinct from an unpressed one in
// default, high-contrast and forced-colours mode, and list buttons (Atlas pins) carry no
// form-button margin. Route hop buttons share the list rule; workflow.spec.ts checks their margin.

async function atlasWithPins(page: Page) {
  // Buttons transition border and background colour; reduced motion removes that so styles are final.
  await page.emulateMedia({ reducedMotion: "reduce" });
  await page.addInitScript(({ atlas }) => {
    const base = atlas.images[0];
    if (!base) throw new Error("Missing synthetic atlas image");
    const pins = Array.from({ length: 3 }, (_, i) => ({ ...base.pins[0], id: `pressed-${i}`, label: `Pressed ${i}`, pixel: { x: (i + 1) * base.width_px / 4, y: base.height_px / 2 } }));
    Object.defineProperty(window, "__TAURI_INTERNALS__", { value: { invoke: async (command: string) => {
      switch (command) {
        case "get_app_status": return { version: "synthetic-pressed", data_mode: "stored", active_profile: null, capabilities: {} };
        case "get_snapshot_watch": return { active: false, status: null, last_import: null, error: null };
        case "get_snapshot_markers": return [];
        case "query_atlas": return { ...atlas, revision: "synthetic-pressed", images: [{ ...base, layers: [], pins }] };
        default: throw new Error(`Unexpected mock command: ${command}`);
      }
    } } });
  }, { atlas });
  await page.goto("/");
  await page.getByRole("button", { name: "Atlas", exact: true }).click();
  const pressed = page.getByRole("button", { name: "Show Pressed 0 on the map", exact: true });
  const other = page.getByRole("button", { name: "Show Pressed 1 on the map", exact: true });
  await pressed.click();
  await expect(pressed).toHaveAttribute("aria-pressed", "true");
  await expect(other).toHaveAttribute("aria-pressed", "false");
  return { pressed, other };
}

const style = (button: Locator) => button.evaluate((el) => {
  const s = getComputedStyle(el);
  return { weight: s.fontWeight, decoration: s.textDecorationLine, border: s.borderTopColor, shadow: s.boxShadow, background: s.backgroundColor, color: s.color, margin: s.margin };
});

test("pressed pin is distinct by shape and weight, and list buttons have no form margin", async ({ page }) => {
  const { pressed, other } = await atlasWithPins(page);
  const [on, off] = [await style(pressed), await style(other)];
  expect(on.weight).toBe("700");
  expect(on.decoration).toBe("underline");
  expect(off.decoration).toBe("none");
  expect(on.shadow).toContain("inset");
  expect(on.margin).toBe("0px");
  expect(off.margin).toBe("0px");
});

test("high contrast keeps the pressed edge, not just bold and underline", async ({ page }) => {
  const { pressed, other } = await atlasWithPins(page);
  await page.evaluate(() => document.documentElement.setAttribute("data-contrast", "high"));
  const [on, off] = [await style(pressed), await style(other)];
  expect(on.border).toBe("rgb(255, 213, 74)"); // --gold in high contrast
  expect(off.border).toBe("rgb(255, 255, 255)"); // --text
  expect(on.shadow).toContain("inset");
  expect(off.shadow).toBe("none");
  expect(on.decoration).toBe("underline");
});

test("forced colours draw a pressed button with the system selection pair", async ({ page }) => {
  const { pressed, other } = await atlasWithPins(page);
  await page.emulateMedia({ reducedMotion: "reduce", forcedColors: "active" });
  expect(await page.evaluate(() => matchMedia("(forced-colors: active)").matches)).toBe(true);
  const [on, off] = [await style(pressed), await style(other)];
  const system = await page.evaluate(() => {
    const probe = document.createElement("span");
    document.body.append(probe);
    probe.style.backgroundColor = "Highlight";
    probe.style.color = "HighlightText";
    const s = getComputedStyle(probe);
    const result = { highlight: s.backgroundColor, highlightText: s.color };
    probe.remove();
    return result;
  });
  expect(on.background).toBe(system.highlight);
  expect(on.color).toBe(system.highlightText);
  expect(off.background).not.toBe(system.highlight);
  expect(on.weight).toBe("700");
  expect(on.decoration).toBe("underline");
  expect(off.decoration).toBe("none");
});
