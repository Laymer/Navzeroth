import { act, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { createClient, type CommandAdapter } from "../commands";
import { PartyPlanning } from "./PartyPlanning";
import type { JointPlanningCatalog, JointPlanningReport } from "../party/contract";
import revision from "../../../../fixtures/desktop/content/demo-revision.json";
const catalog: JointPlanningCatalog = {
  context: { build: { edition: "wow_forever", build: "UNVERIFIED_DEMO", interface: "UNVERIFIED_DEMO" }, namespace: "demo" }, fixture: true, revision: "demo-r1",
  participants: ["one", "two"].map((character) => ({ character, profile_id: character, snapshot_id: `snap-${character}`, freshness: { status: "fresh", age_ms: 0 } })),
  nodes: revision.envelope.payload.nodes as JointPlanningCatalog["nodes"],
  objectives: revision.envelope.payload.objectives as JointPlanningCatalog["objectives"],
};
function fill() {
  fireEvent.change(screen.getByLabelText("Goal profile / utility unit"), { target: { value: "unit-v1" } });
  fireEvent.change(screen.getByLabelText("Shared origin"), { target: { value: "demo-node-1" } });
  fireEvent.change(screen.getByLabelText("Shared stopping point"), { target: { value: "demo-node-1" } });
  fireEvent.change(screen.getByLabelText("Session budget (minutes)"), { target: { value: "10" } });
  fireEvent.click(screen.getByLabelText("Include demo-objective-1"));
  fireEvent.change(screen.getByLabelText("Node for demo-objective-1"), { target: { value: "demo-node-1" } });
  fireEvent.change(screen.getByLabelText("Completion minutes for demo-objective-1"), { target: { value: "1" } });
  fireEvent.change(screen.getByLabelText("Utility for one on demo-objective-1"), { target: { value: "10" } });
  fireEvent.change(screen.getByLabelText("Utility for two on demo-objective-1"), { target: { value: "3" } });
}
const report = {
  request: { revision: "demo-r1" }, result: { plan: { selected: [], expected_total_ms: 0 }, assessments: [{ objective: "demo-objective-1", ready: false, participants: [
    { character: "one", utility: 10, readiness: { truth: "true", reasons: [] } },
    { character: "two", utility: 3, readiness: { truth: "unknown", reasons: [] } },
  ] }], unavailable_edges: [] }, assumptions: ["Manual shared origin assumption"], filtered: [],
} as unknown as JointPlanningReport;
it("requires manual fields and sends every member snapshot and utility", async () => {
  let request: unknown;
  const adapter: CommandAdapter = { invoke(command, input) {
    if (command === "get_joint_planning_catalog") return Promise.resolve(catalog as never);
    request = JSON.parse((input as { text: string }).text); return Promise.resolve(report as never);
  } };
  render(<PartyPlanning client={createClient(adapter)} partyId="duo" refreshKey={0} />);
  await screen.findByText(/SYNTHETIC dataset/);
  expect(screen.getByRole("button", { name: "Calculate joint estimate" })).toBeDisabled();
  fill(); fireEvent.click(screen.getByRole("button", { name: "Calculate joint estimate" }));
  await screen.findByText(/two: readiness unknown/);
  expect(request).toMatchObject({ party_id: "duo", participants: [{ profile_id: "one", snapshot_id: "snap-one" }, { profile_id: "two", snapshot_id: "snap-two" }], revision: "demo-r1", goal_profile: "unit-v1", origin: "demo-node-1", budget_ms: 600000, candidates: [{ utilities: { one: 10, two: 3 }, completion_ms: 60000 }] });
  expect(screen.getByText(/No jointly feasible objective/)).toBeInTheDocument();
});
it("blocks missing, stale and uncertain participants", async () => {
  const adapter: CommandAdapter = { invoke() { return Promise.resolve({ ...catalog, participants: [catalog.participants[0], { ...catalog.participants[1], freshness: { status: "uncertain", reason: "missing_or_untrusted_time" } }] } as never); } };
  render(<PartyPlanning client={createClient(adapter)} partyId="duo" refreshKey={0} />);
  await screen.findByText(/Missing, stale or uncertain age blocks/);
  fill(); expect(screen.getByRole("button", { name: "Calculate joint estimate" })).toBeDisabled();
});
it("ignores an old calculation after refresh while retaining manual edits", async () => {
  let resolve: ((value: never) => void) | undefined;
  const adapter: CommandAdapter = { invoke(command) {
    return command === "get_joint_planning_catalog" ? Promise.resolve(catalog as never) : new Promise<never>((done) => { resolve = done; });
  } };
  const client = createClient(adapter);
  const view = render(<PartyPlanning client={client} partyId="duo" refreshKey={0} />);
  await screen.findByText(/SYNTHETIC dataset/); fill();
  fireEvent.click(screen.getByRole("button", { name: "Calculate joint estimate" }));
  view.rerender(<PartyPlanning client={client} partyId="duo" refreshKey={1} />);
  await waitFor(() => expect(screen.queryByText(/Loading joint planning/)).not.toBeInTheDocument());
  await act(async () => { resolve?.(report as never); });
  expect(screen.queryByLabelText("Joint estimate")).not.toBeInTheDocument();
  expect(screen.getByLabelText("Goal profile / utility unit")).toHaveValue("unit-v1");
  expect(screen.getByLabelText("Utility for two on demo-objective-1")).toHaveValue(3);
});
