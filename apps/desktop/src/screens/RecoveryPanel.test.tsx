import { act, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { createClient, type CommandAdapter } from "../commands";
import { RecoveryPanel } from "./RecoveryPanel";
const status = { state: "ready", error: null, schema_version: 8, restore_limit_bytes: 536870912 };
const preview = { candidate_id: "immutable-candidate", inspection: { source_schema_version: 5, restored_schema_version: 8, rows: { profiles: 2 } } };
it("requires inspection and explicit replacement acknowledgment, then sends only staged identity", async () => {
  const calls: { command: string; input: unknown }[] = []; const restored = vi.fn();
  const adapter: CommandAdapter = { invoke(command, input) { calls.push({ command, input }); return Promise.resolve((command === "get_database_status" ? status : command === "inspect_database_backup" ? preview : { original_files_retained: true, recovery_directory: "local-recovery" }) as never); } };
  render(<RecoveryPanel client={createClient(adapter)} onRestored={restored} />);
  await screen.findByText(/Database ready/);
  expect(screen.queryByRole("button", { name: "Restore inspected backup" })).not.toBeInTheDocument();
  fireEvent.change(screen.getByLabelText("Backup file to restore"), { target: { value: "selected.sqlite3" } });
  fireEvent.click(screen.getByRole("button", { name: "Inspect backup" }));
  await screen.findByText(/Backup schema 5/);
  expect(screen.getByRole("button", { name: "Restore inspected backup" })).toBeDisabled();
  fireEvent.click(screen.getByLabelText("Replace all local database state with this inspected backup"));
  fireEvent.click(screen.getByRole("button", { name: "Restore inspected backup" }));
  await screen.findByText(/Restore complete/);
  expect(calls.find((call) => call.command === "restore_database_backup")).toEqual({ command: "restore_database_backup", input: { candidate_id: "immutable-candidate" } });
  expect(restored).toHaveBeenCalledOnce();
});
it("changing selected source discards preview and failed inspection never enables replacement", async () => {
  let fail = false;
  const adapter: CommandAdapter = { invoke(command) { if (command === "inspect_database_backup" && fail) return Promise.reject({ code: "STORAGE_ERROR", message: "Unsupported backup", retryable: false }); return Promise.resolve((command === "get_database_status" ? status : preview) as never); } };
  render(<RecoveryPanel client={createClient(adapter)} onRestored={() => undefined} />);
  await screen.findByText(/Database ready/);
  fireEvent.change(screen.getByLabelText("Backup file to restore"), { target: { value: "one.sqlite3" } });fireEvent.click(screen.getByRole("button", { name: "Inspect backup" }));await screen.findByText(/Backup schema 5/);
  fail = true;fireEvent.change(screen.getByLabelText("Backup file to restore"), { target: { value: "two.sqlite3" } });
  expect(screen.queryByRole("button", { name: "Restore inspected backup" })).not.toBeInTheDocument();fireEvent.click(screen.getByRole("button", { name: "Inspect backup" }));await screen.findByRole("alert");expect(screen.queryByRole("button", { name: "Restore inspected backup" })).not.toBeInTheDocument();
});
it("corrupt startup still permits inspecting a backup while healthy-only controls stay disabled", async () => {
  const adapter: CommandAdapter = { invoke() { return Promise.resolve({ ...status, state: "recovery_required", error: { code: "STORAGE_ERROR", message: "Database corrupt", retryable: false } } as never); } };
  render(<RecoveryPanel client={createClient(adapter)} onRestored={() => undefined} />);
  await screen.findByText(/Database recovery required/);
  expect(screen.getByRole("button", { name: "Check database integrity" })).toBeDisabled();
  fireEvent.change(screen.getByLabelText("Backup file to restore"), { target: { value: "good.sqlite3" } });expect(screen.getByRole("button", { name: "Inspect backup" })).toBeEnabled();
});
it("late inspection after unmount cannot offer a stale restore", async () => {
  let finish: ((value: never) => void) | undefined;
  const adapter: CommandAdapter = { invoke(command) { return command === "get_database_status" ? Promise.resolve(status as never) : new Promise<never>((resolve) => { finish = resolve; }); } };
  const view = render(<RecoveryPanel client={createClient(adapter)} onRestored={() => undefined} />);await screen.findByText(/Database ready/);fireEvent.change(screen.getByLabelText("Backup file to restore"), { target: { value: "good.sqlite3" } });fireEvent.click(screen.getByRole("button", { name: "Inspect backup" }));await waitFor(() => expect(screen.getByRole("button", { name: "Inspect backup" })).toBeDisabled());view.unmount();await act(async () => finish?.(preview as never));expect(screen.queryByRole("button", { name: "Restore inspected backup" })).not.toBeInTheDocument();
});
it("startup checking is visible and polls until the host becomes ready", async () => {
  let reads = 0; const checked = vi.fn();
  const adapter: CommandAdapter = { invoke() { reads += 1; return Promise.resolve({ ...status, state: reads === 1 ? "checking" : "ready" } as never); } };
  render(<RecoveryPanel client={createClient(adapter)} onRestored={() => undefined} onChecked={checked} />);
  await screen.findByText(/Checking database at startup/);
  fireEvent.change(screen.getByLabelText("Backup file to restore"), { target: { value: "selected.sqlite3" } });expect(screen.getByRole("button", { name: "Inspect backup" })).toBeDisabled();
  await screen.findByText(/Database ready/);expect(checked).toHaveBeenCalledOnce();expect(screen.getByRole("button", { name: "Inspect backup" })).toBeEnabled();
});
it("marker cleanup warnings are visible after an otherwise successful restore", async () => {
  const adapter: CommandAdapter = { invoke(command) { return Promise.resolve((command === "get_database_status" ? status : command === "inspect_database_backup" ? preview : { pending_marker_cleanup_required: true, original_files_retained: true, recovery_directory: "retained" }) as never); } };
  render(<RecoveryPanel client={createClient(adapter)} onRestored={() => undefined} />);await screen.findByText(/Database ready/);
  fireEvent.change(screen.getByLabelText("Backup file to restore"), { target: { value: "selected.sqlite3" } });fireEvent.click(screen.getByRole("button", { name: "Inspect backup" }));await screen.findByText(/Backup schema 5/);fireEvent.click(screen.getByLabelText("Replace all local database state with this inspected backup"));fireEvent.click(screen.getByRole("button", { name: "Restore inspected backup" }));await screen.findByText(/Recovery marker cleanup failed; next startup will still require recovery/);
});
