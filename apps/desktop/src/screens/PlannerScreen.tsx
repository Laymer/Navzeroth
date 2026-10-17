import "./play.css";
import { Button, Field, HelpTip } from "../ui";
import { CatalogPager, OffPageOptions } from "./CatalogPager";
import {PlanPinControl} from "./PlanPinControl";
import { ClusterSummary } from "./ClusterSummary";
import { runPlanningJob, type PlanningJobStatus } from "../commands/planningJobs";
import { RouteInspection } from "./RouteInspection";
import type { RouteInspection as Inspection } from "../commands/routes";
import { useEffect, useRef, useState } from "react";
import type { CommandClient } from "../commands";
import type { PlanningCatalog, PlanningReport, PlanningRequest, ProfileSummary, CommandMap } from "../commands/contract";

// Compare complete serialized reports, including context, fixture and recomputed choices.
// Object property order is not provenance; array order and every recorded value are.
function canonical(value: unknown): string {
  return JSON.stringify(value, (_key, item: unknown) => item && typeof item === "object" && !Array.isArray(item)
    ? Object.fromEntries(Object.entries(item).sort(([a], [b]) => a.localeCompare(b))) : item);
}
export function PlannerScreen({ client, refreshKey }: { client: CommandClient; refreshKey: number }) {
  const [profiles, setProfiles] = useState<ProfileSummary[]>([]);
  const [profile, setProfile] = useState("");
  const previousCatalogBinding=useRef("");
  const [catalog, setCatalog] = useState<PlanningCatalog | null>(null);
  const [origin, setOrigin] = useState("");
  const [stop, setStop] = useState("");
  const [goal, setGoal] = useState("manual-utility-v1");
  const [budget, setBudget] = useState(1800000);
  const [stoppingService, setStoppingService] = useState(0);
  const [candidates, setCandidates] = useState<PlanningRequest["candidates"]>([]);
  const [excluded, setExcluded] = useState<string[]>([]);
  const [pinJob,setPinJob]=useState<string|null>(null);
  const [report, setReport] = useState<PlanningReport | null>(null);
  const routeOwner = useRef<PlanningReport | null>(null);
  const [route, setRoute] = useState<Inspection | null>(null);
  const [exported, setExported] = useState<CommandMap["export_plan"]["output"] | null>(null);
  const [copyStatus, setCopyStatus] = useState("");
  const [error, setError] = useState("");
  const [busy, setBusy] = useState(false);
  const generation = useRef(0);
  const activeJob = useRef<AbortController | null>(null);
  const reportOwner = useRef<{ client: CommandClient; refreshKey: number; profile: string } | null>(null);
  const [progress, setProgress] = useState<{ ticket: number; status: PlanningJobStatus } | null>(null);
  const identity = useRef({ client, refreshKey, profile });
  if (identity.current.client !== client || identity.current.refreshKey !== refreshKey || identity.current.profile !== profile) { identity.current = { client, refreshKey, profile }; generation.current += 1; }
  const abortJob = () => { activeJob.current?.abort(); activeJob.current = null; setProgress(null); };

  const catalogProfile = useRef("");
  const catalogOwner = useRef<{ client: CommandClient; refreshKey: number; profile: string } | null>(null);
  const visibleCatalog = catalog && catalogOwner.current?.client === client && catalogOwner.current.refreshKey === refreshKey && catalogOwner.current.profile === profile ? catalog : null;
  const visibleReport = report && reportOwner.current?.client === client && reportOwner.current.refreshKey === refreshKey && reportOwner.current.profile === profile ? report : null;
  const visibleRoute = visibleReport && routeOwner.current === visibleReport ? route : null;
  const invalidate = () => { generation.current++; abortJob(); setReport(null); setPinJob(null); setRoute(null); setExported(null); setCopyStatus(""); setBusy(false); };
  useEffect(() => {
    let live = true;
    void client.listProfiles().then((result) => {
      if (!live) return;
      if (!result.ok) { setError(result.error.message); return; }
      setProfiles(result.value);
      setProfile((old) => result.value.some((p) => p.id === old) ? old : result.value[0]?.id ?? "");
    });
    return () => { live = false; };
  }, [client, refreshKey]);
  useEffect(() => {
    let live = true;
    const profileChanged = catalogProfile.current !== profile;
    catalogProfile.current = profile;
    if (profileChanged) { setCandidates([]); setExcluded([]); setOrigin(""); setStop(""); }
    generation.current++; abortJob(); setReport(null); setPinJob(null); setRoute(null); setExported(null); setCopyStatus(""); setCatalog(null); setBusy(false);
    if (profile) void client.getPlanningCatalog(profile).then((result) => {
      if (!live) return;
      if (!result.ok) { setError(result.error.message); return; }
      catalogOwner.current = { client, refreshKey, profile }; setCatalog(result.value); setError("");
      const binding=JSON.stringify([profile,result.value.revision,result.value.pages]);
      if(result.value.pages&&previousCatalogBinding.current===binding)return;
      previousCatalogBinding.current=binding;
      const nodes = new Set(result.value.nodes.map((n) => n.id));
      const objectives = new Set(result.value.objectives.map((o) => o.id));
      setCandidates((old) => old.filter((c) => objectives.has(c.objective)).map((c) => nodes.has(c.node) ? c : { ...c, node: "" }));
      setExcluded((old) => old.filter((id) => objectives.has(id)));
      setOrigin((old) => nodes.has(old) ? old : ""); setStop((old) => nodes.has(old) ? old : "");
    });
    return () => { live = false; generation.current++; activeJob.current?.abort(); activeJob.current = null; };
  }, [client, profile, refreshKey]);
  async function calculate() {
    if (!visibleCatalog?.snapshot_id || !visibleCatalog.revision || busy || activeJob.current) return;
    const request: PlanningRequest = { profile_id: profile, snapshot_id: visibleCatalog.snapshot_id, revision: visibleCatalog.revision, goal_profile: goal, origin, stopping_node: stop, budget_ms: budget, stopping_service_ms: stoppingService, candidates, excluded };
    const ticket = ++generation.current;
    const controller = new AbortController(); activeJob.current = controller;
    setBusy(true); setProgress(null); setError("");
    const result = await runPlanningJob<PlanningReport>(client, "solo", JSON.stringify(request), controller.signal, (status) => { if (ticket === generation.current && !controller.signal.aborted) setProgress({ ticket, status }); }, undefined, id=>{if(ticket===generation.current&&!controller.signal.aborted)setPinJob(id);});
    if (ticket !== generation.current) return;
    activeJob.current = null; setBusy(false); setProgress(null);
    if (!result.ok) { setError(result.error.message); if (result.error.code === "SNAPSHOT_OUTDATED" || result.error.code === "BUILD_MISMATCH") { setReport(null); setPinJob(null); setRoute(null); setExported(null); setCopyStatus(""); } }
    else { reportOwner.current = { client, refreshKey, profile }; setReport(result.value); setRoute(null); setExported(null); setCopyStatus(""); }
  }
  async function applyDefaults(){invalidate();const ticket=generation.current;const response=await client.getAppSettings();if(ticket!==generation.current)return;if(!response.ok){setError(response.error.message);return;}const d=response.value.settings.manual_planning_defaults;if(d.goal_profile!==null)setGoal(d.goal_profile);setBudget(d.budget_ms);setStoppingService(d.stopping_service_ms);}
  function cancelCalculation() { generation.current += 1; abortJob(); setBusy(false); setError("Planning request canceled locally; cancellation requested for its background job."); }

  async function exportPlan() {
    if (!visibleReport) return;
    const ticket = ++generation.current;
    setBusy(true); setExported(null); setCopyStatus(""); setError("");
    const result = await client.exportPlan(JSON.stringify(visibleReport.request));
    if (ticket !== generation.current) return;
    setBusy(false);
    if (!result.ok) { setReport(null); setPinJob(null); setRoute(null); setError(result.error.message); }
    else setExported(result.value);
  }
  async function inspectRoute() {
    if (!visibleReport) return;
    const original = visibleReport;
    const ticket = ++generation.current;
    setBusy(true); setRoute(null); setError("");
    const result = await client.inspectPlanRoute(JSON.stringify(original.request));
    if (ticket !== generation.current) return;
    setBusy(false);
    if (!result.ok) { setReport(null); setPinJob(null); setExported(null); setError(result.error.message); }
    else if (canonical(result.value.context) !== canonical(original.context) || result.value.fixture !== original.fixture || canonical(result.value.report) !== canonical(original)) {
      setReport(null); setPinJob(null); setExported(null);
      setError("Route inspection no longer matches the original plan. Calculate again before selecting route segments.");
    } else { routeOwner.current = original; setRoute(result.value); }
  }
  async function copyExport() {
    if (!exported) return;
    const ticket = generation.current;
    try { await navigator.clipboard.writeText(exported.text); if (ticket === generation.current) setCopyStatus("Copied plan export."); }
    catch { if (ticket === generation.current) setCopyStatus("Clipboard unavailable; select the text and copy manually."); }
  }
  const nodeOptions = <><OffPageOptions selected={[origin,stop,...candidates.map(c=>c.node)]} visible={visibleCatalog?.nodes.map(n=>n.id)??[]}/><option value="">Choose a node</option>{visibleCatalog?.nodes.map((n) => <option key={n.id} value={n.id}>{n.id}</option>)}</>;
  return <section aria-label="Planner" className="play-screen">
    <h2>Single-character planner</h2>
    <p>Manual utility and time estimates. No inferred quest unlocks or transport links. Expected durations are uncertain.</p>
    {visibleCatalog?.fixture && <p role="note" className="play-note">SYNTHETIC — engineering fixture, not live game advice.</p>}
    {error && <p role="alert">{error}</p>}
    <Field label="Planning profile" help="The imported character whose saved snapshot supplies the planner inputs. Changing it discards the current estimate.">{(c) => <select {...c} value={profile} onChange={(e) => { invalidate(); setProfile(e.target.value); }}>{profiles.map((p) => <option key={p.id} value={p.id}>{p.display_name}</option>)}</select>}</Field>
    <p>Snapshot: {visibleCatalog?.snapshot_id ?? "none"}; revision: {visibleCatalog?.revision ?? "none — activate Content first"}</p>
    <CatalogPager disabled={busy} client={client} subject={{kind:"character",profile_id:profile}} page={visibleCatalog?.pages?.nodes} label="nodes" onPage={p=>{setCatalog(old=>old?{...old,nodes:p.records as NonNullable<typeof catalog>["nodes"],pages:{...old.pages,nodes:p}}:old);}}/>
<CatalogPager disabled={busy} client={client} subject={{kind:"character",profile_id:profile}} page={visibleCatalog?.pages?.objectives} label="objectives" onPage={p=>{setCatalog(old=>old?{...old,objectives:p.records as NonNullable<typeof catalog>["objectives"],pages:{...old.pages,objectives:p}}:old);}}/>
<Field label="Origin" help="The declared node where the session starts. Nodes come from the active content; an unselected origin stays unknown and blocks planning.">{(c) => <select {...c} value={origin} onChange={(e) => { invalidate(); setOrigin(e.target.value); }}>{nodeOptions}</select>}</Field>
    <Field label="Stopping node" help="The declared node where the session must end. Travel to it is included in the total.">{(c) => <select {...c} value={stop} onChange={(e) => { invalidate(); setStop(e.target.value); }}>{nodeOptions}</select>}</Field>
    <Field label="Versioned goal identifier" help="Names the utility model your objective utilities follow, for example manual-utility-v1. It labels your own assumption and is not verified game advice.">{(c) => <input {...c} value={goal} onChange={(e) => { invalidate(); setGoal(e.target.value); }} />}</Field>
    <Button disabled={busy} onClick={()=>void applyDefaults()}>Apply local planning defaults</Button>
    <Field label="Budget (ms)" help="Session time limit in milliseconds; 1000 ms is one second. The budget must cover travel to the stopping node plus the stopping service, or planning fails with a Stopping point exceeds session budget error.">{(c) => <input {...c} type="number" min="0" step="1" value={budget} onChange={(e) => { invalidate(); setBudget(e.target.valueAsNumber); }} />}</Field>
    <Field label="Stopping service (ms)" help="Extra time in milliseconds you expect to spend after arriving at the stopping node. 0 means you declared none.">{(c) => <input {...c} type="number" min="0" step="1" value={stoppingService} onChange={(e) => { invalidate(); setStoppingService(e.target.valueAsNumber); }} />}</Field>
    <p>{candidates.length} selected objectives; {excluded.length} exclusions (maximum 128 each).</p><Button variant="ghost" onClick={()=>{invalidate();setCandidates([]);setExcluded([]);}}>Clear objective selections</Button> <HelpTip>Clear empties both your ticked objectives and your exclusions. Planner estimates use only ticked objectives; unticked ones add no assumed reward. At most 128 can be selected.</HelpTip>
    {visibleCatalog?.objectives.map((objective) => {
      const candidate = candidates.find((c) => c.objective === objective.id);
      const update = (key: "node" | "utility" | "completion_ms" | "service_ms", value: string | number) => { invalidate(); setCandidates((old) => old.map((c) => c.objective === objective.id ? { ...c, [key]: value } : c)); };
      return <fieldset key={objective.id}><legend>{objective.id}</legend>
        <label><input type="checkbox" checked={candidate !== undefined} disabled={!candidate&&candidates.length>=128} onChange={(e) => { invalidate(); setCandidates((old) => e.target.checked ? [...old, { objective: objective.id, node: "", utility: 0, completion_ms: 0, service_ms: 0 }] : old.filter((c) => c.objective !== objective.id)); }} />Consider objective</label>
        <label><input type="checkbox" checked={excluded.includes(objective.id)} disabled={!excluded.includes(objective.id)&&excluded.length>=128} onChange={(e) => { invalidate(); setExcluded((old) => e.target.checked ? [...old, objective.id] : old.filter((id) => id !== objective.id)); }} />Exclude objective</label>
        {candidate && <><label>Objective node <select value={candidate.node} onChange={(e) => update("node", e.target.value)}>{nodeOptions}</select></label>
          {(["utility", "completion_ms", "service_ms"] as const).map((key) => <label key={key}>{key} <input type="number" min="0" step="1" value={candidate[key]} onChange={(e) => update(key, e.target.valueAsNumber)} /></label>)}</>}
      </fieldset>;
    })}
    {activeJob.current && busy && <div aria-label="Planning job progress">
      <p role="status">{progress?.ticket === generation.current ? `${progress.status.state}; ${progress.status.phase?.replaceAll("_", " ") ?? "awaiting worker"}; ${progress.status.work_units} work units checked` : "Registering planning job…"}</p>
      <Button variant="ghost" onClick={cancelCalculation}>Cancel planning</Button>
      {visibleReport && <p>Previous valid estimate remains visible while this calculation runs.</p>}
    </div>}
    <Button variant="primary" disabled={busy || !visibleCatalog?.snapshot_id || !visibleCatalog.revision || !origin || !stop} onClick={() => void calculate()}>Plan session</Button>
    {visibleReport && <section aria-label="Planning result">
      <ClusterSummary clustering={visibleReport.clustering} />
      <Button disabled={busy || visibleReport.result.selected.length === 0} onClick={() => void exportPlan()}>Export plan</Button>
      <Button disabled={busy} onClick={() => void inspectRoute()}>Inspect route</Button>
      {visibleRoute && <RouteInspection value={visibleRoute} />}
      {exported && <section aria-label="Plan export"><p>{exported.byte_count} bytes; {exported.plan_id}; expires at UTC epoch ms {exported.expires_at_utc_ms}.</p>
        <label>Plan export text <textarea readOnly value={exported.text} rows={8} onFocus={(e) => e.target.select()} /></label>
        <Button onClick={() => void copyExport()}>Copy plan</Button><p role="status">{copyStatus}</p>
      </section>}
      <h3>Estimated plan</h3><p>Total {visibleReport.result.expected_total_ms} ms; final travel {visibleReport.result.stopping.total_ms} ms; stopping service {visibleReport.request.stopping_service_ms} ms.</p>
      <ol>{visibleReport.result.selected.map((step) => <li key={step.candidate}>{step.candidate}: utility {step.utility}; travel {step.travel.total_ms} ms; completion {step.completion_ms} ms; service {step.service_ms} ms</li>)}</ol>
      {visibleReport.result.selected.length === 0 && <p>No feasible objectives.</p>}
      <ul>{visibleReport.assumptions.map((a) => <li key={a}>{a}</li>)}{visibleReport.filtered.map((f) => <li key={f.objective}>{f.objective}: {f.reason}</li>)}</ul>
      <details><summary>Alternatives, uncertainty and decision trace</summary><pre>{JSON.stringify({ alternatives: visibleReport.result.alternatives, trace: visibleReport.result.trace, stopping: visibleReport.result.stopping }, null, 2)}</pre></details>
    {visibleReport && pinJob && <PlanPinControl client={client} jobId={pinJob}/>}
    </section>}
  </section>;
}
