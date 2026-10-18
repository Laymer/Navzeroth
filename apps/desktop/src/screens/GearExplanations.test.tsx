import { fireEvent, render, screen, within } from "@testing-library/react";
import { AcquisitionDetails, BaselineSlots, EligibilityDetails, RankingDetails, RouteTraceDetails, ScoreDetails, formatMilli, reasonText } from "./GearExplanations";
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
    requirements: { truth: "false", reasons: [{participant:"synthetic-character",input:{input:"field",field:{field:"level"}},reason:{reason:"stale"}}] }, reasons: [{ code: "zero_probability" }] }} />);
  expect(screen.getByText("18446744073709551615 copper")).toBeVisible(); expect(screen.getByText("0 copper")).toBeVisible();
  expect(screen.getByText("Restricted")).toBeVisible(); expect(screen.getByText("Unsupported")).toBeVisible(); expect(screen.getByText("Not applicable")).toBeVisible();
  expect(screen.getByText("Unknown (not observed)")).toBeVisible(); expect(screen.getByText(/not satisfied/)).toBeVisible();
  expect(screen.getByText("synthetic-character: level: captured input is stale")).toBeVisible(); expect(screen.getByText("Declared acquisition probability is zero.")).toBeVisible();
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

it("renders the tagged Rust input paths and participants for acquisition and travel gaps", () => {
  const issues = [
    {participant:"synthetic-character",input:{input:"field" as const,field:{field:"quest" as const,quest:"quest-a",state:"turned_in" as const}},reason:{reason:"missing_record"}},
    {participant:"synthetic-character",input:{input:"field" as const,field:{field:"preference" as const,key:"preference-a"}},reason:{reason:"unavailable",status:"restricted"}},
    {participant:"other-character",input:{input:"field" as const,field:{field:"objective" as const,objective:"objective-a"}},reason:{reason:"clock_uncertain"}},
    {participant:"other-character",input:{input:"field" as const,field:{field:"role" as const}},reason:{reason:"unavailable",status:"unknown"}},
    {participant:"other-character",input:{input:"variant" as const,slot:"main_hand"},reason:{reason:"stale"}},
  ];
  render(<AcquisitionDetails acquisition={{...acquisition,requirements:{truth:"unknown",reasons:issues}}}/>);
  expect(screen.getByText("synthetic-character: quest quest-a (turned in): required record is missing")).toBeVisible();
  expect(screen.getByText("synthetic-character: preference preference-a: Restricted")).toBeVisible();
  expect(screen.getByText("other-character: objective objective-a: capture clock is uncertain")).toBeVisible();
  expect(screen.getByText("other-character: role: Unknown (not observed)")).toBeVisible();
  expect(screen.getByText("other-character: item variant (main_hand): captured input is stale")).toBeVisible();
  const trace=route.opportunities[0]?.detour.outbound;if(!trace)throw Error("Missing trace");
  render(<RouteTraceDetails label="Wire path" trace={{...trace,exclusions:[{edge:"edge-a",reason:{code:"requirement_unknown",issues:[issues[0]]}}]}}/>);
  expect(screen.getAllByText("synthetic-character: quest quest-a (turned in): required record is missing")).toHaveLength(2);
});

it("keeps captured empty, matched and unresolved baseline slots distinct", () => {
  const item = gear.candidates[0]?.item; if (!item) throw Error("Missing item fixture");
  render(<BaselineSlots current={{slots:{empty:{state:"empty"},matched:{state:"matched",item},unresolved:{state:"unresolved",gap:{code:"slot_clock_uncertain"}}}}}/>);
  expect(screen.getByText("empty: Captured empty slot")).toBeVisible();
  expect(screen.getByText(`matched: ${item.item_id} (${item.variant})`)).toBeVisible();
  expect(screen.getByText(/unresolved: Unresolved — The equipment-slot clock is uncertain/)).toBeVisible();
});
it("pages valid configurations while keeping uncertain, rejected and excluded outcomes separate", () => {
  const row = gear.theoretical.valid[0]; const item = gear.candidates[0]?.item;
  if (!row || !item) throw Error("Missing ranking fixture");
  render(<RankingDetails ranking={{...gear.theoretical,
    valid:Array.from({length:6},(_,index)=>({...row,rank:index+1})),
    uncertain:[{...row,rank:7,configuration:{...row.configuration,validity:"uncertain",uncertainties:[{code:"unknown_two_handed"}]}}],
    rejected:[{items:{head:item},reasons:[{code:"unique_group_conflict"}]}],
    excluded:[{item,eligibility:{outcome:"ineligible",reasons:[{code:"level_not_reached",required:25}]}}]}}/>);
  expect(screen.queryByText("Configuration #6")).toBeNull();
  expect(screen.getByText("Uncertain configuration #7")).toBeVisible();
  expect(screen.getByText("One-handed/two-handed relationship is unknown.")).toBeVisible();
  expect(screen.getByText("Two items share an incompatible unique group.")).toBeVisible();
  expect(screen.getByText("Required level 25 is not reached.")).toBeVisible();
  fireEvent.click(screen.getByRole("button",{name:"Next theoretical valid configurations"}));
  expect(screen.getByText("Configuration #6")).toBeVisible();
  expect(screen.queryByText("Configuration #1")).toBeNull();
  fireEvent.click(screen.getByRole("button",{name:"Previous theoretical valid configurations"}));
  expect(screen.getByText("Configuration #1")).toBeVisible();
});
it("does not promote absent acquisition inputs or unrecognized effort assumptions", () => {
  const view=render(<AcquisitionDetails acquisition={null}/>);
  expect(screen.getByText("Acquisition source and effort are unknown.")).toBeVisible();
  view.rerender(<AcquisitionDetails acquisition={{...acquisition,requirements:null,feasible_attempts:null,budgeted_cost_copper:null,budgeted_effort_ms:null,success_probability:null,expected_attempts:null,credited_reward:null,
    source:{...acquisition.source,conditions:{status:"unsupported",source:"unrecorded",clock_trust:"uncertain"},evidence:[]},
    effort:{...acquisition.effort,balance_basis:"future-basis",cost_schedule:"future-schedule",probability_model:"future-model"}}}/>);
  expect(screen.getByText("Requirements: not evaluated.")).toBeVisible();
  expect(screen.getByText("Unrecognized spending basis")).toBeVisible();
  expect(screen.getByText("Unrecognized cost schedule")).toBeVisible();
  expect(screen.getByText("Unrecognized probability model")).toBeVisible();
  expect(screen.getByText(/No evidence references recorded/)).toBeVisible();
  expect(screen.getByText(/No additional reward credit/)).toBeVisible();
});
it("keeps unavailable scores and missing routes explicit", () => {
  render(<><ScoreDetails score={{...gear.current_score,contributions:[],weight_source:{type:"documented_model",source:"synthetic-model"}}}/><RouteTraceDetails trace={null} label="Return"/></>);
  expect(screen.getByText(/Unknown attributes are not treated as zero/)).toBeVisible();
  expect(screen.getByText(/declared documented model.*synthetic-model/)).toBeVisible();
  expect(screen.getByText("Return: route not calculated.")).toBeVisible();
});

