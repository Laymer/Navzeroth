import {fireEvent,render,screen,waitFor} from "@testing-library/react";
import {createClient,type CommandAdapter} from "../commands";
import {ResearchDeliveryPanel} from "./ResearchDeliveryPanel";
const revision="a".repeat(64);
const page={revision,domain:"equipment",offset:0,total:1,next_offset:null,rows:[{domain:"equipment",id:1,facts:{name:"Synthetic",required_level:{state:"unknown"}}}],sources:[{table:"Item",state:"projected",reason:"projected"}],gaps:[],guard:{development_match:"matched",actionable_advice:false,acceptance_ready:false,limitations:[]}};
it("requires a selected locale and explicit activation while matched research stays unavailable for advice",async()=>{
  const calls:{command:string;input:unknown}[]=[];
  const adapter:CommandAdapter={invoke(command,input){calls.push({command,input});return Promise.resolve((command==="prepare_research_delivery"?revision:command==="get_research_delivery_page"?page:command==="activate_research_delivery"?{format_version:1,scope:"s",generation:1,delivery_sha256:revision,verified_for_planning:false}:{format_version:1,scope:"s",generation:0,delivery_sha256:null,verified_for_planning:false}) as never);}};
  render(<ResearchDeliveryPanel client={createClient(adapter)} catalog={"c".repeat(64)} preset="small"/>);
  expect(screen.getByRole("button",{name:"Prepare research reference"})).toBeDisabled();
  fireEvent.change(screen.getByLabelText("Installed content locale"),{target:{value:"enUS"}});
  fireEvent.click(screen.getByRole("button",{name:"Prepare research reference"}));await screen.findByText(/Development match: matched/);
  expect(screen.getByText(/Advice and acceptance readiness remain unavailable/)).toBeInTheDocument();
  expect(calls.some(c=>c.command==="activate_research_delivery")).toBe(false);
  expect(calls.find(c=>c.command==="prepare_research_delivery")?.input).toEqual({catalog:"c".repeat(64),locale:"enUS",bindings:{format_version:1,inputs:[],media:[],guide_facts:false,preset:"small"}});
  expect(calls.find(c=>c.command==="get_research_delivery_page")?.input).toEqual({revision,domain:"equipment",offset:0,limit:200});
  fireEvent.click(screen.getByRole("button",{name:"Select this research reference"}));await screen.findByText(/Research reference selected/);
  expect(calls.find(c=>c.command==="activate_research_delivery")?.input).toEqual({revision,expected:0,rollback:false});
});
it("drops stale preparation results when catalog changes",async()=>{
  let resolve!:(value:never)=>void;
  const adapter:CommandAdapter={invoke(){return new Promise<never>(r=>{resolve=r;});}};const client=createClient(adapter);
  const view=render(<ResearchDeliveryPanel client={client} catalog="old"/>);
  fireEvent.change(screen.getByLabelText("Installed content locale"),{target:{value:"enUS"}});fireEvent.click(screen.getByRole("button",{name:"Prepare research reference"}));
  view.rerender(<ResearchDeliveryPanel client={client} catalog="new"/>);resolve(revision as never);
  await waitFor(()=>expect(screen.getByRole("button",{name:"Prepare research reference"})).not.toBeDisabled());
  expect(screen.queryByText(/Development match:/)).not.toBeInTheDocument();
});
it("distinguishes unavailable counts and refuses pages from another revision",async()=>{
  let wrong=false;
  const adapter:CommandAdapter={invoke(command){return Promise.resolve((command==="prepare_research_delivery"?revision:command==="get_research_delivery_page"?{...page,revision:wrong?"wrong":revision,total:null,rows:[],sources:[{table:"Item",state:"gap",reason:"unsupported_layout"}]}:{format_version:1,scope:"s",generation:0,delivery_sha256:null,verified_for_planning:false}) as never);}};
  render(<ResearchDeliveryPanel client={createClient(adapter)} catalog="catalog"/>);
  fireEvent.change(screen.getByLabelText("Installed content locale"),{target:{value:"enUS"}});fireEvent.click(screen.getByRole("button",{name:"Prepare research reference"}));await screen.findByText(/Record count unavailable/);
  expect(screen.getByText(/unsupported_layout/)).toBeInTheDocument();wrong=true;
  fireEvent.click(screen.getByRole("button",{name:"Inspect reference facts"}));await screen.findByText(/Reference page changed or is unavailable/);
  expect(screen.queryByText(/Record count unavailable/)).not.toBeInTheDocument();
});
it("requires an exact rollback comparison and refreshes after a generation refusal",async()=>{
  const previous="b".repeat(64);const target="c".repeat(64);const calls:{command:string;input:unknown}[]=[];
  const adapter:CommandAdapter={invoke(command,input){calls.push({command,input});if(command==="activate_research_delivery")return Promise.reject({code:"INVALID_INPUT",message:"private-path",retryable:false});const args=input as {before?:string;after?:string};return Promise.resolve((command==="prepare_research_delivery"?revision:command==="get_research_delivery_page"?page:command==="diff_research_delivery"?{before:args.before,after:args.after,changed:["media"],same_scope:true,actionable_advice:false}:{format_version:1,scope:"s",generation:3,delivery_sha256:previous,verified_for_planning:false}) as never);}};
  render(<ResearchDeliveryPanel client={createClient(adapter)} catalog="catalog"/>);
  fireEvent.change(screen.getByLabelText("Installed content locale"),{target:{value:"enUS"}});fireEvent.click(screen.getByRole("button",{name:"Prepare research reference"}));await screen.findByText(/Changed bindings: media/);
  expect(screen.getByRole("button",{name:"Restore reviewed reference"})).toBeDisabled();
  fireEvent.change(screen.getByLabelText("Previously selected reference revision"),{target:{value:target}});fireEvent.click(screen.getByRole("button",{name:"Review reference rollback"}));
  await waitFor(()=>expect(screen.getByRole("button",{name:"Restore reviewed reference"})).not.toBeDisabled());
  fireEvent.click(screen.getByRole("button",{name:"Restore reviewed reference"}));await screen.findByText(/Prepare again to refresh its generation/);
  expect(calls.find(c=>c.command==="activate_research_delivery")?.input).toEqual({revision:target,expected:3,rollback:true});
  expect(screen.queryByText(/private-path/)).not.toBeInTheDocument();expect(screen.queryByRole("button",{name:"Restore reviewed reference"})).not.toBeInTheDocument();
});
it("inspects retained references through their manifest and exact comparison before activation",async()=>{
 const catalog="c".repeat(64),previous="b".repeat(64);const calls:{command:string;input:unknown}[]=[];
 const adapter:CommandAdapter={invoke(command,input){calls.push({command,input});return Promise.resolve((
  command==="get_local_artifact_page"?{namespace:"deliveries",revision:"index",offset:0,total:1,next_offset:null,rows:[revision]}:
  command==="get_research_delivery_manifest"?{format_version:1,catalog_sha256:catalog,tuple:{build:70450,locale:"enUS"},fixture:true}:
  command==="get_research_delivery_page"?page:
  command==="diff_research_delivery"?{before:previous,after:revision,changed:["projection"],same_scope:true,actionable_advice:false}:
  {format_version:1,scope:"s",generation:4,delivery_sha256:previous,verified_for_planning:false})as never);}};
 render(<ResearchDeliveryPanel client={createClient(adapter)} catalog={catalog}/>);
 fireEvent.click(screen.getByRole("button",{name:"Review retained research references"}));
 fireEvent.click(await screen.findByRole("button",{name:"Inspect reference aaaaaaaa"}));
 await screen.findByText(/Changed bindings: projection/);
 expect(screen.getByRole("button",{name:"Select this research reference"})).not.toBeDisabled();
 expect(calls).toContainEqual({command:"diff_research_delivery",input:{before:previous,after:revision}});
 expect(calls.some(call=>call.command==="prepare_research_delivery"||call.command==="activate_research_delivery")).toBe(false);
});
