import { render, screen, waitFor } from "@testing-library/react";
import { PartyEligibility, describeCatchUp } from "./PartyEligibility";
import { createClient, type CommandAdapter } from "../commands";
import type { PartyQuestCatalog, PartyQuestReport } from "../commands/contract";
const context = { namespace: "demo" as const, build: { edition: "wow_forever", build: "synthetic", interface: "synthetic" } };
it("passes the active revision and distinguishes catch-up from missing participant state", async () => {
  const calls: unknown[] = [];
  const report: PartyQuestReport = { context, fixture: true, revision: "r1", quest: "quest", joint_status: "missing_or_old_snapshots", participants: [
    { character: "one", snapshot_id: "snap", freshness: { status: "fresh", age_ms: 0 }, assessment: { acceptance: "behind", completion: "incomplete", progress: "in_progress", turn_in: "not_ready", catch_up: { kind: "single", requirement: { kind: "level", level: 23 } }, reasons: [{ phase: "acceptance", message: "Reach level 23" }] } },
    { character: "two", snapshot_id: null, freshness: null, assessment: null },
  ] };
  const adapter: CommandAdapter = { invoke(command, input) {
    calls.push({ command, input });
    return Promise.resolve((command === "get_party_quests" ? { context, fixture: true, revision: "r1", quests: ["quest"] } satisfies PartyQuestCatalog : report) as never);
  } };
  render(<PartyEligibility client={createClient(adapter)} partyId="duo" refreshKey={0} />);
  expect(await screen.findByText(/Joint advice is uncertain/)).toBeInTheDocument();
  expect(screen.getByText(/Missing imported snapshot; eligibility is unknown/)).toBeInTheDocument();
  expect(screen.getByText("Reach level 23")).toBeInTheDocument();
  expect(screen.getByText(/SYNTHETIC — engineering/)).toBeInTheDocument();
  expect(calls).toContainEqual({ command: "get_party_eligibility", input: { party_id: "duo", revision: "r1", quest_id: "quest" } });
});
it("never evaluates when no matching content is active", async () => {
  const calls: string[] = [];
  const adapter: CommandAdapter = { invoke(command) { calls.push(command); return Promise.resolve({ context, fixture: false, revision: null, quests: [] } as never); } };
  render(<PartyEligibility client={createClient(adapter)} partyId="duo" refreshKey={0} />);
  await screen.findByText(/No active content matches/);
  expect(calls).toEqual(["get_party_quests"]);
});
it("clears prior advice when the active revision changes and evaluation fails", async () => {
  let revision = "r1";
  const adapter: CommandAdapter = { invoke(command) {
    if (command === "get_party_quests") return Promise.resolve({ context, fixture: true, revision, quests: ["quest"] } as never);
    if (revision === "r2") return Promise.reject({ code: "INVALID_INPUT", message: "Active content changed; refresh the quest list", retryable: false });
    return Promise.resolve({ context, fixture: true, revision, quest: "quest", joint_status: "jointly_eligible", participants: [] } as never);
  } };
  const client = createClient(adapter); const view = render(<PartyEligibility client={client} partyId="duo" refreshKey={0} />);
  await screen.findByText(/All participants meet/);
  revision = "r2"; view.rerender(<PartyEligibility client={client} partyId="duo" refreshKey={1} />);
  await screen.findByRole("alert");
  await waitFor(() => expect(screen.queryByText(/All participants meet/)).not.toBeInTheDocument());
});
it("keeps alternative catch-up choices distinct from requirements that all apply", () => {
  expect(describeCatchUp({ kind: "one_of", alternatives: [{ kind: "single", requirement: { kind: "level", level: 23 } }, { kind: "single", requirement: { kind: "quest", quest: "q", state: "turned_in" } }] })).toBe("Choose an alternative: Reach level 23; Quest q: turned in");
});

it("reports failed catalog reads without evaluating or retaining advice", async () => {
  const calls:string[]=[];
  const adapter:CommandAdapter={invoke(command){calls.push(command);return Promise.reject({code:"STORAGE_ERROR",message:"Snapshot unavailable",retryable:true});}};
  render(<PartyEligibility client={createClient(adapter)} partyId="duo" refreshKey={0}/>);
  expect(await screen.findByRole("alert")).toHaveTextContent("Snapshot unavailable");
  expect(calls).toEqual(["get_party_quests"]);
  expect(screen.queryByText(/All participants meet/)).toBeNull();
});
it("explains conjunctions without collapsing the alternative branch or generic conditions", () => {
  expect(describeCatchUp({kind:"all_of",items:[{kind:"single",requirement:{kind:"level",level:25}},{kind:"single",requirement:{kind:"other"}}]})).toBe("All required: Reach level 25; Satisfy another condition");
});
