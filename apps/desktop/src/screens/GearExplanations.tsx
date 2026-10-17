import { Button } from "../ui";
import { useState, type ReactNode } from "react";
import type { GearAcquisition, GearEligibility, GearObservation, GearRanking, GearReason, GearReport, GearScore } from "../commands/gear";
import type { AcquisitionRouteTrace } from "../commands/routeAcquisition";
import "./gear-explanations.css";

const record = (value: unknown): Record<string, unknown> => value !== null && typeof value === "object" && !Array.isArray(value) ? value as Record<string, unknown> : {};
const text = (value: unknown): string => typeof value === "string" ? value : "unavailable";
/** Decimal formatting only: preserve every valid Rust milli-unit at the JS-safe boundary. */
export function formatMilli(value: number): string {
  if (!Number.isSafeInteger(value)) return "Unavailable (invalid fixed-point value)";
  const raw = BigInt(value); const magnitude = raw < 0n ? -raw : raw;
  const whole = magnitude / 1000n;
  const fraction = (magnitude % 1000n).toString().padStart(3, "0").replace(/0+$/, "");
  return `${raw < 0n ? "-" : ""}${whole}${fraction ? `.${fraction}` : ""}`;
}
const fixed = formatMilli;
const statusNames: Record<string, string> = {
  known: "Known", unknown: "Unknown (not observed)", unsupported: "Unsupported", restricted: "Restricted", stale: "Stale", not_applicable: "Not applicable",
};
const status = (value: unknown): string => statusNames[text(value)] ?? "Unrecognized observation state";
function issueText(value: unknown): string {
  const issue = record(value); const reason = record(issue.reason); const input = record(issue.input);
  const reasons: Record<string,string> = { missing_record: "required record is missing", stale: "captured input is stale", clock_uncertain: "capture clock is uncertain", expired_rule: "rule has expired" };
  const why = reason.reason === "unavailable" ? status(reason.status) : reasons[text(reason.reason)] ?? "explanation unavailable";
  const field = record(input.field);
  let label = "required input unavailable";
  if(input.input === "field") {
    switch(field.field) {
      case "level": case "role": label=field.field;break;
      case "preference":label=`preference ${text(field.key)}`;break;
      case "quest":label=`quest ${text(field.quest)} (${text(field.state).replaceAll("_"," ")})`;break;
      case "objective":label=`objective ${text(field.objective)}`;break;
    }
  } else {
    const labels:Record<string,string>={equipment:"equipment",variant:"item variant",modifiers:"item modifiers",location:"location",participant:"participant",rule_expiry:"rule expiry"};
    label=labels[text(input.input)]??label;
    if(typeof input.slot === "string")label+=` (${input.slot})`;
  }
  return `${typeof issue.participant === "string" ? `${issue.participant}: ` : ""}${label}: ${why}`;
}
export function reasonText(reason: GearReason): string {
  const unavailable = `${text(reason.field ?? reason.slot ?? reason.attribute ?? "Input")}: ${status(reason.status)}`;
  switch (reason.code) {
    case "listed": return `Unmodeled effect: ${text(reason.text)}`;
    case "unavailable": return `Effects: ${status(reason.status)}`;
    case "attributes_unavailable": return `Item attributes: ${status(reason.status)}`;
    case "effects_unavailable": return `Item effects: ${status(reason.status)}`;
    case "unweighted_attribute": return `No supplied weight for ${text(reason.attribute)}.`;
    case "unmodeled_effects_listed": return `${text(typeof reason.count === "number" ? String(reason.count) : undefined)} listed effects are outside this model.`;
    case "unknown_unique_group": return "Unique-item group is unknown.";
    case "unknown_two_handed": return "One-handed/two-handed relationship is unknown.";
    case "baseline_slot_unresolved": return `Baseline slot ${text(reason.slot)} is unresolved.`;
    case "category_not_proficient": return `Declared ${text(reason.class)} proficiency does not allow ${text(reason.category)}.`;
    case "level_not_reached": return `Required level ${typeof reason.required === "number" ? reason.required : "unknown"} is not reached.`;
    case "requirement_not_met": case "requirements_not_met": return "A required condition is false.";
    case "class_unavailable": return `Class: ${status(reason.status)}`;
    case "category_unavailable": return `Item category: ${status(reason.status)}`;
    case "requirements_unavailable": return `Requirements: ${status(reason.status)}`;
    case "class_not_in_proficiency_table": return `No supplied proficiency mapping for ${text(reason.class)}.`;
    case "category_not_in_proficiency_table": return `No supplied ${text(reason.class)} mapping for ${text(reason.category)}.`;
    case "requirement": return issueText(reason.issue);
    case "requirement_indeterminate": case "requirements_unknown": return "Required conditions cannot be established.";
    case "two_handed_with_off_hand": return "A two-handed main weapon conflicts with the off-hand item.";
    case "two_handed_outside_main_hand": return "A two-handed weapon is outside the declared main-hand slot.";
    case "unique_group_conflict": return "Two items share an incompatible unique group.";
    case "slot_not_recorded": return "This equipment slot was not recorded.";
    case "slot_unavailable": return `Equipment slot: ${status(reason.status)}`;
    case "slot_stale": return "The equipment-slot capture is stale.";
    case "slot_clock_uncertain": return "The equipment-slot clock is uncertain.";
    case "variant_unavailable": return `Item variant: ${status(reason.status)}`;
    case "variant_stale": return "The captured item variant is stale.";
    case "variant_clock_uncertain": return "The item-variant clock is uncertain.";
    case "not_in_catalog": return "The exact captured item/variant is absent from this catalog.";
    case "map_mismatch": return "An edge endpoint belongs to a different map space.";
    case "temporarily_unavailable": return "The travel edge is temporarily unavailable.";
    case "unverified": return "The travel edge is unverified.";
    case "requirement_unavailable": return `Travel requirement: ${status(reason.status)}`;
    case "requirement_unknown": return "Travel requirements cannot be established.";
    case "cost_unknown": return `${reason.term === "wait" ? "Waiting" : "Expected travel"} cost is not known.`;
    case "stateful_not_verified": return "The state-changing travel action is not verified.";
    case "stateful_availability_unknown": return `Resource use: ${status(reason.consumes_resource)}; cooldown: ${status(reason.cooldown)}.`;
    case "stateful_not_final_hop": return "The state-changing action cannot be used as an intermediate hop.";
    case "already_counted": return "This full item identity is already counted as a selected reward.";
    case "field_unavailable": return unavailable;
    case "balance_not_fresh": return `Captured balance: ${text(record(reason.freshness).status)}; it cannot establish available spending.`;
    case "session_too_short": return "The supplied session cannot fit an attempt and its reserved costs.";
    case "insufficient_copper": return "The supplied spending balance cannot cover an attempt.";
    case "zero_probability": return "Declared acquisition probability is zero.";
    case "probability_model_unqualified": return "Independent identical player attempts were not established; a source rate is not a player acquisition probability.";
    case "probabilistic": return "Acquisition is conditional on the supplied independent-attempt model.";
    default: return `Unrecognized explanation code (${reason.code.slice(0, 64)}); no definite conclusion is added.`;
  }
}
function Paged<T>({ label, rows, children, limit = 32 }: { label: string; rows: readonly T[]; children: (row: T, index: number) => ReactNode; limit?: number }) {
  const [page, setPage] = useState(0); const last = Math.max(0, Math.ceil(rows.length / limit) - 1); const current = Math.min(page, last); const start = current * limit;
  return <div className="gear-explanation" aria-label={label}>
    {rows.slice(start, start + limit).map((row, index) => <div key={start + index}>{children(row, start + index)}</div>)}
    {rows.length > limit && <div className="explanation-pages"><p>{label}: {start + 1}–{Math.min(start + limit, rows.length)} of {rows.length}.</p><Button variant="ghost" disabled={current === 0} onClick={() => setPage(current - 1)}>Previous {label}</Button><Button variant="ghost" disabled={current === last} onClick={() => setPage(current + 1)}>Next {label}</Button></div>}
  </div>;
}
function Reasons({ reasons, label }: { reasons: GearReason[]; label: string }) {
  return <Paged label={label} rows={reasons}>{(reason) => <p>{reasonText(reason)}</p>}</Paged>;
}
export function ScoreDetails({ score }: { score: GearScore }) {
  const sourceLabels: Record<string,string> = { user_preference: "explicit user preference", synthetic_test: "synthetic test", documented_model: "declared documented model", permitted_guide: "declared permitted guide" };
  return <section aria-label="Model score explanation" className="gear-explanation">
    <p>Model score {fixed(score.total)}; {score.certainty} input certainty relative to {score.role}. This is not a damage or survivability prediction.</p>
    <p>Weights: {sourceLabels[score.weight_source.type]}{"source" in score.weight_source ? ` (${score.weight_source.source})` : ""}; model {score.model_version}.</p>
    {score.contributions.length === 0 ? <p>No modeled attribute contributions were reported. Unknown attributes are not treated as zero.</p> : <Paged label="attribute contributions" rows={score.contributions}>{(row) => <dl className="explanation-values" aria-label={`Attribute ${row.attribute}`}><dt>Attribute</dt><dd>{row.attribute}</dd><dt>Supplied value</dt><dd>{fixed(row.value)}</dd><dt>Supplied weight</dt><dd>{fixed(row.weight)}</dd><dt>Model contribution</dt><dd>{fixed(row.contribution)}</dd></dl>}</Paged>}
    <h5>Effects outside the model</h5>
    {score.unmodeled_effects.length === 0 ? <p>No item effects were listed here; this does not establish a complete gameplay model.</p> : <Reasons label="unmodeled effects" reasons={score.unmodeled_effects} />}
    <Paged label="profile limitations" rows={score.profile_unmodeled}>{(effect) => <p>{effect}</p>}</Paged>
    <Reasons label="score uncertainty" reasons={score.certainty_reasons} />
  </section>;
}
export function EligibilityDetails({ eligibility }: { eligibility: GearEligibility }) {
  return <div><p>{eligibility.outcome === "eligible" ? "Equipment requirements are satisfied under the supplied inputs." : eligibility.outcome === "future_progression" ? `Future progression: required level ${eligibility.required_level}; unavailable for use now.` : eligibility.outcome === "ineligible" ? "Equipment requirements are not satisfied." : "Equipment eligibility is unknown."}</p>{"reasons" in eligibility && <Reasons label="eligibility reasons" reasons={eligibility.reasons} />}</div>;
}
function ObservationValue<T>({ observation, format }: { observation: GearObservation<T>; format: (value: T) => string }) {
  return <>{observation.status === "known" ? format(observation.value) : status(observation.status)}</>;
}
export function AcquisitionDetails({ acquisition }: { acquisition: GearAcquisition | null }) {
  if (!acquisition) return <p>Acquisition source and effort are unknown.</p>;
  const a = acquisition; const duration = (value: number) => `${value} ms`; const copper = (value: string) => `${value} copper`;
  const probabilityModels: Record<string,string> = { declared_only: "Declared source rate only; player loot eligibility is unknown", independent_player_attempts: "Explicit independent identical player attempts" };
  const balances: Record<string,string> = { caller_budget: "Explicit caller spending ceiling", observed_balance: "Captured balance (freshness required)" };
  return <section aria-label="Acquisition explanation" className="gear-explanation">
    <p>Source {a.source.id} ({a.source.kind}); outcome {a.status}. Useful lifespan is unknown without a progression model.</p>
    <p>Source requirement declaration: {a.source.conditions.status === "known" ? "Available" : status(a.source.conditions.status)}.</p>
    <p>Requirements: {a.requirements ? a.requirements.truth === "true" ? "satisfied under supplied inputs" : a.requirements.truth === "false" ? "not satisfied" : "unknown" : "not evaluated"}.</p>
    <Reasons label="acquisition reasons" reasons={a.reasons} />
    <Paged label="requirement gaps" rows={a.requirements?.reasons ?? []}>{(issue) => <p>{issueText(issue)}</p>}</Paged>
    <Paged label="requirement warnings" rows={a.requirements?.warnings ?? []}>{(issue) => <p>{issueText(issue)}</p>}</Paged>
    <dl className="explanation-values"><dt>Incremental travel</dt><dd><ObservationValue observation={a.effort.incremental_travel_ms} format={duration} /></dd><dt>Formation</dt><dd><ObservationValue observation={a.effort.formation_ms} format={duration} /></dd><dt>One attempt</dt><dd><ObservationValue observation={a.effort.attempt_ms} format={duration} /></dd><dt>Ending reservation</dt><dd><ObservationValue observation={a.effort.stopping_ms} format={duration} /></dd><dt>Available spending</dt><dd><ObservationValue observation={a.effort.available_copper} format={copper} /></dd><dt>Spending basis</dt><dd>{balances[a.effort.balance_basis] ?? "Unrecognized spending basis"}</dd><dt>Source cost</dt><dd><ObservationValue observation={a.source.cost} format={copper} /></dd><dt>Cost schedule</dt><dd>{a.effort.cost_schedule === "once" ? "Once" : a.effort.cost_schedule === "per_attempt" ? "Per attempt" : "Unrecognized cost schedule"}</dd><dt>Session budget</dt><dd>{a.effort.session_budget_ms} ms</dd><dt>Attempt ceiling</dt><dd>{a.effort.max_attempts}</dd><dt>Feasible attempts tonight</dt><dd>{a.feasible_attempts ?? "Unknown"}</dd><dt>Reserved effort</dt><dd>{a.budgeted_effort_ms === null ? "Unknown" : duration(a.budgeted_effort_ms)}</dd><dt>Reserved cost</dt><dd>{a.budgeted_cost_copper === null ? "Unknown" : copper(a.budgeted_cost_copper)}</dd><dt>Declared source probability</dt><dd><ObservationValue observation={a.source.probability} format={(n) => String(n)} /></dd><dt>Probability model</dt><dd>{probabilityModels[a.effort.probability_model] ?? "Unrecognized probability model"}</dd><dt>Conditional session success probability</dt><dd>{a.success_probability ?? "Unknown"}</dd><dt>Infinite-horizon expected attempts</dt><dd>{a.expected_attempts ?? "Unknown"}; this is not tonight’s attempt count.</dd></dl>
    <p>Reward credit: {a.credited_reward ? `${a.credited_reward.item_id} (${a.credited_reward.variant}) — opportunity credit, not observed loot` : "No additional reward credit"}.</p>
    <p>Source evidence references: {a.source.evidence.length ? a.source.evidence.join(", ") : "No evidence references recorded"}.</p>
  </section>;
}
export function BaselineSlots({ current }: { current: GearReport["current"] }) {
  return <Paged label="baseline slots" rows={Object.entries(current.slots)}>{([slot, value]) => <p>{slot}: {value.state === "empty" ? "Captured empty slot" : value.state === "matched" ? `${value.item.item_id} (${value.item.variant})` : `Unresolved — ${reasonText(value.gap)}`}</p>}</Paged>;
}
export function RankingDetails({ ranking }: { ranking: GearRanking }) {
  return <div>
    <Paged label={`${ranking.ranking} valid configurations`} rows={ranking.valid} limit={5}>{(row) => <section><h5>Configuration #{row.rank}</h5><p>{Object.entries(row.configuration.items).map(([slot, item]) => `${slot}: ${item.item.item_id} (${item.item.variant})`).join(", ")}</p><Reasons label="configuration uncertainty" reasons={row.configuration.uncertainties} /><ScoreDetails score={row.score} /></section>}</Paged>
    <Paged label={`${ranking.ranking} uncertain configurations`} rows={ranking.uncertain} limit={5}>{(row) => <section><h5>Uncertain configuration #{row.rank}</h5><p>{Object.entries(row.configuration.items).map(([slot, item]) => `${slot}: ${item.item.item_id} (${item.item.variant})`).join(", ")}</p><Reasons label="configuration uncertainty" reasons={row.configuration.uncertainties} /><ScoreDetails score={row.score} /></section>}</Paged>
    <Paged label="rejected configurations" rows={ranking.rejected}>{(row) => <div><p>Rejected: {Object.entries(row.items).map(([slot, item]) => `${slot}: ${item.item_id} (${item.variant})`).join(", ")}</p><Reasons label="configuration rejection" reasons={row.reasons} /></div>}</Paged>
    <Paged label="excluded equipment" rows={ranking.excluded}>{(row) => <div><p>{row.item.item_id} ({row.item.variant})</p><EligibilityDetails eligibility={row.eligibility} /></div>}</Paged>
  </div>;
}

export function RouteTraceDetails({ trace, label }: { trace: AcquisitionRouteTrace | null; label: string }) {
  if (!trace) return <p>{label}: route not calculated.</p>;
  const path = trace.outcome.status === "complete" ? trace.outcome.route : trace.outcome.reached;
  return <section className="gear-explanation" aria-label={label}><h5>{label}</h5>
    <p>{trace.outcome.status === "complete" ? `Complete declared graph route: ${path.total_ms} ms.` : `Incomplete graph route to ${trace.outcome.target}; no connection across the gap is assumed.`}</p>
    {path.hops.length === 0 && <p>No travel hops in the reported segment; origin {path.start.id}.</p>}
    <Paged label={`${label} hops`} rows={path.hops}>{(hop) => <p>{hop.from.id} → {hop.to.id}: {hop.mode}, {hop.cost_ms} ms ({hop.status}); {hop.stateful ? "state-changing action" : "no state-changing action"}; resource use <ObservationValue observation={hop.consumes_resource} format={(value) => value ? "declared yes" : "declared no"} />; cooldown <ObservationValue observation={hop.cooldown_ms} format={(value) => `${value} ms`} />.</p>}</Paged>
    <Paged label={`${label} excluded edges`} rows={trace.exclusions}>{(edge) => <div><p>Excluded {edge.edge}: {reasonText(edge.reason)}</p><Paged label="travel requirement gaps" rows={Array.isArray(edge.reason.issues) ? edge.reason.issues : []}>{(issue) => <p>{issueText(issue)}</p>}</Paged></div>}</Paged>
  </section>;
}
