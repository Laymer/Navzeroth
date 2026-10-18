import { act, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { vi } from "vitest";
import { ImportScreen, describeImport } from "./ImportScreen";
import { createClient, type CommandAdapter } from "../commands";
import type { CommandMap, CommandName, ImportReceipt, WatchState } from "../commands/contract";
import type { CharacterSnapshotEnvelope } from "../../../../schemas/v1/exchange";
import fixture from "../../../../fixtures/contracts/v1/character-snapshot.json";
const envelope = fixture as unknown as CharacterSnapshotEnvelope;
const receipt: ImportReceipt = {
  profile_id: "stored-context-one", snapshot_id: envelope.payload.id,
  disposition: { outcome: "stored", previous_current: null }, fixture: true,
  namespace: "demo", data_mode: "stored", freshness: { status: "uncertain", reason: "missing_or_untrusted_time" },
};
function mock() {
  const calls: { command: CommandName; input: unknown }[] = [];
  let fail = true;
  let selected = "stored-context-one";
  let watching = false;
  const adapter: CommandAdapter = {
    invoke<C extends CommandName>(command: C, input: CommandMap[C]["input"]): Promise<CommandMap[C]["output"]> {
      calls.push({ command, input });
      let answer: unknown;
      switch (command) {
        case "get_app_status": answer = { version: "0.1.0", data_mode: "stored", active_profile: selected, capabilities: {} }; break;
        case "list_profiles": answer = [
          { id: "stored-context-one", display_name: "One", data_mode: "stored", context: envelope.payload.context, fixture: true },
          { id: "stored-context-two", display_name: "Two", data_mode: "stored", context: envelope.payload.context, fixture: true },
        ]; break;
        case "get_stored_snapshot": answer = { envelope: { ...envelope, payload: { ...envelope.payload, id: selected === "stored-context-one" ? "current-one" : "current-two" } }, profile_id: selected, freshness: receipt.freshness, data_mode: "stored" }; break;
        case "get_snapshot_history": answer = [{ snapshot_id: "history-old", is_current: false, freshness: receipt.freshness, observed_at_utc_ms: null, received_at_utc_ms: null }]; break;
        case "select_profile": selected = (input as CommandMap["select_profile"]["input"]).profile_id; answer = null; break;
        case "import_snapshot_text":
          if (fail) { fail = false; return Promise.reject({ code: "INVALID_INPUT", message: "Invalid snapshot export", retryable: false }); }
          answer = { ...receipt, disposition: { outcome: "history_only", reason: "older" } }; break;
        case "import_snapshot_file": answer = receipt; break;
        case "start_snapshot_watch": watching = true; answer = { active: true, status: "waiting_for_stable_file", last_import: null, error: null } satisfies WatchState; break;
        case "stop_snapshot_watch": watching = false; answer = { active: false, status: "cancelled", last_import: null, error: null } satisfies WatchState; break;
        case "get_snapshot_watch": answer = { active: watching, status: watching ? "waiting_for_stable_file" : null, last_import: null, error: null } satisfies WatchState; break;
        default: return Promise.reject(new Error("Unexpected mock command"));
      }
      return Promise.resolve(answer as CommandMap[C]["output"]);
    },
  };
  return { client: createClient(adapter), calls };
}

it("recovers from malformed input and explains history-only imports without replacing current", async () => {
  const { client } = mock(); const changed = vi.fn();
  render(<ImportScreen client={client} onChanged={changed} />);
  expect(await screen.findByText(/current-one/)).toBeInTheDocument();
  fireEvent.change(screen.getByLabelText("Snapshot export text"), { target: { value: "ANAV1:{" } });
  fireEvent.click(screen.getByRole("button", { name: "Import text" }));
  expect(await screen.findByRole("alert")).toHaveTextContent("INVALID_INPUT");
  expect(screen.getByText(/current-one/)).toBeInTheDocument();
  await waitFor(() => expect(screen.getByRole("button", { name: "Import text" })).toBeEnabled());
  fireEvent.change(screen.getByLabelText("Snapshot export text"), { target: { value: "ANAV1:valid" } });
  fireEvent.click(screen.getByRole("button", { name: "Import text" }));
  expect(await screen.findByText(/Stored in history only.*older/)).toBeInTheDocument();
  expect(screen.queryByRole("alert")).not.toBeInTheDocument();
  expect(screen.getByText(/current-one/)).toBeInTheDocument();
  expect(screen.getByText(/SYNTHETIC/)).toBeInTheDocument();
  expect(changed).toHaveBeenCalledOnce();
});

it("selects opaque profile IDs and queries its stored history", async () => {
  const { client, calls } = mock(); const changed = vi.fn();
  render(<ImportScreen client={client} onChanged={changed} />);
  expect(await screen.findByText(/history-old.*history only/)).toBeInTheDocument();
  fireEvent.change(screen.getByLabelText("Selected profile"), { target: { value: "stored-context-two" } });
  expect(await screen.findByText(/current-two/)).toBeInTheDocument();
  expect(calls).toContainEqual({ command: "select_profile", input: { profile_id: "stored-context-two" } });
  expect(calls).toContainEqual({ command: "get_stored_snapshot", input: { profile_id: "stored-context-two" } });
  expect(changed).toHaveBeenCalledOnce();
});

it("passes only the explicitly chosen path and format to import and watcher controls", async () => {
  const { client, calls } = mock();
  render(<ImportScreen client={client} onChanged={vi.fn()} />);
  expect(await screen.findByText(/current-one/)).toBeInTheDocument();
  fireEvent.change(screen.getByLabelText("Export file path"), { target: { value: "/selected/account/export.lua" } });
  fireEvent.change(screen.getByLabelText("File format"), { target: { value: "saved_variables" } });
  expect(screen.getByLabelText("SavedVariables global")).toHaveValue("AzerothNavigatorDB");
  expect(screen.getByLabelText("Snapshot export key")).toHaveValue("snapshot_export");
  expect(screen.getByRole("button", { name: "Watch selected file" })).toBeEnabled();
  fireEvent.change(screen.getByLabelText("SavedVariables global"), { target: { value: "UserChosenDB" } });
  fireEvent.change(screen.getByLabelText("Snapshot export key"), { target: { value: "snapshot" } });
  fireEvent.click(screen.getByRole("button", { name: "Import selected file" }));
  expect(await screen.findByText("Stored as the current snapshot.")).toBeInTheDocument();
  await waitFor(() => expect(screen.getByRole("button", { name: "Watch selected file" })).toBeEnabled());
  fireEvent.click(screen.getByRole("button", { name: "Watch selected file" }));
  expect(await screen.findByText("Waiting for a complete, stable save.")).toBeInTheDocument();
  await waitFor(() => expect(screen.getByRole("button", { name: "Stop watching" })).toBeEnabled());
  fireEvent.click(screen.getByRole("button", { name: "Stop watching" }));
  expect(await screen.findByText("Watching stopped.")).toBeInTheDocument();
  const input = { path: "/selected/account/export.lua", format: { kind: "saved_variables", global: "UserChosenDB", key: "snapshot" } };
  expect(calls).toContainEqual({ command: "import_snapshot_file", input });
  expect(calls).toContainEqual({ command: "start_snapshot_watch", input });
});

it("explains every unchanged-current disposition distinctly", () => {
  const descriptions = [
    describeImport({ ...receipt, disposition: { outcome: "duplicate" } }),
    ...(["older", "same_position", "incomparable"] as const).map((reason) => describeImport({ ...receipt, disposition: { outcome: "history_only", reason } })),
  ];
  expect(new Set(descriptions).size).toBe(4);
  expect(descriptions.every((description) => description.includes("current snapshot is unchanged"))).toBe(true);
});
it("requires a reason and compares the previewed current snapshot before an explicit promotion",async()=>{
 const {client:base,calls}=mock();const changed=vi.fn();
 const adapter:CommandAdapter={invoke(command,input){
  if(command==="get_historical_snapshot")return Promise.resolve({profile_id:"stored-context-one",envelope:{...envelope,payload:{...envelope.payload,id:"history-old"}},freshness:receipt.freshness,data_mode:"stored"}as never);
  if(command==="promote_snapshot"){calls.push({command,input});return Promise.reject({code:"SNAPSHOT_OUTDATED",message:"Current snapshot changed",retryable:false});}
  throw Error("Unexpected preview command");
 }};
 const preview=createClient(adapter);const client={...base,getHistoricalSnapshot:preview.getHistoricalSnapshot,promoteSnapshot:preview.promoteSnapshot};
 render(<ImportScreen client={client} onChanged={changed}/>);
 fireEvent.click(await screen.findByRole("button",{name:"Preview history-old"}));
 expect(await screen.findByRole("region",{name:"Promotion preview"})).toHaveTextContent("instead of current-one");
 expect(screen.getByRole("button",{name:"Confirm promotion"})).toBeDisabled();
 fireEvent.click(screen.getByRole("button",{name:"Cancel"}));
 expect(screen.queryByRole("region",{name:"Promotion preview"})).toBeNull();
 fireEvent.click(screen.getByRole("button",{name:"Preview history-old"}));
 fireEvent.change(await screen.findByRole("textbox",{name:"Selection reason"}),{target:{value:"Reviewed imported history"}});
 fireEvent.click(screen.getByRole("button",{name:"Confirm promotion"}));
 expect(await screen.findByRole("alert")).toHaveTextContent("Current snapshot changed");
 expect(screen.queryByRole("region",{name:"Promotion preview"})).toBeNull();
 expect(screen.getByText(/current-one:/)).toBeVisible();
 expect(calls).toContainEqual({command:"promote_snapshot",input:{profile_id:"stored-context-one",snapshot_id:"history-old",expected_current:"current-one",reason:"Reviewed imported history"}});
 expect(changed).not.toHaveBeenCalled();
});

it("polls only while watching and notifies once for a repeated imported receipt",async()=>{
 const {client:base}=mock();const changed=vi.fn();let ticks=0;
 const getWatch=vi.fn(async()=>({ok:true as const,value:{active:true,status:"imported" as const,last_import:ticks===0?null:receipt,error:null}}));
 vi.useFakeTimers({toFake:["setInterval","clearInterval"]});
 const client={...base,getWatch};const view=render(<ImportScreen client={client} onChanged={changed}/>);
 try{
  await act(async()=>{});expect(screen.getByText(/current-one:/)).toBeVisible();
  ticks++;await act(async()=>vi.advanceTimersByTimeAsync(1000));
  expect(screen.getByText("Stored as the current snapshot.")).toBeVisible();
  await act(async()=>vi.advanceTimersByTimeAsync(1000));expect(changed).toHaveBeenCalledOnce();
  expect(getWatch).toHaveBeenCalledTimes(3);
  view.unmount();expect(vi.getTimerCount()).toBe(0);
 }finally{view.unmount();vi.useRealTimers();}
});
