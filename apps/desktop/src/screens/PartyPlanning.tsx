import { useEffect, useRef, useState } from "react";
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
  const identity = `${partyId}:${refreshKey}`;
  const current = useRef({ identity, client, edit: 0 });
  if (current.current.identity !== identity || current.current.client !== client) {
    current.current = { identity, client, edit: current.current.edit + 1 };
  }
  const catalogRef = useRef(catalog);
  catalogRef.current = catalog;
  useEffect(() => {
    let active = true;
    setLoading(true); setReport(null); setError(null); setBusy(false);
    void client.getJointPlanningCatalog(partyId).then((response) => {
      if (!active) return;
      setLoading(false);
      if (!response.ok) { setCatalog(null); setError(response.error); return; }
      setCatalog(response.value);
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
    return () => { active = false; };
  }, [client, partyId, refreshKey]);
  function edit(action: () => void) { current.current.edit += 1; setReport(null); setError(null); action(); }
  function candidate(id: string, patch: Partial<EstimateDraft>) { edit(() => setDrafts((old) => old[id] ? ({ ...old, [id]: { ...old[id], ...patch } }) : old)); }
  const expected = catalog?.participants.flatMap((p) => p.snapshot_id ? [{ profile_id: p.profile_id, snapshot_id: p.snapshot_id }] : []) ?? [];
  const fresh = catalog !== null && expected.length === catalog.participants.length && catalog.participants.every((p) => p.freshness?.status === "fresh");
  const estimates: JointEstimate[] = [];
  let validCandidates = true;
  for (const [id, draft] of Object.entries(drafts)) {
    if (!draft.included) continue;
    const completion = duration(draft.minutes); const service = duration(draft.service);
    const values = Object.fromEntries(Object.entries(draft.utilities).map(([key, value]) => [key, utility(value)]));
    if (!draft.node || completion === null || service === null || Object.values(values).some((v) => v === null)) { validCandidates = false; continue; }
    estimates.push({ objective: id, node: draft.node, completion_ms: completion, service_ms: service, utilities: values as Record<string, number> });
  }
  const valid = fresh && catalog?.revision && origin && stop && goal.trim() && duration(budget) !== null && duration(stopService) !== null && validCandidates && estimates.length > 0;
  async function calculate() {
    if (!valid || !catalog?.revision || busy || loading) return;
    const token = { ...current.current }; const source = catalog;
    const request: JointPlanningRequest = { party_id: partyId, participants: expected, revision: catalog.revision,
      goal_profile: goal.trim(), origin, stopping_node: stop, budget_ms: duration(budget) ?? 0,
      stopping_service_ms: duration(stopService) ?? 0, candidates: estimates, excluded: Object.entries(drafts).filter(([,d]) => d.excluded).map(([id]) => id) };
    setBusy(true); setReport(null); setError(null);
    const response = await client.planJointSession(JSON.stringify(request));
    if (token.identity !== current.current.identity || token.client !== current.current.client || token.edit !== current.current.edit || source !== catalogRef.current) return;
    setBusy(false);
    if (response.ok) setReport(response.value); else setError(response.error);
  }
  const chooseNode = (label: string, value: string, change: (value: string) => void) => <label>{label} <select value={value} disabled={busy || loading} onChange={(e) => edit(() => change(e.target.value))}><option value="">Choose a declared node</option>{catalog?.nodes.map((n) => <option key={n.id} value={n.id}>{n.id}</option>)}</select></label>;
  return <section aria-labelledby="joint-planning-title">
    <h3 id="joint-planning-title">Manual joint session planning</h3>
    <p>Choose explicit shared travel assumptions and same-unit utility for each participant. Every selected objective must be ready and benefit every member. Review catch-up reasons above when progress differs; this planner does not predict unlocks or quest sharing.</p>
    {loading && <p role="status">Loading joint planning inputs…</p>}
    {error && <p role="alert">Error {error.code}: {error.message}</p>}
    {catalog?.fixture && <p className="demo-label">SYNTHETIC dataset — engineering demonstration.</p>}
    {catalog && !catalog.revision && <p>No active content revision for this party.</p>}
    {catalog && !fresh && <p>Joint planning requires a fresh trusted imported snapshot from every member. Missing, stale or uncertain age blocks calculation.</p>}
    {catalog && <fieldset disabled={busy || loading}>
      <legend>Manual shared assumptions</legend>
      <label>Goal profile / utility unit <input value={goal} onChange={(e) => edit(() => setGoal(e.target.value))} maxLength={128} /></label>
      {chooseNode("Shared origin", origin, setOrigin)}{chooseNode("Shared stopping point", stop, setStop)}
      <label>Session budget (minutes) <input type="number" min="0" value={budget} onChange={(e) => edit(() => setBudget(e.target.value))} /></label>
      <label>Stopping service time (minutes) <input type="number" min="0" value={stopService} onChange={(e) => edit(() => setStopService(e.target.value))} /></label>
      {catalog.objectives.length === 0 && <p>No authored objectives in this dataset.</p>}
      {catalog.objectives.map((objective) => {
        const draft = drafts[objective.id]; if (!draft) return null;
        return <fieldset key={objective.id}><legend>{objective.id}</legend>
          <p>Authored participants: {objective.participants.join(", ")}</p>
          <label><input type="checkbox" checked={draft.included} onChange={(e) => candidate(objective.id, { included: e.target.checked })} /> Include {objective.id}</label>
          <label><input type="checkbox" checked={draft.excluded} onChange={(e) => candidate(objective.id, { excluded: e.target.checked })} /> Exclude {objective.id} from the plan</label>
          {chooseNode(`Node for ${objective.id}`, draft.node, (node) => candidate(objective.id, { node }))}
          <label>Completion minutes for {objective.id} <input type="number" min="0" value={draft.minutes} onChange={(e) => candidate(objective.id, { minutes: e.target.value })} /></label>
          <label>Service minutes for {objective.id} <input type="number" min="0" value={draft.service} onChange={(e) => candidate(objective.id, { service: e.target.value })} /></label>
          {catalog.participants.map((p) => <label key={p.profile_id}>Utility for {p.character} on {objective.id} <input type="number" min="0" step="1" value={draft.utilities[p.profile_id] ?? ""} onChange={(e) => candidate(objective.id, { utilities: { ...draft.utilities, [p.profile_id]: e.target.value } })} /></label>)}
        </fieldset>;
      })}
      <button type="button" disabled={!valid || busy || loading} onClick={() => void calculate()}>Calculate joint estimate</button>
    </fieldset>}
    {busy && <p role="status">Calculating joint estimate…</p>}
    {report && <div aria-label="Joint estimate">
      <p>Dataset {report.request.revision}; expected {report.result.plan.expected_total_ms / 60_000} minutes. Minimum per-member utility determines priority.</p>
      <ol>{report.result.plan.selected.map((choice) => <li key={choice.candidate}>{choice.candidate}: group utility {choice.utility}, expected session {choice.expected_session_ms / 60_000} minutes</li>)}</ol>
      {report.result.plan.selected.length === 0 && <p>No jointly feasible objective fits the supplied inputs and stopping route.</p>}
      <ul>{report.assumptions.map((a) => <li key={a}>{a}</li>)}</ul>
      <ul>{report.filtered.map((f) => <li key={f.objective}>{f.objective}: {f.reason}</li>)}</ul>
      {report.result.assessments.map((a) => <div key={a.objective}><h4>{a.objective}: {a.ready ? "jointly ready" : "joint progress blocked or unknown"}</h4><ul>{a.participants.map((p) => <li key={p.character}>{p.character}: readiness {p.readiness.truth}; declared utility {p.utility}{p.utility === 0 ? " — no progress contribution; excluded by fairness constraint" : ""}</li>)}</ul></div>)}
      <details><summary>Joint readiness and decision evidence</summary><pre>{JSON.stringify({ participants: report.participants, assessments: report.result.assessments, trace: report.result.plan.trace, stopping: report.result.plan.stopping }, null, 2)}</pre></details>
      <ul>{report.result.unavailable_edges.map((edge) => <li key={`${edge.edge}:${edge.character}`}>Travel {edge.edge} unavailable for {edge.character}: {edge.truth}</li>)}</ul>
    </div>}
  </section>;
}
