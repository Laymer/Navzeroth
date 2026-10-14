import { RouteAcquisitionPanel } from "./RouteAcquisitionPanel";
import { useCallback, useEffect, useRef, useState } from "react";
import type { CommandClient } from "../commands";
import type { ProfileSummary } from "../commands/contract";
import type { GearCatalog, GearInput, GearObservation, GearReport, GearRequest, GearRanking } from "../commands/gear";

const unknown = <T,>(): GearObservation<T> => ({ status: "unknown" });
const known = <T,>(value: T): GearObservation<T> => ({ status: "known", value });
const numberObservation = (value: string): GearObservation<number> => value.trim() === "" ? unknown() : known(Number(value));
const itemId = (input: Pick<GearInput, "item">) => `${input.item.item_id} (${input.item.variant})`;
function RankingView({ name, ranking }: { name: string; ranking: GearRanking }) {
  return <section aria-label={name}><h3>{name}</h3><p>{ranking.label}. {ranking.scope_note}</p>
    <p>{ranking.top_is_definitive ? "Definitive ordering within the supplied model" : "Ordering is not definitive"}. {ranking.valid.length} legal, {ranking.uncertain.length} uncertain, {ranking.rejected.length} rejected configurations.</p>
    {ranking.valid.slice(0, 5).map((configuration) => <p key={configuration.rank}>#{configuration.rank}: {Object.entries(configuration.configuration.items).map(([slot, item]) => `${slot}: ${item.item.item_id} (${item.item.variant})`).join(", ")}; model score {configuration.score.total / 1000}; {configuration.score.certainty}.</p>)}
    <details><summary>Configuration legality, exclusions and score breakdowns</summary><pre>{JSON.stringify(ranking, null, 2)}</pre></details>
  </section>;
}
export function GearScreen({ client, refreshKey }: { client: CommandClient; refreshKey: number }) {
  const [profiles, setProfiles] = useState<ProfileSummary[]>([]);
  const [profile, setProfile] = useState("");
  const [catalog, setCatalog] = useState<GearCatalog | null>(null);
  const [inputs, setInputs] = useState<GearInput[]>([]);
  const [role, setRole] = useState("manual-role");
  const [weights, setWeights] = useState("{}");
  const [className, setClassName] = useState("");
  const [proficiency, setProficiency] = useState("{}");
  const [mainHand, setMainHand] = useState("");
  const [offHand, setOffHand] = useState("");
  const [times, setTimes] = useState({ travel: "", formation: "", attempt: "", stopping: "", budget: "", copper: "", attempts: "1" });
  const [independent, setIndependent] = useState(false);
  const [perAttempt, setPerAttempt] = useState(false);
  const [synthetic, setSynthetic] = useState(false);
  const [reportBasis, setReportBasis] = useState<"manual" | "route" | null>(null);
  const [report, setReport] = useState<GearReport | null>(null);
  const [error, setError] = useState("");
  const [busy, setBusy] = useState(false);
  const generation = useRef(0);
  const scope = useRef<{ client: CommandClient; profile: string } | null>(null);
  const routeReport = useCallback((value: GearReport | null) => { generation.current++; setReport(value); setReportBasis(value ? "route" : null); }, []);
  const invalidate = useCallback(() => { generation.current++; setReport(null); setBusy(false); }, []);
  useEffect(() => {
    let live = true;
    void client.listProfiles().then((result) => {
      if (!live) return;
      if (!result.ok) { setError(result.error.message); return; }
      setProfiles(result.value); setProfile((old) => result.value.some((p) => p.id === old) ? old : result.value[0]?.id ?? "");
    });
    return () => { live = false; };
  }, [client, refreshKey]);
  useEffect(() => {
    let live = true; generation.current++; setReport(null); setBusy(false); setCatalog(null);
    const changedScope = scope.current?.client !== client || scope.current?.profile !== profile;
    scope.current = { client, profile };
    if (changedScope) {
      setInputs([]); setSynthetic(false); setWeights("{}"); setRole("manual-role"); setClassName(""); setProficiency("{}"); setMainHand(""); setOffHand("");
      setTimes({ travel: "", formation: "", attempt: "", stopping: "", budget: "", copper: "", attempts: "1" }); setIndependent(false); setPerAttempt(false);
    }
    if (profile) void client.getGearCatalog(profile).then((result) => {
      if (!live) return;
      if (!result.ok) { setError(result.error.message); return; }
      setCatalog(result.value); setError("");
      const availableSlots = new Set([...result.value.items.flatMap((item) => item.slots), ...Object.keys(result.value.equipment)]);
      setMainHand((old) => availableSlots.has(old) ? old : ""); setOffHand((old) => availableSlots.has(old) ? old : "");
      setInputs((old) => old.filter((input) => result.value.items.some((item) => JSON.stringify(item.key) === JSON.stringify(input.item))).map((input) => ({
        ...input, acquisition: input.acquisition && result.value.acquisitions.some((source) => source.id === input.acquisition?.source && source.items.some((item) => JSON.stringify(item) === JSON.stringify(input.item))) ? input.acquisition : null,
      })));
    });
    return () => { live = false; generation.current++; };
  }, [client, profile, refreshKey]);
  const slots = [...new Set([...(catalog?.items.flatMap((i) => i.slots) ?? []), ...Object.keys(catalog?.equipment ?? {})])].sort();
  function example() {
    if (!catalog?.fixture) return;
    invalidate(); setSynthetic(true); setRole("synthetic-equal-weights");
    const attributes = [...new Set(catalog.items.flatMap((i) => i.attributes.status === "known" ? Object.keys(i.attributes.value) : []))];
    setWeights(JSON.stringify(Object.fromEntries(attributes.map((id) => [id, 1000])), null, 2));
    setInputs(catalog.items.slice(0, 128).map((item) => ({ item: item.key, category: { status: "not_applicable" }, acquisition: null })));
    setTimes({ travel: "0", formation: "0", attempt: "1000", stopping: "0", budget: "10000", copper: "0", attempts: "1" });
    setIndependent(false); setPerAttempt(false); setClassName(""); setProficiency("{}"); setMainHand(""); setOffHand("");
  }
  function gearRequest(): GearRequest {
    if (!catalog?.snapshot_id || !catalog.revision) throw new Error("Import a capture and activate content first.");
    return {
        profile_id: profile, snapshot_id: catalog.snapshot_id, revision: catalog.revision,
        model: { role, version: "gear-model-v1", weight_source: { type: synthetic ? "synthetic_test" : "user_preference" }, weights: JSON.parse(weights) as Record<string, number>, known_unmodeled: [] },
        layout: { slots, hands: mainHand || offHand ? { main_hand: mainHand, off_hand: offHand } : null },
        class: className ? known(className) : unknown(), proficiency: { classes: JSON.parse(proficiency) as GearRequest["proficiency"]["classes"] },
        candidates: inputs.map((input) => ({ ...input, acquisition: input.acquisition ? { source: input.acquisition.source, effort: {
          incremental_travel_ms: numberObservation(times.travel), formation_ms: numberObservation(times.formation), attempt_ms: numberObservation(times.attempt), stopping_ms: numberObservation(times.stopping),
          available_copper: times.copper.trim() ? known(times.copper) : unknown(), session_budget_ms: Number(times.budget), max_attempts: Number(times.attempts), independent_player_attempts: independent, cost_per_attempt: perAttempt,
        } } : null })),
      } satisfies GearRequest;
  }
  async function calculate() {
    if (!catalog?.snapshot_id || !catalog.revision) return;
    const ticket = ++generation.current; setBusy(true); setReport(null); setError("");
    try {
      const request = gearRequest();
      if (inputs.length === 0 || times.budget.trim() === "" || !Object.keys(request.model.weights).length) throw new Error("Select candidates and explicitly supply weights and session budget.");
      const result = await client.compareGear(JSON.stringify(request));
      if (ticket !== generation.current) return;
      if (!result.ok) setError(result.error.message); else { setReport(result.value); setReportBasis("manual"); }
    } catch (failure) { if (ticket === generation.current) setError(failure instanceof Error ? failure.message : "Invalid manual inputs"); }
    finally { if (ticket === generation.current) setBusy(false); }
  }
  return <section aria-label="Gear"><h2>Gear comparisons</h2>
    <p>Model scores use your explicit assumptions. No class, proficiency, category or weight table is supplied. Unknown acquisition or baseline inputs prevent a practical recommendation.</p>
    {catalog?.fixture && <p role="note">SYNTHETIC — engineering fixture, not live game advice.</p>}
    {error && <p role="alert">{error}</p>}
    <label>Gear profile <select value={profile} onChange={(e) => { invalidate(); setProfile(e.target.value); }}>{profiles.map((p) => <option key={p.id} value={p.id}>{p.display_name}</option>)}</select></label>
    <p>Snapshot: {catalog?.snapshot_id ?? "none"}; revision: {catalog?.revision ?? "none — activate Content first"}</p>
    <RouteAcquisitionPanel client={client} profileId={profile} refreshKey={refreshKey} scopeKey={JSON.stringify({weights,role,synthetic,className,proficiency,mainHand,offHand,inputs,times,independent,perAttempt})}
      gearAssumptions={() => { const request=gearRequest(); if(!inputs.length || !Object.keys(request.model.weights).length) throw new Error("Select candidates and supply explicit model weights first."); return request; }}
      onGearReport={routeReport} onBusyChange={setBusy} externalBusy={busy} opportunities={inputs.flatMap(input=>input.acquisition?[{source:input.acquisition.source,target:input.item,formation_ms:numberObservation(times.formation),attempt_ms:numberObservation(times.attempt),available_copper:times.copper.trim()?known(times.copper):unknown<string>(),max_attempts:Number(times.attempts),independent_player_attempts:independent,cost_per_attempt:perAttempt}]:[])} />
    <button type="button" disabled={!catalog?.fixture} onClick={example}>Load labelled synthetic example</button>
    {synthetic && <p>Engineering example: all readable attributes have equal weight 1; categories are explicitly assumed inapplicable, effort is fictional and sources remain unselected. Edit each assumption before use.</p>}
    <fieldset><legend>Manual model</legend>
      <label>Role identifier <input value={role} onChange={(e) => { invalidate(); setRole(e.target.value); }} /></label>
      <label>Attribute weights (JSON, integer thousandths) <textarea value={weights} onChange={(e) => { invalidate(); setWeights(e.target.value); }} rows={4} /></label>
      <label>Assumed class identifier <input value={className} onChange={(e) => { invalidate(); setClassName(e.target.value); }} /></label>
      <label>Class proficiency map (JSON allowed/disallowed categories) <textarea value={proficiency} onChange={(e) => { invalidate(); setProficiency(e.target.value); }} rows={3} /></label>
      {(["Main-hand slot", "Off-hand slot"] as const).map((label, index) => <label key={label}>{label}<select value={index === 0 ? mainHand : offHand} onChange={(e) => { invalidate(); if (index === 0) setMainHand(e.target.value); else setOffHand(e.target.value); }}><option value="">Relationship unknown / inapplicable</option>{slots.map((slot) => <option key={slot}>{slot}</option>)}</select></label>)}
    </fieldset>
    <fieldset><legend>Acquisition assumptions for each selected source</legend><p>Empty effort fields remain unknown. Spending ceiling is your declared budget. Probabilities come from active content; player trial semantics require your explicit assertion.</p>
      {(Object.keys(times) as (keyof typeof times)[]).map((key) => <label key={key}>{key === "copper" ? "Spending ceiling (copper)" : key === "attempts" ? "Maximum trials (1–1024)" : `${key} (ms)`}<input type="number" min="0" step="1" value={times[key]} onChange={(e) => { invalidate(); setTimes((old) => ({ ...old, [key]: e.target.value })); }} /></label>)}
      <label><input type="checkbox" checked={independent} onChange={(e) => { invalidate(); setIndependent(e.target.checked); }} />Assume independent identical player acquisition trials</label>
      <label><input type="checkbox" checked={perAttempt} onChange={(e) => { invalidate(); setPerAttempt(e.target.checked); }} />Pay source cost per attempt</label>
    </fieldset>
    <h3>Active candidates ({catalog?.items.length ?? 0})</h3>
    {catalog?.items.length === 0 && <p>No items in active content. Real catalogs require explicit authoring.</p>}
    {catalog?.items.map((item) => {
      const index = inputs.findIndex((i) => JSON.stringify(i.item) === JSON.stringify(item.key)); const selected = inputs[index];
      const update = (patch: Partial<GearInput>) => { invalidate(); setInputs((old) => old.map((input,i) => i === index ? { ...input, ...patch } : input)); };
      return <fieldset key={JSON.stringify(item.key)}><legend>{item.key.item_id} ({item.key.variant})</legend>
        <label><input type="checkbox" checked={!!selected} onChange={(e) => { invalidate(); setInputs((old) => e.target.checked ? [...old, { item: item.key, category: unknown(), acquisition: null }] : old.filter((_,i) => i !== index)); }} />Compare {item.key.item_id}</label>
        {selected && <><label>Category status for {item.key.item_id}<select value={selected.category.status} onChange={(e) => update({ category: e.target.value === "known" ? known("") : { status: e.target.value as "unknown" | "unsupported" | "restricted" | "not_applicable" } })}>{["unknown","unsupported","restricted","not_applicable","known"].map((status) => <option key={status}>{status}</option>)}</select></label>
          {selected.category.status === "known" && <label>Category identifier for {item.key.item_id}<input value={selected.category.value} onChange={(e) => update({ category: known(e.target.value) })} /></label>}
          <label>Acquisition source for {item.key.item_id}<select value={selected.acquisition?.source ?? ""} onChange={(e) => update({ acquisition: e.target.value ? { source: e.target.value, effort: { incremental_travel_ms: unknown(), formation_ms: unknown(), attempt_ms: unknown(), stopping_ms: unknown(), available_copper: unknown(), session_budget_ms: 0, max_attempts: 1, independent_player_attempts: false, cost_per_attempt: false } } : null })}><option value="">Unknown / unselected</option>{catalog.acquisitions.filter((source) => source.items.some((key) => JSON.stringify(key) === JSON.stringify(item.key))).map((source) => <option key={source.id}>{source.id}</option>)}</select></label>
        </>}
      </fieldset>;
    })}
    <button type="button" disabled={busy || !catalog?.snapshot_id || !catalog.revision} onClick={() => void calculate()}>Compare gear</button>
    <details><summary>Captured equipment and variant/modifier gaps</summary><pre>{JSON.stringify(catalog?.equipment ?? {}, null, 2)}</pre></details>
    {report && <section aria-label="Gear result"><h3>Current baseline</h3><p>Model score {report.current_score.total / 1000}; {report.current_score.certainty}.</p>
      <ul>{report.baseline_gaps.map((gap) => <li key={gap}>{gap}</li>)}</ul><details><summary>Baseline slots and attribute contributions</summary><pre>{JSON.stringify({ slots: report.current, score: report.current_score }, null, 2)}</pre></details>
      <p>{reportBasis === "route" ? "Gear comparison uses backend-calculated route effort and its remaining budget." : "Gear comparison uses your manual effort assumptions."}</p>
      <RankingView name="Theoretical comparison" ranking={report.theoretical} /><RankingView name="Equipment eligibility" ranking={report.equip_eligible} /><RankingView name="Guaranteed individual acquisition" ranking={report.obtainable} />
      <h3>Practical single-acquisition estimates ({report.practical_model_version})</h3><p>Conditional expected positive model gain per reserved effort. No combined acquisition probabilities or useful lifespan.</p>
      {report.practical.length === 0 && <p>No definite practical recommendation. Inspect the missing inputs and rejection reasons below.</p>}
      <ol>{report.practical.map((upgrade) => <li key={`${JSON.stringify(upgrade.item)}:${upgrade.slot}`}>{upgrade.item.item_id}: {upgrade.conditional ? "Conditional estimate" : "Guaranteed acquisition under supplied assumptions"}; positive gain {upgrade.gain_milli / 1000}; expected gain {upgrade.conditional_expected_gain_milli / 1000}; reserved effort {upgrade.reserved_effort_ms} ms; probability {upgrade.probability_millionths / 1000000}.</li>)}</ol>
      {report.candidates.map((candidate) => <section key={JSON.stringify(candidate.item)} aria-label={`Comparison ${candidate.item.item_id}`}><h4>{itemId(candidate)}</h4><p>Eligibility: {candidate.eligibility.outcome}; acquisition: {candidate.acquisition?.status ?? "unknown"}; model score {candidate.score.total / 1000} ({candidate.score.certainty}).</p><ul>{candidate.practical_reasons.map((reason) => <li key={reason}>{reason}</li>)}</ul><details><summary>Score, progression and acquisition details</summary><pre>{JSON.stringify(candidate, null, 2)}</pre></details></section>)}
      <ul>{report.assumptions.map((assumption) => <li key={assumption}>{assumption}</li>)}</ul>
    </section>}
  </section>;
}
