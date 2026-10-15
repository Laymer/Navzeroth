import {fireEvent,render,screen,waitFor} from "@testing-library/react";
import {vi} from "vitest";
import {AuthoredContentPicker} from "./AuthoredContentPicker";
import type {CommandClient} from "../commands";
it("binds selection and subsequent pages to the returned immutable revision",async()=>{
 const getAuthoredCatalog=vi.fn(async()=>({ok:true,value:{revision:"r1",records:[{id:"model-1",label:"Authored model one",evidence:["e1"]}],total:201,next_offset:200}}));
 const onChange=vi.fn();const client={getAuthoredCatalog} as unknown as CommandClient;
 render(<AuthoredContentPicker client={client} kind="models" subject={{kind:"character",profile_id:"p"}} refreshKey={0} value={null} onChange={onChange}/>);
 await screen.findByRole("option",{name:"Authored model one"});
 fireEvent.change(screen.getByLabelText("Authored gear model selection"),{target:{value:"model-1"}});
 expect(onChange).toHaveBeenCalledWith({id:"model-1",revision:"r1"});
 fireEvent.click(screen.getByRole("button",{name:"Next authored records"}));
 await waitFor(()=>expect(getAuthoredCatalog).toHaveBeenCalledTimes(2));
 expect(JSON.parse((getAuthoredCatalog.mock.calls[1] as unknown as [string])[0])).toMatchObject({revision:"r1",offset:200});
});
it("retains a stale selection visibly instead of rebinding it to a new revision",async()=>{
 const client={getAuthoredCatalog:vi.fn(async()=>({ok:true,value:{revision:"r2",records:[],total:0,next_offset:null}}))} as unknown as CommandClient;
 const onChange=vi.fn();render(<AuthoredContentPicker client={client} kind="dungeons" subject={{kind:"character",profile_id:"p"}} refreshKey={0} value={{id:"d1",revision:"r1"}} onChange={onChange}/>);
 expect(await screen.findByRole("alert")).toHaveTextContent("Selected revision changed");
 expect(onChange).not.toHaveBeenCalled();
 expect(screen.getByLabelText("Authored dungeon selection")).toHaveValue("d1");
});
