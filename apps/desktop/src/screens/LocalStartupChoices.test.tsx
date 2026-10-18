import {fireEvent,render,screen} from "@testing-library/react";
import {createClient,type CommandAdapter} from "../commands";
import {LocalStartupChoices} from "./LocalStartupChoices";
const preferences={format_version:1,first_run_completed:false,startup_mode:"later",selections:{}};
it("persists the explicit startup choice before navigating without starting extraction",async()=>{
 const calls:{command:string;input:unknown}[]=[];const navigate=vi.fn();
 const adapter:CommandAdapter={invoke(command,input){calls.push({command,input});return Promise.resolve({format_version:1,revision:command==="get_local_workflow"?4:5,preferences:{...preferences,first_run_completed:command!=="get_local_workflow"}}as never);}};
 render(<LocalStartupChoices client={createClient(adapter)} onNavigate={navigate}/>);
 fireEvent.click(await screen.findByRole("button",{name:"Begin with local research content"}));
 await vi.waitFor(()=>expect(navigate).toHaveBeenCalledWith("local_research"));
 expect(calls.map(call=>call.command)).toEqual(["get_local_workflow","finish_local_startup"]);
 expect(calls[1]?.input).toEqual({expected:4,mode:"local_research",completed:true});
 expect(screen.queryByText("Choose how to begin")).not.toBeInTheDocument();
});
it("retains startup choices after a refused save and does not navigate",async()=>{
 const navigate=vi.fn();
 const adapter:CommandAdapter={invoke(command){return command==="get_local_workflow"?Promise.resolve({format_version:1,revision:0,preferences}as never):Promise.reject({code:"INVALID_INPUT",message:"private-path",retryable:false});}};
 render(<LocalStartupChoices client={createClient(adapter)} onNavigate={navigate}/>);
 fireEvent.click(await screen.findByRole("button",{name:"Choose later"}));
 expect(await screen.findByRole("alert")).toHaveTextContent("INVALID_INPUT");expect(navigate).not.toHaveBeenCalled();expect(screen.queryByText(/private-path/)).not.toBeInTheDocument();
});
it("restores the saved starting workflow without importing or activating anything",async()=>{
 const navigate=vi.fn();const calls:string[]=[];
 const adapter:CommandAdapter={invoke(command){calls.push(command);return Promise.resolve({format_version:1,revision:2,preferences:{...preferences,first_run_completed:true,startup_mode:"local_research"}}as never);}};
 render(<LocalStartupChoices client={createClient(adapter)} onNavigate={navigate}/>);
 await vi.waitFor(()=>expect(navigate).toHaveBeenCalledWith("local_research"));expect(calls).toEqual(["get_local_workflow"]);expect(screen.queryByRole("button",{name:"Choose later"})).not.toBeInTheDocument();
});
it("changes the next-launch workflow with CAS without navigating or invoking data operations",async()=>{
 const {LocalStartupSettings}=await import("./LocalStartupChoices");const calls:{command:string;input:unknown}[]=[];
 const adapter:CommandAdapter={invoke(command,input){calls.push({command,input});return Promise.resolve({format_version:1,revision:2,preferences:{...preferences,first_run_completed:true}}as never);}};
 render(<LocalStartupSettings client={createClient(adapter)}/>);fireEvent.click(screen.getByRole("button",{name:"Review starting workflow"}));
 fireEvent.change(await screen.findByRole("combobox",{name:"Starting workflow selection"}),{target:{value:"snapshots"}});fireEvent.click(screen.getByRole("button",{name:"Save starting workflow"}));
 await screen.findByText(/saved for the next launch/);expect(calls.map(call=>call.command)).toEqual(["get_local_workflow","finish_local_startup"]);expect(calls[1]?.input).toEqual({expected:2,mode:"snapshots",completed:true});
});
