import { ClusterSummary } from "./ClusterSummary";
import { runPlanningJob, type PlanningJobStatus } from "../commands/planningJobs";
import { useEffect, useRef, useState } from "react";
import { PartyAgreement } from "./PartyAgreement";
import type { CommandClient } from "../commands";
import type { AppError } from "../commands/contract";
import type { JointEstimate, JointPlanningCatalog, JointPlanningReport, JointPlanningRequest } from "../party/contract";

type EstimateDraft = { node: string; minutes: string; service: string; utilities: Record<string, string>; included: boolean; excluded: boolean };
function duration(text: string): number | null {
  const number = Number(text);
  return text.trim() && Number.isFinite(number) && number >= 0 && Number.isSafeInteger(number * 60_000) ? number * 60_000 : null;
}
function utility(text: string): number | null {
  const number = Number(text);
  return text.trim() && Number.isInteger(number) && number >= 0 && number <= 4_294_967_295 ? number : null;
}
export function PartyPlanning({ client, partyId, refreshKey }: { client: CommandClient; partyId: string; refreshKey: number }) {
  const [catalog, setCatalog] = useState<JointPlanningCatalog | null>(null);
  const [drafts, setDrafts] = useState<Record<string, EstimateDraft>>({});
  const [origin, setOrigin] = useState("");
  const [stop, setStop] = useState("");
  const [budget, setBudget] = useState("");
  const [stopService, setStopService] = useState("0");
  const [goal, setGoal] = useState("");
  const [report, setReport] = useState<JointPlanningReport | null>(null);
  const [error, setError] = useState<AppError | null>(null);
  const [busy, setBusy] = useState(false);
  const [loading, setLoading] = useState(true);
  const activeJob = useRef<AbortController | null>(null);
  const inputVersion = useRef(0);
  const reportOwner = useRef<{ client: CommandClient; identity: string; inputVersion: number } | null>(null);
  const [progress, setProgress] = useState<{ edit: number; status: PlanningJobStatus } | null>(null);
  const abortJob = () => { activeJob.current?.abort(); activeJob.current = null; setProgress(null); };

  const identity = `${partyId}:${refreshKey}`;
  const current = useRef({ identity, client, edit: 0 });
  if (current.current.identity !== identity || current.current.client !== client) {
    current.current = { identity, client, edit: current.current.edit + 1 }; inputVersion.current += 1;
  }
  const visibleReport = report && reportOwner.current?.client === client && reportOwner.current.identity === identity && reportOwner.current.inputVersion === inputVersion.current ? report : null;
  const catalogOwner = useRef<{ client: CommandClient; identity: string } | null>(null);
  const visibleCatalog = catalog && catalogOwner.current?.client === client && catalogOwner.current.identity === identity ? catalog : null;
  const catalogRef = useRef(visibleCatalog);
  catalogRef.current = visibleCatalog;
  useEffect(() => {
    let active = true;
    abortJob(); setLoading(true); setReport(null); setError(null); setBusy(false);
    void client.getJointPlanningCatalog(partyId).then((response) => {
      if (!active) return;
      setLoading(false);
      if (!response.ok) { setCatalog(null); setError(response.error); return; }
      catalogOwner.current = { client, identity }; setCatalog(response.value);
      setOrigin((old) => response.value.nodes.some((n) => n.id === old) ? old : "");
      setStop((old) => response.value.nodes.some((n) => n.id === old) ? old : "");
      setDrafts((old) => Object.fromEntries(response.value.objectives.map((objective) => {
        const previous = old[objective.id];
        return [objective.id, {
          node: previous && response.value.nodes.some((n) => n.id === previous.node) ? previous.node : "",
          minutes: previous?.minutes ?? "", service: previous?.service ?? "0", included: previous?.included ?? false, excluded: previous?.excluded ?? false,
          utilities: Object.fromEntries(response.value.participants.map((p) => [p.profile_id, previous?.utilities[p.profile_id] ?? ""])),
        }];
      })));
    });
    return () => { active = false; activeJob.current?.abort(); activeJob.current = null; };
  }, [client, partyId, refreshKey]);
  function edit(action: () => void) { current.current.edit += 1; inputVersion.current += 1; abortJob(); setBusy(false); setReport(null); setError(null); action(); }
  function candidate(id: string, patch: Partial<EstimateDraft>) { edit(() => setDrafts((old) => old[id] ? ({ ...old, [id]: { ...old[id], ...patch } }) : old)); }
  const expected = visibleCatalog?.participants.flatMap((p) => p.snapshot_id ? [{ profile_id: p.profile_id, snapshot_id: p.snapshot_id }] : []) ?? [];
  const fresh = visibleCatalog !== null && expected.length === visibleCatalog.participants.length && visibleCatalog.participants.every((p) => p.freshness?.status === "fresh");
  const estimates: JointEstimate[] = [];
  let validCandidates = true;
  for (const [id, draft] of Object.entries(drafts)) {
    if (!draft.included) continue;
    const completion = duration(draft.minutes); const service = duration(draft.service);
    const values = Object.fromEntries(Object.entries(draft.utilities).map(([key, value]) => [key, utility(value)]));
    if (!draft.node || completion === null || service === null || Object.values(values).some((v) => v === null)) { validCandidates = false; continue; }
    estimates.push({ objective: id, node: draft.node, completion_ms: completion, service_ms: service, utilities: values as Record<string, number> });
  }
  const valid = fresh && visibleCatalog?.revision && origin && stop && goal.trim() && duration(budget) !== null && duration(stopService) !== null && validCandidates && estimates.length > 0;
  async function calculate() {
    if (!valid || !visibleCatalog?.revision || busy || loading || activeJob.current) return;
    current.current.edit += 1;
    const token = { ...current.current }; const source = visibleCatalog; const sourceInputVersion = inputVersion.current;
    const request: JointPlanningRequest = { party_id: partyId, participants: expected, revision: visibleCatalog.revision,
      goal_profile: goal.trim(), origin, stopping_node: stop, budget_ms: duration(budget) ?? 0,
      stopping_service_ms: duration(stopService) ?? 0, candidates: estimates, excluded: Object.entries(drafts).filter(([,d]) => d.excluded).map(([id]) => id) };
    const controller = new AbortController(); activeJob.current = controller;
    setBusy(true); setProgress(null); setError(null);
    const response = await runPlanningJob<JointPlanningReport>(client, "joint", JSON.stringify(request), controller.signal, (status) => { if (token.identity === current.current.identity && token.client === current.current.client && token.edit === current.current.edit && !controller.signal.aborted) setProgress({ edit: token.edit, status }); });
    if (token.identity !== current.current.identity || token.client !== current.current.client || token.edit !== current.current.edit || source !== catalogRef.current) return;
    activeJob.current = null; setBusy(false); setProgress(null);
    if (response.ok) { reportOwner.current = { client, identity, inputVersion: sourceInputVersion }; setReport(response.value); } else { setError(response.error); if (response.error.code === "SNAPSHOT_OUTDATED" || response.error.code === "BUILD_MISMATCH") setReport(null); }
  }
  function cancelCalculation() { current.current.edit += 1; abortJob(); setBusy(false); setError({ code: "OPERATION_CANCELED", message: "Planning request canceled locally; cancellation requested for its background job.", retryable: false }); }

  const chooseNode = (label: string, value: string, change: (value: string) => void) => <label>{label} <select value={value} disabled={loading} onChange={(e) => edit(() => change(e.target.value))}><option value="">Choose a declared node</option>{visibleCatalog?.nodes.map((n) => <option key={n.id} value={n.id}>{n.id}</option>)}</select></label>;
  return <section aria-labelledby="joint-planning-title">
    <h3 id="joint-planning-title">Manual joint session planning</h3>
    <p>Choose explicit shared travel assumptions and same-unit utility for each participant. Every selected objective must be ready and benefit every member. Review catch-up reasons above when progress differs; this planner does not predict unlocks or quest sharing.</p>
    {loading && <p role="status">Loading joint planning inputs…</p>}
    {error && <p role="alert">Error {error.code}: {error.message}</p>}
    {visibleCatalog?.fixture && <p className="demo-label">SYNTHETIC dataset — engineering demonstration.</p>}
    {visibleCatalog && !visibleCatalog.revision && <p>No active content revision for this party.</p>}
    {visibleCatalog && !fresh && <p>Joint planning requires a fresh trusted imported snapshot from every member. Missing, stale or uncertain age blocks calculation.</p>}
    {visibleCatalog && <fieldset disabled={loading}>
      <legend>Manual shared assumptions</legend>
      <label>Goal profile / utility unit <input value={goal} onChange={(e) => edit(() => setGoal(e.target.value))} maxLength={128} /></label>
      {chooseNode("Shared origin", origin, setOrigin)}{chooseNode("Shared stopping point", stop, setStop)}
      <label>Session budget (minutes) <input type="number" min="0" value={budget} onChange={(e) => edit(() => setBudget(e.target.value))} /></label>
      <label>Stopping service time (minutes) <input type="number" min="0" value={stopService} onChange={(e) => edit(() => setStopService(e.target.value))} /></label>
      {visibleCatalog.objectives.length === 0 && <p>No authored objectives in this dataset.</p>}
      {visibleCatalog.objectives.map((objective) => {
        const draft = drafts[objective.id]; if (!draft) return null;
        return <fieldset key={objective.id}><legend>{objective.id}</legend>
          <p>Authored participants: {objective.participants.join(", ")}</p>
          <label><input type="checkbox" checked={draft.included} onChange={(e) => candidate(objective.id, { included: e.target.checked })} /> Include {objective.id}</label>
          <label><input type="checkbox" checked={draft.excluded} onChange={(e) => candidate(objective.id, { excluded: e.target.checked })} /> Exclude {objective.id} from the plan</label>
          {chooseNode(`Node for ${objective.id}`, draft.node, (node) => candidate(objective.id, { node }))}
          <label>Completion minutes for {objective.id} <input type="number" min="0" value={draft.minutes} onChange={(e) => candidate(objective.id, { minutes: e.target.value })} /></label>
          <label>Service minutes for {objective.id} <input type="number" min="0" value={draft.service} onChange={(e) => candidate(objective.id, { service: e.target.value })} /></label>
          {visibleCatalog.participants.map((p) => <label key={p.profile_id}>Utility for {p.character} on {objective.id} <input type="number" min="0" step="1" value={draft.utilities[p.profile_id] ?? ""} onChange={(e) => candidate(objective.id, { utilities: { ...draft.utilities, [p.profile_id]: e.target.value } })} /></label>)}
        </fieldset>;
      })}
      <button type="button" disabled={!valid || busy || loading} onClick={() => void calculate()}>Calculate joint estimate</button>
    </fieldset>}
    {activeJob.current && busy && <div aria-label="Joint planning job progress">
      <p role="status">{progress?.edit === current.current.edit ? `${progress.status.state}; ${progress.status.phase?.replaceAll("_", " ") ?? "awaiting worker"}; ${progress.status.work_units} work units checked` : "Registering joint planning job…"}</p>
      <button type="button" onClick={cancelCalculation}>Cancel joint planning</button>
      {visibleReport && <p>Previous valid joint estimate remains visible while this calculation runs.</p>}
    </div>}
    {visibleReport && <div aria-label="Joint estimate">
      <ClusterSummary clustering={visibleReport.clustering} />
      <p>Dataset {visibleReport.request.revision}; expected {visibleReport.result.plan.expected_total_ms / 60_000} minutes. Minimum participant total utility within each same-node cluster determines priority.</p>
      <ol>{visibleReport.result.plan.selected.map((choice) => <li key={choice.candidate}>{choice.candidate}: minimum member contribution {choice.utility}, expected session {choice.expected_session_ms / 60_000} minutes</li>)}</ol>
      {visibleReport.result.plan.selected.length === 0 && <p>No jointly feasible objective fits the supplied inputs and stopping route.</p>}
      <ul>{visibleReport.assumptions.map((a) => <li key={a}>{a}</li>)}</ul>
      <ul>{visibleReport.filtered.map((f) => <li key={f.objective}>{f.objective}: {f.reason}</li>)}</ul>
      {visibleReport.result.assessments.map((a) => <div key={a.objective}><h4>{a.objective}: {a.ready ? "jointly ready" : "joint progress blocked or unknown"}</h4><ul>{a.participants.map((p) => <li key={p.character}>{p.character}: readiness {p.readiness.truth}; declared utility {p.utility}{p.utility === 0 ? " — no progress contribution; excluded by fairness constraint" : ""}</li>)}</ul></div>)}
      <details><summary>Joint readiness and decision evidence</summary><pre>{JSON.stringify({ participants: visibleReport.participants, assessments: visibleReport.result.assessments, trace: visibleReport.result.plan.trace, stopping: visibleReport.result.plan.stopping }, null, 2)}</pre></details>
      <ul>{visibleReport.result.unavailable_edges.map((edge) => <li key={`${edge.edge}:${edge.character}`}>Travel {edge.edge} unavailable for {edge.character}: {edge.truth}</li>)}</ul>
    </div>}
    <PartyAgreement client={client} partyId={partyId} refreshKey={refreshKey} planning={visibleReport} />
  </section>;
}
