import {expect,test} from "@playwright/test";
import snapshot from "../../../fixtures/contracts/v1/character-snapshot.json" with {type:"json"};
/** Mocked Chromium command-boundary exercise; not native or live acceptance. */
test("imported synthetic profile reminder hide restore acknowledgment and checklist",async({page})=>{
 await page.addInitScript(({snapshot})=>{
  let imported=false;let hidden=false;let read=false;let bag=false;let checked=false;
  const reminder=()=>({profile_id:"profile",snapshot_id:snapshot.payload.id,revision:"synthetic",fixture:true,enabled:true,bag_checklist_enabled:bag,bag_checklist_checked:checked,gaps:[],dismissed:hidden?["r"]:[],rules:[{id:"r",text:"Synthetic reminder",dismissal:"permanent",delivery:"synthetic"}],plan:{mode:"non_intrusive",queue:hidden||read?[]:[{rule:"r",priority:1,dismissal:"permanent",kind:{kind:"advice",text:"Synthetic reminder"},evidence:[]}],held:[],excluded:{}}});
  Object.defineProperty(window,"__TAURI_INTERNALS__",{value:{invoke:async(command:string,args:Record<string,unknown>)=>{
   switch(command){
    case "get_app_status":return {version:"mock",data_mode:"stored",active_profile:imported?"profile":null,capabilities:{}};
    case "list_profiles":return imported?[{id:"profile",display_name:"Synthetic imported character",fixture:true,context:snapshot.payload.context,snapshot_count:1,data_mode:"stored"}]:[];
    case "get_stored_snapshot":return {envelope:snapshot,profile_id:"profile",freshness:{status:"uncertain",reason:"missing_or_untrusted_time"},data_mode:"stored"};
    case "get_snapshot_watch":return {active:false,path:null,last_result:null};
    case "import_snapshot_text":imported=true;return {profile_id:"profile",snapshot_id:snapshot.payload.id,fixture:true,disposition:{outcome:"stored",previous_current:null},freshness:{status:"uncertain",reason:"missing_or_untrusted_time"},data_mode:"stored"};
    case "reminder_view":if(args.profile_id!=="profile")throw Error("Wrong profile arg");return reminder();
    case "mutate_reminders":{const req=JSON.parse(String(args.text));if(req.profile_id!=="profile"||req.snapshot_id!==snapshot.payload.id||req.revision!=="synthetic")throw Error("Wrong guards");switch(req.action.kind){case "dismiss":hidden=true;break;case "restore":hidden=false;break;case "acknowledge":read=true;break;case "bag_checklist":bag=req.action.enabled;checked=req.action.checked;break;}return reminder();}
    default:throw {code:"INVALID_INPUT",message:`Unexpected mock command ${command}`,retryable:false};
   }
  }}});
 },{snapshot});
 await page.goto("/");await page.getByRole("button",{name:"Imports",exact:true}).click();
 await page.getByLabel("Snapshot export text").fill("ANAV1:synthetic-command-mock");await page.getByRole("button",{name:"Import text",exact:true}).click();
 await page.getByRole("button",{name:"Reminders",exact:true}).click();await expect(page.getByText("Synthetic reminder")).toBeVisible();
 await page.getByRole("button",{name:"Hide r"}).click();await page.getByRole("button",{name:"Restore r"}).click();
 await page.getByRole("button",{name:"Mark r as read"}).click();await expect(page.getByText(/No new advice/)).toBeVisible();
 await page.getByLabel("Add a bag-space checklist").check();await page.getByLabel("I checked bag space manually").check();await expect(page.getByLabel("I checked bag space manually")).toBeChecked();
});
