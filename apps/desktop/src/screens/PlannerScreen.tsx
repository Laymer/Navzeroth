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
  const [exported, setExported] = useState<CommandMap["export_plan"]["output"] | null>(null);
  const [copyStatus, setCopyStatus] = useState("");
  const [error, setError] = useState("");
  const [busy, setBusy] = useState(false);
  const generation = useRef(0);
  const invalidate = () => { generation.current++; setReport(null); setExported(null); setCopyStatus(""); setBusy(false); };
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
    generation.current++; setReport(null); setExported(null); setCopyStatus(""); setCatalog(null); setBusy(false);
    if (profile) void client.getPlanningCatalog(profile).then((result) => {
      if (!live) return;
      if (!result.ok) { setError(result.error.message); return; }
      setCatalog(result.value); setError(""); setCandidates([]); setExcluded([]); setOrigin(""); setStop("");
    });
    return () => { live = false; generation.current++; };
  }, [client, profile, refreshKey]);
  async function calculate() {
    if (!catalog?.snapshot_id || !catalog.revision) return;
    const request: PlanningRequest = { profile_id: profile, snapshot_id: catalog.snapshot_id, revision: catalog.revision, goal_profile: goal, origin, stopping_node: stop, budget_ms: budget, stopping_service_ms: stoppingService, candidates, excluded };
    const ticket = ++generation.current;
    setBusy(true); setReport(null); setExported(null); setCopyStatus(""); setError("");
    const result = await client.planSession(JSON.stringify(request));
    if (ticket !== generation.current) return;
    setBusy(false);
    if (!result.ok) setError(result.error.message); else setReport(result.value);
  }
  async function exportPlan() {
    if (!report) return;
    const ticket = ++generation.current;
    setBusy(true); setExported(null); setCopyStatus(""); setError("");
    const result = await client.exportPlan(JSON.stringify(report.request));
    if (ticket !== generation.current) return;
    setBusy(false);
    if (!result.ok) { setReport(null); setError(result.error.message); }
    else setExported(result.value);
  }
  async function copyExport() {
    if (!exported) return;
    const ticket = generation.current;
    try { await navigator.clipboard.writeText(exported.text); if (ticket === generation.current) setCopyStatus("Copied plan export."); }
    catch { if (ticket === generation.current) setCopyStatus("Clipboard unavailable; select the text and copy manually."); }
  }
  const nodeOptions = <><option value="">Choose a node</option>{catalog?.nodes.map((n) => <option key={n.id} value={n.id}>{n.id}</option>)}</>;
  return <section aria-label="Planner">
    <h2>Single-character planner</h2>
    <p>Manual utility and time estimates. No inferred quest unlocks or transport links. Expected durations are uncertain.</p>
    {catalog?.fixture && <p role="note">SYNTHETIC — engineering fixture, not live game advice.</p>}
    {error && <p role="alert">{error}</p>}
    <label>Planning profile <select value={profile} onChange={(e) => { invalidate(); setProfile(e.target.value); }}>{profiles.map((p) => <option key={p.id} value={p.id}>{p.display_name}</option>)}</select></label>
    <p>Snapshot: {catalog?.snapshot_id ?? "none"}; revision: {catalog?.revision ?? "none — activate Content first"}</p>
    <label>Origin <select value={origin} onChange={(e) => { invalidate(); setOrigin(e.target.value); }}>{nodeOptions}</select></label>
    <label>Stopping node <select value={stop} onChange={(e) => { invalidate(); setStop(e.target.value); }}>{nodeOptions}</select></label>
    <label>Versioned goal identifier <input value={goal} onChange={(e) => { invalidate(); setGoal(e.target.value); }} /></label>
    <label>Budget (ms) <input type="number" min="0" step="1" value={budget} onChange={(e) => { invalidate(); setBudget(e.target.valueAsNumber); }} /></label>
    <label>Stopping service (ms) <input type="number" min="0" step="1" value={stoppingService} onChange={(e) => { invalidate(); setStoppingService(e.target.valueAsNumber); }} /></label>
    {catalog?.objectives.map((objective) => {
      const candidate = candidates.find((c) => c.objective === objective.id);
      const update = (key: "node" | "utility" | "completion_ms" | "service_ms", value: string | number) => { invalidate(); setCandidates((old) => old.map((c) => c.objective === objective.id ? { ...c, [key]: value } : c)); };
      return <fieldset key={objective.id}><legend>{objective.id}</legend>
        <label><input type="checkbox" checked={candidate !== undefined} onChange={(e) => { invalidate(); setCandidates((old) => e.target.checked ? [...old, { objective: objective.id, node: "", utility: 0, completion_ms: 0, service_ms: 0 }] : old.filter((c) => c.objective !== objective.id)); }} />Consider objective</label>
        <label><input type="checkbox" checked={excluded.includes(objective.id)} onChange={(e) => { invalidate(); setExcluded((old) => e.target.checked ? [...old, objective.id] : old.filter((id) => id !== objective.id)); }} />Exclude objective</label>
        {candidate && <><label>Objective node <select value={candidate.node} onChange={(e) => update("node", e.target.value)}>{nodeOptions}</select></label>
          {(["utility", "completion_ms", "service_ms"] as const).map((key) => <label key={key}>{key} <input type="number" min="0" step="1" value={candidate[key]} onChange={(e) => update(key, e.target.valueAsNumber)} /></label>)}</>}
      </fieldset>;
    })}
    <button type="button" disabled={busy || !catalog?.snapshot_id || !catalog.revision || !origin || !stop} onClick={() => void calculate()}>Plan session</button>
    {report && <section aria-label="Planning result">
      <button type="button" disabled={busy || report.result.selected.length === 0} onClick={() => void exportPlan()}>Export plan</button>
      {exported && <section aria-label="Plan export"><p>{exported.byte_count} bytes; {exported.plan_id}; expires at UTC epoch ms {exported.expires_at_utc_ms}.</p>
        <label>Plan export text <textarea readOnly value={exported.text} rows={8} onFocus={(e) => e.target.select()} /></label>
        <button type="button" onClick={() => void copyExport()}>Copy plan</button><p role="status">{copyStatus}</p>
      </section>}
      <h3>Estimated plan</h3><p>Total {report.result.expected_total_ms} ms; final travel {report.result.stopping.total_ms} ms; stopping service {report.request.stopping_service_ms} ms.</p>
      <ol>{report.result.selected.map((step) => <li key={step.candidate}>{step.candidate}: utility {step.utility}; travel {step.travel.total_ms} ms; completion {step.completion_ms} ms; service {step.service_ms} ms</li>)}</ol>
      {report.result.selected.length === 0 && <p>No feasible objectives.</p>}
      <ul>{report.assumptions.map((a) => <li key={a}>{a}</li>)}{report.filtered.map((f) => <li key={f.objective}>{f.objective}: {f.reason}</li>)}</ul>
      <details><summary>Alternatives, uncertainty and decision trace</summary><pre>{JSON.stringify({ alternatives: report.result.alternatives, trace: report.result.trace, stopping: report.result.stopping }, null, 2)}</pre></details>
    </section>}
  </section>;
}
