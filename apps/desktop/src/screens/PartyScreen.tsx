import { useEffect, useRef, useState } from "react";
import type { CommandClient } from "../commands";
import type { AppError, ProfileSummary, StoredParty } from "../commands/contract";
import { describeAge, describeUncertainty } from "../atlas/describe";
import { describeObservation } from "../observation";
import { PartyEligibility } from "./PartyEligibility";
import { PartyPlanning } from "./PartyPlanning";

type Detail = { phase: "loading" } | { phase: "ready"; party: StoredParty | null } | { phase: "error"; error: AppError };
type Editor = { id: string; members: string[]; existing: boolean };

function profileLabel(profile: ProfileSummary): string {
  const context = profile.context;
  return `${profile.display_name}${context ? ` — ${context.build.edition}/${context.build.build}/${context.build.interface}, ${context.namespace}, ${profile.fixture ? "synthetic" : "imported"}` : " — demo"}`;
}

export function PartyScreen({ client, refreshKey = 0 }: { client: CommandClient; refreshKey?: number }) {
  const [profiles, setProfiles] = useState<ProfileSummary[]>([]);
  const [ids, setIds] = useState<string[]>([]);
  const [selected, setSelected] = useState("");
  const [mode, setMode] = useState<"demo" | "stored" | null>(null);
  const [detail, setDetail] = useState<Detail>({ phase: "loading" });
  const [editor, setEditor] = useState<Editor | null>(null);
  const [error, setError] = useState<AppError | null>(null);
  const [loading, setLoading] = useState(true);
  const [busy, setBusy] = useState(false);
  const busyRef = useRef(false);
  const [message, setMessage] = useState("");
  const [revision, setRevision] = useState(0);

  useEffect(() => {
    let active = true;
    void (async () => {
      const [status, list, parties] = await Promise.all([client.getAppStatus(), client.listProfiles(), client.listParties()]);
      if (!active) return;
      const failure = !status.ok ? status.error : !list.ok ? list.error : !parties.ok ? parties.error : null;
      setLoading(false);
      if (failure) { setError(failure); return; }
      if (status.ok && list.ok && parties.ok) {
        setMode(status.value.data_mode); setProfiles(list.value); setIds(parties.value);
        setSelected((old) => parties.value.includes(old) ? old : parties.value[0] ?? "");
      }
    })();
    return () => { active = false; };
  }, [client, refreshKey, revision]);

  useEffect(() => {
    let active = true;
    if (!selected) return;
    void client.getParty(selected).then((result) => {
      if (active) setDetail(result.ok ? { phase: "ready", party: result.value } : { phase: "error", error: result.error });
    });
    return () => { active = false; };
  }, [client, selected, refreshKey, revision]);

  async function save() {
    if (!editor || busyRef.current) return;
    busyRef.current = true; setBusy(true); setError(null); setMessage("");
    const result = await client.saveParty(editor.id.trim(), editor.members);
    busyRef.current = false; setBusy(false);
    if (!result.ok) { setError(result.error); return; }
    setSelected(editor.id.trim()); setDetail({ phase: "loading" }); setEditor(null);
    setMessage("Party membership saved."); setRevision((old) => old + 1);
  }
  async function remove() {
    if (!selected || busyRef.current) return;
    busyRef.current = true; setBusy(true); setError(null); setMessage("");
    const result = await client.deleteParty(selected);
    busyRef.current = false; setBusy(false);
    if (!result.ok) { setError(result.error); return; }
    setSelected(""); setDetail({ phase: "ready", party: null }); setEditor(null);
    setMessage("Party removed. Character snapshots and preferences are preserved."); setRevision((old) => old + 1);
  }
  const party = selected && detail.phase === "ready" && detail.party?.id === selected ? detail.party : null;
  const editable = mode === "stored" && !busy && !loading;
  const duplicateName = editor !== null && !editor.existing && ids.includes(editor.id.trim());
  return <section className="party-screen" aria-labelledby="party-title">
    <h2 id="party-title">Party</h2>
    <p>Choose separately imported characters for a local party. Each participant keeps their own progress and capture age.</p>
    {mode === "demo" && <p className="demo-label">DEMO — local party changes require the desktop application.</p>}
    {loading && <p role="status">Loading parties…</p>}
    {error && <p role="alert">Error {error.code}: {error.message}</p>}
    {message && <p role="status">{message}</p>}
    <button type="button" disabled={busy} onClick={() => { setError(null); setRevision((old) => old + 1); }}>Refresh parties</button>
    <button type="button" disabled={!editable} onClick={() => { setEditor({ id: "", members: [], existing: false }); setMessage(""); }}>New party</button>
    {!loading && ids.length === 0 && <p>No local parties yet.</p>}
    {ids.length > 0 && <label>Selected party <select value={selected} disabled={busy || editor !== null} onChange={(event) => { setSelected(event.target.value); setDetail({ phase: "loading" }); setError(null); setMessage(""); }}>
      {ids.map((id) => <option key={id} value={id}>{id}</option>)}
    </select></label>}
    {selected && detail.phase === "loading" && <p role="status">Loading participant snapshots…</p>}
    {selected && detail.phase === "error" && <p role="alert">Error {detail.error.code}: {detail.error.message}</p>}
    {selected && detail.phase === "ready" && detail.party === null && <p>This party is no longer available. Refresh the party list.</p>}
    {party && <>
      <h3>{party.id}</h3>
      <button type="button" disabled={!editable || editor !== null} onClick={() => setEditor({ id: party.id, members: party.participants.map((p) => p.profile_id), existing: true })}>Edit membership</button>
      <button type="button" disabled={!editable || editor !== null} onClick={() => void remove()}>Remove local party</button>
      <ul className="party-participants">{party.participants.map((participant) => <li key={participant.profile_id}>
        <h4>{participant.character}</h4>
        {participant.snapshot === null ? <p>Missing imported snapshot. Import this character to restore their state.</p> : <Participant snapshot={participant.snapshot} />}
      </li>)}</ul>
      <PartyEligibility key={party.id} client={client} partyId={party.id} refreshKey={refreshKey + revision} />
      {party.participants.length >= 2 && <PartyPlanning key={`planning:${party.id}`} client={client} partyId={party.id} refreshKey={refreshKey + revision} />}
      {party.participants.length < 2 && <p>Add a second character to plan a shared session.</p>}
    </>}
    {editor && <form onSubmit={(event) => { event.preventDefault(); void save(); }}>
      <fieldset disabled={busy}><legend>{editor.existing ? "Edit party membership" : "Create local party"}</legend>
        <label>Party name <input value={editor.id} maxLength={256} readOnly={editor.existing} required onChange={(event) => setEditor({ ...editor, id: event.target.value })} /></label>
        {duplicateName && <p>A party with this name already exists. Select it to edit membership.</p>}
        <p>Select 1–40 different characters with the same edition, build, interface, namespace and synthetic status.</p>
        {profiles.length === 0 && <p>No stored profiles yet. Import characters first.</p>}
        {profiles.map((profile) => <label key={profile.id}><input type="checkbox" checked={editor.members.includes(profile.id)} disabled={!editor.members.includes(profile.id) && editor.members.length >= 40} onChange={(event) => setEditor({ ...editor, members: event.target.checked ? [...editor.members, profile.id] : editor.members.filter((id) => id !== profile.id) })} />{profileLabel(profile)}</label>)}
        {editor.members.filter((id) => !profiles.some((p) => p.id === id)).map((id) => <label key={id}><input type="checkbox" checked onChange={() => setEditor({ ...editor, members: editor.members.filter((member) => member !== id) })} />Missing profile: {party?.participants.find((p) => p.profile_id === id)?.character ?? id}</label>)}
        <button type="submit" disabled={!editable || editor.members.length === 0 || !editor.id.trim() || duplicateName}>Save membership</button>
        <button type="button" onClick={() => setEditor(null)}>Cancel editing</button>
      </fieldset>
    </form>}
  </section>;
}

