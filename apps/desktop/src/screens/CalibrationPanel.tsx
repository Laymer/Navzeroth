import { useEffect, useRef, useState } from "react";
import type { CommandClient } from "../commands";
import type { CalibrationConditions, CalibrationReport } from "../commands/calibration";
import type { SessionDetail } from "../commands/sessions";
export function CalibrationPanel({client, profile, detail, refreshKey, disabled}: {client: CommandClient; profile: string; detail: SessionDetail | null; refreshKey: number; disabled: boolean}) {
  const [route, setRoute] = useState(""); const [level, setLevel] = useState("1");
  const [role, setRole] = useState(""); const [party, setParty] = useState("1");
  const [baseline, setBaseline] = useState("600"); const [source, setSource] = useState("");
  const [quest, setQuest] = useState(""); const [complete, setComplete] = useState(false);
  const [report, setReport] = useState<CalibrationReport | null>(null);
  const [message, setMessage] = useState(""); const [busy, setBusy] = useState(false);
  const generation = useRef(0);
  useEffect(() => { generation.current++; setReport(null); setMessage(""); return () => { generation.current++; }; }, [client, profile, refreshKey]);
  useEffect(() => { generation.current++; setComplete(false); setQuest(""); setMessage(""); }, [detail]);
  useEffect(() => { setRoute(""); setRole(""); setLevel("1"); setParty("1"); setSource(""); setBaseline("600"); }, [profile]);
  const conditions: CalibrationConditions = {route_stage: route, level: Number(level), role, party_size: Number(party)};
  function invalidate() { generation.current++; setReport(null); setMessage(""); }
  async function run(annotate: boolean) {
    if (busy) return;
    const epoch = generation.current; setBusy(true); setReport(null); setMessage("");
    try {
      const result = annotate && detail ? await client.setSessionCalibration(profile, detail.analytics.summary.session, {conditions, complete, completion_quest: quest}) : await client.getCalibration(profile, {conditions, baseline_ms: Number(baseline)*1000, baseline_source: source});
      if (epoch !== generation.current) return;
      if (!result.ok) setMessage(`Error ${result.error.code}: ${result.error.message}`);
      else if (annotate) setMessage("Explicit association saved. It does not prove route duration; revalidation occurs on every comparison.");
      else setReport(result.value as CalibrationReport);
    } finally { setBusy(false); }
  }
  return <section aria-labelledby="calibration-title">
    <h3 id="calibration-title">Conditional personal estimate comparison</h3>
    <p>Declare a whole-session route/stage association and conditions. This declaration is not proof the route occupied the session. Comparison requires a recorded session start, terminal matching quest turn-in, and complete explicitly observed activity intervals. Unclassified time and gaps are excluded.</p>
    <fieldset disabled={disabled || busy} onChange={invalidate}>
      <legend>Exact comparable conditions</legend>
      <label>Route or quest stage <input value={route} maxLength={128} onChange={e=>setRoute(e.target.value)} /></label>
      <label>Declared level <input type="number" min={1} max={1000} value={level} onChange={e=>setLevel(e.target.value)} /></label>
      <label>Declared role <input value={role} maxLength={64} onChange={e=>setRole(e.target.value)} /></label>
      <label>Declared party size <input type="number" min={1} max={40} value={party} onChange={e=>setParty(e.target.value)} /></label>
      <label>Baseline seconds <input type="number" min={0.001} max={86400} step="any" value={baseline} onChange={e=>setBaseline(e.target.value)} /></label>
      <label>Baseline source or assumption <input value={source} maxLength={256} onChange={e=>setSource(e.target.value)} /></label>
      {detail && <><label>Terminal observed quest ID <input value={quest} maxLength={128} onChange={e=>setQuest(e.target.value)} /></label>
        <label><input type="checkbox" checked={complete} onChange={e=>setComplete(e.target.checked)} />I declare this whole session covers the completed route/stage under these conditions</label>
        <button type="button" onClick={()=>void run(true)}>Save association for {detail.analytics.summary.session}</button></>}
      <button type="button" onClick={()=>void run(false)}>Compare retained sessions</button>
    </fieldset>
    {message && <p role="status">{message}</p>}
    {report && <><p>{report.fixture ? "SYNTHETIC — engineering example. " : ""}Complete comparable samples: {report.sample_count}. Uncertainty: {report.uncertainty.replaceAll("_", " ")}. Conditional blended estimate: {(report.blended_ms/1000).toFixed(1)} s.</p>
      <p>Observed session median: {report.median_ms === null ? "unavailable" : `${report.median_ms/1000} s`}; range: {report.minimum_ms === null ? "unavailable" : `${report.minimum_ms/1000}–${(report.maximum_ms ?? 0)/1000} s`}. Baseline: {report.request.baseline_ms/1000} s ({report.request.baseline_source}).</p>
      <ul>{report.comparisons.map(row=><li key={row.session_id}>{row.session_id}: {row.exclusion ? `excluded — ${row.exclusion.replaceAll("_", " ")}` : `${(row.observed_ms ?? 0)/1000} s; difference from baseline ${(row.difference_ms ?? 0)/1000} s${row.outlier ? "; retained outlier" : ""}`}{row.annotation && <span> — declared {row.annotation.conditions.route_stage}, level {row.annotation.conditions.level}, {row.annotation.conditions.role}, party {row.annotation.conditions.party_size}; terminal quest {row.annotation.completion_quest}; completion {row.annotation.complete ? "declared" : "not declared"}</span>}</li>)}</ul><p>{report.methodology}</p></>}
  </section>;
}
