import { useCallback, useEffect, useRef, useState } from "react";
import type { AppError, CommandClient } from "../../commands";
import type { ProgressionReceipt, ProgressionView } from "../../../../../schemas/v1/progression-api";
import { Button, Field, Panel } from "../../ui";
import { inputError, manualRequest } from "./inputs";
import { ProgressionRecords } from "./ProgressionRecords";

interface Props { client: CommandClient; profileId: string; snapshotId: string; disabled: boolean }
const OUTDATED: AppError = { code: "SNAPSHOT_OUTDATED", message: "The selected snapshot changed. Refresh stored state before opening progression inputs again.", retryable: false };

/** Remount all input and request state when its owning client or predecessor changes. */
export function ProgressionInputs(props: Props) {
  const owner = useRef({ client: props.client, profileId: props.profileId, snapshotId: props.snapshotId, key: 0 });
  if (owner.current.client !== props.client || owner.current.profileId !== props.profileId || owner.current.snapshotId !== props.snapshotId) {
    owner.current = { client: props.client, profileId: props.profileId, snapshotId: props.snapshotId, key: owner.current.key + 1 };
  }
  return <InputToggle key={owner.current.key} {...props} />;
}

function InputToggle(props: Props) {
  const [open, setOpen] = useState(false);
  return <Panel title="Progression inputs">
    <p>Record information for the selected character snapshot. Manual entries describe what you entered; saving does not change the game.</p>
    {open ? <>
      <Button onClick={() => setOpen(false)}>Close progression inputs</Button>
      <InputEditor {...props} />
    </> : <Button disabled={props.disabled} onClick={() => setOpen(true)}>Open progression inputs</Button>}
  </Panel>;
}

function InputEditor({ client, profileId, snapshotId, disabled }: Props) {
  const [view, setView] = useState<ProgressionView | null>(null);
  const [error, setError] = useState<AppError | null>(null);
  const [receipt, setReceipt] = useState<ProgressionReceipt | null>(null);
  const [loading, setLoading] = useState(true);
  const [busy, setBusy] = useState(false);
  const [text, setText] = useState("");
  const [money, setMoney] = useState("");
  const [points, setPoints] = useState("");
  const generation = useRef(0);
  const active = useRef(true);
  const mutating = useRef(false);
  const refresh = useCallback(async () => {
    const ticket = ++generation.current;
    setLoading(true);
    const result = await client.getProgressionView(profileId);
    if (!active.current || generation.current !== ticket) return;
    setLoading(false);
    if (!result.ok) { setView(null); setError(result.error); }
    else if (result.value.profile_id !== profileId || result.value.binding.snapshot !== snapshotId) { setView(null); setError(OUTDATED); }
    else { setView(result.value); setError(null); }
  }, [client, profileId, snapshotId]);
  useEffect(() => {
    active.current = true;
    void refresh();
    const timer = window.setInterval(() => { if (!mutating.current) void refresh(); }, 60000);
    return () => { active.current = false; generation.current++; window.clearInterval(timer); };
  }, [refresh]);

  const invalid = inputError(money, points);
  const blocked = disabled || busy || loading || view === null;
  async function save(kind: "manual" | "import") {
    if (blocked || mutating.current || view === null || (kind === "manual" && invalid !== null) || (kind === "import" && text.trim() === "")) return;
    mutating.current = true; setBusy(true); setError(null); setReceipt(null);
    const ticket = ++generation.current;
    const result = kind === "manual" ? await client.saveManualProgression(manualRequest(view, money, points))
      : await client.importProgression({ profile_id: profileId, expected: view.binding, text });
    if (!active.current || ticket !== generation.current) return;
    mutating.current = false; setBusy(false);
    if (!result.ok) { setError(result.error); if (result.error.code === "SNAPSHOT_OUTDATED") setView(null); return; }
    if (result.value.profile_id !== profileId || result.value.snapshot_id !== snapshotId) { setView(null); setError(OUTDATED); return; }
    setReceipt(result.value);
    if (kind === "manual") { setMoney(""); setPoints(""); } else setText("");
    await refresh();
  }

  return <div>
    {loading && <p role="status">Loading progression observations…</p>}
    {error !== null && <p role="alert">Error {error.code}: {error.message}</p>}
    {receipt !== null && <p role="status">{receipt.stored ? "Saved progression capture" : "Progression capture was already stored"}: {receipt.companion_id}{receipt.fixture ? " (synthetic fixture)" : ""}.</p>}
    <Button disabled={disabled || busy || loading} onClick={() => void refresh()}>Refresh progression observations</Button>
    <fieldset disabled={blocked}>
      <legend>Import progression companion</legend>
      <Field label="Progression export text" hint="Paste an ANAV1 progression companion exported for this exact character snapshot.">{(c) => <textarea {...c} rows={4} value={text} onChange={(e) => setText(e.target.value)} />}</Field>
      <Button disabled={text.trim() === ""} onClick={() => void save("import")}>Import progression companion</Button>
    </fieldset>
    <fieldset disabled={blocked}>
      <legend>Manual progression entry</legend>
      <p>Blank fields remain unknown. Enter 0 only when you know the value is zero. Other observations, including spells, talents, professions and inventory, remain unknown in this entry.</p>
      <Field label="Money (copper)" hint="Total copper; 1 gold = 10000 copper and 1 silver = 100 copper.">{(c) => <input {...c} inputMode="numeric" maxLength={20} value={money} onChange={(e) => setMoney(e.target.value)} />}</Field>
      <Field label="Available talent points">{(c) => <input {...c} inputMode="numeric" maxLength={5} value={points} onChange={(e) => setPoints(e.target.value)} />}</Field>
      {invalid !== null && <p>{invalid}</p>}
      <Button disabled={invalid !== null} onClick={() => void save("manual")}>Save manual entry</Button>
    </fieldset>
    {view !== null && <>
      {view.fixture && <p role="note">Synthetic progression observations for a fixture profile.</p>}
      <ProgressionRecords records={view.imported} origin="Imported" />
      <ProgressionRecords records={view.manual} origin="Manual" />
    </>}
  </div>;
}
