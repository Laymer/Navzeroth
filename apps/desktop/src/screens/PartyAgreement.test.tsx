import { act, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { createClient, type CommandAdapter } from "../commands";
import { PartyAgreement } from "./PartyAgreement";
import type { JointPlanningReport } from "../party/contract";
import type { PartyProposalReport } from "../party/proposals";
const request = { party_id: "duo", participants: [{ profile_id: "one", snapshot_id: "s1" }, { profile_id: "two", snapshot_id: "s2" }], revision: "r1", goal_profile: "unit", origin: "origin", stopping_node: "stop", budget_ms: 600000, stopping_service_ms: 0, candidates: [], excluded: [] };
const planning = { request, participants: [{ profile_id: "one", character: "One" }, { profile_id: "two", character: "Two" }], result: { plan: { selected: [{ candidate: "joint-step" }] } } } as unknown as JointPlanningReport;
function initial(): PartyProposalReport {
  return { party_id: "duo", schema_version: 1, guard: { proposal_id: "a".repeat(64), revision: 1 }, proposer_profile_id: "one", context: { build: { edition: "wow_forever", build: "synthetic", interface: "synthetic" }, namespace: "demo" }, fixture: true, revision: "r1", state: "awaiting_acceptance", reasons: [], request,
    selected_objectives: ["joint-step"], expected_total_ms: 60000, created_at_utc_ms: 1,
    participants: ["one", "two"].map((profile_id, index) => ({ profile_id, expected_snapshot_id: `s${index + 1}`, current_snapshot_id: `s${index + 1}`, accepted: false, accepted_at_utc_ms: null, freshness: { status: "fresh", age_ms: 0 }, reasons: [], progress_changes: [] })) };
}
it("proposes for one explicit participant then requires every local acceptance and permits withdrawal", async () => {
  let stored: PartyProposalReport | null = null; const calls: { command: string; text: unknown }[] = [];
  const adapter: CommandAdapter = { invoke(command, input) {
    if (command === "get_party_plan_proposal") return Promise.resolve(stored as never);
    const payload = JSON.parse((input as { text: string }).text); calls.push({ command, text: payload });
    if (command === "propose_party_plan") stored = initial();
    if (command === "accept_party_plan" && stored) {
      expect(payload.expected).toEqual(stored.guard);
      stored = { ...stored, guard: { ...stored.guard, revision: stored.guard.revision + 1 }, participants: stored.participants.map((p) => p.profile_id === payload.profile_id ? { ...p, accepted: payload.accepted } : p) };
      stored.state = stored.participants.every((p) => p.accepted) ? "agreed" : "awaiting_acceptance";
    }
    return Promise.resolve(stored as never);
  } };
  render(<PartyAgreement client={createClient(adapter)} partyId="duo" refreshKey={0} planning={planning} />);
  await screen.findByText(/No local proposal/);
  expect(screen.getByRole("button", { name: "Propose this estimate" })).toBeDisabled();
  fireEvent.change(screen.getByLabelText("Proposing participant"), { target: { value: "one" } });
  fireEvent.click(screen.getByRole("button", { name: "Propose this estimate" }));
  await screen.findByText(/Awaiting explicit acceptance/);
  expect(screen.getAllByText(/No local acceptance/)).toHaveLength(2);
  expect(calls[0]).toMatchObject({ command: "propose_party_plan", text: { planning: request, proposer_profile_id: "one", expected: null } });
  fireEvent.click(screen.getByRole("button", { name: "Record acceptance for one" }));
  await waitFor(() => expect(screen.getByRole("button", { name: "Record acceptance for one" })).toBeDisabled());
  expect(screen.queryByText(/Every participant explicitly accepted/)).not.toBeInTheDocument();
  fireEvent.click(screen.getByRole("button", { name: "Record acceptance for two" }));
  await screen.findByText(/Every participant explicitly accepted/);
  fireEvent.click(screen.getByRole("button", { name: "Withdraw acceptance for one" }));
  await screen.findByText(/Awaiting explicit acceptance/);
  expect(calls.filter((c) => c.command === "accept_party_plan")).toHaveLength(3);
});
it("invalidated agreements block acceptance but retain explicit withdrawal", async () => {
  const invalid = initial(); invalid.state = "invalidated"; if (invalid.participants[0]) invalid.participants[0].accepted = true;
  const adapter: CommandAdapter = { invoke() { return Promise.resolve(invalid as never); } };
  render(<PartyAgreement client={createClient(adapter)} partyId="duo" refreshKey={0} planning={null} />);
  await screen.findByText(/Agreement invalidated/);
  expect(screen.getByRole("button", { name: "Record acceptance for two" })).toBeDisabled();
  expect(screen.getByRole("button", { name: "Withdraw acceptance for one" })).toBeEnabled();
});
it("rejects a deferred acceptance response after a newer refresh", async () => {
  let invalidate = false; let finish: ((value: never) => void) | undefined;
  const adapter: CommandAdapter = { invoke(command) {
    if (command === "accept_party_plan") return new Promise<never>((resolve) => { finish = resolve; });
    return Promise.resolve({ ...initial(), state: invalidate ? "invalidated" : "awaiting_acceptance" } as never);
  } };
  const client = createClient(adapter); const view = render(<PartyAgreement client={client} partyId="duo" refreshKey={0} planning={planning} />);
  await screen.findByText(/Awaiting explicit acceptance/);
  fireEvent.click(screen.getByRole("button", { name: "Record acceptance for one" }));
  invalidate = true; view.rerender(<PartyAgreement client={client} partyId="duo" refreshKey={1} planning={null} />);
  await screen.findByText(/Agreement invalidated/);
  await act(async () => { finish?.({ ...initial(), state: "agreed" } as never); });
  expect(screen.queryByText(/Every participant explicitly accepted/)).not.toBeInTheDocument();
});
it("refreshes authoritative agreement after a guard conflict", async () => {
  let conflict = false;
  const adapter: CommandAdapter = { invoke(command) {
    if (command === "accept_party_plan") { conflict = true; return Promise.reject({ code: "INVALID_INPUT", message: "Proposal changed", retryable: false }); }
    return Promise.resolve({ ...initial(), state: conflict ? "invalidated" : "awaiting_acceptance" } as never);
  } };
  render(<PartyAgreement client={createClient(adapter)} partyId="duo" refreshKey={0} planning={planning} />);
  await screen.findByText(/Awaiting explicit acceptance/);
  fireEvent.click(screen.getByRole("button", { name: "Record acceptance for one" }));
  await screen.findByText(/Agreement invalidated/);
  expect(screen.getByRole("alert")).toHaveTextContent("Proposal changed");
});

it("does not claim absence or permit replacement when the proposal read failed", async () => {
  const adapter: CommandAdapter = { invoke() { return Promise.reject({ code: "STORAGE_ERROR", message: "Stored proposal unavailable", retryable: false }); } };
  render(<PartyAgreement client={createClient(adapter)} partyId="duo" refreshKey={0} planning={planning} />);
  await screen.findByRole("alert");
  fireEvent.change(screen.getByLabelText("Proposing participant"), { target: { value: "one" } });
  expect(screen.getByRole("button", { name: "Propose this estimate" })).toBeDisabled();
  expect(screen.queryByText(/No local proposal/)).not.toBeInTheDocument();
});

it("exports only after all local acceptances for the explicitly chosen recipient and current guard", async () => {
  const agreed = initial(); agreed.state = "agreed"; agreed.guard.revision = 3;
  agreed.participants = agreed.participants.map((p) => ({ ...p, accepted: true }));
  const calls: unknown[] = [];
  const adapter: CommandAdapter = { invoke(command, input) {
    if (command === "export_agreed_party_plan") { calls.push(input); return Promise.resolve({ text: "ANAV1:recipient-only", byte_count: 20, plan_id: "recipient-plan", expires_at_utc_ms: 100 } as never); }
    return Promise.resolve(agreed as never);
  } };
  render(<PartyAgreement client={createClient(adapter)} partyId="duo" refreshKey={0} planning={null} />);
  await screen.findByText(/Every participant explicitly accepted/);
  const button = screen.getByRole("button", { name: "Export agreed plan for recipient" });
  expect(button).toBeDisabled();
  fireEvent.change(screen.getByLabelText("Export recipient"), { target: { value: "two" } });
  fireEvent.click(button);
  await screen.findByLabelText("Recipient plan text");
  expect(JSON.parse((calls[0] as { text: string }).text)).toEqual({ party_id: "duo", expected: agreed.guard, recipient_profile_id: "two" });
  expect(screen.getByLabelText("Recipient plan text")).toHaveValue("ANAV1:recipient-only");
  fireEvent.change(screen.getByLabelText("Export recipient"), { target: { value: "one" } });
  expect(screen.queryByLabelText("Recipient plan text")).not.toBeInTheDocument();
});
it("blocks export for awaiting or invalidated agreement", async () => {
  const adapter: CommandAdapter = { invoke() { return Promise.resolve(initial() as never); } };
  render(<PartyAgreement client={createClient(adapter)} partyId="duo" refreshKey={0} planning={null} />);
  await screen.findByText(/Awaiting explicit acceptance/);
  expect(screen.getByLabelText("Export recipient")).toBeDisabled();
  expect(screen.getByRole("button", { name: "Export agreed plan for recipient" })).toBeDisabled();
});
it("discards late recipient export after refresh and refreshes agreement on backend invalidation", async () => {
  let agreed = initial(); agreed = { ...agreed, state: "agreed", participants: agreed.participants.map((p) => ({ ...p, accepted: true })) };
  let finish: ((value: never) => void) | undefined; let invalidated = false;
  const adapter: CommandAdapter = { invoke(command) {
    if (command === "export_agreed_party_plan") return new Promise<never>((resolve) => { finish = resolve; });
    return Promise.resolve({ ...agreed, state: invalidated ? "invalidated" : "agreed" } as never);
  } };
  const client = createClient(adapter); const view = render(<PartyAgreement client={client} partyId="duo" refreshKey={0} planning={null} />);
  await screen.findByText(/Every participant explicitly accepted/);
  fireEvent.change(screen.getByLabelText("Export recipient"), { target: { value: "one" } });
  fireEvent.click(screen.getByRole("button", { name: "Export agreed plan for recipient" }));
  invalidated = true; view.rerender(<PartyAgreement client={client} partyId="duo" refreshKey={1} planning={null} />);
  await screen.findByText(/Agreement invalidated/);
  await act(async () => finish?.({ text: "ANAV1:obsolete", byte_count: 14, plan_id: "obsolete", expires_at_utc_ms: 100 } as never));
  expect(screen.queryByLabelText("Recipient plan text")).not.toBeInTheDocument();
});
it("backend export refusal refreshes authoritative agreement and exposes no export bytes", async () => {
  const agreed = { ...initial(), state: "agreed" as const, participants: initial().participants.map((p) => ({ ...p, accepted: true })) }; let failed = false;
  const adapter: CommandAdapter = { invoke(command) {
    if (command === "export_agreed_party_plan") { failed = true; return Promise.reject({ code: "SNAPSHOT_OUTDATED", message: "Agreement changed", retryable: false }); }
    return Promise.resolve({ ...agreed, state: failed ? "invalidated" : "agreed" } as never);
  } };
  render(<PartyAgreement client={createClient(adapter)} partyId="duo" refreshKey={0} planning={null} />);
  await screen.findByText(/Every participant explicitly accepted/);
  fireEvent.change(screen.getByLabelText("Export recipient"), { target: { value: "one" } });
  fireEvent.click(screen.getByRole("button", { name: "Export agreed plan for recipient" }));
  await screen.findByText(/Agreement invalidated/);
  expect(screen.getByRole("alert")).toHaveTextContent("Agreement changed");
  expect(screen.queryByLabelText("Recipient plan text")).not.toBeInTheDocument();
});
