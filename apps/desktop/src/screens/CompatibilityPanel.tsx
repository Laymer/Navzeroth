import { useEffect, useRef, useState } from "react";
import {LocalDataPanel} from "./LocalDataPanel";
import type { CommandClient } from "../commands";
import type { CompatibilityReport, EvidenceState } from "../commands/compatibility";
const LABELS: Record<EvidenceState, string> = { reported_flow: "Limited user-reported flow", function_present_only: "Function present; usability untested", reported_missing: "Reported missing in that diagnostic", unverified: "Unverified", assumed_reference: "Reference assumption; no client session" };
function Counts({ entries }: { entries: Record<string, number> }) { return <ul>{Object.entries(entries).map(([name, count]) => <li key={name}>{name.replaceAll("_", " ")}: {count}</li>)}</ul>; }
export function CompatibilityPanel({ client, profileId, refreshKey }: { client: CommandClient; profileId: string | null; refreshKey: number }) {
  const [report, setReport] = useState<CompatibilityReport | null>(null);
  const [message, setMessage] = useState(""); const [busy, setBusy] = useState(false); const serial = useRef(0);
  useEffect(() => { serial.current++; setReport(null); setMessage(""); setBusy(false); return () => { serial.current++; }; }, [client, profileId, refreshKey]);
  async function load() {
    if (busy) return; const ticket = ++serial.current; setReport(null); setBusy(true); setMessage("");
    const result = await client.getCompatibilityReport(profileId);
    if (ticket !== serial.current) return;
    setBusy(false); if (result.ok) setReport(result.value); else setMessage(`Report unavailable: ${result.error.code}.`);
  }
  return <section aria-labelledby="compatibility-title">
    <h2 id="compatibility-title">Compatibility and source coverage</h2>
    <LocalDataPanel client={client}/>
    <p>Inspect finite recorded client evidence and the exact active dataset. Function presence, synthetic checks and historical user reports do not establish that the installed addon or every game context works.</p>
    <button type="button" disabled={busy} onClick={() => void load()}>Inspect compatibility and coverage</button>
    {busy && <p role="status">Reading local evidence…</p>}{message && <p role="alert">{message}</p>}
    {report && <>
      <p>{report.limitations}</p>
      {report.selected ? <p>Selected build {report.selected.context.build.edition} / {report.selected.context.build.build} / interface {report.selected.context.build.interface}; namespace {report.selected.context.namespace}. {report.selected.fixture ? "SYNTHETIC imported fixture." : "Imported character state."} {report.selected.snapshot_present ? "A current imported snapshot exists." : "Current snapshot missing."}</p> : <p>No selected profile: active dataset coverage is unavailable, not zero.</p>}
      <h3>Recorded build evidence</h3>
      {report.recorded_builds.map((build) => <section key={`${build.build.edition}:${build.build.build}:${build.build.interface}`} aria-label={`Recorded build ${build.build.build}`}>
        <h4>{build.build.edition} / {build.build.build} / interface {build.build.interface}</h4>
        <p>{LABELS[build.basis]}; report date {build.reported_on}; historical diagnostic addon {build.addon_version ?? "unknown"}. Current installed addon revision remains unknown.</p>
        <p>{build.matches_selected_build === null ? "No selected build to compare." : build.matches_selected_build ? "Same build tuple as the selection; evidence remains historical and scoped." : "Different build tuple: these observations do not verify the selected build."}</p>
        <p>{build.limitations}</p>
        <dl>{build.capabilities.map((capability) => <div key={capability.capability}><dt>{capability.capability}: {LABELS[capability.state]}</dt><dd>{capability.statement} Evidence: {capability.source}</dd></div>)}</dl>
      </section>)}
      <h3>Exact active revision coverage</h3>
      {!report.active ? <p>No active revision for this selected context and fixture mode. Coverage remains unavailable.</p> : <>
        <p>Revision {report.active.revision}; namespace {report.active.context.namespace}; {report.active.fixture ? "SYNTHETIC" : "reference"} records. Structural validation {report.active.structurally_valid ? "passes" : "fails"}; this does not mean complete gameplay coverage.</p>
        <p>{report.active.finite_scope}</p>
        <table><thead><tr><th>Collection</th><th>Records</th><th>Records without evidence</th></tr></thead><tbody>{Object.entries(report.active.counts.collections).map(([name, count]) => <tr key={name}><th scope="row">{name.replaceAll("_", " ")}</th><td>{count.total}</td><td>{count.evidence?.records_without_evidence ?? "not tracked"}</td></tr>)}</tbody></table>
        <h4>Declared travel status</h4><Counts entries={report.active.counts.edge_status} />
        <h4>Coverage gaps</h4><Counts entries={report.active.gaps_by_code} />{Object.keys(report.active.gaps_by_code).length === 0 && <p>No gap findings in this finite report; missing playable coverage can still remain.</p>}
        <h4>Source inventory and policy gaps</h4><p>{report.active.sources.registered} registered sources; {report.active.sources.referenced_distinct} referenced sources; {report.active.sources.unregistered_distinct} unregistered source references. {report.active.sources.declared_conflicts} declared conflicts.</p>
        <p>Redistribution records: permitted {report.active.sources.redistribution_values.permitted}, not permitted {report.active.sources.redistribution_values.not_permitted}, conditional {report.active.sources.redistribution_values.conditional}. Unrecorded or conditional permission does not authorize redistribution.</p>
        <p>Offline declarations: known true {report.active.sources.offline_values.known_true}, known false {report.active.sources.offline_values.known_false}; other statuses remain separate below.</p>
        <details><summary>Exact field, confidence, projection and source policy counts</summary><pre>{JSON.stringify({ counts: report.active.counts, sources: report.active.sources, errors: report.active.errors_by_code }, null, 2)}</pre></details>
      </>}
      <h3>Checks still unverified</h3><ul>{report.unverified_checks.map((check) => <li key={check.check}>{check.check.replaceAll("_", " ")}: {check.required_evidence}</li>)}</ul>
      <h3>Official API evidence</h3><p>{report.official_api}</p>
    </>}
  </section>;
}
