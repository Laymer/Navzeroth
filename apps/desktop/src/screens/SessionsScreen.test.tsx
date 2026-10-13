import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { expect, test, vi } from "vitest";
import { createClient } from "../commands";
import type { CommandAdapter } from "../commands/contract";
import type { SessionAnalytics, SessionDetail } from "../commands/sessions";
import { SessionsScreen } from "./SessionsScreen";
import example from "../../../../fixtures/desktop/sessions/analytics.synthetic.json";
const analytics = example as SessionAnalytics;
const detail: SessionDetail = { fixture: true, batch: { character: analytics.summary.character, context: analytics.summary.context, events: [] }, analytics };
function fixture(delayed?: Promise<SessionDetail>, fileImport?: Promise<void>) {
  let stored = false;
  const calls: { command: string; args: unknown }[] = [];
  const adapter: CommandAdapter = { invoke: async (command, args) => {
    calls.push({ command, args });
    let value: unknown;
    switch (command) {
      case "get_app_status": value = { data_mode: "stored", version: "test", active_profile: "one", capabilities: {} }; break;
      case "list_profiles": value = ["one", "two"].map((id) => ({ id, display_name: id, data_mode: "stored", context: analytics.summary.context, fixture: true, snapshot_count: 1 })); break;
      case "list_sessions": value = stored ? [{ session_id: "synthetic-session", analytics }] : []; break;
      case "get_session_detail": value = delayed ? await delayed : detail; break;
      case "delete_session": stored = false; value = true; break;
      case "import_session_file": case "import_session_text": {
        if (command === "import_session_file" && fileImport) await fileImport;
        if (command === "import_session_text" && (args as {text:string}).text === "bad") throw { code: "INVALID_INPUT", message: "Invalid session framing", retryable: false };
        value = { profile_id: "one", inserted: stored ? 0 : 1, duplicates: stored ? 1 : 0, session_ids: ["synthetic-session"] }; stored = true; break;
      }
      default: throw Error("unexpected command");
    }
    return value as never;
  } };
  return { client: createClient(adapter), calls };
}
test("manual text/file import, duplicate detail and explicit deletion preserve distinctions", async () => {
  let finishFileImport!: () => void;
  const pendingFile = new Promise<void>((resolve) => { finishFileImport = resolve; });
  const { client, calls } = fixture(undefined, pendingFile);
  render(<SessionsScreen client={client} refreshKey={0} onChanged={() => {}} />);
  await screen.findByLabelText("Session profile");
  fireEvent.change(screen.getByLabelText("Session export text"), { target: { value: "bad" } });
  fireEvent.click(screen.getByRole("button", { name: "Import session text" }));
  expect(await screen.findByRole("alert")).toHaveTextContent("Invalid session framing");
  fireEvent.change(screen.getByLabelText("Session export text"), { target: { value: "ANAVSESSION1:synthetic-mock" } });
  fireEvent.click(screen.getByRole("button", { name: "Import session text" }));
  await screen.findByText("Imported 1 session(s); 0 already stored.");
  fireEvent.click(await screen.findByRole("button", { name: "Inspect synthetic-session" }));
  await screen.findByText(/UTC mapping: uncertain/);
  expect(screen.getByText(/Unclassified intervals: 10.0 s/)).toHaveTextContent("Gaps: 5.0 s");
  expect(screen.getByText(/Starting level:/)).toHaveTextContent("Observed level change: 1");
  fireEvent.click(screen.getByRole("button", { name: "Import session text" }));
  await screen.findByText("Imported 0 session(s); 1 already stored.");
  fireEvent.change(screen.getByLabelText("Selected session file path"), { target: { value: "/selected/AzerothNavigator.lua" } });
  fireEvent.click(screen.getByRole("button", { name: "Import selected session file" }));
  await waitFor(() => expect(calls).toContainEqual({ command: "import_session_file", args: { profile_id: "one", path: "/selected/AzerothNavigator.lua", format: { kind: "saved_variables", global: "AzerothNavigatorDB", key: "session_export" } } }));
  const deleteButton = screen.getByRole("button", { name: "Delete synthetic-session" });
  expect(deleteButton).toBeDisabled();
  fireEvent.click(deleteButton);
  expect(calls.some((call) => call.command === "delete_session")).toBe(false);
  finishFileImport();
  await screen.findByText("Imported 0 session(s); 1 already stored.");
  await waitFor(() => expect(screen.getByRole("button", { name: "Delete synthetic-session" })).toBeEnabled());
  fireEvent.click(screen.getByRole("button", { name: "Delete synthetic-session" }));
  await screen.findByText("Session deleted.");
  await screen.findByText("No stored sessions for this profile.");
});
test("a superseded detail response cannot restore another profile's session", async () => {
  let resolve!: (value: SessionDetail) => void;
  const pending = new Promise<SessionDetail>((done) => { resolve = done; });
  const { client } = fixture(pending);
  render(<SessionsScreen client={client} refreshKey={0} onChanged={() => {}} />);
  await screen.findByLabelText("Session profile");
  fireEvent.change(screen.getByLabelText("Session export text"), { target: { value: "ANAVSESSION1:synthetic-mock" } });
  fireEvent.click(screen.getByRole("button", { name: "Import session text" }));
  fireEvent.click(await screen.findByRole("button", { name: "Inspect synthetic-session" }));
  fireEvent.change(screen.getByLabelText("Session profile"), { target: { value: "two" } });
  resolve(detail);
  await waitFor(() => expect(screen.queryByText("Session detail: synthetic-session")).not.toBeInTheDocument());
});

