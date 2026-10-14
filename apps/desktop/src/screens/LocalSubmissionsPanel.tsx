import { useEffect, useRef, useState } from "react";
import type { CommandClient } from "../commands";
import type { SubmissionAction, SubmissionPage, SubmissionRecord, LocalSubmissionExport } from "../commands/localSubmissions";
import example from "../../../../fixtures/desktop/submissions/synthetic-price.json";
export function LocalSubmissionsPanel({ client, enabled, refreshKey }: { client: CommandClient; enabled: boolean; refreshKey: number }) {
  const [page, setPage] = useState<SubmissionPage | null>(null);
  const [cursor, setCursor] = useState<string | null>(null);
  const [text, setText] = useState("");
  const [optIn, setOptIn] = useState(false);
  const [busy, setBusy] = useState(false);
  const [message, setMessage] = useState("");
  const [error, setError] = useState("");
  const [output, setOutput] = useState<LocalSubmissionExport | null>(null);
  const [revision, setRevision] = useState(0);
  const generation = useRef(0);
  const lock = useRef(false);
  useEffect(() => {
    const current = ++generation.current;
    setOutput(null); setPage(null);
    if (!enabled) return;
    void client.listLocalSubmissions(cursor).then((result) => {
      if (current !== generation.current) return;
      if (result.ok) setPage(result.value); else { setPage(null); setError(`${result.error.code}: ${result.error.message}`); }
    });
    return () => { generation.current++; };
  }, [client, enabled, cursor, refreshKey, revision]);
  async function run(action: "submit" | "export" | SubmissionAction, record?: SubmissionRecord) {
    if (lock.current || !enabled) return;
    lock.current = true; setBusy(true); setError(""); setMessage(""); setOutput(null);
    const current = generation.current;
    try {
      if (action === "submit") {
        const result = await client.submitLocalObservation(text, optIn);
        if (current !== generation.current) return;
        if (result.ok) { setMessage(result.value.duplicate ? `Already recorded: ${result.value.record.state}. Erased payloads stay erased.` : "Declaration added to the local review queue."); setOptIn(false); setText(""); setCursor(null); setRevision((n) => n + 1); }
        else setError(`${result.error.code}: ${result.error.message}`);
      } else if (record && action === "export") {
        const result = await client.exportLocalSubmission(record.guard);
        if (current !== generation.current) return;
        if (result.ok) setOutput(result.value); else setError(`${result.error.code}: ${result.error.message}`);
      } else if (record) {
        const result = await client.changeLocalSubmission(record.guard, action as SubmissionAction);
        if (current !== generation.current) return;
        if (result.ok) { setMessage(`Local decision: ${result.value.state}.`); setRevision((n) => n + 1); }
        else setError(`${result.error.code}: ${result.error.message}`);
      }
    } finally { lock.current = false; setBusy(false); }
  }
  return <section aria-labelledby="local-submissions-title">
    <h3 id="local-submissions-title">Local observation review</h3>
    <p>Opt in to store a minimal declaration on this device. No names, chat, account paths or credentials. Local acceptance does not verify gameplay, activate content or grant redistribution permission.</p>
    <p>The queue spans all local dataset contexts. Each declaration retains its exact edition, build, interface and synthetic status.</p>
    <p>Use registered dataset, source, evidence and quest/item identities. Unknown counts remain unknown; one success does not establish a drop rate. Withdrawal erases the payload and keeps a disclosed fingerprint tombstone to prevent duplicate recreation.</p>
    {!enabled && <p>Local submissions require the desktop application.</p>}
    {error && <p role="alert">{error}</p>}{message && <p role="status">{message}</p>}
    <form onSubmit={(event) => { event.preventDefault(); void run("submit"); }}>
      <label>Minimal local observation JSON <textarea rows={8} value={text} disabled={busy || !enabled} onChange={(event) => { setText(event.target.value); setOutput(null); }} /></label>
      <button type="button" disabled={busy || !enabled} onClick={() => { setText(JSON.stringify(example, null, 2)); setOptIn(false); }}>Load synthetic price declaration</button>
      <p>The example requires the matching staged demo-r1 revision; its zero price is synthetic.</p>
      <label><input type="checkbox" checked={optIn} disabled={busy || !enabled} onChange={(event) => setOptIn(event.target.checked)} /> I opt in to local storage of this declaration and its deduplication fingerprint.</label>
      <button type="submit" disabled={busy || !enabled || !optIn || !text.trim()}>Submit locally</button>
    </form>
    <button type="button" disabled={busy || !enabled} onClick={() => { setOutput(null); setError(""); setRevision((n) => n + 1); }}>Refresh local queue</button>
    {page && <><p>{page.total} local records including erased tombstones; {page.rows.length} on this page.</p><ul>{page.rows.map((record) => <li key={record.guard.id}>
      <p>{record.base.revision}, {record.base.scope.context.build.edition}/{record.base.scope.context.build.build}/{record.base.scope.context.build.interface}, {record.base.scope.context.namespace}{record.base.scope.fixture ? ", synthetic" : ""}: {record.state}, revision {record.guard.revision}</p>
      <details><summary>Declaration and opaque fingerprint</summary><p>{record.guard.id}</p><pre>{record.document ? JSON.stringify(record.document, null, 2) : "Payload erased; fingerprint, source, base and local decision retained."}</pre></details>
      {record.state === "pending" && <><button disabled={busy || !enabled} onClick={() => void run("accept", record)}>Accept local declaration</button><button disabled={busy || !enabled} onClick={() => void run("reject", record)}>Reject local declaration</button></>}
      {["pending", "accepted", "rejected"].includes(record.state) && <><button disabled={busy || !enabled} onClick={() => void run("withdraw", record)}>Withdraw and erase payload</button><button disabled={busy || !enabled} onClick={() => void run("revoke", record)}>Revoke local declaration</button></>}
      {record.state === "revoked" && record.document && <button disabled={busy || !enabled} onClick={() => void run("erase_revoked", record)}>Erase revoked payload</button>}
      {record.state === "accepted" && <button disabled={busy || !enabled} onClick={() => void run("export", record)}>Prepare permission-gated local export</button>}
    </li>)}</ul>
      <button disabled={busy || !enabled || cursor === null} onClick={() => { setOutput(null); setCursor(null); }}>First local page</button><button disabled={busy || !enabled || page.next === null} onClick={() => { setOutput(null); setCursor(page.next); }}>Next local page</button>
    </>}
    {output && <><p>Local JSON export: {output.byte_count} bytes. Recorded source policy permits this shape; independent permission authentication remains unverified.</p><label>Local observation export <textarea readOnly rows={8} value={output.text} /></label></>}
  </section>;
}
