import { test, expect } from "@playwright/test";
import declaration from "../../../fixtures/desktop/submissions/synthetic-price.json" with { type: "json" };
test("local opt-in review refuses unpermitted export and preserves an erased duplicate tombstone", async ({ page }) => {
  await page.addInitScript(({ declaration }) => {
    const calls: { command: string; args: Record<string, unknown> }[] = [];
    let record: Record<string, unknown> | null = null;
    Object.defineProperty(window, "__SUBMISSION_CALLS__", { value: calls });
    Object.defineProperty(window, "__TAURI_INTERNALS__", { value: { invoke: async (command: string, args: Record<string, unknown>) => {
      calls.push({ command, args });
      if (command === "get_app_status") return { version: "test", data_mode: "stored", active_profile: null, capabilities: {} };
      if (command === "list_profiles") return [];
      if (command === "get_snapshot_watch") return { active: false, status: null, last_import: null, error: null };
      if (command === "list_local_submissions") return { rows: record ? [record] : [], total: record ? 1 : 0, next: null };
      if (command === "submit_local_observation") {
        if (!args.opt_in) throw { code: "INVALID_INPUT", message: "Explicit opt-in required", retryable: false };
        const duplicate = record !== null;
        record ??= { guard: { id: "a".repeat(64), revision: 1 }, base: { scope: { context: declaration.context, fixture: true }, revision: declaration.base_revision }, source: declaration.source, state: "pending", decision_code: "pending_review", document: declaration, created_at_utc_ms: null, changed_at_utc_ms: null };
        return { duplicate, record };
      }
      if (command === "change_local_submission" && record) {
        const request = JSON.parse(String(args.text)) as { expected: { revision: number }; action: string };
        const guard = record.guard as { id: string; revision: number };
        if (request.expected.revision !== guard.revision) throw { code: "INVALID_INPUT", message: "Refresh local guard", retryable: false };
        record = { ...record, guard: { ...guard, revision: guard.revision + 1 }, state: request.action === "accept" ? "accepted" : "withdrawn", document: request.action === "accept" ? declaration : null };
        return record;
      }
      if (command === "export_local_submission") throw { code: "PROVIDER_UNAVAILABLE", message: "Recorded source policy does not permit this local export", retryable: false };
      throw { code: "INVALID_INPUT", message: "Unexpected command mock", retryable: false };
    } } });
  }, { declaration });
  await page.goto("/");
  await page.getByRole("button", { name: "Content", exact: true }).click();
  await page.getByRole("button", { name: "Load synthetic price declaration" }).click();
  await expect(page.getByRole("button", { name: "Submit locally" })).toBeDisabled();
  await page.getByRole("checkbox", { name: /I opt in to local storage/ }).check();
  await page.getByRole("button", { name: "Submit locally" }).click();
  await expect(page.getByText("Declaration added to the local review queue.")).toBeVisible();
  await page.getByRole("button", { name: "Accept local declaration" }).click();
  await expect(page.getByText("Local decision: accepted.")).toBeVisible();
  await page.getByRole("button", { name: "Prepare permission-gated local export" }).click();
  await expect(page.getByRole("alert")).toContainText("Recorded source policy does not permit");
  await page.getByRole("button", { name: "Withdraw and erase payload" }).click();
  await page.getByText("Declaration and opaque fingerprint").click();
  await expect(page.getByText("Payload erased; fingerprint, source, base and local decision retained.")).toBeVisible();
  await page.getByRole("button", { name: "Load synthetic price declaration" }).click();
  await page.getByRole("checkbox", { name: /I opt in to local storage/ }).check();
  await page.getByRole("button", { name: "Submit locally" }).click();
  await expect(page.getByText("Already recorded: withdrawn. Erased payloads stay erased.")).toBeVisible();
  await expect(page.getByRole("button", { name: "Prepare permission-gated local export" })).toHaveCount(0);
  const calls = await page.evaluate(() => (window as unknown as { __SUBMISSION_CALLS__: { command: string; args: Record<string, unknown> }[] }).__SUBMISSION_CALLS__);
  expect(calls.filter((c) => c.command === "submit_local_observation")).toHaveLength(2);
  expect(calls.filter((c) => c.command === "export_local_submission")).toHaveLength(1);
  expect(JSON.parse(String(calls.find((c) => c.command === "export_local_submission")?.args.text))).toEqual({ id: "a".repeat(64), revision: 2 });
});
