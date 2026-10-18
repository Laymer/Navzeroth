import {fireEvent,render,screen} from "@testing-library/react";
import {useState} from "react";
import {createClient,type CommandAdapter} from "../commands";
import {ResearchBindings,type BindingSelection} from "./ResearchBindings";
it("pages unverified candidates and keeps explicit dependency selections separate by kind",async()=>{
 const changes=vi.fn();const calls:{command:string;input:unknown}[]=[];const input="a".repeat(64),media="b".repeat(64);
 const adapter:CommandAdapter={invoke(command,input){calls.push({command,input});const namespace=(input as {namespace:string}).namespace;return Promise.resolve({format_version:1,revision:"list",namespace,offset:0,total:1,rows:[namespace==="inputs"?"a".repeat(64):"b".repeat(64)],next_offset:null,integrity_checked:false,staging_present:false}as never);}};
 function Fixture(){const [value,setValue]=useState<BindingSelection>({media:[],inputs:[],guide_facts:false});return <ResearchBindings client={client} value={value} disabled={false} onChange={next=>{changes(next);setValue(next);}}/>;}
 const client=createClient(adapter);render(<Fixture/>);
 fireEvent.click(screen.getByRole("button",{name:"Review retained dependency revisions"}));fireEvent.click(await screen.findByRole("checkbox",{name:input}));
 fireEvent.change(screen.getByRole("combobox",{name:"Dependency kind"}),{target:{value:"media"}});fireEvent.click(screen.getByRole("button",{name:"Review retained dependency revisions"}));fireEvent.click(await screen.findByRole("checkbox",{name:media}));
 fireEvent.click(screen.getByRole("checkbox",{name:/Build and bind research guide facts/}));
 expect(changes).toHaveBeenLastCalledWith({inputs:[input],media:[media],guide_facts:true});
 expect(calls[0]?.input).toEqual({namespace:"inputs",offset:0,limit:200});
 expect(screen.getByText(/integrity and source compatibility checked during preparation/)).toBeInTheDocument();
});
