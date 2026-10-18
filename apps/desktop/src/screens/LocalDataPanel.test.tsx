import {fireEvent,render,screen,waitFor} from "@testing-library/react";
import {createClient,type CommandAdapter} from "../commands";
import {LocalDataPanel} from "./LocalDataPanel";
import type {LocalDataJob} from "../commands/localData";

it("reviews explicit selection, extracts research only and requests bounded revision pages",async()=>{
  const calls:{command:string;input:unknown}[]=[];
  const job:LocalDataJob={job_id:"job-1",state:"complete",completed:1270,total:1270,error:null,summary:{catalog_sha256:"a".repeat(64),build:70425,tables_processed:1270,tables_decoded:493,actual_workers:1,gameplay_projection_available:false,activated:false}};
  const adapter:CommandAdapter={invoke(command,input){calls.push({command,input});return Promise.resolve((command==="select_local_installation"?{selection_id:"opaque-1",inspection:{metadata:{product:"wow_classic_beta",version:"1.60.1.70425",build:70425,declared_locales:["enUS"]}}}:command==="preview_local_data"?{preset:"max",limits:{workers:8,worker_memory_bytes:8*2**30,total_storage_bytes:128*2**30},required_storage_bytes:null,available_storage_bytes:999,extraction_ready:false,blockers:[]}:command==="start_local_data"?job:{revision:"a".repeat(64),offset:0,total:1270,next_offset:200,rows:[{table:"Map",locale:"enUS",disposition:"unavailable_encrypted",reason:"unavailable_encrypted"}]})as never);}};
  render(<LocalDataPanel client={createClient(adapter)}/>);
  expect(screen.getByRole("button",{name:"Review extraction limits"})).toBeDisabled();
  fireEvent.click(screen.getByRole("button",{name:"Select game installation"}));await screen.findByText(/Selected product wow_classic_beta/);
  expect(screen.queryByRole("button",{name:"Extract research tables"})).not.toBeInTheDocument();
  fireEvent.click(screen.getByRole("button",{name:"Review extraction limits"}));await screen.findByText(/Required storage: unknown/);
  expect(screen.getByRole("button",{name:"Extract research tables"})).toBeDisabled();
  fireEvent.click(screen.getByRole("checkbox",{name:"enUS"}));
  fireEvent.click(screen.getByRole("button",{name:"Extract research tables"}));await screen.findByText(/493 decoded of 1270/);
  expect(calls.find(c=>c.command==="start_local_data")?.input).toEqual({selection_id:"opaque-1",preset:"max",locales:["enUS"]});
  expect(screen.getByText(/no gameplay catalog activated/)).toBeInTheDocument();
  fireEvent.click(screen.getByRole("button",{name:"Verify and inspect inventory"}));await screen.findByRole("row",{name:/Map enUS unavailable encrypted/});
  expect(calls.find(c=>c.command==="get_local_data_inventory")?.input).toEqual({revision:"a".repeat(64),offset:0,limit:200});
});
it("reports safe errors without exposing paths and never starts from an unselected installation",async()=>{
  const adapter:CommandAdapter={invoke(){return Promise.reject({code:"PROVIDER_UNAVAILABLE",message:"private-account-path",retryable:false});}};
  render(<LocalDataPanel client={createClient(adapter)}/>);fireEvent.click(screen.getByRole("button",{name:"Select game installation"}));
  await waitFor(()=>expect(screen.getByRole("alert")).toHaveTextContent("PROVIDER_UNAVAILABLE"));expect(screen.queryByText(/private-account-path/)).not.toBeInTheDocument();
  expect(screen.getByRole("button",{name:"Review extraction limits"})).toBeDisabled();
});

