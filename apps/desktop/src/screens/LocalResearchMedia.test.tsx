import {fireEvent,render,screen} from "@testing-library/react";
import {createClient,type CommandAdapter} from "../commands";
import {LocalResearchMedia} from "./LocalResearchMedia";
const catalog="a".repeat(64),revision="b".repeat(64),id="c".repeat(64);
it("extracts explicitly selected IDs and presents opaque previews alongside unsupported assets",async()=>{
 const calls:{command:string;input:unknown}[]=[];
 const adapter:CommandAdapter={invoke(command,input){calls.push({command,input});return Promise.resolve((command==="discover_local_media"?{revision:"discovery",catalog_sha256:catalog,locale:"enUS",total:1,offset:0,next_offset:null,gaps:["Map:unsupported_layout"],candidates:[{table:"Item",row:1,field:"IconFileDataID",file_data_id:7}]}:command==="extract_local_media"?revision:command==="get_local_media_manifest"?{format_version:1,catalog_sha256:catalog,locale:"enUS",fixture:true,local_only:true,calibrated:false,verified_for_gameplay:false,assets:[{file_data_id:7,disposition:"decoded",read_refusal:null},{file_data_id:8,disposition:"unsupported",read_refusal:"unsupported"}]}:{revision,offset:0,total:1,next_offset:null,assets:[{id,width:8,height:4,url:`atlas://localhost/research/${revision}/${id}`}]})as never);}};
 render(<LocalResearchMedia client={createClient(adapter)} catalog={catalog} selectionId="selected" preset="small"/>);
 expect(screen.getByRole("button",{name:"Discover private media candidates"})).toBeDisabled();
 fireEvent.change(screen.getByRole("textbox",{name:"Media content locale"}),{target:{value:"enUS"}});fireEvent.click(screen.getByRole("button",{name:"Discover private media candidates"}));
 const checkbox=await screen.findByRole("checkbox",{name:/IconFileDataID: 7/});expect(screen.getByRole("button",{name:"Extract selected private media"})).toBeDisabled();fireEvent.click(checkbox);fireEvent.click(screen.getByRole("button",{name:"Extract selected private media"}));
 expect(await screen.findByRole("img")).toHaveAttribute("src",`atlas://localhost/research/${revision}/${id}`);
 expect(screen.getByRole("row",{name:"8 unsupported unsupported"})).toBeInTheDocument();
 expect(calls.find(call=>call.command==="extract_local_media")?.input).toEqual({selection_id:"selected",catalog,locale:"enUS",ids:[7],preset:"small"});
});
it("discards discoveries whose locale differs and does not enable extraction",async()=>{
 const adapter:CommandAdapter={invoke(){return Promise.resolve({catalog_sha256:catalog,locale:"deDE",offset:0,candidates:[]}as never);}};
 render(<LocalResearchMedia client={createClient(adapter)} catalog={catalog} selectionId="selected" preset="max"/>);
 fireEvent.change(screen.getByRole("textbox",{name:"Media content locale"}),{target:{value:"enUS"}});fireEvent.click(screen.getByRole("button",{name:"Discover private media candidates"}));
 await screen.findByText(/Media discovery changed or is unavailable/);expect(screen.queryByRole("button",{name:"Extract selected private media"})).not.toBeInTheDocument();
});
it("binds decoded pagination to its revision and refuses unsafe preview references",async()=>{
 const calls:{command:string;input:unknown}[]=[];
 let stale=false;
 const adapter:CommandAdapter={invoke(command,input){
  calls.push({command,input});
  if(command==="discover_local_media")return Promise.resolve({revision:"discovery",catalog_sha256:catalog,locale:"enUS",total:1,offset:0,next_offset:null,gaps:[],candidates:[{table:"Item",row:1,field:"IconFileDataID",file_data_id:7}]}as never);
  if(command==="extract_local_media")return Promise.resolve(revision as never);
  if(command==="get_local_media_manifest")return Promise.resolve({format_version:1,catalog_sha256:catalog,locale:"enUS",fixture:true,local_only:true,calibrated:false,verified_for_gameplay:false,assets:[]}as never);
  const offset=(input as {offset:number}).offset;
  return Promise.resolve({revision:stale?"d".repeat(64):revision,offset,total:3,next_offset:offset===0?1:2,assets:[{id,width:640,height:320,url:offset===0?`atlas://localhost/research/${revision}/${id}`:"https://remote.invalid/private.png"}]}as never);
 }};
 render(<LocalResearchMedia client={createClient(adapter)} catalog={catalog} selectionId="selected" preset="small"/>);
 fireEvent.change(screen.getByRole("textbox",{name:"Media content locale"}),{target:{value:"enUS"}});
 fireEvent.click(screen.getByRole("button",{name:"Discover private media candidates"}));
 fireEvent.click(await screen.findByRole("checkbox"));
 fireEvent.click(screen.getByRole("button",{name:"Extract selected private media"}));
 const image=await screen.findByRole("img");expect(image).toHaveAttribute("width","320");
 fireEvent.error(image);await screen.findByText(/Private image unavailable/);
 fireEvent.click(screen.getByRole("button",{name:"Next decoded media page"}));
 await screen.findByText("Invalid private asset reference.");expect(screen.queryByRole("img")).toBeNull();
 expect(calls).toContainEqual({command:"get_local_media_page",input:{revision,offset:1,limit:200}});
 stale=true;fireEvent.click(screen.getByRole("button",{name:"Next decoded media page"}));
 await screen.findByText("Media page is stale or unavailable.");
 expect(screen.queryByRole("img")).toBeNull();
});