it.each([
  ["effects_unavailable", {status:"stale"}, "Item effects: Stale"],
  ["unweighted_attribute", {attribute:"armor"}, "No supplied weight for armor"],
  ["unmodeled_effects_listed", {count:0}, "0 listed effects are outside this model"],
  ["unknown_unique_group", {}, "Unique-item group is unknown"],
  ["baseline_slot_unresolved", {slot:"off_hand"}, "Baseline slot off_hand is unresolved"],
  ["category_not_proficient", {class:"warrior",category:"wand"}, "Declared warrior proficiency does not allow wand"],
  ["requirement_not_met", {}, "A required condition is false"],
  ["requirements_not_met", {}, "A required condition is false"],
  ["class_unavailable", {status:"unknown"}, "Class: Unknown (not observed)"],
  ["category_unavailable", {status:"restricted"}, "Item category: Restricted"],
  ["requirements_unavailable", {status:"unsupported"}, "Requirements: Unsupported"],
  ["class_not_in_proficiency_table", {class:"warrior"}, "No supplied proficiency mapping for warrior"],
  ["category_not_in_proficiency_table", {class:"warrior",category:"wand"}, "No supplied warrior mapping for wand"],
  ["requirement_indeterminate", {}, "Required conditions cannot be established"],
  ["requirements_unknown", {}, "Required conditions cannot be established"],
  ["two_handed_with_off_hand", {}, "conflicts with the off-hand item"],
  ["two_handed_outside_main_hand", {}, "outside the declared main-hand slot"],
  ["slot_not_recorded", {}, "This equipment slot was not recorded"],
  ["slot_unavailable", {status:"unknown"}, "Equipment slot: Unknown (not observed)"],
  ["slot_stale", {}, "equipment-slot capture is stale"],
  ["variant_unavailable", {status:"restricted"}, "Item variant: Restricted"],
  ["variant_stale", {}, "captured item variant is stale"],
  ["variant_clock_uncertain", {}, "item-variant clock is uncertain"],
  ["not_in_catalog", {}, "exact captured item/variant is absent"],
  ["map_mismatch", {}, "different map space"],
  ["temporarily_unavailable", {}, "temporarily unavailable"],
  ["unverified", {}, "travel edge is unverified"],
  ["requirement_unavailable", {status:"unsupported"}, "Travel requirement: Unsupported"],
  ["requirement_unknown", {}, "Travel requirements cannot be established"],
  ["cost_unknown", {term:"wait"}, "Waiting cost is not known"],
  ["cost_unknown", {term:"travel"}, "Expected travel cost is not known"],
  ["stateful_not_verified", {}, "state-changing travel action is not verified"],
  ["stateful_not_final_hop", {}, "cannot be used as an intermediate hop"],
  ["already_counted", {}, "already counted as a selected reward"],
  ["field_unavailable", {field:"level",status:"stale"}, "level: Stale"],
  ["balance_not_fresh", {freshness:{status:"uncertain"}}, "cannot establish available spending"],
  ["session_too_short", {}, "cannot fit an attempt and its reserved costs"],
  ["insufficient_copper", {}, "spending balance cannot cover an attempt"],
  ["probability_model_unqualified", {}, "source rate is not a player acquisition probability"],
  ["probabilistic", {}, "conditional on the supplied independent-attempt model"],
] as const)("explains %s without promoting missing eligibility or acquisition evidence", (code, fields, expected) => {
  const result=reasonText({code,...fields,private_text:"private detail must not be displayed"});
  expect(result).toContain(expected);
  expect(result).not.toContain("private detail");
});
it("handles malformed and future requirement paths without exposing arbitrary fields", () => {
  expect(reasonText({code:"requirement",issue:null})).toBe("required input unavailable: explanation unavailable");
  expect(reasonText({code:"requirement",issue:{input:{input:"rule_expiry"},reason:{reason:"expired_rule"}}})).toBe("rule expiry: rule has expired");
  expect(reasonText({code:"requirement",issue:{input:{input:"field",field:{field:"future",secret:"do not display"}},reason:{reason:"unavailable",status:"future"}}})).toBe("required input unavailable: Unrecognized observation state");
});