function Participant({ snapshot }: { snapshot: NonNullable<StoredParty["participants"][number]["snapshot"]> }) {
  const envelope = snapshot.envelope;
  return <>
    {(envelope.fixture || envelope.namespace === "demo") && <p className="demo-label">SYNTHETIC — fixture or demo data, not from your game</p>}
    <p>Last imported: {describeAge(snapshot.freshness)}</p>
    {snapshot.freshness.status === "uncertain" && <p>{describeUncertainty(snapshot.freshness.reason)}</p>}
    {snapshot.freshness.status !== "fresh" && <p className="stale-note">This participant’s progress may have changed since capture.</p>}
    <dl><dt>Snapshot</dt><dd>{envelope.payload.id}</dd>
      <dt>Level</dt><dd>{describeObservation(envelope.payload.level)}</dd>
      <dt>Role</dt><dd>{describeObservation(envelope.payload.role)}</dd>
    </dl>
    <h5>Imported quest progress</h5>
    {Object.keys(envelope.payload.quests).length === 0 ? <p>No quest records in this import.</p> : <ul>{Object.values(envelope.payload.quests).filter((quest) => quest !== undefined).map((quest) => <li key={quest.quest}>
      {quest.quest}: accepted {describeObservation(quest.accepted)}, completed {describeObservation(quest.completed)}, turned in {describeObservation(quest.turned_in)}
      {Object.entries(quest.objectives).map(([id, observation]) => observation && <p key={id}>Objective {id}: {describeObservation(observation)}</p>)}
    </li>)}</ul>}
  </>;
}
