import { useEffect, useRef, useState } from "react";
import type { CommandClient } from "../commands";
import type { AppError, ContentInspection, ProfileSummary } from "../commands/contract";
import { ContentDispositionPanel } from "./ContentDispositionPanel";
import { describeObservation } from "../observation";
import planningExample from "../../../../fixtures/desktop/planning/synthetic-revision.json";
import example from "../../../../fixtures/desktop/content/demo-revision.json";

export function ContentScreen({ client, onChanged, refreshKey }: { client: CommandClient; onChanged: () => void; refreshKey: number }) {
  const [profiles, setProfiles] = useState<ProfileSummary[]>([]);
  const [profile, setProfile] = useState("");
  const [active, setActive] = useState<ContentInspection | null>(null);
  const [staged, setStaged] = useState<ContentInspection | null>(null);
  const [text, setText] = useState("");
  const [error, setError] = useState<AppError | null>(null);
  const [message, setMessage] = useState("");
  const [busy, setBusy] = useState(false);
  const lock = useRef(false);
  const [loading, setLoading] = useState(true);
  const [native, setNative] = useState(false);
  const [revision, setRevision] = useState(0);
  useEffect(() => {
    let current = true;
    void Promise.all([client.listProfiles(), client.getAppStatus()]).then(([list, status]) => {
      if (!current) return;
      setLoading(false);
      if (!list.ok) { setError(list.error); return; }
      if (!status.ok) { setError(status.error); return; }
      setProfiles(list.value); setNative(status.value.data_mode === "stored");
      setProfile((old) => list.value.some((p) => p.id === old) ? old : list.value[0]?.id ?? "");
    });
    return () => { current = false; };
  }, [client, refreshKey, revision]);
  useEffect(() => {
    let current = true;
    if (!profile) return;
    void client.getActiveContent(profile).then((result) => {
      if (!current) return;
      if (result.ok) setActive(result.value); else { setActive(null); setError(result.error); }
    });
    return () => { current = false; };
  }, [client, profile, refreshKey, revision]);
  async function mutate(action: "stage" | "activate" | "rollback") {
    if (lock.current) return;
    lock.current = true; setBusy(true); setError(null); setMessage("");
    if (action === "stage") {
      const result = await client.stageContent(text);
      if (result.ok) { setStaged(result.value); setMessage("Validated and staged. Activation requires the matching profile context."); }
      else setError(result.error);
    } else if (action === "activate" && staged) {
      const result = await client.activateContent(profile, staged.key.revision);
      if (result.ok) { setActive(result.value); setMessage("Content activated. Dependent views will refresh."); onChanged(); }
      else setError(result.error);
    } else if (action === "rollback") {
      const result = await client.rollbackContent(profile);
      if (result.ok) { setActive(result.value.active); setMessage(result.value.changed ? "Previous content revision restored." : "No previous revision is available."); if (result.value.changed) onChanged(); }
      else setError(result.error);
    }
    lock.current = false; setBusy(false);
  }
  const selected = profiles.find((p) => p.id === profile);
  const compatible = staged !== null && selected?.fixture === staged.key.scope.fixture && selected.context?.namespace === staged.key.scope.context.namespace && selected.context.build.edition === staged.key.scope.context.build.edition && selected.context.build.build === staged.key.scope.context.build.build && selected.context.build.interface === staged.key.scope.context.build.interface;
  return <section className="snapshot-import" aria-labelledby="content-title">
    <h2 id="content-title">Content</h2>
    <p>Stage a local data revision, inspect coverage and source policies, then activate it for a matching character context.</p>
    {!loading && !native && <p className="demo-label">DEMO — content changes require the desktop application.</p>}
    {loading && <p role="status">Loading content profiles…</p>}
    {error && <p role="alert">Error {error.code}: {error.message}</p>}
    {message && <p role="status">{message}</p>}
    <button type="button" disabled={busy} onClick={() => { setError(null); setRevision((n) => n + 1); }}>Refresh content</button>
    {profiles.length === 0 ? <p>Import a character before activating content.</p> : <label>Content profile <select value={profile} disabled={busy} onChange={(event) => { setProfile(event.target.value); setActive(null); setError(null); }}>
      {profiles.map((p) => <option key={p.id} value={p.id}>{p.display_name} — {p.context?.build.build ?? "demo"}, {p.context?.namespace ?? "demo"}{p.fixture ? ", synthetic" : ""}</option>)}
    </select></label>}
    {native&&<ContentDispositionPanel client={client} profileId={profile} refreshKey={refreshKey+revision} disabled={busy} onBusyChange={setBusy} onChanged={()=>{setActive(null);setStaged(null);setRevision(n=>n+1);onChanged();}}/>}
    <h3>Active revision</h3>
    {active ? <Inspection value={active} /> : <p>No active content for this profile.</p>}
    <button type="button" disabled={!native || busy || !active} onClick={() => void mutate("rollback")}>Restore previous revision</button>
    <form onSubmit={(event) => { event.preventDefault(); void mutate("stage"); }}>
      <label>Content revision JSON <textarea value={text} onChange={(event) => { setText(event.target.value); setStaged(null); }} rows={8} disabled={busy} /></label>
      <button type="button" disabled={busy} onClick={() => { setText(JSON.stringify(example, null, 2)); setStaged(null); }}>Load synthetic example</button>
      <button type="button" disabled={busy} onClick={() => setText(JSON.stringify(planningExample, null, 2))}>Load synthetic planning example</button>
      <button type="submit" disabled={!native || busy || !text.trim()}>Validate and stage content</button>
    </form>
    {staged && <><h3>Staged revision</h3><Inspection value={staged} />
      {!compatible && <p>This revision does not match the selected profile’s edition, build, interface, namespace and synthetic status.</p>}
      <button type="button" disabled={!native || busy || !compatible} onClick={() => void mutate("activate")}>Activate staged revision</button></>}
  </section>;
}
function Inspection({ value }: { value: ContentInspection }) {
  const scope = value.key.scope;
  return <>
    {scope.fixture && <p className="demo-label">SYNTHETIC — engineering content, not verified gameplay coverage</p>}
    <p>{value.key.revision}: {scope.context.build.edition}/{scope.context.build.build}/{scope.context.build.interface}, {scope.context.namespace}</p>
    <p>Validation passed: {String(value.report.passes)}. Coverage gaps: {value.report.gaps.length}. Conflicts: {value.conflicts.length}.</p>
    {value.report.coverage && <><ul>{Object.entries(value.report.coverage.collections).map(([kind, collection]) => <li key={kind}>{kind.replaceAll("_", " ")}: {collection.total}</li>)}</ul>
      <p>Evidence categories</p><ul>{Object.entries(value.report.coverage.evidence_records.by_confidence).map(([kind, count]) => <li key={kind}>{kind.replaceAll("_", " ")}: {count}</li>)}</ul></>}
    <details><summary>Coverage gaps</summary><ul>{value.report.gaps.map((gap, n) => <li key={n}>{gap.code.replaceAll("_", " ")}</li>)}</ul></details>
    <h4>Source policies</h4><ul>{value.sources.map((source) => <li key={source.id}>{source.publisher} ({source.id})
      <p>License: {describeObservation(source.license)}. Redistribution: {describeObservation(source.redistribution)}. Offline use: {describeObservation(source.offline_usable)}.</p>
      <p>Attribution: {describeObservation(source.attribution)}</p>
    </li>)}</ul>
    <p>Passing structural validation does not establish source permissions or verified gameplay coverage.</p>
  </>;
}