it("restores saved presets and resumes interrupted work through opaque identities",async()=>{
  const calls:{command:string;input:unknown}[]=[];
  const job:LocalDataJob={job_id:"saved-job",state:"interrupted",completed:1,total:2,error:"Worker interrupted; checkpoint retained",summary:null};
  const adapter:CommandAdapter={invoke(command,input){calls.push({command,input});return Promise.resolve((command==="list_local_installations"?[{selection_id:"opaque-saved",product:"wow_classic_beta",preset:"small"}]:command==="list_local_data_jobs"?[job]:command==="restore_local_installation"?{selection_id:"opaque-saved",preset:"small",inspection:{metadata:{product:"wow_classic_beta",version:"1.60.1.70450",build:70450,declared_locales:["enUS"]}}}:command==="resume_local_data"?{...job,state:"cancelled",error:null}:null)as never);}};
  render(<LocalDataPanel client={createClient(adapter)}/>);
  fireEvent.click(screen.getByRole("button",{name:"Review saved extraction state"}));
  fireEvent.click(await screen.findByRole("button",{name:/Restore installation/}));await screen.findByText(/Selected product wow_classic_beta/);
  expect(screen.getByRole("combobox",{name:"Extraction resources"})).toHaveValue("small");
  fireEvent.click(screen.getByRole("button",{name:"Inspect saved job saved-job"}));
  expect(screen.getByText(/Extraction interrupted: 1 of 2/)).toBeInTheDocument();
  fireEvent.click(screen.getByRole("button",{name:"Resume unchanged extraction"}));await screen.findByText(/Extraction cancelled/);
  expect(calls.find(c=>c.command==="restore_local_installation")?.input).toEqual({selection_id:"opaque-saved"});
  expect(calls.find(c=>c.command==="resume_local_data")?.input).toEqual({job_id:"saved-job"});
});
it("guards research activation with the displayed generation and clears a refused guard",async()=>{
  const calls:{command:string;input:unknown}[]=[];const digest="b".repeat(64);
  const job:LocalDataJob={job_id:"staged",state:"complete",completed:1,total:1,error:null,summary:{catalog_sha256:digest,build:70450,tables_processed:1,tables_decoded:0,actual_workers:1,gameplay_projection_available:false,activated:false}};
  const adapter:CommandAdapter={invoke(command,input){calls.push({command,input});if(command==="activate_local_data")return Promise.reject({code:"INVALID_INPUT",message:"private-path",retryable:false});return Promise.resolve((command==="list_local_installations"?[]:command==="list_local_data_jobs"?[job]:{format_version:1,scope:"c".repeat(64),generation:2,catalog_sha256:"a".repeat(64),verified_for_planning:false})as never);}};
  render(<LocalDataPanel client={createClient(adapter)}/>);fireEvent.click(screen.getByRole("button",{name:"Review saved extraction state"}));fireEvent.click(await screen.findByRole("button",{name:"Inspect saved job staged"}));
  expect(screen.queryByRole("button",{name:"Activate research revision"})).not.toBeInTheDocument();
  fireEvent.click(screen.getByRole("button",{name:"Review research activation"}));await screen.findByText(/generation 2/);fireEvent.click(screen.getByRole("button",{name:"Activate research revision"}));
  await screen.findByText(/Research activation refused: INVALID_INPUT/);
  expect(calls.find(c=>c.command==="activate_local_data")?.input).toEqual({revision:digest,expected:2,rollback:false});
  expect(screen.queryByRole("button",{name:"Activate research revision"})).not.toBeInTheDocument();expect(screen.queryByText(/private-path/)).not.toBeInTheDocument();
});

it("offers cancellation during fingerprint preparation and does not offer resume or forgetting",async()=>{
 const job:LocalDataJob={job_id:"hashing",state:"preparing",completed:0,total:0,error:null,summary:null};
 const adapter:CommandAdapter={invoke(command){return Promise.resolve((command==="list_local_installations"?[]:command==="list_local_data_jobs"?[job]:{...job,state:"cancelled"})as never);}};
 render(<LocalDataPanel client={createClient(adapter)}/>);fireEvent.click(screen.getByRole("button",{name:"Review saved extraction state"}));fireEvent.click(await screen.findByRole("button",{name:"Inspect saved job hashing"}));
 expect(screen.getByText(/Extraction preparing: 0 of unknown/)).toBeInTheDocument();expect(screen.queryByRole("button",{name:"Resume unchanged extraction"})).not.toBeInTheDocument();expect(screen.queryByRole("button",{name:"Forget saved job"})).not.toBeInTheDocument();fireEvent.click(screen.getByRole("button",{name:"Cancel extraction"}));await screen.findByText(/Extraction cancelled/);
});
it("restores the same installation twice without separating checked locales from extraction",async()=>{
 const calls:{command:string;input:unknown}[]=[];
 const adapter:CommandAdapter={invoke(command,input){calls.push({command,input});return Promise.resolve((command==="select_local_installation"?{selection_id:"same",inspection:{metadata:{product:"wow_classic_beta",version:"1.60.1.70450",build:70450,declared_locales:["enUS"]}}}:command==="get_local_workflow"?{format_version:1,revision:1,preferences:{format_version:1,first_run_completed:true,startup_mode:"later",selections:{same:{locales:["enUS"],preset:"max",change_detection:false,baseline_sha256:null}}}}:command==="preview_local_data"?{preset:"max",limits:{},required_storage_bytes:null,available_storage_bytes:null,blockers:[]}:null)as never);}};
 render(<LocalDataPanel client={createClient(adapter)}/>);
 for(let attempt=0;attempt<2;attempt++){
  fireEvent.click(screen.getByRole("button",{name:"Select game installation"}));
  await waitFor(()=>expect(screen.getByRole("checkbox",{name:"enUS"})).toBeChecked());
  fireEvent.click(screen.getByRole("button",{name:"Review extraction limits"}));
  await waitFor(()=>expect(screen.getByRole("button",{name:"Extract research tables"})).toBeEnabled());
 }
 expect(calls.filter(call=>call.command==="get_local_workflow")).toHaveLength(2);
});
