import catalogJson from "../../../../fixtures/desktop/gear/catalog.json";
import reportJson from "../../../../fixtures/desktop/gear/report.json";
import type { GearCatalog, GearReport } from "../commands/gear";
it("reads actual Rust-generated gear DTOs without losing false, zero or conditional values", () => {
  const catalog = catalogJson as GearCatalog;
  const report = reportJson as GearReport;
  expect(catalog.fixture).toBe(true);
  expect(report.current.slots["head"]).toEqual({state:"empty"});
  expect(report.candidates[0]?.eligibility.outcome).toBe("eligible");
  expect(report.practical[0]?.conditional).toBe(false);
  expect(report.practical[0]?.reserved_effort_ms).toBe(0);
  expect(report.practical[1]?.conditional).toBe(true);
  expect(report.candidates[1]?.acquisition?.success_probability).toBe(0.5);
  expect(report.candidates[0]?.acquisition?.effort.probability_model).toBe("independent_player_attempts");
  for (const candidate of report.candidates) expect(candidate.score.contributions.reduce((sum,row)=>sum+row.contribution,0)).toBe(candidate.score.total);
});
