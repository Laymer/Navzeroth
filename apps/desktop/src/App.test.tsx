import { render, screen } from "@testing-library/react";
import { App } from "./App";
import { createClient, type CommandAdapter } from "./commands";
import { demoAdapter } from "./commands/demoAdapter";
import { toAppError } from "./commands/client";
import { describeObservation } from "./observation";

describe("demo mode", () => {
  it("shows the Demo label and the synthetic snapshot", async () => {
    render(<App client={createClient(demoAdapter)} />);
    expect(await screen.findByText(/DEMO — synthetic data/)).toBeInTheDocument();
    expect(screen.getByText("demo-snapshot-001")).toBeInTheDocument();
    expect(screen.getByText("22")).toBeInTheDocument();
    // role is "unknown" in the fixture and must not render as a value
    expect(screen.getByText("Unknown (not observed)")).toBeInTheDocument();
  });

  it("rejects an unknown profile with a structured error", async () => {
    const r = await createClient(demoAdapter).getCharacterSnapshot("nope");
    expect(r).toEqual({ ok: false, error: { code: "INVALID_INPUT", message: "No such profile.", retryable: false } });
  });
});

describe("non-demo and failure states", () => {
  const stored: CommandAdapter = {
    invoke: (c) =>
      c === "get_app_status"
        ? Promise.resolve({ version: "0.1.0", data_mode: "stored", active_profile: null, capabilities: {} } as never)
        : Promise.reject(new Error("unexpected")),
  };
  it("does not label stored data as demo and reports an empty state", async () => {
    render(<App client={createClient(stored)} />);
    expect(await screen.findByText("No character snapshot has been imported.")).toBeInTheDocument();
    expect(screen.queryByText(/DEMO/)).not.toBeInTheDocument();
  });

  it("loads once a startup database check finishes before the recovery panel polls", async () => {
    // The native smoke race: get_app_status fails while the startup check runs,
    // and the check is already done when the recovery panel first asks.
    let statusCalls = 0;
    const racing: CommandAdapter = {
      invoke: (c) => {
        if (c === "get_app_status") {
          statusCalls += 1;
          return statusCalls === 1
            ? Promise.reject({ code: "PROVIDER_UNAVAILABLE", message: "Database initialization is still checking", retryable: true })
            : Promise.resolve({ version: "0.1.0", data_mode: "stored", active_profile: null, capabilities: {} } as never);
        }
        if (c === "get_database_status")
          return Promise.resolve({ state: "ready", error: null, schema_version: 8, restore_limit_bytes: 1 } as never);
        return Promise.reject(new Error("unexpected"));
      },
    };
    render(<App client={createClient(racing)} />);
    expect(await screen.findByText("No character snapshot has been imported.")).toBeInTheDocument();
    expect(statusCalls).toBe(2);
  });

  it("surfaces backend failures without leaking raw error text", async () => {
    const failing: CommandAdapter = { invoke: () => Promise.reject(new Error("secret path /home/x")) };
    render(<App client={createClient(failing)} />);
    const alerts = await screen.findAllByRole("alert");
    expect(alerts.some((alert) => alert.textContent?.includes("INTERNAL"))).toBe(true);
    for (const alert of screen.getAllByRole("alert")) expect(alert).not.toHaveTextContent("secret");
  });
});

describe("helpers", () => {
  it("keeps observation states distinct", () => {
    expect(describeObservation({ status: "known", value: 0 })).toBe("0");
    const labels = (["unknown", "unsupported", "restricted", "not_applicable"] as const).map((status) =>
      describeObservation({ status }),
    );
    expect(new Set(labels).size).toBe(4);
  });
  it("normalizes malformed errors", () => {
    expect(toAppError({ code: "BOGUS", message: "x" }).code).toBe("INTERNAL");
    expect(toAppError({ code: "STORAGE_ERROR", message: "m", retryable: true })).toEqual({
      code: "STORAGE_ERROR", message: "m", retryable: true,
    });
  });
});

