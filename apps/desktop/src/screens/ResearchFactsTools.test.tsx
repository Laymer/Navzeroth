import {fireEvent,render,screen} from "@testing-library/react";
import {createClient,type CommandAdapter} from "../commands";
import {ResearchFactsTools} from "./ResearchFactsTools";
const catalog="a".repeat(64),revision="b".repeat(64),inventory="c".repeat(64);
const manifest={format_version:1,catalog_sha256:catalog,projection_sha256:"d".repeat(64),guide_facts_sha256:null,tuple:{build:70450,locale:"enUS"},fixture:true};
it("shows exact IDs while preserving unavailable source and unknown independent completeness",async()=>{
 const calls:{command:string;input:unknown}[]=[];
 const adapter:CommandAdapter={invoke(command,input){calls.push({command,input});return Promise.resolve((command==="get_research_delivery_manifest"?manifest:command==="build_local_inventory"?inventory:command==="get_local_inventory_summary"?{revision:inventory,acceptance_ready:false,domains:[{domain:"quests",imported_count:0,source_available:false,independently_required_count:null,independently_complete:false,gaps:["prerequisites unavailable"]}]}:{revision:inventory,domain:"quests",offset:0,total:0,next_offset:null,source_available:false,independently_complete:false,records:[]})as never);}};
 render(<ResearchFactsTools client={createClient(adapter)} revision={revision} catalog={catalog} preset="small"/>);
 fireEvent.click(await screen.findByRole("button",{name:"Build exact nine-domain inventory"}));
 expect(await screen.findByRole("row",{name:/quests 0 no unknown/})).toBeInTheDocument();
 fireEvent.click(screen.getByRole("button",{name:"Inspect exact record IDs"}));await screen.findByText(/0 imported IDs; source unavailable/);
 expect(calls.find(call=>call.command==="get_local_inventory_page")?.input).toEqual({revision:inventory,domain:"quests",offset:0,limit:200,authored:null});
});
it("renders tooltip text inertly and exposes unsupported expressions",async()=>{
 const adapter:CommandAdapter={invoke(command){return Promise.resolve((command==="get_research_delivery_manifest"?manifest:{text:"<script>private()</script>",gaps:["unsupported_expression"],verified_for_gameplay:false})as never);}};
 const view=render(<ResearchFactsTools client={createClient(adapter)} revision={revision} catalog={catalog} preset="max"/>);
 fireEvent.change(await screen.findByRole("spinbutton",{name:"Research tooltip spell ID"}),{target:{value:"42"}});fireEvent.click(screen.getByRole("button",{name:"Inspect inert research tooltip"}));
 await screen.findByText("<script>private()</script>");expect(view.container.querySelector("script")).toBeNull();expect(screen.getByText(/unsupported_expression/)).toBeInTheDocument();
});
it("binds subsequent guide findings to the replay digest and clears findings when class changes",async()=>{
 const calls:{command:string;input:unknown}[]=[];const replay="e".repeat(64);
 const adapter:CommandAdapter={invoke(command,input){calls.push({command,input});const request=(input as {request?:{offset:number;expected:string|null}})?.request;return Promise.resolve((command==="get_research_delivery_manifest"?{...manifest,guide_facts_sha256:"f".repeat(64)}:command==="select_local_material"?{id:"guide",kind:"guide"}:{format_version:1,revision:replay,delivery_sha256:revision,guide_sha256:"1".repeat(64),facts_manifest_sha256:"f".repeat(64),offset:request?.offset??0,total:201,next_offset:request?.offset===0?200:null,totals:{guides:1,steps:2,errors:0,warnings:0,unverified:201,unsupported:0,asserted:0},findings:[{check:"quest_prerequisites",status:"unverified",line:7,quest:1}],acceptance_ready:false,duo_eligibility_verified:false,limitations:["research_only"]})as never);}};
 render(<ResearchFactsTools client={createClient(adapter)} revision={revision} catalog={catalog} preset="max"/>);
 fireEvent.click(await screen.findByRole("button",{name:"Select private guide for inert review"}));
 fireEvent.click(await screen.findByRole("button",{name:"Review guide against bound research facts"}));
 await screen.findByText(/201 unverified/);fireEvent.click(screen.getByRole("button",{name:"Next guide finding page"}));
 await vi.waitFor(()=>expect(calls.filter(call=>call.command==="review_selected_local_guide")).toHaveLength(2));
 expect(calls.filter(call=>call.command==="review_selected_local_guide")[1]?.input).toEqual({request:{format_version:1,delivery:revision,id:"guide",class_name:"Warrior",mode:"solo",expected:replay,offset:200,limit:200}});
 await vi.waitFor(()=>expect(screen.queryByRole("button",{name:"Next guide finding page"})).not.toBeInTheDocument());
 fireEvent.change(screen.getByRole("combobox",{name:"Human research guide class"}),{target:{value:"Mage"}});expect(screen.queryByText(/201 unverified/)).not.toBeInTheDocument();
});
it("restores scoped materials, pages exact IDs and clears imported views when the pack is forgotten",async()=>{
 const calls:{command:string;input:unknown}[]=[];
 const adapter:CommandAdapter={invoke(command,input){calls.push({command,input});
  const value=command==="get_research_delivery_manifest"?manifest:
   command==="list_local_materials"?[{id:"pack-opaque",kind:"authored_pack"},{id:"guide-opaque",kind:"guide"},{id:"cache-opaque",kind:"hotfix_cache"}]:
   command==="build_local_inventory"?inventory:
   command==="get_local_inventory_summary"?{revision:inventory,acceptance_ready:false,domains:[{domain:"quests",imported_count:2,source_available:true,independently_required_count:null,independently_complete:false,gaps:[]}]}:
   command==="get_local_inventory_page"?{revision:inventory,domain:"quests",offset:(input as {offset:number}).offset,total:2,next_offset:(input as {offset:number}).offset===0?1:null,source_available:true,independently_complete:false,records:[{id:(input as {offset:number}).offset===0?"synthetic-first":"synthetic-second",kind:"synthetic",source_sha256:catalog}]}:null;
  return Promise.resolve(value as never);
 }};
 render(<ResearchFactsTools client={createClient(adapter)} revision={revision} catalog={catalog} preset="small"/>);
 fireEvent.click(await screen.findByRole("button",{name:"Review saved guide and pack selections"}));
 fireEvent.click(await screen.findByRole("button",{name:"Use material pack-opa"}));
 expect(screen.queryByText(/cache-opaque/)).toBeNull();
 fireEvent.click(screen.getByRole("button",{name:"Build exact nine-domain inventory"}));
 fireEvent.click(await screen.findByRole("button",{name:"Inspect exact record IDs"}));
 await screen.findByText("synthetic-first");fireEvent.click(screen.getByRole("button",{name:"Next exact ID page"}));
 await screen.findByText("synthetic-second");expect(screen.queryByText("synthetic-first")).toBeNull();
 expect(calls).toContainEqual({command:"get_local_inventory_page",input:{revision:inventory,domain:"quests",offset:1,limit:200,authored:"pack-opaque"}});
 fireEvent.click(screen.getByRole("button",{name:"Forget selected authored pack"}));
 await screen.findByText(/immutable imported revisions remain intact/);
 expect(screen.queryByText("synthetic-second")).toBeNull();
 fireEvent.click(screen.getByRole("button",{name:"Use material guide-op"}));
 fireEvent.click(screen.getByRole("button",{name:"Forget selected guide"}));
 await vi.waitFor(()=>expect(screen.queryByRole("button",{name:"Forget selected guide"})).toBeNull());
 expect(calls.filter(call=>call.command==="forget_local_material").map(call=>call.input)).toEqual([{id:"pack-opaque"},{id:"guide-opaque"}]);
});
