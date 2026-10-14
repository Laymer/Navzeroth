import { expect, test } from "@playwright/test";
test("local diagnostics require explicit preview/export and expose only fixed failure codes", async ({ page }) => {
  await page.addInitScript(() => {
    const calls: { command: string; args: Record<string, unknown> | undefined }[] = [];
    Object.defineProperty(window, "__DIAGNOSTIC_CALLS__", { value: calls });
    const report = {format:"azeroth-local-diagnostics",schema_version:1,versions:{app:"test-package",bundled_addon:"test-package",installed_addon:null},os:"windows",game:null,capabilities:{character_level:{availability:"unknown",check:null,verified_available:null,freshness:null}},historical_user_report:{build:70338,interface:16001,source:"limited_user_report",fields:["level","clock"],installed_revision_match:null},snapshot_availability:{},snapshot_freshness:null,field_freshness:{},dataset:null,dataset_context_mismatch:false,telemetry:{scope:"current_process_collector",event_limit:64,truncated:false,import_attempts:0,import_succeeded:0,import_failed:0,events:[]}};
    let exports = 0;
    Object.defineProperty(window, "__TAURI_INTERNALS__", { value: { invoke: async (command: string, args: Record<string, unknown>) => {
      calls.push({command,args});
      switch(command) {
        case "get_app_status": return {version:"test",data_mode:"stored",active_profile:null,capabilities:{}};
        case "get_snapshot_watch": return {active:false,status:null,last_import:null,error:null};
        case "preview_diagnostics": return report;
        case "export_diagnostics": if (++exports === 1) return {schema_version:1,bytes:123}; throw {code:"STORAGE_ERROR",message:"private-secret-account-path",retryable:false};
        default: throw {code:"INVALID_INPUT",message:"Unexpected mocked command",retryable:false};
      }
    } } });
  });
  await page.goto("/");
  await page.getByRole("button", {name:"Local diagnostics",exact:true}).click();
  await expect(page.getByRole("heading", {name:"Local diagnostics",exact:true})).toBeVisible();
  await expect(page.getByRole("button", {name:"Create local diagnostic file"})).toBeDisabled();
  const calls = () => page.evaluate(() => (window as unknown as {__DIAGNOSTIC_CALLS__: {command:string;args:Record<string,unknown>}[]}).__DIAGNOSTIC_CALLS__);
  expect((await calls()).filter(c=>c.command.includes("diagnostics"))).toEqual([]);
  await page.getByRole("button", {name:"Preview diagnostic metadata"}).click();
  await expect(page.getByText("Recorded imports: 0.")).toBeVisible();
  expect((await calls()).filter(c=>c.command==="export_diagnostics")).toEqual([]);
  await page.getByLabel("New diagnostic file path").fill("selected-new-diagnostics.json");
  await page.getByRole("button", {name:"Create local diagnostic file"}).click();
  await expect(page.getByText("Local diagnostic file created (123 bytes).")).toBeVisible();
  await page.getByRole("button", {name:"Create local diagnostic file"}).click();
  await expect(page.getByText("Export failed: STORAGE_ERROR.")).toBeVisible();
  await expect(page.getByText("Recorded imports: 0.")).toBeVisible();
  await expect(page.locator("body")).not.toContainText("private-secret");
  expect((await calls()).filter(c=>c.command==="export_diagnostics").map(c=>c.args)).toEqual([{path:"selected-new-diagnostics.json"},{path:"selected-new-diagnostics.json"}]);
});
