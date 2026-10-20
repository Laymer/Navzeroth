import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { vi } from "vitest";
import { ContentScreen } from "./ContentScreen";
import { createClient, type CommandAdapter } from "../commands";
import type { ContentInspection } from "../commands/contract";
const context = { namespace: "demo" as const, build: { edition: "wow_forever", build: "synthetic", interface: "synthetic" } };
const inspection: ContentInspection = { key: { scope: { context, fixture: true }, revision: "r1" }, report: { passes: true, report_version: 1, errors: [], gaps: [{ code: "unverified_evidence" }], coverage: { collections: { quests: { total: 1, fields: {} } }, evidence_records: { by_confidence: { verified_on_build: 0, inferred: 1 } } } }, sources: [{ id: "source", publisher: "Synthetic author", license: { status: "unknown" }, redistribution: { status: "known", value: "not_permitted" }, attribution: { status: "unknown" }, offline_usable: { status: "known", value: false } }], conflicts: [] };
function mock(content:ContentInspection=inspection,staging:ContentInspection[]=[],initialActive=false) {
  let active: ContentInspection | null = initialActive ? content : null;
  let fail = false;
  const calls: { command: string; input: unknown }[] = [];
  const adapter: CommandAdapter = { invoke(command, input) {
    calls.push({ command, input });
    switch (command) {
      case "get_app_status": return Promise.resolve({ data_mode: "stored" } as never);
      case "list_profiles": return Promise.resolve([{ id: "stored-one", display_name: "One", context, fixture: true, data_mode: "stored" }] as never);
      case "list_local_submissions": return Promise.resolve({rows:[],next:null,total:0} as never);
      case "get_content_dispositions": return Promise.resolve({schema_version:1,scope:inspection.key.scope,active:active?.key??null,revisions:[]} as never);
      case "get_active_content": return Promise.resolve(active as never);
      case "stage_verified_content_bundle":
      case "stage_content": if (staging.length > 0 && !fail) return Promise.resolve(staging.shift() as never); return fail ? Promise.reject({ code: "INVALID_INPUT", message: "Invalid content revision JSON", retryable: false }) : Promise.resolve(content as never);
      case "get_content_inspection_page": {const request=JSON.parse((input as {text:string}).text) as {offset:number};return Promise.resolve({key:content.key,content_sha256:content.pages?.content_sha256,collection:"sources",offset:request.offset,total:201,next_offset:null,records:[{...inspection.sources[0],id:"last-source",publisher:"Last synthetic source"}]} as never);}
      case "activate_content": active = content; return Promise.resolve(content as never);
      case "activate_content_set": { const set = { ...content, key: { ...content.key, revision: "set-abc" } }; active = set; return Promise.resolve(set as never); }
      case "rollback_content": return Promise.resolve({ changed: false, active } as never);
      default: return Promise.reject(new Error("Unexpected command"));
    }
  } };
  return { client: createClient(adapter), calls, fail: (value: boolean) => { fail = value; } };
}
it("stages separately from activation and exposes uncertainty and source policy negatives", async () => {
  const { client, calls } = mock(); const changed = vi.fn();
  render(<ContentScreen client={client} onChanged={changed} refreshKey={0} />);
  await screen.findByLabelText("Content profile");
  fireEvent.click(screen.getByRole("button", { name: "Load synthetic example" }));
  fireEvent.click(screen.getByRole("button", { name: "Validate and stage content" }));
  await screen.findByText(/Validated and staged/);
  expect(screen.getByText("No active content for this profile.")).toBeInTheDocument();
  expect(screen.getByText(/Redistribution: not_permitted. Offline use: false/)).toBeInTheDocument();
  expect(screen.getByText("verified on build: 0")).toBeInTheDocument();
  expect(changed).not.toHaveBeenCalled();
  fireEvent.click(screen.getByRole("button", { name: "Activate staged revision" }));
  await screen.findByText(/Content activated/);
  expect(calls).toContainEqual({ command: "activate_content", input: { profile_id: "stored-one", revision: "r1" } });
  expect(changed).toHaveBeenCalledOnce();
  fireEvent.click(screen.getByRole("button", { name: "Restore previous revision" }));
  await screen.findByText("No previous revision is available.");
  expect(changed).toHaveBeenCalledOnce();
});
it("failed staging preserves active data and supports retry", async () => {
  const model = mock(); render(<ContentScreen client={model.client} onChanged={vi.fn()} refreshKey={0} />);
  await screen.findByLabelText("Content profile"); model.fail(true);
  fireEvent.change(screen.getByLabelText("Content revision JSON"), { target: { value: "bad" } });
  fireEvent.click(screen.getByRole("button", { name: "Validate and stage content" }));
  expect(await screen.findByRole("alert")).toHaveTextContent("INVALID_INPUT");
  expect(screen.getByText("No active content for this profile.")).toBeInTheDocument();
  model.fail(false);
  await waitFor(() => expect(screen.getByRole("button", { name: "Validate and stage content" })).toBeEnabled());
  fireEvent.click(screen.getByRole("button", { name: "Validate and stage content" }));
  await screen.findByText(/Validated and staged/);
  expect(screen.queryByRole("alert")).not.toBeInTheDocument();
});

