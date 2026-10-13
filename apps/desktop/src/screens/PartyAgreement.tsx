import { useEffect, useRef, useState } from "react";
import type { CommandClient } from "../commands";
import type { AppError } from "../commands/contract";
import type { JointPlanningReport } from "../party/contract";
import type { PartyProposalReport } from "../party/proposals";

export function PartyAgreement({ client, partyId, refreshKey, planning }: { client: CommandClient; partyId: string; refreshKey: number; planning: JointPlanningReport | null }) {
  const [proposal, setProposal] = useState<PartyProposalReport | null>(null);
  const [proposer, setProposer] = useState("");
  const [loading, setLoading] = useState(true);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<AppError | null>(null);
  const identity = `${partyId}:${refreshKey}`;
  const active = useRef({ identity, client, serial: 0 });
  const busyRef = useRef(false);
  if (active.current.identity !== identity || active.current.client !== client) {
    active.current = { identity, client, serial: active.current.serial + 1 };
    busyRef.current = false;
  }
  useEffect(() => {
    let mounted = true; const token = { ...active.current };
    setLoading(true); setProposal(null); setError(null); setBusy(false);
    void client.getPartyPlanProposal(partyId).then((result) => {
      if (!mounted || token.serial !== active.current.serial) return;
      setLoading(false);
      if (result.ok) setProposal(result.value); else setError(result.error);
    });
    return () => { mounted = false; active.current.serial += 1; };
  }, [client, partyId, refreshKey]);
  const canPropose = planning !== null && planning.result.plan.selected.length > 0 && planning.participants.some((p) => p.profile_id === proposer);
  async function mutate(kind: "propose" | "accept", profile?: string, accepted?: boolean) {
    if (busyRef.current || loading) return;
    if (kind === "propose" && !canPropose) return;
    if (kind === "accept" && (!proposal || !profile)) return;
    busyRef.current = true; setBusy(true); setError(null);
    const token = { ...active.current };
    const result = kind === "propose"
      ? await client.proposePartyPlan(JSON.stringify({ planning: planning?.request, proposer_profile_id: proposer, expected: proposal?.guard ?? null }))
      : await client.acceptPartyPlan(JSON.stringify({ party_id: partyId, profile_id: profile, expected: proposal?.guard, accepted }));
    if (token.serial !== active.current.serial || token.identity !== active.current.identity || token.client !== active.current.client) return;
    if (result.ok) { busyRef.current = false; setBusy(false); setProposal(result.value); return; }
    setError(result.error);
    const refreshed = await client.getPartyPlanProposal(partyId);
    if (token.serial !== active.current.serial || token.identity !== active.current.identity || token.client !== active.current.client) return;
    busyRef.current = false; setBusy(false);
    if (refreshed.ok) setProposal(refreshed.value);
    else setProposal((old) => old ? { ...old, state: "invalidated", reasons: [...old.reasons, "Agreement could not be refreshed after the failed change"] } : null);
  }
  return <section aria-labelledby="party-agreement-title">
    <h3 id="party-agreement-title">Local plan proposal and agreement</h3>
    <p>Record each participant's explicit agreement locally. These declarations do not confirm a remote identity or synchronize with the game. Proposing never accepts for another participant.</p>
    {loading && <p role="status">Loading local agreement…</p>}
    {error && <p role="alert">Error {error.code}: {error.message}</p>}
    <label>Proposing participant <select value={proposer} disabled={busy || loading || !planning} onChange={(e) => setProposer(e.target.value)}><option value="">Choose who proposes</option>{planning?.participants.map((p) => <option key={p.profile_id} value={p.profile_id}>{p.character}</option>)}</select></label>
    <button type="button" disabled={busy || loading || !canPropose} onClick={() => void mutate("propose")}>{proposal ? "Replace proposal with this estimate" : "Propose this estimate"}</button>
    {!proposal && !loading && <p>No local proposal. Calculate a jointly feasible estimate to propose a plan.</p>}
    {proposal && <div aria-label="Local agreement">
      <p>Plan {proposal.guard.proposal_id}; schema {proposal.schema_version}; agreement revision {proposal.guard.revision}.</p>
      <p>{proposal.state === "agreed" ? "Every participant explicitly accepted this current proposal." : proposal.state === "invalidated" ? "Agreement invalidated: current inputs differ or cannot support this advice." : "Awaiting explicit acceptance from every participant, including the proposer."}</p>
      {proposal.fixture && <p className="demo-label">SYNTHETIC proposal — engineering sample.</p>}
      <p>Proposed objectives: {proposal.selected_objectives.join(", ")}. Expected {proposal.expected_total_ms / 60_000} minutes.</p>
      <p>Manual origin {proposal.request.origin}; stopping point {proposal.request.stopping_node}; budget {proposal.request.budget_ms / 60_000} minutes; goal {proposal.request.goal_profile}; dataset {proposal.revision}.</p>
      <ul>{proposal.reasons.map((reason) => <li key={reason}>{reason}</li>)}</ul>
      {proposal.participants.map((p) => <div key={p.profile_id}>
        <h4>{p.profile_id}</h4>
        <p>{p.accepted ? "Local acceptance recorded" : "No local acceptance"}; expected snapshot {p.expected_snapshot_id}; current {p.current_snapshot_id ?? "missing"}.</p>
        <ul>{p.reasons.map((reason) => <li key={reason}>{reason}</li>)}</ul>
        <button type="button" disabled={busy || loading || p.accepted || proposal.state === "invalidated"} onClick={() => void mutate("accept", p.profile_id, true)}>Record acceptance for {p.profile_id}</button>
        <button type="button" disabled={busy || loading || !p.accepted} onClick={() => void mutate("accept", p.profile_id, false)}>Withdraw acceptance for {p.profile_id}</button>
        {p.progress_changes.length > 0 && <details><summary>Imported quest observation changes for {p.profile_id}</summary><pre>{JSON.stringify(p.progress_changes, null, 2)}</pre></details>}
      </div>)}
      <details><summary>Review exact proposed assumptions and participant inputs</summary><pre>{JSON.stringify(proposal.request, null, 2)}</pre></details>
    </div>}
  </section>;
}
