import { fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { PartyScreen } from "./PartyScreen";
import { createClient, type CommandAdapter } from "../commands";
import { demoAdapter } from "../commands/demoAdapter";
import type { StoredParty, CommandMap, CommandName } from "../commands/contract";
import type { CharacterSnapshotEnvelope } from "../../../../schemas/v1/exchange";
import fixture from "../../../../fixtures/contracts/v1/character-snapshot.json";

const envelope = fixture as unknown as CharacterSnapshotEnvelope;
function mock() {
  const calls: { command: string; input: unknown }[] = [];
  let party: StoredParty | null = null;
  let failSave = false;
  let failRead = false;
  let fresh = false;
  let captureTwo = false;
  let deferred: Promise<StoredParty | null> | null = null;
  const adapter: CommandAdapter = {
    invoke<C extends CommandName>(command: C, input: CommandMap[C]["input"]): Promise<CommandMap[C]["output"]> {
      calls.push({ command, input });
      let answer: unknown;
      switch (command) {
        case "get_app_status": answer = { data_mode: "stored", active_profile: null, capabilities: {}, version: "test" }; break;
        case "list_profiles": answer = [
          { id: "one", display_name: "One", data_mode: "stored", context: envelope.payload.context, fixture: true },
          { id: "two", display_name: "Two", data_mode: "stored", context: envelope.payload.context, fixture: true },
        ]; break;
        case "get_party_quests": answer = { context: envelope.payload.context, fixture: true, revision: null, quests: [] }; break;
        case "list_parties": answer = party ? [party.id] : []; break;
        case "get_party":
          if (deferred) return deferred as Promise<CommandMap[C]["output"]>;
          if (failRead) return Promise.reject({ code: "STORAGE_ERROR", message: "Stored party failed validation", retryable: false });
          answer = party ? { ...party, participants: party.participants.map((p) => p.snapshot && fresh ? { ...p, snapshot: { ...p.snapshot, freshness: { status: "fresh", age_ms: 0 } } } : p) } : null; break;
        case "save_party": {
          if (failSave) return Promise.reject({ code: "INVALID_INPUT", message: "Party profiles require matching data contexts", retryable: false });
          const args = input as CommandMap["save_party"]["input"];
          party = { id: args.party_id, data_mode: "stored", participants: args.profile_ids.map((id) => {
            const individual = structuredClone(envelope);
            individual.payload.character = id === "one" ? "character-one" : "character-two";
            if (id === "two") {
              const quest = individual.payload.quests["demo-quest-1"];
              if (quest) quest.completed = { status: "known", value: true };
            }
            return { profile_id: id, character: individual.payload.character, snapshot: id === "one" || captureTwo ? { envelope: individual, profile_id: id, data_mode: "stored", freshness: { status: "stale", age_ms: 3_600_000 } } : null };
          }) };
          answer = null; break;
        }
        case "delete_party": party = null; answer = true; break;
        default: return Promise.reject(new Error("Unexpected command"));
      }
      return Promise.resolve(answer as CommandMap[C]["output"]);
    },
  };
  return { client: createClient(adapter), calls, captureTwo: () => { captureTwo = true; }, failSave: (value: boolean) => { failSave = value; }, failRead: (value: boolean) => { failRead = value; }, fresh: () => { fresh = true; }, defer: (value: Promise<StoredParty | null> | null) => { deferred = value; } };
}

async function createParty(client: ReturnType<typeof createClient>) {
  await client.saveParty("Duo", ["one", "two"]);
}

it("creates a manual party, displays individual stale/missing state and removes only configuration", async () => {
  const { client, calls } = mock();
  render(<PartyScreen client={client} />);
  expect(await screen.findByText("No local parties yet.")).toBeInTheDocument();
  fireEvent.click(screen.getByRole("button", { name: "New party" }));
  fireEvent.change(screen.getByLabelText("Party name"), { target: { value: "Duo" } });
  fireEvent.click(screen.getByLabelText(/^One —/)); fireEvent.click(screen.getByLabelText(/^Two —/));
  fireEvent.click(screen.getByRole("button", { name: "Save membership" }));
  expect(await screen.findByText("character-one")).toBeInTheDocument();
  expect(screen.getByText("Last imported: captured 1 h ago (stale)")).toBeInTheDocument();
  expect(screen.getByText(/Missing imported snapshot/)).toBeInTheDocument();
  expect(screen.getByText(/SYNTHETIC/)).toBeInTheDocument();
  expect(screen.getByText(/accepted true, completed false/)).toBeInTheDocument();
  expect(screen.getByText("Unknown (not observed)")).toBeInTheDocument();
  expect(calls).toContainEqual({ command: "save_party", input: { party_id: "Duo", profile_ids: ["one", "two"] } });
  fireEvent.click(screen.getByRole("button", { name: "Remove local party" }));
  expect(await screen.findByText(/Character snapshots and preferences are preserved/)).toBeInTheDocument();
  expect(await screen.findByText("No local parties yet.")).toBeInTheDocument();
  expect(calls.some((call) => call.command === "delete_profile")).toBe(false);
});

it("keeps saved membership and draft edits after a rejected replacement and refresh", async () => {
  const model = mock(); await createParty(model.client);
  const view = render(<PartyScreen client={model.client} />);
  await screen.findByText("character-one");
  fireEvent.click(screen.getByRole("button", { name: "Edit membership" }));
  fireEvent.click(screen.getByLabelText(/^Two —/));
  model.failSave(true);
  fireEvent.click(screen.getByRole("button", { name: "Save membership" }));
  expect(await screen.findByRole("alert")).toHaveTextContent("matching data contexts");
  expect(screen.getByText("character-two")).toBeInTheDocument();
  model.fresh(); view.rerender(<PartyScreen client={model.client} refreshKey={1} />);
  expect(await screen.findByText("Last imported: captured 0 s ago")).toBeInTheDocument();
  expect(screen.getByLabelText(/^Two —/)).not.toBeChecked();
  expect(screen.getByLabelText("Selected party")).toHaveValue("Duo");
  model.failSave(false); fireEvent.click(screen.getByRole("button", { name: "Save membership" }));
  await screen.findByText("Party membership saved.");
  await waitFor(() => expect(screen.queryByText("character-two")).not.toBeInTheDocument());
});

it("one participant's completion never completes the other participant's quest", async () => {
  const model = mock(); model.captureTwo(); await createParty(model.client);
  render(<PartyScreen client={model.client} />);
  const one = (await screen.findByText("character-one")).closest("li");
  const two = screen.getByText("character-two").closest("li");
  if (!one || !two) throw new Error("Missing participant panels");
  expect(within(one).getByText(/accepted true, completed false/)).toBeInTheDocument();
  expect(within(two).getByText(/accepted true, completed true/)).toBeInTheDocument();
  expect(within(one).queryByText(/completed true/)).not.toBeInTheDocument();
});

it("recovers from a failed party read without showing stale results as current", async () => {
  const model = mock(); await createParty(model.client); model.failRead(true);
  render(<PartyScreen client={model.client} />);
  expect(await screen.findByRole("alert")).toHaveTextContent("STORAGE_ERROR");
  expect(screen.queryByText("character-one")).not.toBeInTheDocument();
  model.failRead(false); fireEvent.click(screen.getByRole("button", { name: "Refresh parties" }));
  expect(await screen.findByText("character-one")).toBeInTheDocument();
  expect(screen.queryByRole("alert")).not.toBeInTheDocument();
});

it("prevents accidental overwrite by a new party and disables mutations in the browser demo", async () => {
  const model = mock(); await createParty(model.client);
  const view = render(<PartyScreen client={model.client} />);
  await screen.findByText("character-one");
  fireEvent.click(screen.getByRole("button", { name: "New party" }));
  fireEvent.change(screen.getByLabelText("Party name"), { target: { value: "Duo" } });
  fireEvent.click(screen.getByLabelText(/^One —/));
  expect(screen.getByRole("button", { name: "Save membership" })).toBeDisabled();
  view.unmount(); render(<PartyScreen client={createClient(demoAdapter)} />);
  await screen.findByText(/DEMO — local party/);
  expect(screen.getByRole("button", { name: "New party" })).toBeDisabled();
});

it("ignores a superseded read after a refresh has returned newer state", async () => {
  const model = mock(); await createParty(model.client);
  let complete!: (value: StoredParty | null) => void;
  model.defer(new Promise((resolve) => { complete = resolve; }));
  const view = render(<PartyScreen client={model.client} />);
  await waitFor(() => expect(model.calls.some((c) => c.command === "get_party")).toBe(true));
  model.defer(null); model.fresh();
  view.rerender(<PartyScreen client={model.client} refreshKey={1} />);
  await screen.findByText("Last imported: captured 0 s ago");
  complete(null);
  await waitFor(() => expect(screen.getByText("character-one")).toBeInTheDocument());
  expect(screen.queryByText(/no longer available/)).not.toBeInTheDocument();
});
