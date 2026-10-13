import { test, expect } from "@playwright/test";
import catalog from "../../../fixtures/desktop/gear/catalog.json" with { type: "json" };
import report from "../../../fixtures/desktop/gear/report.json" with { type: "json" };

test("gear manual assumptions display actual Rust fixture explanations and invalidate results", async ({ page }) => {
  await page.addInitScript(({ catalog, report }) => {
    Object.defineProperty(window, "__TAURI_INTERNALS__", { value: { invoke: async (command: string, args: Record<string,unknown>) => {
      switch(command) {
        case "get_app_status": return { version:"mock",data_mode:"stored",active_profile:null,capabilities:{} };
        case "get_snapshot_watch": return {active:false,last_import:null,error:null,status:null};
        case "list_profiles": return [{id:report.request.profile_id,display_name:"Synthetic gear profile",data_mode:"stored",fixture:true}];
        case "get_gear_catalog": return catalog;
        case "compare_gear": return {...report,request:JSON.parse(String(args.text))};
        default: throw {code:"INVALID_INPUT",message:"Unexpected gear command mock",retryable:false};
      }
    } } });
  }, {catalog,report});
  await page.goto("/");
  await page.getByRole("button",{name:"Gear",exact:true}).click();
  await expect(page.getByText("SYNTHETIC — engineering fixture, not live game advice.")).toBeVisible();
  await page.getByRole("button",{name:"Compare gear",exact:true}).click();
  await expect(page.getByRole("alert")).toContainText("explicitly supply weights");
  await page.getByRole("button",{name:"Load labelled synthetic example"}).click();
  await expect(page.getByText(/Engineering example:/)).toBeVisible();
  await page.getByRole("button",{name:"Compare gear",exact:true}).click();
  const result=page.getByRole("region",{name:"Gear result"});
  await expect(result).toBeVisible();
  await expect(result.getByRole("heading",{name:"Theoretical comparison"})).toBeVisible();
  await expect(result.getByRole("heading",{name:"Guaranteed individual acquisition"})).toBeVisible();
  await expect(result.getByText(/synthetic-b: Conditional estimate/)).toBeVisible();
  await expect(result.getByText(/synthetic-a: Guaranteed acquisition/)).toBeVisible();
  await expect(result.getByText(/No combined acquisition probabilities or useful lifespan/)).toBeVisible();
  await page.getByLabel("Attribute weights (JSON, integer thousandths)").fill('{"synthetic-power":2000}');
  await expect(result).toHaveCount(0);
});
