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
 await page.getByRole("button",{name:"Settings",exact:true}).click();await expect(page.getByRole("heading",{name:"Settings",exact:true})).toBeVisible();await page.getByLabel("Chosen game installation path", { exact: true }).fill("/local/explicit/game");await page.getByLabel("Chosen snapshot export file", { exact: true }).fill("/local/explicit/snapshot.txt");await page.getByRole("combobox",{name:"Contrast",exact:true}).selectOption("high");await page.getByRole("combobox",{name:"Motion",exact:true}).selectOption("reduced");await page.getByRole("button",{name:"Save local settings"}).click();await expect(page.getByLabel("Chosen game installation path", { exact: true })).toHaveValue("/local/explicit/game");await expect(page.locator("html")).toHaveAttribute("data-contrast","high");await expect(page.locator("html")).toHaveAttribute("data-motion","reduced");
 const settings=await page.locator(".settings").evaluate(el=>({width:el.clientWidth,scroll:el.scrollWidth,overflow:getComputedStyle(el).overflowX}));expect(settings.scroll).toBeLessThanOrEqual(settings.width);expect(settings.overflow).not.toBe("hidden");
 await page.getByLabel("Chosen game installation path", { exact: true }).focus();await expect(page.getByLabel("Chosen game installation path", { exact: true })).toBeFocused();await page.keyboard.press("Tab");await expect(page.locator(":focus")).toHaveAccessibleName(/^Help/);await page.keyboard.press("Tab");await expect(page.getByLabel("Chosen snapshot export file", { exact: true })).toBeFocused();
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
test("Imports applies the saved file editor choice only after an explicit click",async({page})=>{
 await page.clock.install({time:fixture.now_utc_ms});
 await page.addInitScript(fixture=>{const calls:string[]=[];Object.defineProperty(window,"syntheticPathCalls",{get:()=>calls});(window as unknown as {__TAURI_INTERNALS__:unknown}).__TAURI_INTERNALS__={invoke:async(command:string)=>{calls.push(command);switch(command){
  case "get_app_status":return {version:"synthetic",data_mode:"stored",active_profile:fixture.profiles[0]?.id,capabilities:{}};
  case "get_character_snapshot":return fixture.snapshot;
  case "get_stored_snapshot":return {envelope:fixture.snapshot,profile_id:fixture.profiles[0]?.id,data_mode:"stored",freshness:fixture.today.members[0]?.freshness};
  case "get_app_settings":return {...fixture.settings,settings:{...fixture.settings.settings,selected_export:{path:"/explicit/local/snapshot.lua",format:{kind:"saved_variables",global:"AzerothNavigatorDB",key:"snapshot_export"}}}};
  case "get_today_view":return fixture.today;
  case "list_profiles":return fixture.profiles;
  case "get_snapshot_history":return [{snapshot_id:fixture.snapshot.payload.id,is_current:true,freshness:fixture.today.members[0]?.freshness,observed_at_utc_ms:fixture.snapshot.payload.capture.observed_at_utc_ms,received_at_utc_ms:fixture.now_utc_ms}];
  case "get_snapshot_watch":return {active:false,status:null,last_import:null,error:null};
  default:throw {code:"PROVIDER_UNAVAILABLE",message:"Synthetic command unavailable",retryable:false};
 }}};},fixture);
 await page.goto("/");await page.getByRole("button",{name:"Imports",exact:true}).click();await expect(page.getByLabel("Export file path", { exact: true })).toHaveValue("");await page.getByRole("button",{name:"Use saved export-file choice"}).click();await expect(page.getByLabel("Export file path", { exact: true })).toHaveValue("/explicit/local/snapshot.lua");await expect(page.getByLabel("SavedVariables global", { exact: true })).toHaveValue("AzerothNavigatorDB");await expect(page.getByLabel("Snapshot export key", { exact: true })).toHaveValue("snapshot_export");await expect(page.getByText("File watching is off.",{exact:true})).toBeVisible();const calls=await page.evaluate(()=>(window as unknown as {syntheticPathCalls:string[]}).syntheticPathCalls);expect(calls).not.toContain("import_snapshot_file");expect(calls).not.toContain("start_snapshot_watch");expect(calls).not.toContain("set_reference_window");
});
