import { act, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { vi } from "vitest";
import { GearScreen } from "./GearScreen";
import { createClient, type CommandAdapter } from "../commands";
import type { GearCatalog, GearReport, GearRequest } from "../commands/gear";
import dataset from "../../../../fixtures/contracts/v1/reference-dataset.json";
const catalog: GearCatalog = { snapshot_id: "s", revision: "r", fixture: true, context: dataset.payload.items[0]?.key.context, equipment: {}, items: dataset.payload.items, acquisitions: dataset.payload.acquisitions } as GearCatalog;
function mockReport(request: GearRequest): GearReport {
  const score: GearReport["current_score"] = { label: "model score", model_version: "gear-model-v1", role: request.model.role, weight_source: request.model.weight_source, contributions: [], total: 0, unmodeled_effects: [], profile_unmodeled: [], certainty: "low", certainty_reasons: [] };
  const ranking = { ranking: "theoretical", label: "Best among verified candidates", scope_note: "Explicit manual fixture", model_version: "gear-model-v1", role: request.model.role, valid: [], uncertain: [], rejected: [], excluded: [], top_is_definitive: false } as const;
  return { request, fixture: true, context: catalog.context, current: { slots: {} }, current_score: score, baseline_gaps: ["Baseline slot missing"], theoretical: { ...ranking, valid: [], uncertain: [], rejected: [], excluded: [] }, equip_eligible: { ...ranking, valid: [], uncertain: [], rejected: [], excluded: [] }, obtainable: { ...ranking, ranking: "obtainable", valid: [], uncertain: [], rejected: [], excluded: [] }, candidates: [], practical: [], practical_model_version: "gear-practical-v1", assumptions: ["Explicit manual assumptions"] };
}
function adapter(compare: (request: GearRequest) => Promise<GearReport>, fixture = true): CommandAdapter {
  return { invoke: vi.fn((command, input) => {
    if (command === "list_profiles") return Promise.resolve([{ id: "p", display_name: "Synthetic profile", fixture }] as never);
    if (command === "get_planning_catalog") return Promise.resolve({snapshot_id:"s",revision:"r",fixture,nodes:[],objectives:[]} as never);
    if (command === "get_authored_catalog") return Promise.resolve({revision:"r",records:[],total:0,next_offset:null} as never);
    if (command === "get_gear_catalog") return Promise.resolve({ ...catalog, fixture } as never);
    if (command === "compare_gear") return compare(JSON.parse((input as {text:string}).text) as GearRequest) as Promise<never>;
    return Promise.reject(new Error("unexpected command"));
  }) };
}
it("requires explicit manual weights and budgets and labels the synthetic example", async () => {
  const compare = vi.fn((request: GearRequest) => Promise.resolve(mockReport(request)));
  render(<GearScreen client={createClient(adapter(compare))} refreshKey={0} />);
  await screen.findByText(/Snapshot: s; revision: r/);
  fireEvent.click(screen.getByRole("button", { name: "Compare gear" }));
  expect(await screen.findByRole("alert")).toHaveTextContent("explicitly supply weights and session budget");
  expect(compare).not.toHaveBeenCalled();
  fireEvent.click(screen.getByRole("button", { name: "Load labelled synthetic example" }));
  expect(screen.getByText(/Engineering example:/)).toBeInTheDocument();
  fireEvent.click(screen.getByRole("button", { name: "Compare gear" }));
  await screen.findByRole("region", { name: "Gear result" });
  expect(compare.mock.calls[0]?.[0].model.weight_source).toEqual({ type: "synthetic_test" });
  expect(compare.mock.calls[0]?.[0].candidates.every((candidate) => candidate.acquisition === null)).toBe(true);
  expect(screen.getByText("Baseline slot missing")).toBeInTheDocument();
  expect(screen.getByText(/No definite practical recommendation/)).toBeInTheDocument();
  fireEvent.change(screen.getByLabelText("Role identifier"), { target: { value: "edited" } });
  expect(screen.queryByRole("region", { name: "Gear result" })).not.toBeInTheDocument();
});
it("ignores superseded comparison responses and refreshes captured inputs", async () => {
  let finish: (report: GearReport) => void = () => {};
  let pending: GearRequest | null = null;
  const compare = vi.fn((request: GearRequest) => { pending = request; return new Promise<GearReport>((resolve) => { finish = resolve; }); });
  const client = createClient(adapter(compare));
  const view = render(<GearScreen client={client} refreshKey={0} />);
  await screen.findByText(/Snapshot: s; revision: r/);
  fireEvent.click(screen.getByRole("button", { name: "Load labelled synthetic example" }));
  fireEvent.click(screen.getByRole("button", { name: "Compare gear" }));
  await waitFor(() => expect(compare).toHaveBeenCalledOnce());
  fireEvent.change(screen.getByLabelText("attempt (ms)"), { target: { value: "2000" } });
  await act(async () => { if (pending === null) throw new Error("missing request"); finish(mockReport(pending)); });
  expect(screen.queryByRole("region", { name: "Gear result" })).not.toBeInTheDocument();
  view.rerender(<GearScreen client={client} refreshKey={1} />);
  await screen.findByText(/Snapshot: s; revision: r/);
  expect(screen.getByText(/Engineering example:/)).toBeInTheDocument();
  expect(screen.getByLabelText("attempt (ms)")).toHaveValue(2000);
  expect(screen.getByLabelText("Attribute weights (JSON, integer thousandths)")).not.toHaveValue("{}");
  expect(screen.getAllByLabelText(/^Compare demo-item/).every((checkbox) => (checkbox as HTMLInputElement).checked)).toBe(true);
});
it("disables synthetic assumptions for real data and surfaces backend input invalidation", async () => {
  const compare = vi.fn(() => Promise.reject({code:"INVALID_INPUT",message:"Active content changed; refresh gear inputs",retryable:false}));
  render(<GearScreen client={createClient(adapter(compare, false))} refreshKey={0} />);
  await screen.findByText(/Snapshot: s; revision: r/);
  expect(screen.getByRole("button", { name: "Load labelled synthetic example" })).toBeDisabled();
  fireEvent.change(screen.getByLabelText("Attribute weights (JSON, integer thousandths)"), {target:{value:'{"manual":1000}'}});
  fireEvent.change(screen.getByLabelText("budget (ms)"), {target:{value:"1000"}});
  const checkbox = screen.getAllByLabelText(/^Compare demo-item/)[0]; if (!checkbox) throw new Error("missing candidate"); fireEvent.click(checkbox);
  fireEvent.click(screen.getByRole("button", {name:"Compare gear"}));
  expect(await screen.findByRole("alert")).toHaveTextContent("Active content changed; refresh gear inputs");
});
it("retains model edits while pruning removed candidate and hand references", async () => {
  let currentCatalog = catalog;
  const client = createClient({ invoke: vi.fn((command) => {
    if (command === "list_profiles") return Promise.resolve([{id:"p",display_name:"Synthetic",fixture:true}] as never);
    if (command === "get_planning_catalog") return Promise.resolve({snapshot_id:"s",revision:"r",fixture:true,nodes:[],objectives:[]} as never);
    if (command === "get_authored_catalog") return Promise.resolve({revision:"r",records:[],total:0,next_offset:null} as never);
    if (command === "get_gear_catalog") return Promise.resolve(currentCatalog as never);
    return Promise.reject(new Error("unexpected"));
  }) });
  const view = render(<GearScreen client={client} refreshKey={0} />);
  await screen.findByText(/Snapshot: s; revision: r/);
  fireEvent.click(screen.getByRole("button", {name:"Load labelled synthetic example"}));
  fireEvent.change(screen.getByLabelText("Main-hand slot"), {target:{value:"main_hand"}});
  const savedWeights = (screen.getByLabelText("Attribute weights (JSON, integer thousandths)") as HTMLTextAreaElement).value;
  fireEvent.change(screen.getByLabelText("Acquisition source for demo-item-1"), {target:{value:"demo-acq-1"}});
  currentCatalog = {...catalog, revision:"no-sources", acquisitions:[]};
  view.rerender(<GearScreen client={client} refreshKey={1} />);
  await screen.findByText(/revision: no-sources/);
  expect(screen.getByLabelText("Acquisition source for demo-item-1")).toHaveValue("");
  currentCatalog = {...catalog, revision:"replaced", items:[],equipment:{},acquisitions:[]};
  view.rerender(<GearScreen client={client} refreshKey={2} />);
  await screen.findByText(/revision: replaced/);
  expect(screen.getByLabelText("Attribute weights (JSON, integer thousandths)")).toHaveValue(savedWeights);
  expect(screen.getByLabelText("Main-hand slot")).toHaveValue("");
  expect(screen.queryAllByLabelText(/^Compare demo-item/)).toHaveLength(0);
});
it("retains bounded candidates when browsing and refreshing an immutable catalog",async()=>{
 const first=catalog.items[0];if(!first)throw Error('missing fixture');
 const second={...first,key:{...first.key,item_id:'page-two-item'}};
 const info={revision:'r',catalog_sha256:'a'.repeat(64),collection:'items' as const,offset:0,total:201,next_offset:200};
 const base=adapter(async request=>mockReport(request));
 const client=createClient({invoke:vi.fn((command,input)=>{
  if(command==='get_gear_catalog')return Promise.resolve({...catalog,items:[first],pages:{items:info}} as never);
  if(command==='get_catalog_page')return Promise.resolve({...info,offset:200,next_offset:null,records:[second]} as never);
  return base.invoke(command,input);
 })});
 const view=render(<GearScreen client={client} refreshKey={0}/>);
 fireEvent.click(await screen.findByLabelText(`Compare ${first.key.item_id}`));
 fireEvent.click(screen.getByText('Next items'));
 await screen.findByLabelText('Compare page-two-item');
 expect(screen.getByText('Selected candidates: 1 / 128')).toBeInTheDocument();
 expect(screen.getByRole('button',{name:`Remove ${first.key.item_id}`})).toBeInTheDocument();
 fireEvent.click(screen.getByLabelText('Compare page-two-item'));
 view.rerender(<GearScreen client={client} refreshKey={1}/>);
 await screen.findByLabelText(`Compare ${first.key.item_id}`);
 expect(screen.getByText('Selected candidates: 2 / 128')).toBeInTheDocument();
 expect(screen.getByRole('button',{name:'Remove page-two-item'})).toBeInTheDocument();
});
