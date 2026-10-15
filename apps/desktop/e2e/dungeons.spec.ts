import { expect, test } from "@playwright/test";
import snapshot from "../../../fixtures/contracts/v1/character-snapshot.json" with { type: "json" };
import report from "../../../fixtures/desktop/preparation/report.json" with { type: "json" };
test("local dungeon reference keeps stale members and unknown source evidence explicit",async({page})=>{
  await page.addInitScript(({snapshot,report})=>{
    let reportCalls=0;
    const nativeWindow=window as unknown as {__TAURI_INTERNALS__:{invoke:(command:string,input:unknown)=>Promise<unknown>}};
    nativeWindow.__TAURI_INTERNALS__={invoke:async(command,input)=>{
      switch(command){
        case "get_authored_catalog":return {revision:null,records:[],total:0,next_offset:null};
        case "get_app_status":return {version:"synthetic",data_mode:"stored",active_profile:report.members[0]?.profile_id,capabilities:{}};
        case "get_character_snapshot":return snapshot;
        case "get_stored_snapshot":return {envelope:snapshot,freshness:{status:"fresh",age_ms:100},profile_id:report.members[0]?.profile_id,data_mode:"stored"};
        case "list_profiles":return report.members.map(m=>({id:m.profile_id,display_name:m.character,data_mode:"stored"}));
        case "list_parties":return ["preparation-party"];
        case "get_dungeon_preparation":{const request=JSON.parse((input as {text:string}).text);if(request.subject.kind!=="party")throw Error("Expected explicit local party");reportCalls++;return report;}
        default:throw {code:"PROVIDER_UNAVAILABLE",message:"Synthetic command unavailable",retryable:false};
      }
    }};
    Object.defineProperty(window,"syntheticPreparationCalls",{get:()=>reportCalls});
  },{snapshot,report});
  await page.setViewportSize({width:320,height:900});await page.goto("/");
  await page.getByRole("button",{name:"Dungeons",exact:true}).click();
  await expect(page.getByRole("heading",{name:"Dungeon preparation",exact:true})).toBeVisible();
  await expect(page.getByText("No authored records on this page.",{exact:false})).toBeVisible();
  await expect(page.getByLabel("Local preparation package JSON")).not.toBeVisible();
  await page.getByLabel("Preparation members").selectOption("party");
  await page.getByText("Advanced local reference editor",{exact:true}).click();
  await page.getByRole("button",{name:"Load synthetic preparation reference"}).click();
  await page.getByRole("button",{name:"Inspect preparation gaps"}).click();
  await expect(page.getByText(/Declared group formation: 0 to 0 ms/)).toBeVisible();
  await expect(page.getByText("member-1 — capture stale",{exact:true})).toBeVisible();
  await page.getByText("member-0 — capture fresh",{exact:true}).click();
  await expect(page.getByText(/Synthetic declared equipment condition: known; condition false/)).toBeVisible();
  await expect(page.getByText(/limited_sample; source demo-source-1/)).toBeVisible();
  await page.getByRole("checkbox",{name:/Review missing or stale participant/}).check();
  await expect(page.getByRole("checkbox",{name:/Review missing or stale participant/})).toBeChecked();
  expect(await page.evaluate(()=>(window as unknown as {syntheticPreparationCalls:number}).syntheticPreparationCalls)).toBe(1);
  const width=await page.locator(".dungeon-preparation").evaluate(e=>({width:e.clientWidth,scroll:e.scrollWidth}));expect(width.scroll).toBeLessThanOrEqual(width.width);
});
