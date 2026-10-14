import { runPlanningJob, type PlanningJobStatus } from "../commands/planningJobs";
import { RouteInspection } from "./RouteInspection";
import type { RouteInspection as Inspection } from "../commands/routes";
import { useEffect, useRef, useState } from "react";
import type { CommandClient } from "../commands";
import type { PlanningCatalog, PlanningReport, PlanningRequest, ProfileSummary, CommandMap } from "../commands/contract";

export function PlannerScreen({ client, refreshKey }: { client: CommandClient; refreshKey: number }) {
  const [profiles, setProfiles] = useState<ProfileSummary[]>([]);
  const [profile, setProfile] = useState("");
  const [catalog, setCatalog] = useState<PlanningCatalog | null>(null);
  const [origin, setOrigin] = useState("");
  const [stop, setStop] = useState("");
  const [goal, setGoal] = useState("manual-utility-v1");
  const [budget, setBudget] = useState(1800000);
  const [stoppingService, setStoppingService] = useState(0);
  const [candidates, setCandidates] = useState<PlanningRequest["candidates"]>([]);
  const [excluded, setExcluded] = useState<string[]>([]);
  const [report, setReport] = useState<PlanningReport | null>(null);
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
  const invalidate = () => { generation.current++; abortJob(); setReport(null); setRoute(null); setExported(null); setCopyStatus(""); setBusy(false); };
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
    generation.current++; abortJob(); setReport(null); setRoute(null); setExported(null); setCopyStatus(""); setCatalog(null); setBusy(false);
    if (profile) void client.getPlanningCatalog(profile).then((result) => {
      if (!live) return;
      if (!result.ok) { setError(result.error.message); return; }
      catalogOwner.current = { client, refreshKey, profile }; setCatalog(result.value); setError("");
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
    const result = await runPlanningJob<PlanningReport>(client, "solo", JSON.stringify(request), controller.signal, (status) => { if (ticket === generation.current && !controller.signal.aborted) setProgress({ ticket, status }); });
    if (ticket !== generation.current) return;
    activeJob.current = null; setBusy(false); setProgress(null);
    if (!result.ok) { setError(result.error.message); if (result.error.code === "SNAPSHOT_OUTDATED" || result.error.code === "BUILD_MISMATCH") { setReport(null); setRoute(null); setExported(null); setCopyStatus(""); } }
    else { reportOwner.current = { client, refreshKey, profile }; setReport(result.value); setRoute(null); setExported(null); setCopyStatus(""); }
  }
  function cancelCalculation() { generation.current += 1; abortJob(); setBusy(false); setError("Planning request canceled locally; cancellation requested for its background job."); }

  async function exportPlan() {
    if (!visibleReport) return;
    const ticket = ++generation.current;
    setBusy(true); setExported(null); setCopyStatus(""); setError("");
    const result = await client.exportPlan(JSON.stringify(visibleReport.request));
    if (ticket !== generation.current) return;
    setBusy(false);
    if (!result.ok) { setReport(null); setRoute(null); setError(result.error.message); }
    else setExported(result.value);
  }
  async function inspectRoute() {
    if (!visibleReport) return;
    const ticket = ++generation.current;
    setBusy(true); setRoute(null); setError("");
    const result = await client.inspectPlanRoute(JSON.stringify(visibleReport.request));
    if (ticket !== generation.current) return;
    setBusy(false);
    if (!result.ok) { setReport(null); setExported(null); setError(result.error.message); }
    else setRoute(result.value);
  }
  async function copyExport() {
    if (!exported) return;
    const ticket = generation.current;
    try { await navigator.clipboard.writeText(exported.text); if (ticket === generation.current) setCopyStatus("Copied plan export."); }
    catch { if (ticket === generation.current) setCopyStatus("Clipboard unavailable; select the text and copy manually."); }
  }
  const nodeOptions = <><option value="">Choose a node</option>{visibleCatalog?.nodes.map((n) => <option key={n.id} value={n.id}>{n.id}</option>)}</>;
  return <section aria-label="Planner">
    <h2>Single-character planner</h2>
    <p>Manual utility and time estimates. No inferred quest unlocks or transport links. Expected durations are uncertain.</p>
    {visibleCatalog?.fixture && <p role="note">SYNTHETIC — engineering fixture, not live game advice.</p>}
    {error && <p role="alert">{error}</p>}
    <label>Planning profile <select value={profile} onChange={(e) => { invalidate(); setProfile(e.target.value); }}>{profiles.map((p) => <option key={p.id} value={p.id}>{p.display_name}</option>)}</select></label>
    <p>Snapshot: {visibleCatalog?.snapshot_id ?? "none"}; revision: {visibleCatalog?.revision ?? "none — activate Content first"}</p>
    <label>Origin <select value={origin} onChange={(e) => { invalidate(); setOrigin(e.target.value); }}>{nodeOptions}</select></label>
    <label>Stopping node <select value={stop} onChange={(e) => { invalidate(); setStop(e.target.value); }}>{nodeOptions}</select></label>
    <label>Versioned goal identifier <input value={goal} onChange={(e) => { invalidate(); setGoal(e.target.value); }} /></label>
    <label>Budget (ms) <input type="number" min="0" step="1" value={budget} onChange={(e) => { invalidate(); setBudget(e.target.valueAsNumber); }} /></label>
    <label>Stopping service (ms) <input type="number" min="0" step="1" value={stoppingService} onChange={(e) => { invalidate(); setStoppingService(e.target.valueAsNumber); }} /></label>
    {visibleCatalog?.objectives.map((objective) => {
      const candidate = candidates.find((c) => c.objective === objective.id);
      const update = (key: "node" | "utility" | "completion_ms" | "service_ms", value: string | number) => { invalidate(); setCandidates((old) => old.map((c) => c.objective === objective.id ? { ...c, [key]: value } : c)); };
      return <fieldset key={objective.id}><legend>{objective.id}</legend>
        <label><input type="checkbox" checked={candidate !== undefined} onChange={(e) => { invalidate(); setCandidates((old) => e.target.checked ? [...old, { objective: objective.id, node: "", utility: 0, completion_ms: 0, service_ms: 0 }] : old.filter((c) => c.objective !== objective.id)); }} />Consider objective</label>
        <label><input type="checkbox" checked={excluded.includes(objective.id)} onChange={(e) => { invalidate(); setExcluded((old) => e.target.checked ? [...old, objective.id] : old.filter((id) => id !== objective.id)); }} />Exclude objective</label>
        {candidate && <><label>Objective node <select value={candidate.node} onChange={(e) => update("node", e.target.value)}>{nodeOptions}</select></label>
          {(["utility", "completion_ms", "service_ms"] as const).map((key) => <label key={key}>{key} <input type="number" min="0" step="1" value={candidate[key]} onChange={(e) => update(key, e.target.valueAsNumber)} /></label>)}</>}
      </fieldset>;
    })}
    {activeJob.current && busy && <div aria-label="Planning job progress">
      <p role="status">{progress?.ticket === generation.current ? `${progress.status.state}; ${progress.status.phase?.replaceAll("_", " ") ?? "awaiting worker"}; ${progress.status.work_units} work units checked` : "Registering planning job…"}</p>
      <button type="button" onClick={cancelCalculation}>Cancel planning</button>
      {visibleReport && <p>Previous valid estimate remains visible while this calculation runs.</p>}
    </div>}
    <button type="button" disabled={busy || !visibleCatalog?.snapshot_id || !visibleCatalog.revision || !origin || !stop} onClick={() => void calculate()}>Plan session</button>
    {visibleReport && <section aria-label="Planning result">
      <button type="button" disabled={busy || visibleReport.result.selected.length === 0} onClick={() => void exportPlan()}>Export plan</button>
      <button type="button" disabled={busy} onClick={() => void inspectRoute()}>Inspect route</button>
      {route && <RouteInspection value={route} />}
      {exported && <section aria-label="Plan export"><p>{exported.byte_count} bytes; {exported.plan_id}; expires at UTC epoch ms {exported.expires_at_utc_ms}.</p>
        <label>Plan export text <textarea readOnly value={exported.text} rows={8} onFocus={(e) => e.target.select()} /></label>
        <button type="button" onClick={() => void copyExport()}>Copy plan</button><p role="status">{copyStatus}</p>
      </section>}
      <h3>Estimated plan</h3><p>Total {visibleReport.result.expected_total_ms} ms; final travel {visibleReport.result.stopping.total_ms} ms; stopping service {visibleReport.request.stopping_service_ms} ms.</p>
      <ol>{visibleReport.result.selected.map((step) => <li key={step.candidate}>{step.candidate}: utility {step.utility}; travel {step.travel.total_ms} ms; completion {step.completion_ms} ms; service {step.service_ms} ms</li>)}</ol>
      {visibleReport.result.selected.length === 0 && <p>No feasible objectives.</p>}
      <ul>{visibleReport.assumptions.map((a) => <li key={a}>{a}</li>)}{visibleReport.filtered.map((f) => <li key={f.objective}>{f.objective}: {f.reason}</li>)}</ul>
      <details><summary>Alternatives, uncertainty and decision trace</summary><pre>{JSON.stringify({ alternatives: visibleReport.result.alternatives, trace: visibleReport.result.trace, stopping: visibleReport.result.stopping }, null, 2)}</pre></details>
    </section>}
  </section>;
}
