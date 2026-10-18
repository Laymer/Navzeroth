import {fireEvent,render,screen} from "@testing-library/react";
import {createClient,type CommandAdapter} from "../commands";
import {LocalWorkflowChoices} from "./LocalWorkflowChoices";
import type {LocalWorkflowRecord} from "../commands/localData";
const record:LocalWorkflowRecord={format_version:1,revision:3,preferences:{format_version:1,first_run_completed:false,startup_mode:"later",selections:{selected:{locales:["enUS"],preset:"small",change_detection:true,baseline_sha256:"a".repeat(64)}}}};
it("restores durable locales, saves with a generation guard and acknowledges only a staged build",async()=>{
 const calls:{command:string;input:unknown}[]=[];const changed=vi.fn();
 const adapter:CommandAdapter={invoke(command,input){calls.push({command,input});return Promise.resolve((command==="get_local_change_detectors"?[{format_version:1,selection_id:"selected",status:"staged",candidate_sha256:"b".repeat(64),auto_activation:false}]:record)as never);}};
 render(<LocalWorkflowChoices client={createClient(adapter)} selectionId="selected" locales={["deDE","enUS"]} preset="small" onLocales={changed}/>);
 await screen.findByRole("button",{name:"Save content choices"});
 await vi.waitFor(()=>expect(screen.getByRole("checkbox",{name:"enUS"})).toBeChecked());
 expect(changed).toHaveBeenCalledWith(["enUS"]);
 fireEvent.click(screen.getByRole("checkbox",{name:"deDE"}));fireEvent.click(screen.getByRole("button",{name:"Save content choices"}));
 await screen.findByText(/Choices saved/);
 expect(calls.find(call=>call.command==="save_local_workflow_choices")?.input).toEqual({selection_id:"selected",expected:3,locales:["deDE","enUS"],preset:"small",detect:true});
 fireEvent.click(screen.getByRole("button",{name:"Review detected build changes"}));
 fireEvent.click(await screen.findByRole("button",{name:"Acknowledge staged build metadata"}));
 await screen.findByText(/No extraction or activation was performed/);
 expect(calls.find(call=>call.command==="acknowledge_local_build_change")?.input).toEqual({selection_id:"selected",expected:3,candidate:"b".repeat(64)});
 expect(calls.some(call=>["start_local_data","activate_local_data"].includes(call.command))).toBe(false);
});
it("drops stale preference responses after selection unmount",async()=>{
 let resolve!:(value:unknown)=>void;const changed=vi.fn();
 const adapter:CommandAdapter={invoke(){return new Promise(done=>{resolve=done;})as never;}};
 const view=render(<LocalWorkflowChoices client={createClient(adapter)} selectionId="selected" locales={["enUS"]} preset="max" onLocales={changed}/>);
 view.unmount();resolve(record);await Promise.resolve();expect(changed).not.toHaveBeenCalled();
});
it("keeps explicit edits made while durable choices are still loading",async()=>{
 let resolve!:(value:unknown)=>void;const changed=vi.fn();
 const adapter:CommandAdapter={invoke(){return new Promise(done=>{resolve=done;})as never;}};
 render(<LocalWorkflowChoices client={createClient(adapter)} selectionId="selected" locales={["deDE","enUS"]} preset="max" onLocales={changed}/>);
 fireEvent.click(screen.getByRole("checkbox",{name:"deDE"}));
 resolve(record);
 await vi.waitFor(()=>expect(screen.getByRole("button",{name:"Save content choices"})).toBeEnabled());
 expect(screen.getByRole("checkbox",{name:"deDE"})).toBeChecked();expect(screen.getByRole("checkbox",{name:"enUS"})).not.toBeChecked();expect(changed).toHaveBeenLastCalledWith(["deDE"]);
});
