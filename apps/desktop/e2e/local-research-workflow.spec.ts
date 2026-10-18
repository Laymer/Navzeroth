import {expect,test} from "@playwright/test";
import AxeBuilder from "@axe-core/playwright";
import {installMock} from "./support/mockInvoke";
import type {LocalWorkflowRecord} from "../src/commands/localData";
for(const width of [320,1100])test(`synthetic local choices stay explicit, accessible and bounded at ${width}px`,async({page})=>{
 const id="a".repeat(64);let record:LocalWorkflowRecord={format_version:1,revision:0,preferences:{format_version:1,first_run_completed:false,startup_mode:"later",selections:{}}};
 const {calls}=await installMock(page,{
  get_app_status:()=>({version:"synthetic-development-check",data_mode:"stored",active_profile:null,capabilities:{}}),
  get_local_workflow:()=>record,
  finish_local_startup:input=>(record={...record,revision:record.revision+1,preferences:{...record.preferences,first_run_completed:input.completed,startup_mode:input.mode}}),
  list_profiles:()=>[],get_snapshot_watch:()=>({active:false,status:"cancelled",last_import:null,error:null}),
  select_local_installation:()=>({selection_id:id,preset:"small",inspection:{format_version:1,metadata:{product:"wow_classic_beta",version:"1.60.1.70450",build:70450,declared_locales:["deDE","enUS"]},build_info_sha256:"b".repeat(64),consistency:"unverified",limitations:[]}}),
  save_local_workflow_choices:input=>(record={...record,revision:record.revision+1,preferences:{...record.preferences,selections:{[id]:{locales:input.locales,preset:input.preset,change_detection:input.detect,baseline_sha256:input.detect?"b".repeat(64):null}}}}),
  get_local_change_detectors:()=>[{format_version:1,selection_id:id,status:"staged",candidate_sha256:"c".repeat(64),auto_activation:false}],
 });
 await page.setViewportSize({width,height:1000});await page.goto("/");
 await page.getByRole("button",{name:"Begin with local research content"}).click();
 await page.getByRole("button",{name:"Select game installation"}).click();
 await page.getByRole("checkbox",{name:"enUS",exact:true}).check();
 await page.getByRole("checkbox",{name:"Watch selected build metadata while the app is open"}).check();
 await page.getByRole("button",{name:"Save content choices"}).click();await expect(page.getByText(/Choices saved/)).toBeVisible();
 await page.getByRole("button",{name:"Review detected build changes"}).click();await expect(page.getByText(/Staged metadata/)).toBeVisible();
 expect(calls).not.toContain("start_local_data");expect(calls).not.toContain("activate_local_data");
 expect(record.preferences.selections[id]?.locales).toEqual(["enUS"]);
 await page.addStyleTag({content:":root {font-size:200%}"});
 const sizes=await page.evaluate(()=>({scroll:document.documentElement.scrollWidth,width:document.documentElement.clientWidth}));expect(sizes.scroll).toBeLessThanOrEqual(sizes.width);
 const result=await new AxeBuilder({page}).include("main").analyze();expect(result.violations).toEqual([]);
 await page.getByRole("button",{name:"Review detected build changes"}).focus();await expect(page.getByRole("button",{name:"Review detected build changes"})).toBeFocused();await page.keyboard.press("Tab");await expect(page.getByRole("button",{name:"Acknowledge staged build metadata"})).toBeFocused();
});
