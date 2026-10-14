import {expect,test} from "@playwright/test";
import {readFileSync} from "node:fs";
const report=JSON.parse(readFileSync(new URL("../../../fixtures/desktop/reference/imported.json",import.meta.url),"utf8"));
test("fixed reference page renders only imported facts with separate ages and reflows",async({page})=>{
 await page.addInitScript((report)=>{
  const calls:string[]=[];
  Object.defineProperty(window,"__REFERENCE_CALLS__",{value:calls});
  Object.defineProperty(window,"__TAURI_INTERNALS__",{value:{invoke:async(command:string)=>{
   calls.push(command);if(command==="get_reference_view")return report;
   throw {code:"INVALID_INPUT",message:"Unexpected mock command",retryable:false};
  }}});
 },report);
 await page.setViewportSize({width:320,height:900});await page.goto("/#reference");
 await expect(page.getByRole("heading",{name:"Imported reference",exact:true})).toBeVisible();
 await expect(page.locator(".reference-view")).toContainText("captured 0 s ago");
 await expect(page.locator(".reference-view")).toContainText("stale");
 await expect(page.locator(".reference-view")).toContainText("Unknown (not observed)");
 await expect(page.locator(".reference-view")).toContainText("Restricted (hidden by the game)");
 await expect(page.locator("nav")).toHaveCount(0);
 await expect(page.getByRole("button")).toHaveCount(1);
 for(const fontSize of ["100%","200%"]){await page.evaluate(size=>{document.documentElement.style.fontSize=size;},fontSize);
  expect(await page.locator(".reference-view").evaluate(el=>el.getBoundingClientRect().right<=window.innerWidth+1)).toBe(true);
  expect(await page.evaluate(()=>document.documentElement.scrollWidth<=window.innerWidth+1)).toBe(true);
 }
 const readsBefore=await page.evaluate(()=>(window as unknown as {__REFERENCE_CALLS__:string[]}).__REFERENCE_CALLS__.length);
 await page.getByRole("button",{name:"Refresh imported reference"}).click();
 await expect.poll(()=>page.evaluate(()=>(window as unknown as {__REFERENCE_CALLS__:string[]}).__REFERENCE_CALLS__.length)).toBe(readsBefore+1);
 expect((await page.evaluate(()=>(window as unknown as {__REFERENCE_CALLS__:string[]}).__REFERENCE_CALLS__)).every(command=>command==="get_reference_view")).toBe(true);
});
test("main reference controls require explicit show/topmost and retain fallback after native failure",async({page})=>{
 await page.addInitScript((report)=>{
  const calls:{command:string;args:Record<string,unknown>}[]=[];let shows=0;
  Object.defineProperty(window,"__REFERENCE_CALLS__",{value:calls});
  Object.defineProperty(window,"__TAURI_INTERNALS__",{value:{invoke:async(command:string,args:Record<string,unknown>)=>{
   calls.push({command,args});switch(command){
    case "get_app_status":return {version:"test",data_mode:"stored",active_profile:null,capabilities:{}};
    case "get_snapshot_watch":return {active:false,status:null,last_import:null,error:null};
    case "get_reference_window_state":return {schema_version:1,exists:false,visible:false,always_on_top:false};
    case "get_reference_view":return report;
    case "toggle_reference_window":if(++shows===2)throw {code:"PROVIDER_UNAVAILABLE",message:"private-native-path",retryable:false};return {schema_version:1,requested_visible:true,requested_interaction:args.interaction_mode,dispatch:"accepted",reason:null,operations:{creation:"accepted",focusability:"accepted",pointer_events:"accepted",topmost:"accepted",visibility:"accepted",state_query:"accepted"},window:{schema_version:1,exists:true,visible:true,always_on_top:args.always_on_top}};
    case "set_reference_window":return {schema_version:1,exists:true,visible:args.visible,always_on_top:args.always_on_top};
    default:throw {code:"INVALID_INPUT",message:"Unexpected mock command",retryable:false};
   }
  }}});
 },report);
 await page.goto("/");await page.getByText("Optional reference window",{exact:true}).click();
 const calls=()=>page.evaluate(()=>(window as unknown as {__REFERENCE_CALLS__:{command:string;args:Record<string,unknown>}[]}).__REFERENCE_CALLS__);
 await expect(page.getByText("Last observed window: closed; keep on top: off. Hide turns keep-on-top off.")).toBeVisible();
 expect((await calls()).filter(c=>c.command==="set_reference_window")).toEqual([]);
 await page.getByRole("checkbox",{name:"Keep on top when shown (explicit preference)"}).check();
 await page.getByRole("button",{name:"Show ordinary reference",exact:true}).click();
 await expect(page.getByText("Last observed window: shown; keep on top: on. Hide turns keep-on-top off.")).toBeVisible();
 await page.getByRole("button",{name:"Hide ordinary reference",exact:true}).click();
 await expect(page.getByRole("checkbox",{name:"Keep on top when shown (explicit preference)"})).not.toBeChecked();
 await page.getByRole("button",{name:"Show ordinary reference",exact:true}).click();
 await expect(page.getByText(/Reference window unavailable: PROVIDER_UNAVAILABLE/)).toBeVisible();
 await page.getByRole("button",{name:"Open normal desktop reference"}).click();
 await expect(page.getByRole("heading",{name:"Imported reference",exact:true})).toBeVisible();
 await expect(page.locator("body")).not.toContainText("private-native-path");
 expect((await calls()).filter(c=>c.command==="set_reference_window").map(c=>c.args)).toEqual([{visible:false,always_on_top:false}]);
 expect((await calls()).filter(c=>c.command==="toggle_reference_window").map(c=>c.args)).toEqual([{always_on_top:true,interaction_mode:"interactive"},{always_on_top:false,interaction_mode:"interactive"}]);
});
test("focused desktop shortcut is explicitly enabled and interaction dispatch failures stay truthful",async({page})=>{
 await page.addInitScript(()=>{let exists=false,visible=false,toggles=0;const calls:{command:string;args:Record<string,unknown>}[]=[];Object.defineProperty(window,"__REFERENCE_CALLS__",{value:calls});const state=()=>({schema_version:1,exists,visible,always_on_top:false});Object.defineProperty(window,"__TAURI_INTERNALS__",{value:{invoke:async(command:string,args:Record<string,unknown>)=>{calls.push({command,args});switch(command){case "get_app_status":return {version:"test",data_mode:"stored",active_profile:null,capabilities:{}};case "get_snapshot_watch":return {active:false,status:null,last_import:null,error:null};case "get_reference_window_state":return state();case "toggle_reference_window":exists=true;visible=!visible;toggles++;return {schema_version:1,requested_visible:visible,requested_interaction:visible?args.interaction_mode:"interactive",dispatch:"accepted",reason:null,operations:{creation:toggles===1?"accepted":"not_attempted",focusability:"accepted",pointer_events:"accepted",topmost:"accepted",visibility:"accepted",state_query:"accepted"},window:state()};case "set_reference_interaction":return {schema_version:1,requested_visible:null,requested_interaction:args.interaction_mode,dispatch:"partial_failure",reason:"pointer_failed",operations:{creation:"not_attempted",focusability:"accepted",pointer_events:"failed",topmost:"not_attempted",visibility:"not_attempted",state_query:"accepted"},window:state()};default:throw {code:"INVALID_INPUT",message:"Unexpected mock command",retryable:false};}}}});});
 await page.setViewportSize({width:320,height:1000});await page.goto("/");await page.getByText("Optional reference window",{exact:true}).click();await expect(page.getByText(/Last observed window: closed/)).toBeVisible();
 const toggles=()=>page.evaluate(()=>(window as unknown as {__REFERENCE_CALLS__:{command:string}[]}).__REFERENCE_CALLS__.filter(c=>c.command==="toggle_reference_window").length);
 await page.keyboard.press("Control+Shift+F8");expect(await toggles()).toBe(0);
 await page.getByRole("checkbox",{name:/Enable Ctrl/}).check();await page.getByLabel("Requested reference interaction").selectOption("pass_through");await page.getByLabel("Requested reference interaction").focus();await expect(page.getByLabel("Requested reference interaction")).toBeFocused();await page.keyboard.press("Control+Shift+F8");expect(await toggles()).toBe(0);
 await page.getByRole("heading",{name:"Ordinary reference window",exact:true}).click();await page.keyboard.press("Control+Shift+F8");await expect(page.getByText(/Last observed window: shown/)).toBeVisible();expect(await toggles()).toBe(1);
 await page.getByRole("button",{name:"Apply requested interaction"}).click();await expect(page.getByText(/dispatch partial failure.*pointer failed/)).toBeVisible();await expect(page.getByText(/Last observed window: shown/)).toBeVisible();await expect(page.getByText(/This does not verify native interaction behavior/)).toBeVisible();
 await page.getByRole("heading",{name:"Ordinary reference window",exact:true}).click();await page.keyboard.press("Control+Shift+F8");await expect(page.getByText(/Last observed window: hidden/)).toBeVisible();expect(await toggles()).toBe(2);
 for(const size of ["100%","200%"]){await page.evaluate(size=>document.documentElement.style.fontSize=size,size);expect(await page.locator(".reference-controls").evaluate(el=>el.scrollWidth<=el.clientWidth+1&&[el,...el.querySelectorAll("*")].every(child=>child.getBoundingClientRect().right<=window.innerWidth+1))).toBe(true);}
});