test("a same-profile refresh preserves the pending import receipt and refreshes history", async () => {
  let finish!: () => void;
  const pending = new Promise<void>((resolve) => { finish = resolve; });
  const { client, calls } = fixture(undefined, pending);
  const changed = vi.fn();
  const view = render(<SessionsScreen client={client} refreshKey={0} onChanged={changed} />);
  await screen.findByLabelText("Session profile");
  fireEvent.change(screen.getByLabelText("Selected session file path"), { target: { value: "/selected/session.lua" } });
  fireEvent.click(screen.getByRole("button", { name: "Import selected session file" }));
  await waitFor(() => expect(calls.some((call) => call.command === "import_session_file")).toBe(true));
  const reads = calls.filter((call) => call.command === "list_sessions").length;
  view.rerender(<SessionsScreen client={client} refreshKey={1} onChanged={changed} />);
  await waitFor(() => expect(calls.filter((call) => call.command === "list_sessions").length).toBeGreaterThan(reads));
  finish();
  await screen.findByText("Imported 1 session(s); 0 already stored.");
  await screen.findByRole("button", { name: "Inspect synthetic-session" });
  expect(changed).toHaveBeenCalledTimes(1);
});

test("a same-profile refresh preserves a pending import failure", async () => {
  let fail!: (error: unknown) => void;
  const pending = new Promise<void>((_, reject) => { fail = reject; });
  const { client, calls } = fixture(undefined, pending);
  const changed = vi.fn();
  const view = render(<SessionsScreen client={client} refreshKey={0} onChanged={changed} />);
  await screen.findByLabelText("Session profile");
  fireEvent.change(screen.getByLabelText("Selected session file path"), { target: { value: "/selected/session.lua" } });
  fireEvent.click(screen.getByRole("button", { name: "Import selected session file" }));
  await waitFor(() => expect(calls.some((call) => call.command === "import_session_file")).toBe(true));
  const reads = calls.filter((call) => call.command === "list_sessions").length;
  view.rerender(<SessionsScreen client={client} refreshKey={1} onChanged={changed} />);
  await waitFor(() => expect(calls.filter((call) => call.command === "list_sessions").length).toBeGreaterThan(reads));
  fail({ code: "INVALID_INPUT", message: "Session identity conflict", retryable: false });
  expect(await screen.findByRole("alert")).toHaveTextContent("Session identity conflict");
  expect(changed).not.toHaveBeenCalled();
});

test("a replaced client cannot show an old pending import receipt", async () => {
  let finish!: () => void;
  const pending = new Promise<void>((resolve) => { finish = resolve; });
  const original = fixture(undefined, pending);
  const replacement = fixture();
  const changed = vi.fn();
  const view = render(<SessionsScreen client={original.client} refreshKey={0} onChanged={changed} />);
  await screen.findByLabelText("Session profile");
  fireEvent.change(screen.getByLabelText("Selected session file path"), { target: { value: "/selected/session.lua" } });
  fireEvent.click(screen.getByRole("button", { name: "Import selected session file" }));
  await waitFor(() => expect(original.calls.some((call) => call.command === "import_session_file")).toBe(true));
  view.rerender(<SessionsScreen client={replacement.client} refreshKey={1} onChanged={changed} />);
  await waitFor(() => expect(replacement.calls.some((call) => call.command === "list_sessions")).toBe(true));
  finish();
  await waitFor(() => expect(screen.getByRole("button", { name: "Import selected session file" })).toBeEnabled());
  expect(changed).not.toHaveBeenCalled();
  expect(screen.queryByText("Imported 1 session(s); 0 already stored.")).not.toBeInTheDocument();
  expect(screen.getByText("No stored sessions for this profile.")).toBeInTheDocument();
});
