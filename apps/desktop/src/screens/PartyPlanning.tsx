import { Button, Field, Panel } from "../ui";
import { CatalogPager, OffPageOptions } from "./CatalogPager";
import {PlanPinControl} from "./PlanPinControl";
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
  const previousCatalogBinding=useRef("");
  const [catalog, setCatalog] = useState<JointPlanningCatalog | null>(null);
  const [drafts, setDrafts] = useState<Record<string, EstimateDraft>>({});
  const [origin, setOrigin] = useState("");
  const [stop, setStop] = useState("");
  const [budget, setBudget] = useState("");
  const [stopService, setStopService] = useState("0");
  const [goal, setGoal] = useState("");
  const [pinJob,setPinJob]=useState<string|null>(null);
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
    abortJob(); setLoading(true); setReport(null); setPinJob(null); setError(null); setBusy(false);
    void client.getJointPlanningCatalog(partyId).then((response) => {
      if (!active) return;
      setLoading(false);
      if (!response.ok) { setCatalog(null); setError(response.error); return; }
      catalogOwner.current = { client, identity }; setCatalog(response.value);
      const binding=JSON.stringify([partyId,response.value.revision,response.value.pages,response.value.participants.map(p=>p.profile_id)]);
      if(response.value.pages&&previousCatalogBinding.current===binding)return;
      previousCatalogBinding.current=binding;
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
  function edit(action: () => void) { current.current.edit += 1; inputVersion.current += 1; abortJob(); setBusy(false); setReport(null); setPinJob(null); setError(null); action(); }
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
  async function applyDefaults(){edit(()=>{});const token={...current.current};const response=await client.getAppSettings();if(token.identity!==current.current.identity||token.client!==current.current.client||token.edit!==current.current.edit)return;if(!response.ok){setError(response.error);return;}const d=response.value.settings.manual_planning_defaults;if(d.goal_profile!==null)setGoal(d.goal_profile);setBudget(String(d.budget_ms/60000));setStopService(String(d.stopping_service_ms/60000));}
  async function calculate() {
    if (!valid || !visibleCatalog?.revision || busy || loading || activeJob.current) return;
    current.current.edit += 1;
    const token = { ...current.current }; const source = visibleCatalog; const sourceInputVersion = inputVersion.current;
    const request: JointPlanningRequest = { party_id: partyId, participants: expected, revision: visibleCatalog.revision,
      goal_profile: goal.trim(), origin, stopping_node: stop, budget_ms: duration(budget) ?? 0,
      stopping_service_ms: duration(stopService) ?? 0, candidates: estimates, excluded: Object.entries(drafts).filter(([,d]) => d.excluded).map(([id]) => id) };
    const controller = new AbortController(); activeJob.current = controller;
    setBusy(true); setProgress(null); setError(null);
    const response = await runPlanningJob<JointPlanningReport>(client, "joint", JSON.stringify(request), controller.signal, (status) => { if (token.identity === current.current.identity && token.client === current.current.client && token.edit === current.current.edit && !controller.signal.aborted) setProgress({ edit: token.edit, status }); }, undefined, id=>{if(token.identity===current.current.identity&&token.client===current.current.client&&token.edit===current.current.edit&&!controller.signal.aborted)setPinJob(id);});
    if (token.identity !== current.current.identity || token.client !== current.current.client || token.edit !== current.current.edit || source !== catalogRef.current) return;
    activeJob.current = null; setBusy(false); setProgress(null);
    if (response.ok) { reportOwner.current = { client, identity, inputVersion: sourceInputVersion }; setReport(response.value); } else { setError(response.error); if (response.error.code === "SNAPSHOT_OUTDATED" || response.error.code === "BUILD_MISMATCH") setReport(null); setPinJob(null); }
  }
  function cancelCalculation() { current.current.edit += 1; abortJob(); setBusy(false); setError({ code: "OPERATION_CANCELED", message: "Planning request canceled locally; cancellation requested for its background job.", retryable: false }); }

  const chooseNode = (label: string, value: string, change: (value: string) => void) => <Field label={label} help="A declared node from the active content. Leave it unselected if you do not know it; nothing is guessed.">{(c) => <select {...c} value={value} disabled={loading} onChange={(e) => edit(() => change(e.target.value))}><OffPageOptions selected={[value]} visible={visibleCatalog?.nodes.map(n=>n.id)??[]}/><option value="">Choose a declared node</option>{visibleCatalog?.nodes.map((n) => <option key={n.id} value={n.id}>{n.id}</option>)}</select>}</Field>;
  return <Panel title="Manual joint session planning" level={3}>
    <p>Choose explicit shared travel assumptions and same-unit utility for each participant. Every selected objective must be ready and benefit every member. Review catch-up reasons above when progress differs; this planner does not predict unlocks or quest sharing.</p>
    {loading && <p role="status">Loading joint planning inputs…</p>}
    {error && <p role="alert">Error {error.code}: {error.message}</p>}
    {visibleCatalog?.fixture && <p className="demo-label">SYNTHETIC dataset — engineering demonstration.</p>}
    {visibleCatalog && !visibleCatalog.revision && <p>No active content revision for this party.</p>}
    {visibleCatalog && !fresh && <p>Joint planning requires a fresh trusted imported snapshot from every member. Missing, stale or uncertain age blocks calculation.</p>}
    {visibleCatalog && <fieldset disabled={loading}>
      <legend>Manual shared assumptions</legend>
      <Field label="Goal profile / utility unit" help="Names the unit your utility numbers use. It labels your assumption and is not verified game advice.">{(c) => <input {...c} value={goal} onChange={(e) => edit(() => setGoal(e.target.value))} maxLength={128} />}</Field>
      <CatalogPager disabled={busy} client={client} subject={{kind:"party",party_id:partyId}} page={visibleCatalog?.pages?.nodes} label="nodes" onPage={p=>{setCatalog(old=>old?{...old,nodes:p.records as NonNullable<typeof catalog>["nodes"],pages:{...old.pages,nodes:p}}:old);}}/>
<CatalogPager disabled={busy} client={client} subject={{kind:"party",party_id:partyId}} page={visibleCatalog?.pages?.objectives} label="objectives" onPage={p=>{setDrafts(old=>({...Object.fromEntries((p.records as NonNullable<typeof catalog>["objectives"]).map(o=>[o.id,{node:"",minutes:"",service:"0",included:false,excluded:false,utilities:Object.fromEntries((visibleCatalog?.participants??[]).map(member=>[member.profile_id,""]))}])),...Object.fromEntries(Object.entries(old).filter(([id,d])=>d.included||d.excluded||p.records.some(r=>(r as {id:string}).id===id)))}));setCatalog(old=>old?{...old,objectives:p.records as NonNullable<typeof catalog>["objectives"],pages:{...old.pages,objectives:p}}:old);}}/>
{chooseNode("Shared origin", origin, setOrigin)}{chooseNode("Shared stopping point", stop, setStop)}
      <Button disabled={busy} onClick={()=>void applyDefaults()}>Apply local planning defaults</Button>
      <Field label="Session budget (minutes)" help="Shared time limit for the whole group in minutes.">{(c) => <input {...c} type="number" min="0" value={budget} onChange={(e) => edit(() => setBudget(e.target.value))} />}</Field>
      <Field label="Stopping service time (minutes)" help="Extra minutes you expect to spend after the group reaches the stopping point. 0 means none declared.">{(c) => <input {...c} type="number" min="0" value={stopService} onChange={(e) => edit(() => setStopService(e.target.value))} />}</Field>
      <p>{Object.values(drafts).filter(d=>d.included).length} selected objectives; {Object.values(drafts).filter(d=>d.excluded).length} exclusions (maximum 128 each).</p><Button variant="ghost" onClick={()=>edit(()=>setDrafts(old=>Object.fromEntries(Object.entries(old).filter(([id])=>visibleCatalog.objectives.some(o=>o.id===id)).map(([id,d])=>[id,{...d,included:false,excluded:false}]))))}>Clear objective selections</Button>
      {visibleCatalog.objectives.length === 0 && <p>No authored objectives in this dataset.</p>}
      {visibleCatalog.objectives.map((objective) => {
        const draft = drafts[objective.id]; if (!draft) return null;
        return <fieldset key={objective.id}><legend>{objective.id}</legend>
          <p>Authored participants: {objective.participants.join(", ")}</p>
          <label><input type="checkbox" checked={draft.included} disabled={!draft.included&&Object.values(drafts).filter(d=>d.included).length>=128} onChange={(e) => candidate(objective.id, { included: e.target.checked })} /> Include {objective.id}</label>
          <label><input type="checkbox" checked={draft.excluded} disabled={!draft.excluded&&Object.values(drafts).filter(d=>d.excluded).length>=128} onChange={(e) => candidate(objective.id, { excluded: e.target.checked })} /> Exclude {objective.id} from the plan</label>
          {chooseNode(`Node for ${objective.id}`, draft.node, (node) => candidate(objective.id, { node }))}
          <label>Completion minutes for {objective.id} <input type="number" min="0" value={draft.minutes} onChange={(e) => candidate(objective.id, { minutes: e.target.value })} /></label>
          <label>Service minutes for {objective.id} <input type="number" min="0" value={draft.service} onChange={(e) => candidate(objective.id, { service: e.target.value })} /></label>
          {visibleCatalog.participants.map((p) => <label key={p.profile_id}>Utility for {p.character} on {objective.id} <input type="number" min="0" step="1" value={draft.utilities[p.profile_id] ?? ""} onChange={(e) => candidate(objective.id, { utilities: { ...draft.utilities, [p.profile_id]: e.target.value } })} /></label>)}
        </fieldset>;
      })}
      <Button disabled={!valid || busy || loading} onClick={() => void calculate()}>Calculate joint estimate</Button>
    </fieldset>}
    {activeJob.current && busy && <div aria-label="Joint planning job progress">
      <p role="status">{progress?.edit === current.current.edit ? `${progress.status.state}; ${progress.status.phase?.replaceAll("_", " ") ?? "awaiting worker"}; ${progress.status.work_units} work units checked` : "Registering joint planning job…"}</p>
      <Button variant="ghost" onClick={cancelCalculation}>Cancel joint planning</Button>
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
  {visibleReport && pinJob && <PlanPinControl client={client} jobId={pinJob}/>}</Panel>;
}
