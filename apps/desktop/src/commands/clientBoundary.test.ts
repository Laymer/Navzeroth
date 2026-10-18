import {createClient,type CommandAdapter} from "./index";

function transport(){
 const calls:{command:string;input:unknown}[]=[];
 const adapter:CommandAdapter={invoke(command,input){calls.push({command,input});return Promise.reject({code:"INVALID_INPUT",message:"Stale selection",retryable:false});}};
 return {calls,client:createClient(adapter)};
}
it("keeps extraction job handles opaque and returns native refusal without replacing it",async()=>{
 const {client,calls}=transport();
 expect(await client.getLocalDataJob("job-selected")).toEqual({ok:false,error:{code:"INVALID_INPUT",message:"Stale selection",retryable:false}});
 await client.forgetLocalDataJob("job-selected");
 expect(calls).toEqual([{command:"get_local_data_job",input:{job_id:"job-selected"}},{command:"forget_local_data_job",input:{job_id:"job-selected"}}]);
});
it("preserves false window choices and explicit interaction mode across the native boundary",async()=>{
 const {client,calls}=transport();
 await client.toggleReferenceWindow(false,"interactive");
 await client.setReferenceInteraction("interactive");
 await client.setReferenceWindow(false,false);
 expect(calls).toEqual([
  {command:"toggle_reference_window",input:{always_on_top:false,interaction_mode:"interactive"}},
  {command:"set_reference_interaction",input:{interaction_mode:"interactive"}},
  {command:"set_reference_window",input:{visible:false,always_on_top:false}},
 ]);
});
it("passes exact selected paths and backup handles, without substituting a default source",async()=>{
 const {client,calls}=transport();
 await client.createDatabaseBackup("/selected/synthetic.sqlite3");
 await client.inspectDatabaseBackup("/selected/synthetic.sqlite3");
 await client.restoreDatabaseBackup("opaque-reviewed-candidate");
 await client.exportDiagnostics("/selected/diagnostics.json");
 await client.importSessionFile("opaque-profile","/selected/sessions.lua",{kind:"saved_variables",global:"UserDB",key:"sessions"});
 expect(calls).toEqual([
  {command:"create_database_backup",input:{path:"/selected/synthetic.sqlite3"}},
  {command:"inspect_database_backup",input:{path:"/selected/synthetic.sqlite3"}},
  {command:"restore_database_backup",input:{candidate_id:"opaque-reviewed-candidate"}},
  {command:"export_diagnostics",input:{path:"/selected/diagnostics.json"}},
  {command:"import_session_file",input:{profile_id:"opaque-profile",path:"/selected/sessions.lua",format:{kind:"saved_variables",global:"UserDB",key:"sessions"}}},
 ]);
});
