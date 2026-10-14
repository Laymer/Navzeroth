import { act, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { createClient, type CommandAdapter } from "../commands";
import type { SubmissionPage, SubmissionRecord } from "../commands/localSubmissions";
import { LocalSubmissionsPanel } from "./LocalSubmissionsPanel";
import example from "../../../../fixtures/desktop/submissions/synthetic-price.json";
const row: SubmissionRecord = { guard: { id: "a".repeat(64), revision: 1 }, base: { scope: { context: { ...example.context, namespace: "demo" }, fixture: true }, revision: "demo-r1" }, source: example.source, state: "pending", decision_code: "pending_review", document: example as SubmissionRecord["document"], created_at_utc_ms: null, changed_at_utc_ms: null };
function mock() {
  let record = structuredClone(row);
  const calls: { command: string; input: unknown }[] = [];
  const adapter: CommandAdapter = { async invoke(command, input) {
    calls.push({ command, input });
    if (command === "list_local_submissions") return { rows: [record], next: null, total: 1 } as never;
    if (command === "submit_local_observation") return { duplicate: false, record } as never;
    if (command === "change_local_submission") { const request = JSON.parse((input as { text: string }).text) as { action: string }; record = { ...record, guard: { ...record.guard, revision: record.guard.revision + 1 }, state: request.action === "accept" ? "accepted" : "withdrawn", document: request.action === "accept" ? record.document : null }; return record as never; }
    if (command === "export_local_submission") throw { code: "PROVIDER_UNAVAILABLE", message: "Recorded source policy does not permit this local export", retryable: false };
    throw new Error("Unexpected command");
  } };
  return { client: createClient(adapter), calls };
}
it("requires opt-in, sends the explicit declaration and resets consent after storage", async () => {
  const model = mock(); render(<LocalSubmissionsPanel client={model.client} enabled refreshKey={0} />);
  await screen.findByText(/1 local records/);
  fireEvent.click(screen.getByRole("button", { name: "Load synthetic price declaration" }));
  expect(screen.getByRole("button", { name: "Submit locally" })).toBeDisabled();
  fireEvent.click(screen.getByRole("checkbox"));
  fireEvent.click(screen.getByRole("button", { name: "Submit locally" }));
  await screen.findByText("Declaration added to the local review queue.");
  expect(model.calls.find((c) => c.command === "submit_local_observation")?.input).toEqual({ text: JSON.stringify(example, null, 2), opt_in: true });
  expect(screen.getByRole("checkbox")).not.toBeChecked();
});
it("local acceptance enables permission-gated export, preserves policy refusal, and withdrawal erases the payload", async () => {
  const model = mock(); render(<LocalSubmissionsPanel client={model.client} enabled refreshKey={0} />);
  fireEvent.click(await screen.findByRole("button", { name: "Accept local declaration" }));
  await screen.findByText("Local decision: accepted.");
  expect(model.calls.find((c) => c.command === "change_local_submission")?.input).toEqual({ text: JSON.stringify({ expected: row.guard, action: "accept" }) });
  fireEvent.click(await screen.findByRole("button", { name: "Prepare permission-gated local export" }));
  expect(await screen.findByRole("alert")).toHaveTextContent("Recorded source policy does not permit");
  fireEvent.click(screen.getByRole("button", { name: "Withdraw and erase payload" }));
  await screen.findByText("Payload erased; fingerprint, source, base and local decision retained.");
  expect(screen.queryByRole("button", { name: "Prepare permission-gated local export" })).not.toBeInTheDocument();
});
it("suppresses an export response after the queue refresh context changes", async () => {
  let finish!: (value: unknown) => void;
  const accepted = { ...row, state: "accepted" as const };
  const adapter: CommandAdapter = { invoke(command) { if (command === "list_local_submissions") return Promise.resolve({ rows: [accepted], next: null, total: 1 } as never); if (command === "export_local_submission") return new Promise((resolve) => { finish = resolve as (value: unknown) => void; }); return Promise.reject(new Error("Unexpected")); } };
  const client = createClient(adapter); const view = render(<LocalSubmissionsPanel client={client} enabled refreshKey={0} />);
  fireEvent.click(await screen.findByRole("button", { name: "Prepare permission-gated local export" }));
  view.rerender(<LocalSubmissionsPanel client={client} enabled refreshKey={1} />);
  await act(async () => finish({ format: "azeroth-local-observation", schema_version: 1, text: "stale export", byte_count: 12, sha256: "b".repeat(64) }));
  expect(screen.queryByLabelText("Local observation export")).not.toBeInTheDocument();
});
it("pages with the opaque cursor and keeps the local workflow unavailable in the browser demo", async () => {
  const calls: unknown[] = []; const page: SubmissionPage = { rows: [], total: 33, next: "c".repeat(64) };
  const adapter: CommandAdapter = { async invoke(command, input) { calls.push(input); if (command === "list_local_submissions") return page as never; throw new Error("Unexpected"); } };
  const client = createClient(adapter); const view = render(<LocalSubmissionsPanel client={client} enabled refreshKey={0} />);
  fireEvent.click(await screen.findByRole("button", { name: "Next local page" })); await waitFor(() => expect(calls).toContainEqual({ after: "c".repeat(64), limit: 32 }));
  view.rerender(<LocalSubmissionsPanel client={client} enabled={false} refreshKey={1} />);
  expect(screen.getByRole("button", { name: "Submit locally" })).toBeDisabled();
  expect(screen.getByText("Local submissions require the desktop application.")).toBeInTheDocument();
});
