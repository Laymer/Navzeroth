import { test, expect } from "@playwright/test";
// @ts-expect-error Node driver helper has no desktop runtime dependency/declaration.
import { waitForSyntheticContentInspection } from "../../../scripts/windows/native-smoke-lib.mjs";
import { readFileSync } from "node:fs";
const golden = JSON.parse(readFileSync(new URL("../../../fixtures/desktop/content-governance/synthetic-flow.json", import.meta.url), "utf8")) as { profiles: {id:string}[]; inspection: {key:{revision:string;scope:{fixture:boolean;context:{build:{edition:string;build:string;interface:string};namespace:string}}}} };
const first = golden.profiles[0];
if (!first) throw Error("Missing Rust-written golden profile");
const expected = { profile_id: first.id, ...golden.inspection.key };
const text = "demo-r1: wow_forever/UNVERIFIED_DEMO/UNVERIFIED_DEMO, demo";

test.beforeEach(async ({ page }) => {
  await page.goto("/e2e/support/native-upgrade-selection.html");
  await expect(page.getByText(text, { exact: true })).toBeVisible();
});

test("old unconditional same selection clears actual React inspection without refetch", async ({ page }) => {
  const readCount = () => page.evaluate(() => (window as unknown as {__selectionRegression:{calls:string[]}}).__selectionRegression.calls.filter(c=>c.startsWith("get_active_content")).length);
  const reads = await readCount();
  await page.getByLabel("Content profile", { exact: true }).selectOption(first.id);
  await expect(page.getByText("No active content for this profile.", { exact: true })).toBeVisible();
  await expect(page.getByText(text, { exact: true })).toHaveCount(0);
  await page.waitForTimeout(200); // Greater than the controlled adapter's100ms delay.
  expect(await readCount()).toBe(reads);
});

test("production helper preserves selected inspection and switches other profile with actual refetch", async ({ page }) => {
  let changes = 0;
  await page.exposeFunction("recordSelectionChange", () => { changes++; });
  await page.getByLabel("Content profile", { exact: true }).evaluate(el => el.addEventListener("change", () => { void (window as unknown as {recordSelectionChange:()=>Promise<void>}).recordSelectionChange(); }));
  await waitForSyntheticContentInspection(page, expected, 1000);
  expect(changes).toBe(0);
  await expect(page.getByText(text, { exact: true })).toBeVisible();
  const secondId = `stored-${"b".repeat(64)}`;
  await waitForSyntheticContentInspection(page, { ...expected, profile_id: secondId, revision: "other-r1" }, 1000);
  expect(changes).toBe(1);
  await expect(page.getByText("other-r1: wow_forever/UNVERIFIED_DEMO/UNVERIFIED_DEMO, demo", { exact: true })).toBeVisible();
  expect(await page.evaluate(() => (window as unknown as {__selectionRegression:{calls:string[]}}).__selectionRegression.calls)).toContain(`get_active_content:${secondId}`);
});

test("production helper still refuses absent or wrong inspection within its bound", async ({ page }) => {
  await expect(waitForSyntheticContentInspection(page, { ...expected, revision: "wrong-revision" }, 150)).rejects.toThrow();
  await expect(waitForSyntheticContentInspection(page, { ...expected, scope: { ...expected.scope, context: { ...expected.scope.context, namespace: "reference" } } }, 150)).rejects.toThrow();
  await expect(waitForSyntheticContentInspection(page, { ...expected, profile_id: `stored-${"c".repeat(64)}` }, 300)).rejects.toThrow();
  await expect(page.getByText("No active content for this profile.", { exact: true })).toBeVisible();
});
