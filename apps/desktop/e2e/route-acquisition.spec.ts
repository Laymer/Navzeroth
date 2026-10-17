import {test,expect} from "@playwright/test";
import baseline from "../../../fixtures/desktop/route-acquisition/baseline.json" with {type:"json"};
import catalog from "../../../fixtures/desktop/route-acquisition/catalog.json" with {type:"json"};
import gearCatalog from "../../../fixtures/desktop/route-acquisition/gear-catalog.json" with {type:"json"};
import stored from "../../../fixtures/desktop/route-acquisition/stored-snapshot.json" with {type:"json"};
import report from "../../../fixtures/desktop/route-acquisition/report-with-gear.json" with {type:"json"};
test("explicit Gear route opportunities use Rust fixture detour/residual budget and invalidate edited assumptions",async({page})=>{
 await page.addInitScript(({baseline,catalog,gearCatalog,stored,report})=>{
  const calls:{command:string;args:Record<string,unknown>}[]=[];
  Object.defineProperty(window,"__ROUTE_CALLS__",{value:calls});
  Object.defineProperty(window,"__TAURI_INTERNALS__",{value:{invoke:async(command:string,args:Record<string,unknown>)=>{
   calls.push({command,args});switch(command){
    case "get_app_status":return {version:"0.5.0",data_mode:"stored",active_profile:stored.profile_id,capabilities:{}};
    case "get_stored_snapshot":return stored;
    case "get_snapshot_watch":return {active:false,last_import:null,error:null,status:null};
    case "list_profiles":return [{id:stored.profile_id,display_name:"Synthetic route profile",data_mode:"stored",fixture:true}];
    case "get_gear_catalog":return gearCatalog;
    case "get_planning_catalog":return catalog;
    case "preview_acquisition_route":return baseline;
    case "assess_route_acquisition":return report;
    default:throw {code:"INVALID_INPUT",message:"Unexpected route acquisition mock",retryable:false};
   }
  }}});
 },{baseline,catalog,gearCatalog,stored,report});
 await page.goto("/");await page.getByRole("button",{name:"Gear",exact:true}).click();
 await page.getByRole("button",{name:"Load labelled synthetic example"}).click();
 await page.getByLabel("Role identifier",{exact:true}).fill("manual");
 await page.getByLabel("formation (ms)",{exact:true}).fill("2000");await page.getByLabel("attempt (ms)",{exact:true}).fill("10000");
 await page.getByLabel("Spending ceiling (copper)",{exact:true}).fill("");await page.getByLabel("Maximum trials (1–1024)", { exact: true }).fill("3");
 await page.getByLabel("Assume independent identical player acquisition trials", { exact: true }).check();
 await page.getByRole("combobox",{name:"Acquisition source for synthetic-upgrade",exact:true}).selectOption("synthetic-source");
 const panel=page.getByRole("region",{name:"Route-aware acquisition"});
 await panel.getByLabel("Route goal",{exact:true}).fill("manual");await panel.getByLabel("Route session budget (ms)", { exact: true }).fill("100000");
 await panel.getByRole("combobox",{name:"Route origin",exact:true}).selectOption("demo-node-1");await panel.getByRole("combobox",{name:"Route ending",exact:true}).selectOption("demo-node-1");
 await panel.getByLabel("Include synthetic-service in route", { exact: true }).check();await panel.getByRole("combobox",{name:"Objective node for synthetic-service",exact:true}).selectOption("demo-node-2");
 await panel.getByLabel("utility for synthetic-service",{exact:true}).fill("10");await panel.getByLabel("completion for synthetic-service",{exact:true}).fill("1000");await panel.getByLabel("service for synthetic-service",{exact:true}).fill("0");
 await panel.getByRole("button",{name:"Preview reserved route"}).click();await expect(panel.getByText("Reserved route: 61000 ms. Remaining budget: 39000 ms.")).toBeVisible();
 await panel.getByRole("combobox",{name:"Source node for synthetic-upgrade",exact:true}).selectOption("synthetic-source-node");await panel.getByRole("button",{name:"Assess independent route opportunities"}).click();
 await expect(panel.getByText("Incremental travel: 5000 ms.")).toBeVisible();await expect(panel.getByText("Acquisition: guaranteed; feasible trials: 3; reserved total: 98000 ms.")).toBeVisible();
 await expect(page.getByText("Gear comparison uses backend-calculated route effort and its remaining budget.")).toBeVisible();
 await panel.getByText("Requirements, route gaps and uncertainty",{exact:true}).click();
 await expect(panel.getByRole("region",{name:"Outbound source route"}).getByText(/Complete declared graph route: 5000 ms/)).toBeVisible();
 await expect(panel.getByRole("region",{name:"Rejoin reserved route"}).getByText(/Complete declared graph route: 30000 ms/)).toBeVisible();
 await expect(panel.getByText("Explicit caller spending ceiling")).toBeVisible();
 await expect(panel.locator("pre")).toHaveCount(0);
 const calls=await page.evaluate(()=>(window as unknown as {__ROUTE_CALLS__:{command:string;args:{text:string}}[]}).__ROUTE_CALLS__);
 const preview=calls.find(c=>c.command==="preview_acquisition_route");const assessment=calls.find(c=>c.command==="assess_route_acquisition");expect(preview).toBeDefined();expect(assessment).toBeDefined();
 expect(JSON.parse(preview?.args.text??"{}")).toEqual(baseline.planning_report.request);
 const request=JSON.parse(assessment?.args.text??"{}");expect(request.expected_route_fingerprint).toBe(baseline.route_fingerprint);expect(request.opportunities[0].node.id).toBe("synthetic-source-node");expect(request.gear.candidates[0].acquisition).toBeNull();expect(request.opportunities[0]).not.toHaveProperty("incremental_travel_ms");
 await panel.getByLabel("Route session budget (ms)", { exact: true }).fill("110000");await expect(panel.getByText(/Reserved route:/)).toHaveCount(0);await expect(page.getByRole("region",{name:"Gear result"})).toHaveCount(0);
});
