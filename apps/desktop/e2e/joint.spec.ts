import { expect, test } from "@playwright/test";
import content from "../../../fixtures/desktop/content/demo-revision.json" with { type: "json" };
import snapshot from "../../../fixtures/contracts/v1/character-snapshot.json" with { type: "json" };

test("joint planning sends exact participant guards and hard exclusions, then invalidates refreshed advice", async ({ page }) => {
  await page.addInitScript(({ content, snapshot }) => {
    const calls: { command: string; args: Record<string, unknown> }[] = [];
    let stale = false;
    let planningReport: unknown = null;
    const profiles = ["one", "two"];
    const members = () => profiles.map((character) => ({ character, profile_id: character, snapshot_id: `snapshot-${character}`, freshness: stale ? { status: "uncertain", reason: "missing_or_untrusted_time" } : { status: "fresh", age_ms: 0 } }));
    Object.defineProperty(window, "__JOINT_CALLS__", { value: calls });
    Object.defineProperty(window, "__TAURI_INTERNALS__", { value: { invoke: async (command: string, args: Record<string, unknown>) => {
      calls.push({ command, args });
      switch (command) {
        case "get_app_status": return { version: "test", data_mode: "stored", active_profile: null, capabilities: {} };
        case "list_profiles": return profiles.map((id) => ({ id, display_name: id, data_mode: "stored", context: snapshot.payload.context, fixture: true }));
        case "list_parties": return ["duo"];
        case "get_party": return { id: "duo", data_mode: "stored", participants: profiles.map((id) => ({ character: id, profile_id: id, snapshot: { envelope: { ...snapshot, payload: { ...snapshot.payload, id: `snapshot-${id}`, character: id } }, profile_id: id, data_mode: "stored", freshness: { status: "fresh", age_ms: 0 } } })) };
        case "get_party_plan_proposal": return null;
        case "get_party_quests": return { context: snapshot.payload.context, fixture: true, revision: null, quests: [] };
        case "get_joint_planning_catalog": return { context: snapshot.payload.context, fixture: true, revision: "demo-r1", participants: members(), nodes: content.envelope.payload.nodes, objectives: content.envelope.payload.objectives };
        case "start_planning_job": {
          if (args.kind !== "joint") throw new Error("Wrong planning kind");
          const request = JSON.parse(String(args.text));
          planningReport = { request, context: snapshot.payload.context, fixture: true, participants: members(), filtered: [], assumptions: ["Manual utility and shared origin"], result: { assessments: [], unavailable_edges: [], plan: { selected: [], expected_total_ms: 0, estimated: true, alternatives: [], trace: [], termination: "no_feasible_candidate", stopping: { total_ms: 0 } } } };
          return { job_schema_version: 1, job_id: "joint-mock-job" };
        }
        case "get_planning_job": return { job_schema_version: 1, job_id: args.job_id, kind: "joint", state: "succeeded", cancel_requested: false, phase: "selection", work_units: 8, result_available: true, error: null };
        case "take_planning_job_result": { const result = { job_schema_version: 1, job_id: args.job_id, kind: "joint", report_json: JSON.stringify(planningReport) }; stale = true; return result; }
        case "cancel_planning_job": return null;
        default: throw { code: "INVALID_INPUT", message: "Unexpected mock command", retryable: false };
      }
    } } });
  }, { content, snapshot });
  await page.goto("/");
  await page.getByRole("button", { name: "Party", exact: true }).click();
  await expect(page.getByRole("heading", { name: "Manual joint session planning" })).toBeVisible();
  const calculate = page.getByRole("button", { name: "Calculate joint estimate" });
  await expect(calculate).toBeDisabled();
  await page.getByLabel("Goal profile / utility unit", { exact: true }).fill("manual-goal-v1");
  await page.getByLabel("Shared origin", { exact: true }).selectOption("demo-node-1");
  await page.getByLabel("Shared stopping point", { exact: true }).selectOption("demo-node-1");
  await page.getByLabel("Session budget (minutes)", { exact: true }).fill("10");
  await page.getByLabel("Include demo-objective-1", { exact: true }).check();
  await page.getByLabel("Exclude demo-objective-1 from the plan", { exact: true }).check();
  await page.getByLabel("Node for demo-objective-1", { exact: true }).selectOption("demo-node-1");
  await page.getByLabel("Completion minutes for demo-objective-1", { exact: true }).fill("1");
  await page.getByLabel("Utility for one on demo-objective-1", { exact: true }).fill("10");
  await page.getByLabel("Utility for two on demo-objective-1", { exact: true }).fill("3");
  await calculate.click();
  await expect(page.getByText("No jointly feasible objective fits the supplied inputs and stopping route.")).toBeVisible();
  const calls = await page.evaluate(() => (window as unknown as { __JOINT_CALLS__: { command: string; args: Record<string, unknown> }[] }).__JOINT_CALLS__);
  const invocation = calls.find((c) => c.command === "start_planning_job");
  expect(invocation?.args.kind).toBe("joint");
  expect(JSON.parse(String(invocation?.args.text))).toMatchObject({ party_id: "duo", revision: "demo-r1", excluded: ["demo-objective-1"], participants: [{ profile_id: "one", snapshot_id: "snapshot-one" }, { profile_id: "two", snapshot_id: "snapshot-two" }], candidates: [{ utilities: { one: 10, two: 3 } }] });
  await page.getByRole("button", { name: "Refresh parties" }).click();
  await expect(page.getByText(/Missing, stale or uncertain age blocks calculation/)).toBeVisible();
  await expect(page.getByLabel("Joint estimate", { exact: true })).toHaveCount(0);
  await expect(calculate).toBeDisabled();
  await expect(page.getByLabel("Utility for two on demo-objective-1", { exact: true })).toHaveValue("3");
});
