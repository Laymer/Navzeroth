import { expect, test } from "@playwright/test";
import example from "../../../fixtures/desktop/sessions/analytics.synthetic.json" with { type: "json" };
import snapshot from "../../../fixtures/contracts/v1/character-snapshot.json" with { type: "json" };
import type { CalibrationRequest, SessionCalibrationAnnotation } from "../src/commands/calibration";
test("explicit session association and conditional comparison retain excluded evidence", async ({page})=> {
  await page.addInitScript(({example,snapshot})=> {
    let annotation: SessionCalibrationAnnotation | null=null;
    Object.defineProperty(window,"__TAURI_INTERNALS__",{value:{invoke:async(command:string,args:Record<string,unknown>)=> {
      switch(command) {
        case "get_app_status": return {version:"test",data_mode:"stored",active_profile:"profile",capabilities:{}};
        case "list_profiles": return [{id:"profile",display_name:"Synthetic character",fixture:true,context:example.summary.context,snapshot_count:1,data_mode:"stored"}];
        case "get_stored_snapshot": return {envelope:snapshot,profile_id:"profile",freshness:{status:"uncertain",reason:"missing_or_untrusted_time"},data_mode:"stored"};
        case "list_sessions": return [{session_id:example.summary.session,analytics:example}];
        case "get_session_detail": return {fixture:true,batch:{character:example.summary.character,context:example.summary.context,events:[]},analytics:example};
        case "set_session_calibration": {
          annotation=args.annotation as SessionCalibrationAnnotation;
          if(args.profile_id!=="profile" || args.session_id!==example.summary.session || annotation.conditions.route_stage!=="declared-stage" || annotation.conditions.level!==12 || annotation.conditions.role!=="healer" || annotation.conditions.party_size!==2 || !annotation.complete || annotation.completion_quest!=="quest-a") throw {code:"INVALID_INPUT",message:"Wrong annotation wrapper arguments",retryable:false};
          return null;
        }
        case "get_calibration": {
          const request=args.request as CalibrationRequest;
          if(args.profile_id!=="profile" || request.baseline_ms!==600000 || request.baseline_source!=="documented user assumption" || !annotation || JSON.stringify(request.conditions)!==JSON.stringify(annotation.conditions)) throw {code:"INVALID_INPUT",message:"Wrong comparison wrapper arguments",retryable:false};
          return {methodology_version:1,character:example.summary.character,context:example.summary.context,fixture:true,request,sample_count:0,median_ms:null,minimum_ms:null,maximum_ms:null,blended_ms:request.baseline_ms,uncertainty:"no_complete_samples",comparisons:[{session_id:example.summary.session,annotation,observed_ms:null,difference_ms:null,exclusion:"gaps_or_uncovered_time",outlier:false}],methodology:"Synthetic command mock: unclassified intervals and gaps never become route duration."};
        }
        default: throw {code:"INVALID_INPUT",message:"Unexpected mock command",retryable:false};
      }
    }}});
  },{example,snapshot});
  await page.goto("/");
  await page.getByRole("button",{name:"Sessions",exact:true}).click();
  await page.getByRole("button",{name:"Inspect synthetic-session"}).click();
  await page.getByLabel("Route or quest stage", { exact: true }).fill("declared-stage");
  await page.getByLabel("Declared level", { exact: true }).fill("12");
  await page.getByLabel("Declared role", { exact: true }).fill("healer");
  await page.getByLabel("Declared party size", { exact: true }).fill("2");
  await page.getByLabel("Baseline source or assumption", { exact: true }).fill("documented user assumption");
  await page.getByLabel("Terminal observed quest ID", { exact: true }).fill("quest-a");
  await page.getByLabel("I declare this whole session").check();
  await page.getByRole("button",{name:"Save association for synthetic-session"}).click();
  await expect(page.getByRole("status")).toContainText("Explicit association saved");
  await page.getByRole("button",{name:"Compare retained sessions"}).click();
  await expect(page.getByText(/Complete comparable samples: 0/)).toContainText("SYNTHETIC");
  await expect(page.getByText(/synthetic-session: excluded/)).toContainText("gaps or uncovered time");
  await expect(page.getByText(/Observed session median:/)).toContainText("unavailable");
  await page.getByLabel("Declared party size", { exact: true }).fill("3");
  await expect(page.getByText(/Complete comparable samples:/)).toHaveCount(0);
});