it("verifies a chosen pack directory without activating it",async()=>{
 const model=mock();const changed=vi.fn();render(<ContentScreen client={model.client} onChanged={changed} refreshKey={0}/>);
 await screen.findByLabelText("Content profile");fireEvent.change(screen.getByLabelText("Verified content pack directory"),{target:{value:"C:\\content\\candidate"}});
 fireEvent.click(screen.getByRole("button",{name:"Verify and stage content pack"}));await screen.findByText(/Validated and staged/);
 expect(model.calls).toContainEqual({command:"stage_verified_content_bundle",input:{path:"C:\\content\\candidate"}});
 expect(model.calls.some(c=>c.command==="activate_content")).toBe(false);expect(changed).not.toHaveBeenCalled();
});

it("requests bounded inspection pages using the staged revision hash",async()=>{
 const source=inspection.sources[0];if(!source)throw Error("Missing synthetic source");
 const many:ContentInspection={...inspection,pages:{content_sha256:"a".repeat(64),sources:201,gaps:1,errors:0,conflicts:0},sources:Array.from({length:200},(_,i)=>({...source,id:`s-${i}`}))};
 const model=mock(many);render(<ContentScreen client={model.client} onChanged={vi.fn()} refreshKey={0}/>);await screen.findByLabelText("Content profile");
 fireEvent.click(screen.getByRole("button",{name:"Load synthetic example"}));fireEvent.click(screen.getByRole("button",{name:"Validate and stage content"}));await screen.findByText(/Validated and staged/);
 fireEvent.click(screen.getByText("Source policies (201)"));fireEvent.click(screen.getByRole("button",{name:"Next sources"}));await screen.findByText(/Last synthetic source/);
 const call=model.calls.find(c=>c.command==="get_content_inspection_page");expect(JSON.parse((call?.input as {text:string}).text)).toEqual({key:inspection.key,content_sha256:"a".repeat(64),collection:"sources",offset:200});
});

it("accepts inferred travel with a single activation through the content set command", async () => {
  const { client, calls } = mock();
  render(<ContentScreen client={client} onChanged={vi.fn()} refreshKey={0} />);
  await screen.findByLabelText("Content profile");
  fireEvent.click(screen.getByRole("button", { name: "Load synthetic example" }));
  fireEvent.click(screen.getByRole("button", { name: "Validate and stage content" }));
  await screen.findByText(/Validated and staged/);
  fireEvent.click(screen.getByRole("checkbox", { name: /Accept inferred travel/ }));
  fireEvent.click(screen.getByRole("button", { name: "Activate staged revision" }));
  await screen.findByText(/inferred travel is accepted as estimates/);
  expect(calls).toContainEqual({ command: "activate_content_set", input: { profile_id: "stored-one", revisions: ["r1"], accept_inferred_travel: true } });
  expect(calls.some((c) => c.command === "activate_content")).toBe(false);
});
it("activates several staged packs of the selected context as one set", async () => {
  const world: ContentInspection = { ...inspection, key: { ...inspection.key, revision: "world" } };
  const other: ContentInspection = { ...inspection, key: { scope: { context: { ...context, build: { ...context.build, build: "other" } }, fixture: true }, revision: "elsewhere" } };
  const { client, calls } = mock(inspection, [inspection, world, other]);
  const changed = vi.fn();
  render(<ContentScreen client={client} onChanged={changed} refreshKey={0} />);
  await screen.findByLabelText("Content profile");
  expect(screen.queryByRole("button", { name: "Activate all staged packs as one set" })).not.toBeInTheDocument();
  fireEvent.change(screen.getByLabelText("Content revision JSON"), { target: { value: "a" } });
  for (let n = 0; n < 3; n++) {
    fireEvent.click(screen.getByRole("button", { name: "Validate and stage content" }));
    await waitFor(() => expect(calls.filter((c) => c.command === "stage_content")).toHaveLength(n + 1));
    await screen.findByText(/Validated and staged/);
  }
  fireEvent.click(await screen.findByRole("button", { name: "Activate all staged packs as one set" }));
  await screen.findByText(/2 packs activated together as set-abc/);
  expect(calls).toContainEqual({ command: "activate_content_set", input: { profile_id: "stored-one", revisions: ["r1", "world"], accept_inferred_travel: false } });
  expect(changed).toHaveBeenCalledOnce();
});

it("discloses version default resets and the persisted revision travel choice", async () => {
  const content:ContentInspection = {...inspection,accepts_inferred_travel:true,bundle_defaults:{app_version:"1.0.0",previous_app_version:"1.0.0-rc.1",revision:"r1"}};
  const {client}=mock(content,[],true);
  render(<ContentScreen client={client} onChanged={()=>{}} refreshKey={0}/>);
  expect(await screen.findByText(/Bundled defaults applied for 1.0.0 after 1.0.0-rc.1/)).toBeInTheDocument();
  expect(screen.getByText("Active revision travel estimates: accepted.")).toBeInTheDocument();
});
