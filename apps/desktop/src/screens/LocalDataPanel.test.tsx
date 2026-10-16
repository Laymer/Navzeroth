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
  fireEvent.click(screen.getByRole("button",{name:"Extract research tables"}));await screen.findByText(/493 decoded of 1270/);
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
