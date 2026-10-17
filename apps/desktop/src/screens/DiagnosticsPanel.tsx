import { useEffect, useRef, useState } from "react";
import type { DiagnosticsClient, DiagnosticReport } from "../commands/diagnostics";
import { Button, Field, Panel } from "../ui";
const CODES = new Set(["INVALID_INPUT", "UNSUPPORTED_SCHEMA", "BUILD_MISMATCH", "SNAPSHOT_OUTDATED", "MISSING_REQUIRED_OBSERVATION", "DATASET_INCOMPLETE", "PROVIDER_UNAVAILABLE", "IMPORT_TOO_LARGE", "OPERATION_CANCELED", "STORAGE_ERROR", "INTERNAL"]);
function code(value: string) { return CODES.has(value) ? value : "INTERNAL"; }
export function DiagnosticsPanel({ client, refreshKey = 0 }: { client: DiagnosticsClient; refreshKey?: number }) {
  const [path, setPath] = useState(""); const [report, setReport] = useState<DiagnosticReport | null>(null);
  const [message, setMessage] = useState(""); const [busy, setBusy] = useState(false); const generation = useRef(0);
  useEffect(() => { generation.current++; setReport(null); setMessage(""); setBusy(false); return () => { generation.current++; }; }, [client, refreshKey]);
  async function run(exportFile: boolean) {
    if (busy) return; const epoch = generation.current; setBusy(true); setMessage("");
    try {
      if (exportFile) {
        const result = await client.exportDiagnostics(path);
        if (epoch !== generation.current) return;
        setMessage(result.ok ? `Local diagnostic file created (${result.value.bytes} bytes).` : `Export failed: ${code(result.error.code)}.`);
      } else {
        const result = await client.previewDiagnostics();
        if (epoch !== generation.current) return;
        if (result.ok) { setReport(result.value); setMessage("Preview captured. No file was created."); }
        else { setReport(null); setMessage(`Preview failed: ${code(result.error.code)}.`); }
      }
    } catch { if (epoch === generation.current) setMessage("Diagnostic operation failed: INTERNAL."); }
    finally { if (epoch === generation.current) setBusy(false); }
  }
  return <Panel title="Local diagnostics" level={3}>
    <p>Create an explicit local report of versions, build compatibility, coverage and bounded operation codes. Personal exports, account paths, chat, tokens and character identities are excluded. Nothing is uploaded.</p>
    <p>Bundled addon version describes this package; the installed addon version remains unknown. Historical client reports do not verify the currently installed revision. Import statistics and timing cover foreground attempts observed in this process, including successful no-op imports. Watcher imports and earlier processes are excluded. Missing evidence remains unknown.</p>
    <Button disabled={busy} onClick={() => void run(false)}>Preview diagnostic metadata</Button>
    <Field label="New diagnostic file path" help="Path of a file that does not exist yet. Existing files are never overwritten. Nothing is uploaded.">{(c) => <input {...c} value={path} maxLength={4096} disabled={busy} onChange={e => setPath(e.target.value)} />}</Field>
    <Button disabled={busy || !path.trim()} onClick={() => void run(true)}>Create local diagnostic file</Button>
    <p>Choose a new file. Existing files are preserved. Export captures current metadata again.</p>
    {message && <p role="status">{message}</p>}
    {report && <><p>App {report.versions.app}; bundled addon {report.versions.bundled_addon}; installed addon unknown. OS: {report.os}.</p>
      {report.dataset_context_mismatch && <p role="alert">Dataset context mismatch: coverage excluded.</p>}
      <p>Recorded imports: {report.telemetry.import_attempts === null ? "unknown" : report.telemetry.import_attempts}.</p>
      <details><summary>Allowlisted metadata preview</summary><pre>{JSON.stringify(report, null, 2)}</pre></details></>}
  </Panel>;
}
