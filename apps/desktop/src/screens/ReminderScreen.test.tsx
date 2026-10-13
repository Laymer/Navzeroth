import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { expect, test } from "vitest";
import { createClient } from "../commands";
import type { CommandAdapter } from "../commands/contract";
import type { ReminderView } from "../commands/reminders";
import { ReminderScreen } from "./ReminderScreen";
function fixture() {
 const calls: {command:string;args:unknown}[]=[];
 let view:ReminderView={profile_id:"one",snapshot_id:"snapshot",revision:"revision",fixture:true,enabled:true,dismissed:[],rules:[{id:"r",text:"Synthetic advice",dismissal:"permanent"}],plan:{mode:"non_intrusive",queue:[{rule:"r",priority:1,dismissal:"permanent",kind:{kind:"advice",text:"Synthetic advice"},evidence:[]}],held:[],excluded:{}}};
 const adapter:CommandAdapter={invoke:async(command,args)=>{
  calls.push({command,args});let value:unknown;
  if(command==="list_profiles")value=[{id:"one",display_name:"One",data_mode:"stored",snapshot_count:1,fixture:true}];
  else if(command==="reminder_view")value=view;
  else if(command==="mutate_reminders") {const request=JSON.parse((args as {request_json:string}).request_json);if(request.action.kind==="acknowledge")view={...view,plan:{...view.plan!,queue:[],excluded:{r:{code:"state_unchanged"}}}};else if(request.action.kind==="dismiss")view={...view,dismissed:["r"],plan:{...view.plan!,queue:[]}};else if(request.action.kind==="restore")view={...view,dismissed:[]};else view={...view,enabled:request.action.enabled};value=view;}
  else throw Error(`Unexpected ${command}`);
  return value as never;
 }};
 return {client:createClient(adapter),calls};
}
test("inspection stays nonintrusive and presentation needs explicit acknowledgment",async()=>{
 const {client,calls}=fixture();render(<ReminderScreen client={client} refreshKey={0}/>);
 await screen.findByText("Synthetic advice");expect(screen.getByText(/Presentation:/)).toHaveTextContent("non intrusive");
 expect(calls.some(c=>c.command==="mutate_reminders")).toBe(false);
 fireEvent.click(screen.getByRole("button",{name:"Mark r as read"}));
 await screen.findByText(/No new advice/);
 await waitFor(()=>expect(calls.filter(c=>c.command==="mutate_reminders")).toHaveLength(1));
 const request=JSON.parse((calls.find(c=>c.command==="mutate_reminders")!.args as {request_json:string}).request_json);
 expect(request).toEqual({profile_id:"one",snapshot_id:"snapshot",revision:"revision",action:{kind:"acknowledge",rules:["r"]}});
});
test("hide, restore and profile preferences remain explicit",async()=>{
 const {client}=fixture();render(<ReminderScreen client={client} refreshKey={0}/>);await screen.findByText("Synthetic advice");
 fireEvent.click(screen.getByRole("button",{name:"Hide r"}));await screen.findByRole("button",{name:"Restore r"});
 fireEvent.click(screen.getByRole("button",{name:"Restore r"}));await waitFor(()=>expect(screen.queryByRole("button",{name:"Restore r"})).toBeNull());
 fireEvent.click(screen.getByLabelText("Enable reminders for this profile"));await waitFor(()=>expect(screen.getByLabelText("Enable reminders for this profile")).not.toBeChecked());
});
