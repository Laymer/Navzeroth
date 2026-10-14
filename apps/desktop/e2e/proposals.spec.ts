import { expect, test } from "@playwright/test";
import snapshot from "../../../fixtures/contracts/v1/character-snapshot.json" with { type: "json" };
test("local proposal retains per-member acceptance and invalidates after refresh", async ({ page }) => {
  await page.addInitScript(({ snapshot }) => {
    const calls: { command: string; args: Record<string, unknown> }[] = [];
    let revision = 1; const accepted = new Set<string>(); let invalidated = false;
    const request = { party_id: "duo", participants: [{ profile_id: "one", snapshot_id: "s1" }, { profile_id: "two", snapshot_id: "s2" }], revision: "r1", goal_profile: "manual-unit", origin: "start", stopping_node: "stop", budget_ms: 600000, stopping_service_ms: 0, candidates: [], excluded: [] };
    const report = () => ({ party_id: "duo", schema_version: 1, guard: { proposal_id: "a".repeat(64), revision }, proposer_profile_id: "one", context: snapshot.payload.context, fixture: true, revision: "r1", state: invalidated ? "invalidated" : accepted.size === 2 ? "agreed" : "awaiting_acceptance", reasons: invalidated ? ["Participant snapshot changed"] : [], selected_objectives: ["joint-step"], expected_total_ms: 60000, created_at_utc_ms: 1, request, participants: ["one", "two"].map((profile_id, index) => ({ profile_id, expected_snapshot_id: `s${index + 1}`, current_snapshot_id: `s${index + 1}`, accepted: accepted.has(profile_id), accepted_at_utc_ms: null, freshness: { status: "fresh", age_ms: 0 }, reasons: [], progress_changes: [] })) });
    Object.defineProperty(window, "__PROPOSAL_CALLS__", { value: calls });
    Object.defineProperty(window, "__TAURI_INTERNALS__", { value: { invoke: async (command: string, args: Record<string, unknown>) => {
      calls.push({ command, args });
      switch (command) {
        case "get_app_status": return { version: "test", data_mode: "stored", active_profile: null, capabilities: {} };
        case "list_profiles": return ["one", "two"].map((id) => ({ id, display_name: id, fixture: true, data_mode: "stored", context: snapshot.payload.context }));
        case "list_parties": return ["duo"];
        case "get_party": return { id: "duo", data_mode: "stored", participants: ["one", "two"].map((id) => ({ character: id, profile_id: id, snapshot: null })) };
        case "get_party_quests": return { context: snapshot.payload.context, fixture: true, revision: null, quests: [] };
        case "get_joint_planning_catalog": return { context: snapshot.payload.context, fixture: true, revision: null, participants: [], nodes: [], objectives: [] };
        case "get_party_plan_proposal": if (accepted.size === 2) invalidated = true; return report();
        case "accept_party_plan": {
          const input = JSON.parse(String(args.text));
          if (input.expected.revision !== revision || input.expected.proposal_id !== "a".repeat(64)) throw { code: "INVALID_INPUT", message: "Proposal changed", retryable: false };
          if (input.accepted) accepted.add(input.profile_id); else accepted.delete(input.profile_id);
          revision += 1; return report();
        }
        default: throw { code: "INVALID_INPUT", message: "Unexpected mock command", retryable: false };
      }
    } } });
  }, { snapshot });
  await page.goto("/"); await page.getByRole("button", { name: "Party", exact: true }).click();
  await expect(page.getByText(/Awaiting explicit acceptance from every participant/)).toBeVisible();
  await page.getByRole("button", { name: "Record acceptance for one" }).click();
  await expect(page.getByRole("button", { name: "Record acceptance for one" })).toBeDisabled();
  await expect(page.getByRole("button", { name: "Record acceptance for two" })).toBeEnabled();
  await page.getByRole("button", { name: "Record acceptance for two" }).click();
  await expect(page.getByText("Every participant explicitly accepted this current proposal.")).toBeVisible();
  await page.getByRole("button", { name: "Refresh parties" }).click();
  await expect(page.getByText(/Agreement invalidated: current inputs differ/)).toBeVisible();
  await expect(page.getByRole("button", { name: "Record acceptance for two" })).toBeDisabled();
  await page.getByRole("button", { name: "Withdraw acceptance for one" }).click();
  await expect(page.getByRole("button", { name: "Withdraw acceptance for one" })).toBeDisabled();
  const calls = await page.evaluate(() => (window as unknown as { __PROPOSAL_CALLS__: { command: string; args: { text: string } }[] }).__PROPOSAL_CALLS__);
  const inputs = calls.filter((c) => c.command === "accept_party_plan").map((c) => JSON.parse(c.args.text));
  expect(inputs).toMatchObject([{ profile_id: "one", accepted: true, expected: { revision: 1 } }, { profile_id: "two", accepted: true, expected: { revision: 2 } }, { profile_id: "one", accepted: false, expected: { revision: 3 } }]);
});
