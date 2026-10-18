import { act, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { vi } from "vitest";
import { PlannerScreen } from "./PlannerScreen";
import { createClient, type CommandAdapter } from "../commands";
import { syntheticRoute, required } from "./test-fixtures/route";
import type { PlanningRequest } from "../commands/contract";
import fixture from "../../../../fixtures/desktop/planning/synthetic-revision.json";
const catalog = { snapshot_id: "s", revision: "r", fixture: true, nodes: fixture.envelope.payload.nodes, objectives: [] };
function result(request: PlanningRequest) { return { ...syntheticRoute(0).report, request, assumptions: ["Retained valid estimate"] }; }
const status = { job_schema_version: 1, job_id: "job-a", kind: "solo", state: "succeeded", cancel_requested: false, phase: "selection", work_units: 5, result_available: true, error: null };
function fields() { fireEvent.change(screen.getByLabelText("Origin"), { target: { value: "demo-node-1" } });fireEvent.change(screen.getByLabelText("Stopping node"), { target: { value: "demo-node-1" } }); }
it("cancels registration that arrives after input edits and ignores its old result", async () => {
  let finish: ((value: never) => void) | undefined; const canceled: unknown[] = [];
  const adapter: CommandAdapter = { invoke(command, input) {
    if (command === "list_profiles") return Promise.resolve([{ id: "p", display_name: "Synthetic", fixture: true }] as never);
    if (command === "get_planning_catalog") return Promise.resolve(catalog as never);
    if (command === "start_planning_job") return new Promise<never>((resolve) => { finish = resolve; });
    if (command === "cancel_planning_job") { canceled.push(input);return Promise.resolve(null as never); }
    return Promise.reject(new Error("unexpected"));
  } };
  render(<PlannerScreen client={createClient(adapter)} refreshKey={0} />);await screen.findByText(/Snapshot: s; revision: r/);fields();fireEvent.click(screen.getByRole("button", { name: "Plan session" }));
  expect(screen.getByRole("button", { name: "Cancel planning" })).toBeEnabled();fireEvent.change(screen.getByLabelText("Budget (ms)"), { target: { value: "5000" } });
  await act(async () => { finish?.({ job_schema_version: 1, job_id: "job-a" } as never); });
  expect(canceled).toEqual([{ job_id: "job-a" }]);expect(screen.queryByRole("region", { name: "Planning result" })).not.toBeInTheDocument();expect(screen.queryByLabelText("Planning job progress")).not.toBeInTheDocument();
});
it("discards route responses when the planning inputs change", async () => {
  let finish: ((value: never) => void) | undefined; let request: PlanningRequest | undefined;
  const adapter: CommandAdapter = { invoke(command, input) {
    if (command === "list_profiles") return Promise.resolve([{ id: "p", display_name: "Synthetic", fixture: true }] as never);
    if (command === "get_planning_catalog") return Promise.resolve(catalog as never);
    if (command === "start_planning_job") { request = JSON.parse((input as { text: string }).text);return Promise.resolve({ job_schema_version: 1, job_id: "job-a" } as never); }
    if (command === "get_planning_job") return Promise.resolve(status as never);
    if (command === "take_planning_job_result" && request) return Promise.resolve({ job_schema_version: 1, job_id: "job-a", kind: "solo", report_json: JSON.stringify(result(request)) } as never);
    if (command === "inspect_plan_route") return new Promise<never>((resolve) => { finish = resolve; });
    return Promise.reject(new Error("unexpected"));
  } };
  render(<PlannerScreen client={createClient(adapter)} refreshKey={0} />);await screen.findByText(/Snapshot: s; revision: r/);fields();fireEvent.click(screen.getByRole("button", { name: "Plan session" }));fireEvent.click(await screen.findByRole("button", { name: "Inspect route" }));fireEvent.change(screen.getByLabelText("Budget (ms)"), { target: { value: "5000" } });await act(async () => { finish?.({} as never); });expect(screen.queryByRole("region", { name: "Route inspection" })).toBeNull();
});
it("preserves same-profile manual planning inputs during routine refresh", async () => {
  const client = createClient({ invoke: vi.fn((command) => {
    if (command === "list_profiles") return Promise.resolve([{ id: "p", display_name: "Synthetic", fixture: true }] as never);
    if (command === "get_planning_catalog") return Promise.resolve({ ...catalog, objectives: fixture.envelope.payload.objectives } as never);
    return Promise.reject(new Error("unexpected"));
  }) });
  const view = render(<PlannerScreen client={client} refreshKey={0} />);await screen.findByText(/Snapshot: s; revision: r/);fields();fireEvent.change(screen.getByLabelText("Budget (ms)"), { target: { value: "5000" } });fireEvent.click(screen.getByLabelText("Consider objective"));fireEvent.change(screen.getByLabelText("utility"), { target: { value: "42" } });view.rerender(<PlannerScreen client={client} refreshKey={1} />);await screen.findByText(/Snapshot: s; revision: r/);
  expect(screen.getByLabelText("Consider objective")).toBeChecked();expect(screen.getByLabelText("utility")).toHaveValue(42);expect(screen.getByLabelText("Objective node")).toHaveValue("");expect(screen.getByLabelText("Origin")).toHaveValue("demo-node-1");expect(screen.getByLabelText("Budget (ms)")).toHaveValue(5000);
});
it("retains the previous valid same-input estimate on cancel and ordinary failure but clears stale publication", async () => {
  let request: PlanningRequest | undefined; let mode: "success" | "running" | "failed" | "stale" = "success"; const canceled: unknown[] = [];
  const adapter: CommandAdapter = { invoke(command, input) {
    if (command === "list_profiles") return Promise.resolve([{ id: "p", display_name: "Synthetic", fixture: true }] as never);
    if (command === "get_planning_catalog") return Promise.resolve(catalog as never);
    if (command === "start_planning_job") { request = JSON.parse((input as { text: string }).text);return Promise.resolve({ job_schema_version: 1, job_id: "job-a" } as never); }
    if (command === "get_planning_job") return Promise.resolve({ ...status, state: mode === "success" ? "succeeded" : mode === "running" ? "running" : "failed", error: mode === "failed" || mode === "stale" ? { code: mode === "stale" ? "SNAPSHOT_OUTDATED" : "PROVIDER_UNAVAILABLE", message: mode === "stale" ? "Snapshot changed" : "Work unavailable", retryable: false } : null } as never);
    if (command === "take_planning_job_result" && request) return Promise.resolve({ job_schema_version: 1, job_id: "job-a", kind: "solo", report_json: JSON.stringify(result(request)) } as never);
    if (command === "cancel_planning_job") { canceled.push(input);return Promise.resolve(null as never); }
    return Promise.reject(new Error("unexpected"));
  } };
  render(<PlannerScreen client={createClient(adapter)} refreshKey={0} />);await screen.findByText(/Snapshot: s; revision: r/);fields();fireEvent.click(screen.getByRole("button", { name: "Plan session" }));await screen.findByText("Retained valid estimate");
  mode = "running";fireEvent.click(screen.getByRole("button", { name: "Plan session" }));await screen.findByText(/running; selection; 5 work units/);expect(screen.getByText("Retained valid estimate")).toBeInTheDocument();fireEvent.click(screen.getByRole("button", { name: "Cancel planning" }));await waitFor(() => expect(canceled).toEqual([{ job_id: "job-a" }]));expect(screen.getByText("Retained valid estimate")).toBeInTheDocument();
  mode = "failed";fireEvent.click(screen.getByRole("button", { name: "Plan session" }));await screen.findByText("Work unavailable");expect(screen.getByText("Retained valid estimate")).toBeInTheDocument();
  mode = "stale";fireEvent.click(screen.getByRole("button", { name: "Plan session" }));await screen.findByText("Snapshot changed");expect(screen.queryByRole("region", { name: "Planning result" })).not.toBeInTheDocument();
});
it("profile changes abort running work and ignore late progress; navigation aborts a pending registration", async () => {
  let finishStatus: ((value: never) => void) | undefined; let finishStart: ((value: never) => void) | undefined; let delayedStart = false; const canceled: unknown[] = [];
  const adapter: CommandAdapter = { invoke(command, input) {
    if (command === "list_profiles") return Promise.resolve([{ id: "p", display_name: "One" }, { id: "q", display_name: "Two" }] as never);
    if (command === "get_planning_catalog") return Promise.resolve(catalog as never);
    if (command === "start_planning_job") return delayedStart ? new Promise<never>((resolve) => { finishStart = resolve; }) : Promise.resolve({ job_schema_version: 1, job_id: "job-a" } as never);
    if (command === "get_planning_job") return new Promise<never>((resolve) => { finishStatus = resolve; });
    if (command === "cancel_planning_job") { canceled.push(input);return Promise.resolve(null as never); }
    return Promise.reject(new Error("unexpected"));
  } };
  const view = render(<PlannerScreen client={createClient(adapter)} refreshKey={0} />);await screen.findByText(/Snapshot: s; revision: r/);fields();fireEvent.click(screen.getByRole("button", { name: "Plan session" }));await waitFor(() => expect(finishStatus).toBeDefined());fireEvent.change(screen.getByLabelText("Planning profile"), { target: { value: "q" } });
  await act(async () => { finishStatus?.({ ...status, state: "running", work_units: 999 } as never); });expect(screen.queryByText(/999 work units/)).not.toBeInTheDocument();await screen.findByText(/Snapshot: s; revision: r/);fields();delayedStart = true;fireEvent.click(screen.getByRole("button", { name: "Plan session" }));view.unmount();await act(async () => { finishStart?.({ job_schema_version: 1, job_id: "job-b" } as never); });expect(canceled).toEqual([{ job_id: "job-a" }, { job_id: "job-b" }]);
});
for (const mismatch of ["context", "fixture", "choices"] as const) it(`rejects route ${mismatch} changes even when revision and node labels match`, async () => {
  let request: PlanningRequest | undefined;
  const client = createClient({ invoke(command, input) {
    if(command === "list_profiles") return Promise.resolve([{id:"p",display_name:"Synthetic",fixture:true}] as never);
    if(command === "get_planning_catalog") return Promise.resolve(catalog as never);
    if(command === "start_planning_job") { request=JSON.parse((input as {text:string}).text); return Promise.resolve({job_schema_version:1,job_id:"job-a"} as never); }
    if(command === "get_planning_job") return Promise.resolve(status as never);
    if(command === "take_planning_job_result" && request) return Promise.resolve({job_schema_version:1,job_id:"job-a",kind:"solo",report_json:JSON.stringify(result(request))} as never);
    if(command === "inspect_plan_route" && request) {
      const route=syntheticRoute(); route.report=result(request);
      if(mismatch === "context") route.context={...route.context,build:{...route.context.build,build:"another-build"}};
      if(mismatch === "fixture") route.fixture=false;
      if(mismatch === "choices") route.report.result.stopping.total_ms=1;
      return Promise.resolve(route as never);
    }
    return Promise.reject(new Error("unexpected"));
  }});
  render(<PlannerScreen client={client} refreshKey={0}/>);await screen.findByText(/Snapshot: s; revision: r/);fields();fireEvent.click(screen.getByRole("button",{name:"Plan session"}));fireEvent.click(await screen.findByRole("button",{name:"Inspect route"}));
  await screen.findByText(/Route inspection no longer matches/);expect(screen.queryByRole("region",{name:"Planning result"})).toBeNull();expect(screen.queryByRole("region",{name:"Route inspection"})).toBeNull();
});
it("discards deferred route projections across same-profile content refresh and accepts the new original report", async () => {
  let request:PlanningRequest|undefined;let finish:((v:never)=>void)|undefined;let refreshed=false;
  const client=createClient({invoke(command,input){
    if(command === "list_profiles")return Promise.resolve([{id:"p",display_name:"Synthetic",fixture:true}] as never);
    if(command === "get_planning_catalog")return Promise.resolve({...catalog,snapshot_id:refreshed?"new-snapshot":"s"} as never);
    if(command === "start_planning_job"){request=JSON.parse((input as {text:string}).text);return Promise.resolve({job_schema_version:1,job_id:"job-a"} as never);}
    if(command === "get_planning_job")return Promise.resolve(status as never);
    if(command === "take_planning_job_result"&&request)return Promise.resolve({job_schema_version:1,job_id:"job-a",kind:"solo",report_json:JSON.stringify(result(request))} as never);
    if(command === "inspect_plan_route")return new Promise<never>(resolve=>{finish=resolve;});
    return Promise.reject(new Error("unexpected"));
  }});
  const view=render(<PlannerScreen client={client} refreshKey={0}/>);await screen.findByText(/Snapshot: s; revision: r/);fields();fireEvent.click(screen.getByRole("button",{name:"Plan session"}));fireEvent.click(await screen.findByRole("button",{name:"Inspect route"}));
  const oldRoute={...syntheticRoute(),report:result(required(request))};
  refreshed=true;view.rerender(<PlannerScreen client={client} refreshKey={1}/>);
  await screen.findByText(/Snapshot: new-snapshot; revision: r/);
  await act(async()=>{finish?.(oldRoute as never);});expect(screen.queryByRole("region",{name:"Route inspection"})).toBeNull();
  fireEvent.click(screen.getByRole("button",{name:"Plan session"}));fireEvent.click(await screen.findByRole("button",{name:"Inspect route"}));
  await act(async()=>{finish?.({...syntheticRoute(1),report:result(required(request))} as never);});
  expect(screen.getByRole("region",{name:"Route inspection"})).toBeInTheDocument();expect(screen.getByText(/Snapshot new-snapshot; revision r/)).toBeInTheDocument();
});
it("exports only the calculated request and reports clipboard failure without changing export text",async()=>{
 let request:PlanningRequest|undefined;const calls:{command:string;input:unknown}[]=[];
 const adapter:CommandAdapter={invoke(command,input){calls.push({command,input});
  if(command==="list_profiles")return Promise.resolve([{id:"p",display_name:"Synthetic",fixture:true}]as never);
  if(command==="get_planning_catalog")return Promise.resolve(catalog as never);
  if(command==="start_planning_job"){request=JSON.parse((input as {text:string}).text);return Promise.resolve({job_schema_version:1,job_id:"job-a"}as never);}
  if(command==="get_planning_job")return Promise.resolve(status as never);
  if(command==="take_planning_job_result"&&request){const report=result(request);report.result.selected=[{candidate:"synthetic-service",utility:0,incremental_ms:0,completion_ms:0,service_ms:0,travel:{total_ms:0},expected_session_ms:0}];return Promise.resolve({job_schema_version:1,job_id:"job-a",kind:"solo",report_json:JSON.stringify(report)}as never);}
  if(command==="export_plan")return Promise.resolve({text:"ANAV1:synthetic-plan",byte_count:20,plan_id:"synthetic-plan",expires_at_utc_ms:0}as never);
  return Promise.reject(new Error("Unexpected command"));
 }};
 const writeText=vi.fn().mockRejectedValueOnce(new Error("Unavailable clipboard")).mockResolvedValueOnce(undefined);
 const clipboard=Object.getOwnPropertyDescriptor(navigator,"clipboard");Object.defineProperty(navigator,"clipboard",{configurable:true,value:{writeText}});
 try{
  render(<PlannerScreen client={createClient(adapter)} refreshKey={0}/>);
  await screen.findByText(/Snapshot: s; revision: r/);fields();
  fireEvent.click(screen.getByRole("button",{name:"Plan session"}));
  fireEvent.click(await screen.findByRole("button",{name:"Export plan"}));
  const text=await screen.findByRole("textbox",{name:"Plan export text"});expect(text).toHaveValue("ANAV1:synthetic-plan");
  expect(calls).toContainEqual({command:"export_plan",input:{text:JSON.stringify(request)}});
  fireEvent.focus(text);fireEvent.click(screen.getByRole("button",{name:"Copy plan"}));
  await screen.findByText("Clipboard unavailable; select the text and copy manually.");expect(text).toHaveValue("ANAV1:synthetic-plan");
  fireEvent.click(screen.getByRole("button",{name:"Copy plan"}));await screen.findByText("Copied plan export.");
  expect(writeText).toHaveBeenNthCalledWith(1,"ANAV1:synthetic-plan");expect(writeText).toHaveBeenCalledTimes(2);
 }finally{if(clipboard)Object.defineProperty(navigator,"clipboard",clipboard);else Reflect.deleteProperty(navigator,"clipboard");}
});
