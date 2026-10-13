import { useEffect, useRef, useState } from "react";
import type { AppError, CommandClient, ProfileSummary } from "../commands";
import type { FileInputFormat } from "../commands/contract";
import type { SessionAnalytics, SessionDetail, SessionListEntry } from "../commands/sessions";
import { describeObservation } from "../observation";

function duration(ms: number): string { return `${(ms / 1000).toFixed(1)} s`; }
export function SessionsScreen({ client, refreshKey, onChanged }: { client: CommandClient; refreshKey: number; onChanged: () => void }) {
  const [profiles, setProfiles] = useState<ProfileSummary[]>([]);
  const [profile, setProfile] = useState("");
  const [rows, setRows] = useState<SessionListEntry[]>([]);
  const [detail, setDetail] = useState<SessionDetail | null>(null);
  const [text, setText] = useState("");
  const [path, setPath] = useState("");
  const [saved, setSaved] = useState(true);
  const [error, setError] = useState<AppError | null>(null);
  const [message, setMessage] = useState("");
  const [busy, setBusy] = useState(false);
  const [native, setNative] = useState(false);
  const [revision, setRevision] = useState(0);
  const active = useRef(true);
  const generation = useRef(0);
  const detailRequest = useRef(0);
  const mutation = useRef(false);
  useEffect(() => {
    active.current = true;
    return () => { active.current = false; generation.current++; detailRequest.current++; };
  }, [client]);
  useEffect(() => {
    let current = true;
    void Promise.all([client.listProfiles(), client.getAppStatus()]).then(([list, status]) => {
      if (!current) return;
      if (!list.ok) { setError(list.error); return; }
      if (!status.ok) { setError(status.error); return; }
      setProfiles(list.value); setNative(status.value.data_mode === "stored");
      setProfile((old) => list.value.some((p) => p.id === old) ? old : list.value.find((p) => p.id === status.value.active_profile)?.id ?? list.value[0]?.id ?? "");
    });
    return () => { current = false; };
  }, [client, refreshKey]);
  useEffect(() => {
    generation.current++; detailRequest.current++;
  }, [client, profile, native]);
  useEffect(() => {
    let current = true;
    detailRequest.current++; setRows([]); setDetail(null);
    if (profile && native) void client.listSessions(profile).then((result) => {
      if (!current) return;
      if (result.ok) setRows(result.value); else setError(result.error);
    });
    return () => { current = false; };
  }, [client, profile, native, revision, refreshKey]);
  function select(id: string) {
    generation.current++; detailRequest.current++;
    setProfile(id); setRows([]); setDetail(null); setError(null); setMessage("");
  }
  async function inspect(id: string) {
    const request = ++detailRequest.current;
    setDetail(null); setError(null);
    const result = await client.getSessionDetail(profile, id);
    if (!active.current || request !== detailRequest.current) return;
    if (result.ok) setDetail(result.value); else setError(result.error);
  }
  async function mutate(kind: "text" | "file" | "delete", id?: string) {
    if (mutation.current) return;
    mutation.current = true; setBusy(true); setError(null); setMessage("");
    const request = generation.current;
    const format: FileInputFormat = saved ? { kind: "saved_variables", global: "AzerothNavigatorDB", key: "session_export" } : { kind: "anav1_text" };
    const result = kind === "text" ? await client.importSessionText(profile, text) : kind === "file" ? await client.importSessionFile(profile, path, format) : await client.deleteSession(profile, id ?? "");
    mutation.current = false;
    if (!active.current) return;
    setBusy(false);
    if (request !== generation.current) return;
    if (!result.ok) { setError(result.error); return; }
    if (typeof result.value === "boolean") setMessage(result.value ? "Session deleted." : "Session was already absent.");
    else setMessage(`Imported ${result.value.inserted} session(s); ${result.value.duplicates} already stored.`);
    setDetail(null); setRevision((value) => value + 1); onChanged();
  }
  const selected = profiles.find((p) => p.id === profile);
  return <section className="snapshot-import" aria-labelledby="sessions-title">
    <h2 id="sessions-title">Sessions</h2>
    <p>Manually recorded observations. Monotonic coverage measures intervals between samples; it does not prove active play or wall-clock duration.</p>
    {!native && <p className="demo-label">DEMO — importing and storing session history requires the desktop application.</p>}
    {error && <p role="alert">Error {error.code}: {error.message}</p>}
    {message && <p role="status">{message}</p>}
    {profiles.length ? <label>Session profile <select value={profile} disabled={busy} onChange={(event) => select(event.target.value)}>{profiles.map((p) => <option key={p.id} value={p.id}>{p.display_name} — {p.context?.build.build ?? "demo"}, {p.context?.namespace ?? "demo"}{p.fixture ? ", synthetic" : ""}</option>)}</select></label> : <p>Import a character snapshot before importing matching sessions.</p>}
    {selected?.fixture && <p className="demo-label">SYNTHETIC — engineering examples, not live gameplay evidence.</p>}
    <form onSubmit={(event) => { event.preventDefault(); void mutate("text"); }}>
      <label>Session export text <textarea rows={6} value={text} disabled={busy} onChange={(event) => setText(event.target.value)} placeholder="ANAVSESSION1:…" /></label>
      <button type="submit" disabled={!native || !profile || busy || !text.trim()}>Import session text</button>
    </form>
    <form onSubmit={(event) => { event.preventDefault(); void mutate("file"); }}>
      <label>Selected session file path <input value={path} disabled={busy} onChange={(event) => setPath(event.target.value)} /></label>
      <label>Session file format <select value={saved ? "saved_variables" : "text"} disabled={busy} onChange={(event) => setSaved(event.target.value === "saved_variables")}><option value="saved_variables">SavedVariables: AzerothNavigatorDB / session_export</option><option value="text">ANAVSESSION1 text file</option></select></label>
      <button type="submit" disabled={!native || !profile || busy || !path.trim()}>Import selected session file</button>
    </form>
    <p>History retains at most 128 sessions per exact profile. Capacity refusal preserves history; delete an entry explicitly to make space. Identical exports do not add entries.</p>
    <button type="button" disabled={!native || !profile || busy} onClick={() => { setError(null); setRevision((value) => value + 1); }}>Refresh sessions</button>
    <h3>Session history ({rows.length})</h3>
    {rows.length === 0 ? <p>No stored sessions for this profile.</p> : <ul>{rows.map((row) => <li key={row.session_id}>{row.session_id} — coverage {duration(row.analytics.timing.covered_ms)}
      <button type="button" disabled={busy} onClick={() => void inspect(row.session_id)}>Inspect {row.session_id}</button>
      <button type="button" disabled={busy} onClick={() => void mutate("delete", row.session_id)}>Delete {row.session_id}</button>
    </li>)}</ul>}
    {detail && <><h3>Session detail: {detail.analytics.summary.session}</h3><Analytics value={detail.analytics} />
      <details><summary>Recorded events ({detail.batch.events.length})</summary><pre>{JSON.stringify(detail.batch.events, null, 2)}</pre></details></>}
    <p>Addon: /anavsession start &lt;anonymous-profile-id&gt;, sample, pause, resume, stop, then export. Export a stopped recording before reload; reload never resumes it. Save with reload/logout and select session_export here. Every sampled running span defaults to unclassified.</p>
  </section>;
}
function Analytics({ value }: { value: SessionAnalytics }) {
  const totals = value.timing.totals;
  return <>
    <p>{value.summary.context.build.edition}/{value.summary.context.build.build}/{value.summary.context.build.interface}, {value.summary.context.namespace}</p>
    <p>Measured monotonic coverage: {duration(value.timing.covered_ms)}. Measured intervals: {duration(totals.observed_ms)}. Estimated intervals: {duration(totals.estimated_ms)}. Unclassified intervals: {duration(totals.unclassified_ms)}. Gaps: {duration(totals.gap_ms)}. Uncovered intervals: {duration(value.timing.uncovered_ms)}.</p>
    <p>Starting level: {describeObservation(value.summary.starting_level)}. Ending level: {describeObservation(value.summary.ending_level)}. Observed level change: {value.progression.observed_level_change ?? "unknown"}.</p>
    <p>Observed quest turn-ins: {value.progression.unique_observed_quest_count}. Equipment-change item identities: {value.progression.equipment_change_item_count}. Missing observations do not prove absence of progression or acquisition.</p>
    <p>UTC mapping: {value.utc.status === "mapped" ? `trusted start epoch ${value.utc.start_utc_ms} ms` : `uncertain (${value.utc.reason.code.replaceAll("_", " ")})`}. Clock flags: {value.clock_flags.length}.</p>
    {value.clock_flags.length > 0 && <ul>{value.clock_flags.map((flag, index) => <li key={index}>{flag.code.replaceAll("_", " ")} at {duration(flag.at_ms)}</li>)}</ul>}
    <h4>Intervals and gaps</h4><ul>{value.summary.intervals.map((span, index) => <li key={`interval-${index}`}>{duration(span.start_ms)}–{duration(span.end_ms)}: {span.label}, {span.quality}</li>)}{value.summary.gaps.map((gap, index) => <li key={`gap-${index}`}>{duration(gap.start_ms)}–{duration(gap.end_ms)}: gap ({gap.reason})</li>)}</ul>
    <p>{value.summary.methodology}</p>
    <p>These observations remain separate from planner estimates. Personalized calibration is deferred.</p>
  </>;
}
