import { expect, test } from "@playwright/test";
import { fail, installMock } from "./support/mockInvoke";
import type { CharacterSnapshotEnvelope } from "../../../schemas/v1/exchange";
import type { ManualProgressionRequest, ProgressionRecord, ProgressionView } from "../../../schemas/v1/progression-api";
import snapshot from "../../../fixtures/contracts/v1/character-snapshot.json" with { type: "json" };
import companion from "../../../fixtures/contracts/v1/progression-snapshot.json" with { type: "json" };

// Browser journey across the real React/typed command boundary; mock storage only.
test("companion import and manual input preserve origins and survive UI reload", async ({ page }) => {
  const profile = "synthetic-profile";
  const freshness = { status: "uncertain", reason: "missing_or_untrusted_time" } as const;
  const imported: ProgressionRecord[] = []; const manual: ProgressionRecord[] = [];
  const binding = companion.payload.binding as ProgressionView["binding"];
  let saved: ManualProgressionRequest | null = null;
  const { calls } = await installMock(page, {
    get_app_status: () => ({ version: "synthetic-test", data_mode: "stored", active_profile: profile, capabilities: {} }),
    list_profiles: () => [{ id: profile, display_name: "Synthetic character", context: binding.context, fixture: true, data_mode: "stored", snapshot_count: 1 }],
    get_stored_snapshot: () => ({ envelope: snapshot as CharacterSnapshotEnvelope, profile_id: profile, freshness, data_mode: "stored" }),
    get_snapshot_history: () => [],
    get_snapshot_watch: () => ({ active: false, status: null, last_import: null, error: null }),
    get_progression_view: () => ({ profile_id: profile, fixture: true, binding, imported, manual }),
    import_progression_text: ({ text }) => {
      const request = JSON.parse(text) as { text: string; expected: unknown };
      if (request.text !== "ANAV1:synthetic-companion") throw fail("INVALID_INPUT", "Invalid progression export");
      expect(request.expected).toEqual(binding);
      imported.push({ snapshot: companion.payload as ProgressionRecord["snapshot"], freshness });
      return { profile_id: profile, snapshot_id: binding.snapshot, companion_id: companion.payload.id, stored: true, fixture: true };
    },
    save_manual_progression: ({ text }) => {
      saved = JSON.parse(text) as ManualProgressionRequest;
      expect(saved.expected).toEqual(binding);
      manual.push({ snapshot: { ...companion.payload as ProgressionRecord["snapshot"], id: saved.id, money: saved.money, available_talent_points: saved.available_talent_points,
        recipes: { status: "unknown" }, bags: { status: "unknown" }, bank: { status: "unknown" } }, freshness });
      return { profile_id: profile, snapshot_id: binding.snapshot, companion_id: saved.id, stored: true, fixture: true };
    },
  });
  await page.goto("/");
  await page.getByRole("button", { name: "Imports", exact: true }).click();
  const panel = page.getByRole("region", { name: "Progression inputs", exact: true });
  await expect(panel.getByRole("button", { name: "Open progression inputs" })).toBeEnabled();
  expect(calls.filter((v) => v === "get_progression_view")).toEqual([]);
  await panel.getByRole("button", { name: "Open progression inputs" }).click();
  await expect(panel.getByText(/No imported progression observations/)).toBeVisible();
  await panel.getByLabel("Progression export text", { exact: true }).fill("invalid-companion");
  await panel.getByRole("button", { name: "Import progression companion", exact: true }).click();
  await expect(panel.getByRole("alert")).toContainText("Invalid progression export");
  await panel.getByLabel("Progression export text", { exact: true }).fill("ANAV1:synthetic-companion");
  await panel.getByRole("button", { name: "Import progression companion", exact: true }).click();
  await expect(panel.getByRole("alert")).toHaveCount(0);
  await panel.getByText(/Imported capture/).click();
  await expect(panel.getByText("0 recorded entries; complete enumeration")).toBeVisible();
  await expect(panel.getByText(/partial enumeration, other entries unknown/)).toBeVisible();
  await panel.getByLabel("Money (copper)", { exact: true }).fill("0");
  await panel.getByRole("button", { name: "Save manual entry" }).click();
  await expect(panel.getByText(/Saved progression capture: manual-/)).toBeVisible();
  expect(saved).toMatchObject({ money: { status: "known", value: "0" }, available_talent_points: { status: "unknown" }, recipes: { status: "unknown" } });
  await page.reload();
  await page.getByRole("button", { name: "Imports", exact: true }).click();
  await panel.getByRole("button", { name: "Open progression inputs" }).click();
  const records = panel.getByRole("region", { name: "Manual progression observations" });
  await records.getByText(/Manual capture/).click();
  await expect(records.getByText("0 copper")).toBeVisible();
  await expect(records.getByText("Source: Synthetic fixture.")).toBeVisible();
  await expect(records.getByText("Unknown (not observed)").first()).toBeVisible();
  await expect(panel.getByText(/Imported capture/)).toBeVisible();
});
