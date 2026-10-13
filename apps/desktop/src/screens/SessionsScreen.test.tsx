import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { expect, test } from "vitest";
import { createClient } from "../commands";
import type { CommandAdapter } from "../commands/contract";
import type { SessionAnalytics, SessionDetail } from "../commands/sessions";
import { SessionsScreen } from "./SessionsScreen";
import example from "../../../../fixtures/desktop/sessions/analytics.synthetic.json";
const analytics = example as SessionAnalytics;
const detail: SessionDetail = { fixture: true, batch: { character: analytics.summary.character, context: analytics.summary.context, events: [] }, analytics };
function fixture(delayed?: Promise<SessionDetail>) {
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
  const { client, calls } = fixture();
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
  fireEvent.click(await screen.findByRole("button", { name: "Delete synthetic-session" }));
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
