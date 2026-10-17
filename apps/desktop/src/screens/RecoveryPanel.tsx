import { useEffect, useRef, useState } from "react";
import type { CommandClient } from "../commands";
import type { BackupPreview, DatabaseStatus, RestoreReceipt } from "../commands/recovery";
import { Button, Field, Panel } from "../ui";
const ROW_LABELS: Record<string, string> = { profiles: "Character profiles", snapshots: "Imported snapshots", snapshot_observations: "Imported observations", profile_preferences: "Profile preferences", imports: "Import history", dataset_revisions: "Content revisions", source_records: "Content sources", source_conflicts: "Source conflicts", dataset_manifests: "Content manifests", active_datasets: "Active content selections", party_profiles: "Local parties", party_participants: "Party participants", snapshot_promotions: "Snapshot selection history", sessions: "Recorded sessions", reminder_state: "Reminder preferences and history", session_calibration_annotations: "Session annotations", party_plan_proposals: "Local plan proposals" };
export function RecoveryPanel({ client, onRestored, onChecked }: { client: CommandClient; onRestored: () => void; onChecked?: () => void }) {
  const [status, setStatus] = useState<DatabaseStatus | null>(null);
  const [backupPath, setBackupPath] = useState("");
  const [restorePath, setRestorePath] = useState("");
  const [preview, setPreview] = useState<BackupPreview | null>(null);
  const [confirmed, setConfirmed] = useState(false);
  const [busy, setBusy] = useState(false);
  const [message, setMessage] = useState("");
  const [error, setError] = useState("");
  const token = useRef(0);
  const working = useRef(false);
  useEffect(() => {
    const serial = ++token.current;
    working.current = false; setBusy(false); setPreview(null); setConfirmed(false); setStatus(null);
    let timer: ReturnType<typeof setTimeout> | undefined;
    const poll = () => { void client.getDatabaseStatus().then((result) => {
      if (token.current !== serial) return;
      if (result.ok) { setStatus(result.value); if (result.value.state === "checking") timer = setTimeout(poll, 500); else if (timer !== undefined) (onChecked ?? onRestored)(); } else setError(result.error.message);
    }); }; poll();
    return () => { token.current += 1; if (timer !== undefined) clearTimeout(timer); };
  }, [client, onRestored, onChecked]);
  async function action(kind: "check" | "backup" | "inspect" | "restore") {
    if (working.current) return;
    if (kind === "restore" && (!preview || !confirmed)) return;
    working.current = true; setBusy(true); setError(""); setMessage("");
    const serial = token.current;
    const result = kind === "check" ? await client.checkDatabase() : kind === "backup" ? await client.createDatabaseBackup(backupPath) : kind === "inspect" ? await client.inspectDatabaseBackup(restorePath) : await client.restoreDatabaseBackup(preview?.candidate_id ?? "");
    if (token.current !== serial) return;
    working.current = false; setBusy(false);
    if (!result.ok) { setError(result.error.message); if (kind === "inspect" || kind === "restore") { setPreview(null); setConfirmed(false); } return; }
    if (kind === "inspect") { setPreview(result.value as BackupPreview); setConfirmed(false); setMessage("Backup validated and staged. Review before replacing local state."); }
    else if (kind === "restore") { const receipt = result.value as RestoreReceipt; setPreview(null); setConfirmed(false); setMessage(`Restore complete. ${receipt.original_files_retained ? "Original files preserved" : "Recovery directory created"} at ${receipt.recovery_directory}. File watching is stopped.${receipt.pending_marker_cleanup_required ? " Recovery marker cleanup failed; next startup will still require recovery." : ""}`); onRestored(); const current = await client.getDatabaseStatus(); if (token.current === serial && current.ok) setStatus(current.value); }
    else setMessage(kind === "backup" ? "Consistent backup created at the selected new path." : "Database integrity and stored identities passed validation.");
  }
  return <Panel title="Database backup and recovery">
    <p>Choose local SQLite files explicitly. Restore replaces all local profiles, content and feature history with the inspected backup. Original files are preserved; file watching stops. Close other application instances before restoring.</p>
    {status && <p>{status.state === "ready" ? "Database ready" : status.state === "checking" ? "Checking database at startup…" : "Database recovery required"}; supported schema {status.schema_version}; restore limit {status.restore_limit_bytes / (1024 * 1024)} MiB.</p>}
    {status?.preserved_recovery_directory && <p>Interrupted restore retained files at {status.preserved_recovery_directory}. Inspect a compatible SQLite backup from that directory.</p>}
    {status?.error && <p role="alert">{status.error.message}</p>}
    {error && <p role="alert">{error}</p>}
    {message && <p role="status">{message}</p>}
    <Button disabled={busy || status?.state !== "ready"} onClick={() => void action("check")}>Check database integrity</Button>
    <Field label="New backup file path" help="Path of a SQLite file that does not exist yet. The backup is a consistent copy of all local state.">{(c) => <input {...c} value={backupPath} disabled={busy} onChange={(event) => setBackupPath(event.target.value)} />}</Field>
    <Button disabled={busy || status?.state !== "ready" || !backupPath.trim()} onClick={() => void action("backup")}>Create consistent backup</Button>
    <Field label="Backup file to restore" help="A SQLite backup made by this application. Inspecting only validates and stages it; nothing is replaced until you confirm a restore.">{(c) => <input {...c} value={restorePath} disabled={busy} onChange={(event) => { setRestorePath(event.target.value); setPreview(null); setConfirmed(false); }} />}</Field>
    <Button disabled={busy || !status || status.state === "checking" || !restorePath.trim()} onClick={() => void action("inspect")}>Inspect backup</Button>
    {preview && <div>
      <p>Backup schema {preview.inspection.source_schema_version}; restored schema {preview.inspection.restored_schema_version}.</p>
      <ul>{Object.entries(preview.inspection.rows).map(([table, rows]) => <li key={table}>{ROW_LABELS[table] ?? "Stored records"}: {rows}</li>)}</ul>
      <label><input type="checkbox" disabled={busy} checked={confirmed} onChange={(event) => setConfirmed(event.target.checked)} />Replace all local database state with this inspected backup</label>
      <Button variant="danger" disabled={busy || !confirmed} onClick={() => void action("restore")}>Restore inspected backup</Button>
    </div>}
  </Panel>;
}
