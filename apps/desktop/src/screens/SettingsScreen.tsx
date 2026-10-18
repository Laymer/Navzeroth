import {LocalStartupSettings} from "./LocalStartupChoices";
import "./dataScreens.css";
import { useEffect, useRef, useState } from "react";
import type { CommandClient } from "../commands";
import type { ProfileSummary } from "../commands/contract";
import type { AppSettings, SettingsRecord } from "../commands/today";
import { Button, Field, Panel } from "../ui";
import "./settings.css";

type Loaded = { client: CommandClient; record: SettingsRecord; profiles: ProfileSummary[]; parties: string[] };

export function SettingsScreen({ client, refreshKey = 0, onChanged }: { client: CommandClient; refreshKey?: number; onChanged: () => void }) {
  const [loaded, setLoaded] = useState<Loaded | null>(null);
  const [draft, setDraft] = useState<AppSettings | null>(null);
  // Stored settings that differ from the revision the unsaved edits were made against.
  const [newer, setNewer] = useState<SettingsRecord | null>(null);
  const [loadError, setLoadError] = useState("");
  const [saveError, setSaveError] = useState("");
  const [message, setMessage] = useState("");
  const [busy, setBusy] = useState(false);
  const [nonce, setNonce] = useState(0);
  // Refresh and save have separate tickets: a refresh tick must never cancel a save result.
  const loadTicket = useRef(0);
  const saveTicket = useRef(0);
  const saving = useRef(false);
  // Bumped whenever a save settles, so a refresh that started earlier cannot overwrite its result.
  const saveEpoch = useRef(0);
  const activeClient = useRef(client);
  activeClient.current = client;
  // Mirrors of the state the asynchronous handlers need: the draft, and the stored record it is based on.
  const draftRef = useRef<AppSettings | null>(null);
  const baseRef = useRef<SettingsRecord | null>(null);
  const shown = useRef<CommandClient | null>(null);
  const same = (a: AppSettings | null, b: AppSettings | null) => JSON.stringify(a) === JSON.stringify(b);

  useEffect(() => {
    const mine = ++loadTicket.current;
    const epoch = saveEpoch.current;
    if (shown.current !== client) {
      // A different client is a different store: nothing from the old one may stay on screen.
      saveTicket.current++;
      saving.current = false;
      draftRef.current = null;
      baseRef.current = null;
      setLoaded(null);
      setDraft(null);
      setNewer(null);
      setSaveError("");
      setMessage("");
      setBusy(true);
    }
    void Promise.all([client.getAppSettings(), client.listProfiles(), client.listParties()]).then(([settings, p, ids]) => {
      if (mine !== loadTicket.current || epoch !== saveEpoch.current || saving.current) return;
      setBusy(false);
      if (!settings.ok) { setLoadError(settings.error.message); return; }
      if (settings.value.settings.schema_version !== 1 || !Number.isSafeInteger(settings.value.revision) || settings.value.revision < 0) { setLoadError("Unsupported local settings response"); return; }
      if (!p.ok) { setLoadError(p.error.message); return; }
      if (!ids.ok) { setLoadError(ids.error.message); return; }
      setLoadError("");
      const base = shown.current === client ? baseRef.current : null;
      shown.current = client;
      if (base === null || same(draftRef.current, base.settings)) {
        // A clean draft follows the store.
        baseRef.current = settings.value;
        draftRef.current = settings.value.settings;
        setLoaded({ client, record: settings.value, profiles: p.value, parties: ids.value });
        setDraft(settings.value.settings);
        setNewer(null);
      } else {
        // Unsaved edits are kept with the revision they were made against. If the store moved
        // on, say so and let the person choose; nothing is overwritten silently.
        setLoaded({ client, record: base, profiles: p.value, parties: ids.value });
        setNewer(settings.value.revision === base.revision ? null : settings.value);
      }
    });
    return () => { loadTicket.current++; };
  }, [client, refreshKey, nonce]);

  const current = loaded?.client === client ? loaded : null;
  function edit(update: (value: AppSettings) => AppSettings) {
    if (draftRef.current === null) return;
    const next = update(draftRef.current);
    draftRef.current = next;
    setMessage("");
    setDraft(next);
  }
  async function save() {
    const sending = draftRef.current;
    const base = baseRef.current;
    if (!sending || !base || !current || saving.current) return;
    const mine = ++saveTicket.current;
    const owner = client;
    saving.current = true;
    setBusy(true);
    setSaveError("");
    const result = await client.updateAppSettings(JSON.stringify({ schema_version: 1, expected_revision: base.revision, settings: sending }));
    if (mine !== saveTicket.current || activeClient.current !== owner) return;
    saving.current = false;
    saveEpoch.current++;
    setBusy(false);
    if (result.ok) {
      baseRef.current = result.value;
      draftRef.current = result.value.settings;
      setLoaded((old) => (old ? { ...old, record: result.value } : old));
      setDraft(result.value.settings);
      setNewer(null);
      setMessage("Local settings saved. Watching and windows remain explicit controls.");
      onChanged();
    } else {
      setSaveError(result.error.message);
      setNonce((n) => n + 1); // look at the store now: a stale revision becomes an explicit choice
    }
  }
  function keepMine() {
    if (!newer) return;
    baseRef.current = newer;
    setLoaded((old) => (old ? { ...old, record: newer } : old));
    setNewer(null);
    setSaveError("");
    setMessage("Your edits are kept. Save to replace the newer settings with them.");
  }
  function discardMine() {
    if (!newer) return;
    baseRef.current = newer;
    draftRef.current = newer.settings;
    setLoaded((old) => (old ? { ...old, record: newer } : old));
    setDraft(newer.settings);
    setNewer(null);
    setSaveError("");
    setMessage("Your edits were discarded; the stored settings are shown.");
  }

  const selected = draft?.selected_subject;
  const selection = selected?.kind === "character" ? `character:${selected.profile_id}` : selected?.kind === "party" ? `party:${selected.party_id}` : "";
  const profiles = current?.profiles ?? [];
  const parties = current?.parties ?? [];
  return (
    <div className="settings">
      <Panel title="Settings">
        <p>Local choices. Saving does not scan an installation, import files, start watching, open a window, or enable an integration.</p>
        <p className="affiliation-note">Azeroth Navigator is an independent project. It is not affiliated with or endorsed by Blizzard Entertainment.</p>
        {loadError && <p role="alert">{loadError}</p>}
        {saveError && <p role="alert">{saveError}</p>}
        {newer && (
          <div role="alert">
            <p>Settings changed elsewhere since you started editing (stored revision {newer.revision}, yours was made against {baseRef.current?.revision}). Nothing has been overwritten.</p>
            <Button onClick={keepMine}>Keep my edits and save over the newer settings</Button>
            <Button onClick={discardMine}>Discard my edits and load stored settings</Button>
          </div>
        )}
        {message && <p role="status">{message}</p>}
        {!current || !draft ? (
          <p role="status">Loading local settings…</p>
        ) : (
          <fieldset disabled={busy}>
            <legend>Local configuration</legend>
            <Field label="Character or party" help="The character or party the Today screen shows. None selected leaves Today without a subject; other screens have their own selectors.">
              {(c) => (
                <select {...c} value={selection} onChange={(e) => edit((s) => ({ ...s, selected_subject: e.target.value.startsWith("character:") ? { kind: "character", profile_id: e.target.value.slice(10) } : e.target.value.startsWith("party:") ? { kind: "party", party_id: e.target.value.slice(6) } : null }))}>
                  <option value="">None selected</option>
                  {selection && !profiles.some((p) => selection === `character:${p.id}`) && !parties.some((id) => selection === `party:${id}`) && <option value={selection}>Missing saved subject</option>}
                  {profiles.map((p) => <option key={p.id} value={`character:${p.id}`}>{p.display_name} — {p.context?.build.edition ?? "demo"}/{p.context?.build.build ?? "unknown"}, {p.fixture ? "synthetic" : "imported"}</option>)}
                  {parties.map((id) => <option key={id} value={`party:${id}`}>Party: {id}</option>)}
                </select>
              )}
            </Field>
            <Field label="Chosen game installation path" help="Remembered only. Saving never scans the folder; scanning starts from the Compatibility screen.">
              {(c) => <input {...c} maxLength={4096} value={draft.selected_installation ?? ""} onChange={(e) => edit((s) => ({ ...s, selected_installation: e.target.value || null }))} />}
            </Field>
            <Field label="Chosen snapshot export file" help="The export file the Imports screen offers through Use saved export-file choice. Saving does not read or watch it.">
              {(c) => <input {...c} maxLength={4096} value={draft.selected_export?.path ?? ""} onChange={(e) => edit((s) => ({ ...s, selected_export: e.target.value ? { path: e.target.value, format: s.selected_export?.format ?? { kind: "anav1_text" } } : null }))} />}
            </Field>
            {draft.selected_export && (
              <>
                <Field label="Export format">
                  {(c) => (
                    <select {...c} value={draft.selected_export?.format.kind} onChange={(e) => edit((s) => ({ ...s, selected_export: s.selected_export ? { ...s.selected_export, format: e.target.value === "anav1_text" ? { kind: "anav1_text" } : { kind: "saved_variables", global: "AzerothNavigatorDB", key: "snapshot_export" } } : null }))}>
                      <option value="anav1_text">ANAV1 text export</option>
                      <option value="saved_variables">Addon SavedVariables</option>
                    </select>
                  )}
                </Field>
                {draft.selected_export.format.kind === "saved_variables" &&
                  (["global", "key"] as const).map((field) => (
                    <Field key={field} label={`SavedVariables ${field}`} help={field === "global" ? "Name of the addon's saved global table." : "Key inside that table that holds the snapshot export string."}>
                      {(c) => <input {...c} value={draft.selected_export?.format.kind === "saved_variables" ? draft.selected_export.format[field] : ""} onChange={(e) => edit((s) => ({ ...s, selected_export: s.selected_export?.format.kind === "saved_variables" ? { ...s.selected_export, format: { ...s.selected_export.format, [field]: e.target.value } } : s.selected_export }))} />}
                    </Field>
                  ))}
              </>
            )}
            <Field label="Manual goal identifier" help="Optional goal the planner fills in when you press Apply local planning defaults. It is never applied automatically. Empty means no default goal.">
              {(c) => <input {...c} maxLength={256} value={draft.manual_planning_defaults.goal_profile ?? ""} onChange={(e) => edit((s) => ({ ...s, manual_planning_defaults: { ...s.manual_planning_defaults, goal_profile: e.target.value || null } }))} />}
            </Field>
            <Field label="Default time budget (minutes)" help="How long a plan may take, in minutes. Zero is a real budget of no time, not unlimited.">
              {(c) => <input {...c} type="number" min="0" step="1" value={draft.manual_planning_defaults.budget_ms / 60000} onChange={(e) => edit((s) => ({ ...s, manual_planning_defaults: { ...s.manual_planning_defaults, budget_ms: Number(e.target.value) * 60000 } }))} />}
            </Field>
            <Field label="Default stopping service (seconds)" help="Seconds of service time assumed at each stop of a route.">
              {(c) => <input {...c} type="number" min="0" step="1" value={draft.manual_planning_defaults.stopping_service_ms / 1000} onChange={(e) => edit((s) => ({ ...s, manual_planning_defaults: { ...s.manual_planning_defaults, stopping_service_ms: Number(e.target.value) * 1000 } }))} />}
            </Field>
            <Field label="Text size">
              {(c) => <select {...c} value={draft.accessibility.text_scale_percent} onChange={(e) => edit((s) => ({ ...s, accessibility: { ...s.accessibility, text_scale_percent: Number(e.target.value) as AppSettings["accessibility"]["text_scale_percent"] } }))}>{[100, 125, 150, 175, 200].map((n) => <option key={n} value={n}>{n}%</option>)}</select>}
            </Field>
            <Field label="Contrast" help="System follows the operating system. High contrast uses a black and white palette with high-contrast accents.">
              {(c) => <select {...c} value={draft.accessibility.contrast} onChange={(e) => edit((s) => ({ ...s, accessibility: { ...s.accessibility, contrast: e.target.value as "system" | "high" } }))}><option value="system">System</option><option value="high">High contrast</option></select>}
            </Field>
            <Field label="Motion" help="System follows the operating system. Reduced stops animation such as the drifting embers.">
              {(c) => <select {...c} value={draft.accessibility.motion} onChange={(e) => edit((s) => ({ ...s, accessibility: { ...s.accessibility, motion: e.target.value as "system" | "reduced" } }))}><option value="system">System</option><option value="reduced">Reduced</option></select>}
            </Field>
            <label className="settings__check">
              <input type="checkbox" checked={draft.reference_always_on_top} onChange={(e) => edit((s) => ({ ...s, reference_always_on_top: e.target.checked }))} />
              Prefer keep-on-top when explicitly showing the reference window
            </label>
            <Button variant="primary" onClick={() => void save()}>Save local settings</Button>
            <p>Planning defaults are manual assumptions, separate from imported preferences. Apply them explicitly in the planner.</p>
          </fieldset>
        )}
        <LocalStartupSettings client={client}/>
        <h3>Optional integrations</h3>
        <p>Official game API: unverified and unavailable. Hosted sharing and telemetry upload: disabled; no account or credential is configured here.</p>
      </Panel>
    </div>
  );
}
