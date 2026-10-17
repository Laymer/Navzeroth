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
  const [error, setError] = useState("");
  const [message, setMessage] = useState("");
  const [busy, setBusy] = useState(false);
  // The newest request wins; a response for an older request or another client is dropped.
  const ticket = useRef(0);
  const activeClient = useRef(client);
  activeClient.current = client;
  // True once the person has edited the draft since it was last loaded or saved. The 60 s
  // refresh tick (refreshKey) reloads backend state but must never replace unsaved input.
  const dirty = useRef(false);
  const shown = useRef<CommandClient | null>(null);

  useEffect(() => {
    const mine = ++ticket.current;
    if (shown.current !== client) {
      // A different client is a different store: nothing from the old one may stay on screen.
      dirty.current = false;
      setLoaded(null);
      setDraft(null);
      setBusy(true);
    }
    setError("");
    void Promise.all([client.getAppSettings(), client.listProfiles(), client.listParties()]).then(([settings, p, ids]) => {
      if (mine !== ticket.current) return;
      setBusy(false);
      if (!settings.ok) { setError(settings.error.message); return; }
      if (settings.value.settings.schema_version !== 1 || !Number.isSafeInteger(settings.value.revision) || settings.value.revision < 0) { setError("Unsupported local settings response"); return; }
      if (!p.ok) { setError(p.error.message); return; }
      if (!ids.ok) { setError(ids.error.message); return; }
      const keep = dirty.current && shown.current === client;
      shown.current = client;
      // Unsaved edits keep the revision they were made against, so saving them over a
      // changed store fails the revision check instead of silently overwriting it.
      setLoaded((old) => ({ client, record: keep && old ? old.record : settings.value, profiles: p.value, parties: ids.value }));
      if (!keep) { setDraft(settings.value.settings); dirty.current = false; }
    });
    return () => { ticket.current++; };
  }, [client, refreshKey]);

  const current = loaded?.client === client ? loaded : null;
  function edit(update: (value: AppSettings) => AppSettings) { dirty.current = true; setMessage(""); setDraft((old) => (old ? update(old) : old)); }
  async function save() {
    if (!draft || !current) return;
    const mine = ++ticket.current;
    const saving = client;
    setBusy(true);
    setError("");
    const result = await client.updateAppSettings(JSON.stringify({ schema_version: 1, expected_revision: current.record.revision, settings: draft }));
    if (mine !== ticket.current || activeClient.current !== saving) return;
    setBusy(false);
    if (result.ok) {
      dirty.current = false;
      setLoaded({ ...current, record: result.value });
      setDraft(result.value.settings);
      setMessage("Local settings saved. Watching and windows remain explicit controls.");
      onChanged();
    } else setError(result.error.message);
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
        {error && <p role="alert">{error}</p>}
        {message && <p role="status">{message}</p>}
        {!current || !draft ? (
          <p role="status">Loading local settings…</p>
        ) : (
          <fieldset disabled={busy}>
            <legend>Local configuration</legend>
            <Field label="Character or party" help="The character or party the Today screen and planners use by default. None selected leaves the choice to each screen.">
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
            <Field label="Manual goal identifier" help="Optional goal profile the planner starts from. Empty means no default goal.">
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
            <Field label="Contrast" help="System follows the operating system. High contrast always uses the black and white palette.">
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
        <h3>Optional integrations</h3>
        <p>Official game API: unverified and unavailable. Hosted sharing and telemetry upload: disabled; no account or credential is configured here.</p>
      </Panel>
    </div>
  );
}
