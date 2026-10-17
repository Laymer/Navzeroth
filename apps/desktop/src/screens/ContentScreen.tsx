import "./dataScreens.css";
import { LocalSubmissionsPanel } from "./LocalSubmissionsPanel";
import { useEffect, useRef, useState } from "react";
import type { CommandClient } from "../commands";
import type { AppError, ContentInspection, ProfileSummary } from "../commands/contract";
import { ContentDispositionPanel } from "./ContentDispositionPanel";
import { describeObservation } from "../observation";
import planningExample from "../../../../fixtures/desktop/planning/synthetic-revision.json";
import example from "../../../../fixtures/desktop/content/demo-revision.json";
import { Button, Field, Panel } from "../ui";

export function ContentScreen({ client, onChanged, refreshKey }: { client: CommandClient; onChanged: () => void; refreshKey: number }) {
  const [profiles, setProfiles] = useState<ProfileSummary[]>([]);
  const [profile, setProfile] = useState("");
  const [active, setActive] = useState<ContentInspection | null>(null);
  const [staged, setStaged] = useState<ContentInspection | null>(null);
  const [bundlePath,setBundlePath]=useState("");
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
  async function mutate(action: "stage" | "bundle" | "activate" | "rollback") {
    if (lock.current) return;
    lock.current = true; setBusy(true); setError(null); setMessage("");
    if (action === "stage" || action === "bundle") {
      const result = action === "bundle" ? await client.stageVerifiedContentBundle(bundlePath) : await client.stageContent(text);
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
  return <div className="snapshot-import"><Panel title="Content">
    <p>Stage a local data revision, inspect coverage and source policies, then activate it for a matching character context.</p>
    {!loading && !native && <p className="demo-label">DEMO — content changes require the desktop application.</p>}
    {loading && <p role="status">Loading content profiles…</p>}
    {error && <p role="alert">Error {error.code}: {error.message}</p>}
    {message && <p role="status">{message}</p>}
    <Button disabled={busy} onClick={() => { setError(null); setRevision((n) => n + 1); }}>Refresh content</Button>
    {profiles.length === 0 ? <p>Import a character before activating content.</p> : <Field label="Content profile" help="Content is activated per character context: edition, build, interface and namespace must match.">{(c) => <select {...c} value={profile} disabled={busy} onChange={(event) => { setProfile(event.target.value); setActive(null); setError(null); }}>
      {profiles.map((p) => <option key={p.id} value={p.id}>{p.display_name} — {p.context?.build.build ?? "demo"}, {p.context?.namespace ?? "demo"}{p.fixture ? ", synthetic" : ""}</option>)}
    </select>}</Field>}
    {native&&<ContentDispositionPanel client={client} profileId={profile} refreshKey={refreshKey+revision} disabled={busy} onBusyChange={setBusy} onChanged={()=>{setActive(null);setStaged(null);setRevision(n=>n+1);onChanged();}}/>}
    <h3>Active revision</h3>
    {active ? <Inspection client={client} value={active} /> : <p>No active content for this profile.</p>}
    <Button disabled={!native || busy || !active} onClick={() => void mutate("rollback")}>Restore previous revision</Button>
    <form onSubmit={event=>{event.preventDefault();void mutate("bundle");}}><Field label="Verified content pack directory" help="An extracted content pack folder. The desktop checks the catalog, source registry and declared images before staging; staging does not activate.">{(c)=><input {...c} value={bundlePath} maxLength={4096} disabled={busy} onChange={event=>{setBundlePath(event.target.value);setStaged(null);}}/>}</Field><p>Choose an extracted content pack directory. The desktop verifies the catalog, source registry and declared image files before staging them together.</p><Button type="submit" variant="primary" disabled={!native||busy||!bundlePath.trim()}>Verify and stage content pack</Button></form>
    <form onSubmit={(event) => { event.preventDefault(); void mutate("stage"); }}>
      <Field label="Content revision JSON" help="A data revision as JSON. It is validated as bounded data and staged only; activation is a separate step.">{(c) => <textarea {...c} value={text} onChange={(event) => { setText(event.target.value); setStaged(null); }} rows={8} disabled={busy} />}</Field>
      <Button disabled={busy} onClick={() => { setText(JSON.stringify(example, null, 2)); setStaged(null); }}>Load synthetic example</Button>
      <Button disabled={busy} onClick={() => setText(JSON.stringify(planningExample, null, 2))}>Load synthetic planning example</Button>
      <Button type="submit" variant="primary" disabled={!native || busy || !text.trim()}>Validate and stage content</Button>
    </form>
    {staged && <><h3>Staged revision</h3><Inspection client={client} value={staged} />
      {!compatible && <p>This revision does not match the selected profile’s edition, build, interface, namespace and synthetic status.</p>}
      <Button disabled={!native || busy || !compatible} onClick={() => void mutate("activate")}>Activate staged revision</Button></>}
    <LocalSubmissionsPanel client={client} enabled={native} refreshKey={refreshKey + revision} />
  </Panel></div>;
}
function Inspection({ client, value }: { client:CommandClient; value: ContentInspection }) {
  const scope = value.key.scope;
  return <>
    {scope.fixture && <p className="demo-label">SYNTHETIC — engineering content, not verified gameplay coverage</p>}
    <p>{value.key.revision}: {scope.context.build.edition}/{scope.context.build.build}/{scope.context.build.interface}, {scope.context.namespace}</p>
    <p>Validation passed: {String(value.report.passes)}. Coverage gaps: {value.pages?.gaps??value.report.gaps.length}. Conflicts: {value.pages?.conflicts??value.conflicts.length}.</p>
    {value.report.coverage && <><ul>{Object.entries(value.report.coverage.collections).map(([kind, collection]) => <li key={kind}>{kind.replaceAll("_", " ")}: {collection.total}</li>)}</ul>
      <p>Evidence categories</p><ul>{Object.entries(value.report.coverage.evidence_records.by_confidence).map(([kind, count]) => <li key={kind}>{kind.replaceAll("_", " ")}: {count}</li>)}</ul></>}
    <InspectionList client={client} value={value} collection="gaps"/>
    <InspectionList client={client} value={value} collection="errors"/>
    <InspectionList client={client} value={value} collection="conflicts"/>
    <InspectionList client={client} value={value} collection="sources"/>
    <p>Passing structural validation does not establish source permissions or verified gameplay coverage.</p>
  </>;
}

function InspectionList({client,value,collection}:{client:CommandClient;value:ContentInspection;collection:"sources"|"conflicts"|"gaps"|"errors"}) {
 const initial=collection==="gaps"||collection==="errors"?value.report[collection]:value[collection];
 const identity=JSON.stringify([value.key,value.pages?.content_sha256,collection]);
 const [loaded,setLoaded]=useState<{identity:string;offset:number;records:unknown[]}|null>(null);
 const [busy,setBusy]=useState(false);const [error,setError]=useState("");const ticket=useRef(0);
 const current=useRef({identity,client});if(current.current.identity!==identity||current.current.client!==client){current.current={identity,client};ticket.current++;}
 useEffect(()=>{setLoaded(null);setBusy(false);setError("");return()=>{ticket.current++;};},[identity,client]);
 const page=loaded?.identity===identity?loaded:{offset:0,records:initial};
 const total=value.pages?.[collection]??initial.length;
 async function move(offset:number){if(!value.pages||busy)return;const request=++ticket.current;setBusy(true);setError("");const result=await client.getContentInspectionPage(JSON.stringify({key:value.key,content_sha256:value.pages.content_sha256,collection,offset}));if(request!==ticket.current)return;setBusy(false);if(!result.ok){setError(result.error.message);return;}const p=result.value;if(JSON.stringify(p.key)!==JSON.stringify(value.key)||p.content_sha256!==value.pages.content_sha256||p.collection!==collection||p.offset!==offset||p.total!==total||p.records.length>200){setError("Inspection changed; refresh content.");return;}setLoaded({identity,offset,records:p.records});}
 return <details><summary>{collection==="sources"?"Source policies":collection.replaceAll("_"," ")} ({total})</summary>
 {total>200&&<nav aria-label={`${collection} inspection pages`}><Button disabled={busy||page.offset===0} onClick={()=>void move(Math.max(0,page.offset-200))}>Previous {collection}</Button><span>{page.offset+1}–{Math.min(page.offset+200,total)} of {total}</span><Button disabled={busy||page.offset+200>=total} onClick={()=>void move(page.offset+200)}>Next {collection}</Button></nav>}
 {error&&<p role="alert">{error}</p>}<ul>{page.records.map((record,n)=>{if(collection==="sources"){const source=record as ContentInspection["sources"][number];return <li key={source.id}>{source.publisher} ({source.id})<p>License: {describeObservation(source.license)}. Redistribution: {describeObservation(source.redistribution)}. Offline use: {describeObservation(source.offline_usable)}.</p><p>Attribution: {describeObservation(source.attribution)}</p></li>;}const finding=record as {code?:string};return <li key={n}>{finding.code?.replaceAll("_"," ")??JSON.stringify(record)}</li>;})}</ul></details>;
}
