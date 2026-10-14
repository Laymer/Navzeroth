import {test,expect} from "@playwright/test";
import fixture from "../../../fixtures/desktop/today/synthetic-view.json" with {type:"json"};
for(const scale of [100,200])test(`Today and explicit Settings remain readable at 320px and ${scale}% text`,async({page})=>{
 await page.clock.install({time:fixture.now_utc_ms});
 await page.addInitScript(({fixture,scale})=>{
  let settings=structuredClone(fixture.settings);settings.settings.accessibility.text_scale_percent=scale;
  const calls:string[]=[];Object.defineProperty(window,"syntheticTodayCalls",{get:()=>calls});
  (window as unknown as {__TAURI_INTERNALS__:unknown}).__TAURI_INTERNALS__={invoke:async(command:string,input:unknown)=>{
   calls.push(command);switch(command){
    case "get_app_status":return {version:"synthetic",data_mode:"stored",active_profile:fixture.profiles[0]?.id,capabilities:{}};
    case "get_character_snapshot":return fixture.snapshot;
    case "get_stored_snapshot":return {envelope:fixture.snapshot,profile_id:fixture.profiles[0]?.id,data_mode:"stored",freshness:fixture.today.members[0]?.freshness};
    case "get_app_settings":return settings;
    case "get_today_view":return fixture.today;
    case "list_profiles":return fixture.profiles;
    case "list_parties":return [];
    case "update_app_settings":{const request=JSON.parse((input as {text:string}).text);if(request.expected_revision!==settings.revision)throw {code:"INVALID_INPUT",message:"Stale settings",retryable:false};settings={revision:settings.revision+1,settings:request.settings};return settings;}
    default:throw {code:"PROVIDER_UNAVAILABLE",message:"Synthetic command unavailable",retryable:false};
   }
  }};
 },{fixture,scale});
 await page.setViewportSize({width:320,height:900});await page.goto("/");
 await expect(page.getByText("Next action: synthetic-service",{exact:true})).toBeVisible();await expect(page.getByText(/SYNTHETIC — fixture plan/)).toBeVisible();
 const today=page.locator(".today-screen");const first=await today.evaluate(el=>({width:el.clientWidth,scroll:el.scrollWidth}));expect(first.scroll).toBeLessThanOrEqual(first.width);
 await page.getByRole("button",{name:"Settings",exact:true}).click();await expect(page.getByRole("heading",{name:"Settings",exact:true})).toBeVisible();await page.getByLabel("Chosen game installation path").fill("/local/explicit/game");await page.getByLabel("Chosen snapshot export file").fill("/local/explicit/snapshot.txt");await page.getByLabel("Contrast",{exact:true}).selectOption("high");await page.getByLabel("Motion",{exact:true}).selectOption("reduced");await page.getByRole("button",{name:"Save local settings"}).click();await expect(page.getByLabel("Chosen game installation path")).toHaveValue("/local/explicit/game");await expect(page.locator("html")).toHaveAttribute("data-contrast","high");await expect(page.locator("html")).toHaveAttribute("data-motion","reduced");
 const settings=await page.locator(".settings").evaluate(el=>({width:el.clientWidth,scroll:el.scrollWidth,overflow:getComputedStyle(el).overflowX}));expect(settings.scroll).toBeLessThanOrEqual(settings.width);expect(settings.overflow).not.toBe("hidden");
 await page.getByLabel("Chosen game installation path").focus();await expect(page.getByLabel("Chosen game installation path")).toBeFocused();await page.keyboard.press("Tab");await expect(page.getByLabel("Chosen snapshot export file")).toBeFocused();
 const calls=await page.evaluate(()=>(window as unknown as {syntheticTodayCalls:string[]}).syntheticTodayCalls);expect(calls).toContain("update_app_settings");expect(calls).not.toContain("start_snapshot_watch");expect(calls).not.toContain("import_snapshot_file");expect(calls).not.toContain("start_planning_job");expect(calls).not.toContain("set_reference_window");
});
test("historical advice keeps original steps without a next action",async({page})=>{
 await page.addInitScript(fixture=>{(window as unknown as {__TAURI_INTERNALS__:unknown}).__TAURI_INTERNALS__={invoke:async(command:string)=>{
  if(command==="get_app_status")return {version:"synthetic",data_mode:"stored",active_profile:null,capabilities:{}};
  if(command==="get_app_settings")return fixture.settings;
  if(command==="get_today_view")return {...fixture.today,current_plan:{...fixture.today.current_plan,state:"historical",reason:"dependencies_or_freshness_changed"}};
  throw {code:"PROVIDER_UNAVAILABLE",message:"Synthetic unavailable",retryable:false};
 }};},fixture);
 await page.goto("/");await expect(page.getByText(/Historical saved advice/)).toBeVisible();await expect(page.getByText(/Next action:/)).toHaveCount(0);await page.getByText("Historical original steps",{exact:true}).click();await expect(page.getByText(/synthetic-service at demo-node-2/)).toBeVisible();
});
