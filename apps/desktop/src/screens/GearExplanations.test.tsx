import { fireEvent, render, screen, within } from "@testing-library/react";
import { AcquisitionDetails, EligibilityDetails, RouteTraceDetails, ScoreDetails, formatMilli, reasonText } from "./GearExplanations";
import type { GearReport } from "../commands/gear";
import type { RouteAcquisitionReport } from "../commands/routeAcquisition";
import gearJSON from "../../../../fixtures/desktop/gear/report.json";
import routeJSON from "../../../../fixtures/desktop/route-acquisition/report.json";
const gear = gearJSON as unknown as GearReport;
const route = routeJSON as unknown as RouteAcquisitionReport;
const acquisition = gear.candidates[0]?.acquisition;
if (!acquisition) throw Error("Missing Rust acquisition fixture");

it("retains every signed milli-unit at the Rust JS-safe boundary", () => {
  expect(formatMilli(9007199254740991)).toBe("9007199254740.991");
  expect(formatMilli(9007199254740990)).toBe("9007199254740.99");
  expect(formatMilli(-9007199254740991)).toBe("-9007199254740.991");
  expect(formatMilli(-1)).toBe("-0.001"); expect(formatMilli(0)).toBe("0");
  expect(formatMilli(9007199254740992)).toContain("Unavailable");
});
it("shows zero contributions, unknown effects and explicit model limitations without assigning them zero", () => {
  render(<ScoreDetails score={{ ...gear.current_score, certainty: "low", unmodeled_effects: [{ code: "unavailable", status: "restricted" }, { code: "listed", text: "Synthetic proc outside model" }], profile_unmodeled: ["No combat simulation"], certainty_reasons: [{ code: "attributes_unavailable", status: "unsupported" }] }} />);
  const row = screen.getByLabelText("Attribute synthetic-power");
  expect(within(row).getAllByText("0")).toHaveLength(2); expect(within(row).getByText("1")).toBeVisible();
  expect(screen.getByText("Effects: Restricted")).toBeVisible(); expect(screen.getByText("Item attributes: Unsupported")).toBeVisible();
  expect(screen.getByText(/Synthetic proc outside model/)).toBeVisible(); expect(screen.getByText("No combat simulation")).toBeVisible();
  expect(screen.getByText(/not a damage or survivability prediction/)).toBeVisible();
});
it("distinguishes zero, lossless copper, unknown, unsupported, restricted and not-applicable acquisition fields", () => {
  render(<AcquisitionDetails acquisition={{ ...acquisition, feasible_attempts: 0, budgeted_effort_ms: 0, budgeted_cost_copper: "18446744073709551615", success_probability: 0, expected_attempts: null,
    effort: { ...acquisition.effort, incremental_travel_ms: { status: "known", value: 0 }, formation_ms: { status: "restricted" }, attempt_ms: { status: "unsupported" }, stopping_ms: { status: "not_applicable" }, available_copper: { status: "unknown" } },
    source: { ...acquisition.source, cost: { status: "known", value: "0", source: "unrecorded", clock_trust: "uncertain" } },
    requirements: { truth: "false", reasons: [{input:{input:"field",field:"level"},reason:{reason:"stale"}}] }, reasons: [{ code: "zero_probability" }] }} />);
  expect(screen.getByText("18446744073709551615 copper")).toBeVisible(); expect(screen.getByText("0 copper")).toBeVisible();
  expect(screen.getByText("Restricted")).toBeVisible(); expect(screen.getByText("Unsupported")).toBeVisible(); expect(screen.getByText("Not applicable")).toBeVisible();
  expect(screen.getByText("Unknown (not observed)")).toBeVisible(); expect(screen.getByText(/not satisfied/)).toBeVisible();
  expect(screen.getByText("level: captured input is stale")).toBeVisible(); expect(screen.getByText("Declared acquisition probability is zero.")).toBeVisible();
  expect(screen.getByText(/not tonight’s attempt count/)).toBeVisible();
});
it("pages every unmodeled effect instead of hiding the tail or treating absence as completeness", () => {
  render(<ScoreDetails score={{ ...gear.current_score, unmodeled_effects: Array.from({length:33},(_,index)=>({code:"listed",text:`Synthetic effect ${index}`})) }} />);
  expect(screen.queryByText("Unmodeled effect: Synthetic effect 32")).toBeNull();
  fireEvent.click(screen.getByRole("button",{name:"Next unmodeled effects"}));
  expect(screen.getByText("Unmodeled effect: Synthetic effect 32")).toBeVisible(); expect(screen.getByText(/33–33 of 33/)).toBeVisible();
});
it("renders graph uncertainty and known false separately without connecting incomplete routes", () => {
  const trace = route.opportunities[0]?.detour.outbound;if(!trace||trace.outcome.status!=="complete")throw Error("Missing Rust trace");
  render(<RouteTraceDetails label="Synthetic outbound" trace={{ ...trace, outcome:{status:"incomplete",target:"missing-target",reached:trace.outcome.route,reachable:[]}, exclusions:[{edge:"unusable",reason:{code:"stateful_availability_unknown",consumes_resource:"known",cooldown:"unknown"}}] }} />);
  expect(screen.getByText(/no connection across the gap is assumed/)).toBeVisible();
  expect(screen.getByText(/declared no/)).toBeVisible(); expect(screen.getByText(/Resource use: Known; cooldown: Unknown/)).toBeVisible();
  expect(screen.queryByText(/Unrecognized observation state/)).toBeNull();
});
it("keeps future equipment unavailable now and handles an unknown reason without inventing eligibility", () => {
  render(<EligibilityDetails eligibility={{outcome:"future_progression",required_level:20}} />);
  expect(screen.getByText(/unavailable for use now/)).toBeVisible();
  expect(reasonText({code:"future_reason",private_text:"do not display"})).toBe("Unrecognized explanation code (future_reason); no definite conclusion is added.");
});
