import {createClient} from "./client";
import {demoAdapter} from "./demoAdapter";
import type {CommandName,CommandMap} from "./contract";

it.each([
 ["get_database_status",undefined],
 ["check_database",undefined],
 ["get_compatibility_report",{profile_id:null}],
 ["clear_current_plan",{expected_revision:0}],
 ["get_dungeon_preparation",{text:"{}"}],
 ["set_reference_window",{visible:false,always_on_top:false}],
 ["list_sessions",{profile_id:"demo"}],
 ["stage_content",{text:"{}"}],
 ["save_party",{text:"{}"}],
 ["import_snapshot_text",{text:"ANAV1:synthetic"}],
] as const)("browser demo refuses native workflow %s instead of fabricating observations",(command,input)=>{
 expect(()=>demoAdapter.invoke(command,input as CommandMap[typeof command]["input"])).toThrow(expect.objectContaining({code:"PROVIDER_UNAVAILABLE",retryable:false}));
});
it("keeps its one synthetic profile consistent and refuses other profile identities",async()=>{
 const client=createClient(demoAdapter);const listed=await client.listProfiles();
 if(!listed.ok||!listed.value[0])throw Error("Demo profile missing");
 const id=listed.value[0].id;
 expect(await client.selectProfile(id)).toEqual({ok:true,value:null});
 const stored=await client.getStoredSnapshot(id);const snapshot=await client.getCharacterSnapshot(id);
 if(!stored.ok||!stored.value||!snapshot.ok)throw Error("Missing synthetic snapshot");
 expect(stored.value.envelope).toEqual(snapshot.value);
 expect(stored.value.data_mode).toBe("demo");
 expect(stored.value.freshness.status).toBe("uncertain");
 expect(snapshot.value.fixture).toBe(true);
 for(const request of [client.selectProfile("other-profile"),client.getStoredSnapshot("other-profile"),client.getCharacterSnapshot("other-profile")]){
  expect(await request).toMatchObject({ok:false,error:{code:"INVALID_INPUT"}});
 }
});
it("keeps absent browser storage and inactive file watching explicit",async()=>{
 const client=createClient(demoAdapter);
 expect(await client.getWatch()).toEqual({ok:true,value:{active:false,status:null,last_import:null,error:null}});
 expect(await client.stopWatch()).toEqual({ok:true,value:{active:false,status:null,last_import:null,error:null}});
 expect(await client.getActiveContent("demo")).toEqual({ok:true,value:null});
 expect(await client.getParty("demo-party")).toEqual({ok:true,value:null});
 expect(await client.getAuthoredCatalog("{}")).toEqual({ok:true,value:{revision:null,records:[],total:0,next_offset:null}});
 expect(()=>demoAdapter.invoke("future_command" as CommandName,undefined)).toThrow(expect.objectContaining({code:"INVALID_INPUT",message:"Unknown command."}));
});
