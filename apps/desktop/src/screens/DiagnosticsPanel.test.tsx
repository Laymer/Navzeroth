import {fireEvent,render,screen,waitFor} from "@testing-library/react";
import {vi} from "vitest";
import {DiagnosticsPanel} from "./DiagnosticsPanel";
import type {DiagnosticsClient,DiagnosticReport} from "../commands/diagnostics";
const report:DiagnosticReport={format:"azeroth-local-diagnostics",schema_version:1,versions:{app:"0.4.0",bundled_addon:"0.4.0",installed_addon:null},os:"linux",game:null,capabilities:{},historical_user_report:{build:70338,interface:16001,source:"limited_user_report",fields:["level","clock"],installed_revision_match:null},snapshot_availability:{},snapshot_freshness:null,field_freshness:{},dataset:null,dataset_context_mismatch:false,telemetry:{scope:"current_process_collector",event_limit:64,truncated:null,import_attempts:null,import_succeeded:null,import_failed:null,events:[]}};
function mock(){const previewDiagnostics=vi.fn(async()=>({ok:true as const,value:report}));const exportDiagnostics=vi.fn(async()=>({ok:true as const,value:{schema_version:1 as const,bytes:123}}));return {previewDiagnostics,exportDiagnostics} satisfies DiagnosticsClient;}
test("preview and local export require separate explicit actions; missing counters stay unknown",async()=>{const client=mock();render(<DiagnosticsPanel client={client}/>);expect(client.previewDiagnostics).not.toHaveBeenCalled();expect(client.exportDiagnostics).not.toHaveBeenCalled();expect(screen.getByRole("button",{name:"Create local diagnostic file"})).toBeDisabled();fireEvent.click(screen.getByRole("button",{name:"Preview diagnostic metadata"}));await screen.findByText("Recorded imports: unknown.");expect(client.exportDiagnostics).not.toHaveBeenCalled();fireEvent.change(screen.getByLabelText("New diagnostic file path"),{target:{value:"/selected/report.json"}});fireEvent.click(screen.getByRole("button",{name:"Create local diagnostic file"}));await screen.findByText(/Local diagnostic file created/);expect(client.exportDiagnostics).toHaveBeenCalledWith("/selected/report.json");});
test("failed export only shows fixed code and retains preview",async()=>{const client:DiagnosticsClient={previewDiagnostics:async()=>({ok:true,value:report}),exportDiagnostics:async()=>({ok:false,error:{code:"STORAGE_ERROR",message:"private-secret-account-path",retryable:false}})};render(<DiagnosticsPanel client={client}/>);fireEvent.click(screen.getByRole("button",{name:"Preview diagnostic metadata"}));await screen.findByText("Recorded imports: unknown.");fireEvent.change(screen.getByLabelText("New diagnostic file path"),{target:{value:"/selected/report.json"}});fireEvent.click(screen.getByRole("button",{name:"Create local diagnostic file"}));await screen.findByText("Export failed: STORAGE_ERROR.");expect(screen.queryByText(/private-secret/)).toBeNull();expect(screen.getByText("Recorded imports: unknown.")).toBeVisible();});
test("refresh rejects an old pending preview",async()=>{let resolve:((value:{ok:true;value:DiagnosticReport})=>void)|undefined;const client:DiagnosticsClient={previewDiagnostics:()=>new Promise(r=>{resolve=r}),exportDiagnostics:async()=>({ok:true,value:{schema_version:1,bytes:1}})};const view=render(<DiagnosticsPanel client={client} refreshKey={0}/>);fireEvent.click(screen.getByRole("button",{name:"Preview diagnostic metadata"}));await waitFor(()=>expect(resolve).toBeDefined());view.rerender(<DiagnosticsPanel client={client} refreshKey={1}/>);resolve?.({ok:true,value:report});await waitFor(()=>expect(screen.getByRole("button",{name:"Preview diagnostic metadata"})).toBeEnabled());expect(screen.queryByText("Recorded imports: unknown.")).toBeNull();});
test("old completion cannot unlock or replace a newer pending request after refresh",async()=>{
 const pending:((value:{ok:true;value:DiagnosticReport})=>void)[]=[];
 const client:DiagnosticsClient={previewDiagnostics:()=>new Promise(resolve=>pending.push(resolve)),exportDiagnostics:async()=>({ok:true,value:{schema_version:1,bytes:1}})};
 const view=render(<DiagnosticsPanel client={client} refreshKey={0}/>);
 fireEvent.click(screen.getByRole("button",{name:"Preview diagnostic metadata"}));
 await waitFor(()=>expect(pending).toHaveLength(1));
 view.rerender(<DiagnosticsPanel client={client} refreshKey={1}/>);
 expect(screen.getByRole("button",{name:"Preview diagnostic metadata"})).toBeEnabled();
 fireEvent.click(screen.getByRole("button",{name:"Preview diagnostic metadata"}));
 await waitFor(()=>expect(pending).toHaveLength(2));
 const old=pending[0]; if(!old)throw Error("Missing old request"); old({ok:true,value:{...report,versions:{...report.versions,app:"old-source"}}});
 await waitFor(()=>expect(screen.getByRole("button",{name:"Preview diagnostic metadata"})).toBeDisabled());
 expect(screen.queryByText(/old-source/)).toBeNull();
 const next=pending[1]; if(!next)throw Error("Missing new request"); next({ok:true,value:{...report,versions:{...report.versions,app:"new-source"},dataset:{revision_fingerprint_sha256:"a".repeat(64),coverage_class:"demo",collections:{},errors:0,gaps:0}}});
 await screen.findByText(/App new-source/);
 expect(screen.getByRole("button",{name:"Preview diagnostic metadata"})).toBeEnabled();
 expect(screen.getByText(/"coverage_class": "demo"/)).toBeInTheDocument();
});