it("labels stored synthetic snapshots and uncertain age explicitly", async () => {
  const snapshot = (await createClient(demoAdapter).getCharacterSnapshot("demo-character-paladin"));
  if (!snapshot.ok) throw new Error("Expected demo fixture");
  const adapter: CommandAdapter = {
    invoke: (command) => {
      if (command === "get_app_status") return Promise.resolve({ version: "0.1.0", data_mode: "stored", active_profile: "stored-full-context", capabilities: {} } as never);
      if (command === "get_stored_snapshot") return Promise.resolve({ envelope: snapshot.value, profile_id: "stored-full-context", data_mode: "stored", freshness: { status: "uncertain", reason: "missing_or_untrusted_time" } } as never);
      return Promise.reject(new Error("Unexpected command"));
    },
  };
  render(<App client={createClient(adapter)} />);
  expect(await screen.findByText(/SYNTHETIC — fixture or demo data/)).toBeInTheDocument();
  expect(screen.getByText(/Last imported:.*uncertain/)).toBeInTheDocument();
  expect(screen.getByText("demo-snapshot-001")).toBeInTheDocument();
});

test("startup failure keeps explicit local diagnostics reachable without gameplay navigation", async()=>{
  const {fireEvent}=await import("@testing-library/react");
  const calls:string[]=[];
  const adapter:CommandAdapter={invoke:async(command)=>{ calls.push(command); if(command==="preview_diagnostics") return {format:"azeroth-local-diagnostics",schema_version:1,versions:{app:"test",bundled_addon:"test",installed_addon:null},os:"linux",game:null,capabilities:{},historical_user_report:{build:70338,interface:16001,source:"limited_user_report",fields:["level","clock"],installed_revision_match:null},snapshot_availability:{},snapshot_freshness:null,field_freshness:{},dataset:null,dataset_context_mismatch:false,telemetry:{scope:"current_process_collector",event_limit:64,truncated:false,import_attempts:0,import_succeeded:0,import_failed:0,events:[]}} as never; throw {code:"STORAGE_ERROR",message:"Recovery required",retryable:false};}};
  render(<App client={createClient(adapter)}/>);
  fireEvent.click(await screen.findByRole("button",{name:"Local diagnostics"}));
  expect(screen.queryByRole("button",{name:"Planner"})).toBeNull();
  expect(calls).not.toContain("preview_diagnostics");
  fireEvent.click(screen.getByRole("button",{name:"Preview diagnostic metadata"}));
  await screen.findByText("Recorded imports: 0.");
});

test("backend recovery does not reapply a saved or newly changed next-launch workflow",async()=>{
 const {fireEvent,waitFor}=await import("@testing-library/react");
 let statusCalls=0;let databaseCalls=0;let failing=false;let mode="snapshots";
 const adapter:CommandAdapter={invoke(command){
  if(command==="get_app_status") {statusCalls++;return failing?Promise.reject({code:"STORAGE_ERROR",message:"Synthetic recovery",retryable:false}):Promise.resolve({version:"synthetic",data_mode:"stored",active_profile:null,capabilities:{}}as never);}
  if(command==="get_database_status")return Promise.resolve({state:databaseCalls++===0?"checking":"ready",error:null,schema_version:8,restore_limit_bytes:1024}as never);
  if(command==="get_local_workflow")return Promise.resolve({format_version:1,revision:1,preferences:{format_version:1,first_run_completed:true,startup_mode:mode,selections:{}}}as never);
  return Promise.reject({code:"INVALID_INPUT",message:"Not provided by synthetic fixture",retryable:false});
 }};
 render(<App client={createClient(adapter)}/>);
 await waitFor(()=>expect(screen.getByRole("button",{name:"Imports"})).toHaveAttribute("aria-current","page"));
 fireEvent.click(screen.getByRole("button",{name:"Backup and recovery"}));
 await screen.findByText(/Checking database at startup/);
 mode="local_research";failing=true;
 const retry=await screen.findByRole("button",{name:"Retry loading"});
 const before=statusCalls;failing=false;fireEvent.click(retry);
 await waitFor(()=>expect(statusCalls).toBe(before+1));
 await waitFor(()=>expect(screen.getByRole("button",{name:"Backup and recovery"})).toHaveAttribute("aria-current","page"));
 expect(screen.getByRole("button",{name:"Compatibility and coverage"})).not.toHaveAttribute("aria-current","page");
});
