import { useCallback, useEffect, useRef, useState } from "react";
import type { AppError, CommandClient, ProfileSummary } from "../commands";
import type { FileInputFormat, HistoryEntry, ImportReceipt, StoredSnapshot, WatchState, WatchStatus } from "../commands/contract";
import { describeAge } from "../atlas/describe";
import { describeObservation } from "../observation";

const WATCH_LABELS: Record<WatchStatus, string> = {
  waiting_for_stable_file: "Waiting for a complete, stable save.", imported: "The saved snapshot was imported.",
  missing: "The selected file is missing; waiting for it to return.", unreadable: "The selected file cannot be read.",
  invalid_file: "Select a regular export file.", retry_pending: "The save is incomplete or invalid; retrying.",
  retry_exhausted: "The unchanged file failed three checks. Correct the export or save it again.", cancelled: "Watching stopped.",
};
export function describeImport(receipt: ImportReceipt): string {
  const disposition = receipt.disposition;
  if (disposition.outcome === "duplicate") return "Already stored: the current snapshot is unchanged.";
  if (disposition.outcome === "stored") return "Stored as the current snapshot.";
  const reasons = {
    older: "It is older than the current snapshot.",
    same_position: "It records the same capture position with different content.",
    incomparable: "Its capture cannot be reliably ordered against the current snapshot.",
  };
  return `Stored in history only. ${reasons[disposition.reason]} The current snapshot is unchanged.`;
}
const EMPTY_WATCH: WatchState = { active: false, status: null, last_import: null, error: null };
export function ImportScreen({ client, onChanged }: { client: CommandClient; onChanged: () => void }) {
  const [profiles, setProfiles] = useState<ProfileSummary[]>([]);
  const [selected, setSelected] = useState("");
  const [current, setCurrent] = useState<StoredSnapshot | null>(null);
  const [history, setHistory] = useState<HistoryEntry[]>([]);
  const [text, setText] = useState("");
  const [path, setPath] = useState("");
  const [formatKind, setFormatKind] = useState<"anav1_text" | "saved_variables">("anav1_text");
  const [global, setGlobal] = useState("");
  const [key, setKey] = useState("");
  const [watch, setWatch] = useState<WatchState>(EMPTY_WATCH);
  const [receipt, setReceipt] = useState<ImportReceipt | null>(null);
  const [error, setError] = useState<AppError | null>(null);
  const [busy, setBusy] = useState(false);
  const lastWatched = useRef("");
  const mounted = useRef(true);
  const format: FileInputFormat = formatKind === "anav1_text" ? { kind: "anav1_text" } : { kind: "saved_variables", global, key };
  const fileReady = path.trim() !== "" && (formatKind === "anav1_text" || (global !== "" && key !== ""));
  const loadProfile = useCallback(async (id: string) => {
    const [snapshot, entries] = await Promise.all([client.getStoredSnapshot(id), client.getSnapshotHistory(id)]);
    if (!mounted.current) return;
    if (!snapshot.ok) { setError(snapshot.error); return; }
    if (!entries.ok) { setError(entries.error); return; }
    setCurrent(snapshot.value); setHistory(entries.value);
  }, [client]);
  const refresh = useCallback(async (preferred?: string) => {
    const [list, status] = await Promise.all([client.listProfiles(), client.getAppStatus()]);
    if (!mounted.current) return;
    if (!list.ok) { setError(list.error); return; }
    if (!status.ok) { setError(status.error); return; }
    setProfiles(list.value);
    const id = preferred ?? status.value.active_profile ?? "";
    setSelected(id);
    if (id !== "") await loadProfile(id);
    else { setCurrent(null); setHistory([]); }
  }, [client, loadProfile]);
  useEffect(() => {
    mounted.current = true;
    void refresh();
    void client.getWatch().then((result) => { if (mounted.current && result.ok) setWatch(result.value); });
    return () => { mounted.current = false; };
  }, [client, refresh]);
  useEffect(() => {
    if (!watch.active) return;
    let live = true;
    const timer = window.setInterval(() => {
      void client.getWatch().then((result) => {
        if (!live) return;
        if (!result.ok) { setError(result.error); return; }
        setWatch(result.value);
        if (result.value.error !== null) setError(result.value.error);
        const imported = result.value.last_import;
        if (imported !== null && JSON.stringify(imported) !== lastWatched.current) {
          lastWatched.current = JSON.stringify(imported);
          setReceipt(imported); void refresh(); onChanged();
        }
      });
    }, 1000);
    return () => { live = false; window.clearInterval(timer); };
  }, [client, watch.active, refresh, onChanged]);
  async function importSnapshot(kind: "text" | "file") {
    setBusy(true); setError(null);
    const result = kind === "text" ? await client.importText(text) : await client.importFile(path, format);
    if (!mounted.current) return;
    if (result.ok) {
      setReceipt(result.value); await refresh(result.value.profile_id); onChanged();
    } else setError(result.error);
    setBusy(false);
  }
  async function select(id: string) {
    setBusy(true); setError(null);
    const result = await client.selectProfile(id);
    if (!mounted.current) return;
    if (result.ok) { setSelected(id); setReceipt(null); await loadProfile(id); onChanged(); }
    else setError(result.error);
    setBusy(false);
  }
  async function controlWatch(start: boolean) {
    setBusy(true); setError(null);
    const result = start ? await client.startWatch(path, format) : await client.stopWatch();
    if (!mounted.current) return;
    if (result.ok) { setWatch(result.value); if (start) lastWatched.current = ""; }
    else setError(result.error);
    setBusy(false);
  }
  return <section aria-labelledby="import-heading" className="snapshot-import">
    <h2 id="import-heading">Snapshot imports</h2>
    <p>Desktop state is a saved snapshot. Choose the export for your installation, account and character. Watching follows only the selected file.</p>
    <p>ANAV1 snapshot exports are accepted. Diagnostic probe reports are separate.</p>
    {error !== null && <p role="alert">Error {error.code}: {error.message}</p>}
    <label>Snapshot export text<textarea value={text} onChange={(e) => setText(e.target.value)} rows={5} /></label>
    <button type="button" disabled={busy || text.trim() === ""} onClick={() => void importSnapshot("text")}>Import text</button>
    <fieldset disabled={busy}>
      <legend>Selected export file</legend>
      <label>Export file path<input value={path} onChange={(e) => setPath(e.target.value)} /></label>
      <label>File format<select value={formatKind} onChange={(e) => setFormatKind(e.target.value as typeof formatKind)}>
        <option value="anav1_text">ANAV1 text</option><option value="saved_variables">SavedVariables export string</option>
      </select></label>
      {formatKind === "saved_variables" && <>
        <label>SavedVariables global<input value={global} onChange={(e) => setGlobal(e.target.value)} /></label>
        <label>Snapshot export key<input value={key} onChange={(e) => setKey(e.target.value)} /></label>
      </>}
      <button type="button" disabled={!fileReady} onClick={() => void importSnapshot("file")}>Import selected file</button>
      <button type="button" disabled={!fileReady || watch.active} onClick={() => void controlWatch(true)}>Watch selected file</button>
      <button type="button" disabled={!watch.active} onClick={() => void controlWatch(false)}>Stop watching</button>
    </fieldset>
    <p role="status">{watch.status === null ? "File watching is off." : WATCH_LABELS[watch.status]}</p>
    {receipt !== null && <div role="status">
      <p>{describeImport(receipt)}</p>
      <p>Imported {receipt.snapshot_id}: {describeAge(receipt.freshness)} ({receipt.namespace}{receipt.fixture ? ", synthetic fixture" : ""}).</p>
    </div>}
    <h3>Profiles</h3>
    {profiles.length === 0 ? <p>No stored profiles yet.</p> : <label>Selected profile<select value={selected} disabled={busy} onChange={(e) => void select(e.target.value)}>
      {profiles.map((p) => <option key={p.id} value={p.id}>{p.display_name}{p.context ? ` · ${p.context.build.edition}/${p.context.build.build}/${p.context.build.interface} · ${p.context.namespace}` : ""}{p.fixture || p.data_mode === "demo" ? " · synthetic" : ""}</option>)}
    </select></label>}
    {current !== null && <>
      <h3>{current.data_mode === "demo" ? "Demo snapshot" : "Current stored snapshot"}</h3>
      {(current.envelope.fixture || current.envelope.namespace === "demo") && <p role="note" className="demo-label">SYNTHETIC — fixture or demo data, not from your game</p>}
      <p>{current.envelope.payload.id}: {describeAge(current.freshness)}</p>
      <p>Level: {describeObservation(current.envelope.payload.level)}</p>
      <p>Location: {describeObservation(current.envelope.payload.location, (p) => `${p.map.map_id} (${p.x}, ${p.y})`)}</p>
    </>}
    <h3>Recent snapshot history</h3>
    <p>Showing up to 32 recent snapshots.</p>
    {history.length === 0 ? <p>No stored snapshot history.</p> : <ul>{history.map((entry) => <li key={entry.snapshot_id}>{entry.snapshot_id}{entry.is_current ? " (current)" : " (history only)"}: {describeAge(entry.freshness)}</li>)}</ul>}
    <button type="button" disabled={busy} onClick={() => { setError(null); void refresh(); }}>Refresh stored state</button>
  </section>;
}
