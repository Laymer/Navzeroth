import "./play.css";
import { describeObservation } from "../observation";
import { Button, Field, HelpTip, OriginMarker, Panel } from "../ui";
import { useRecordOrigins } from "./useRecordOrigins";
import { originKey } from "../commands/world";
import { CatalogPager, OffPageOptions } from "./CatalogPager";
import {AuthoredContentPicker} from "./AuthoredContentPicker";
import type {AuthoredSelection} from "../commands/authoredContent";
import { AcquisitionDetails, BaselineSlots, EligibilityDetails, RankingDetails, ScoreDetails, formatMilli } from "./GearExplanations";
import { RouteAcquisitionPanel } from "./RouteAcquisitionPanel";
import { useCallback, useEffect, useRef, useState } from "react";
import type { CommandClient } from "../commands";
import type { ProfileSummary } from "../commands/contract";
import type { AuthoredScores, GearCatalog, GearInput, GearObservation, GearReport, GearRequest, GearRanking } from "../commands/gear";

const TIME_HELP: Record<string, string> = {
  travel: "Extra travel time to reach the source, in milliseconds. Empty stays unknown and blocks a practical recommendation.",
  formation: "Time to form the group, in milliseconds. Empty stays unknown.",
  attempt: "Time for one attempt, in milliseconds. Empty stays unknown.",
  stopping: "Time to get back to your stopping point, in milliseconds. Empty stays unknown.",
  budget: "Your session time limit in milliseconds; 1000 ms is one second. A budget is required to compare.",
  copper: "The most copper you declare you will spend. Empty stays unknown; 0 means you declared you will spend nothing.",
  attempts: "How many trials you allow for each source, from 1 to 1024.",
};
const CATEGORY_HELP = "The item's armour or weapon category as you declare it; the item data has none. Choose known and enter its identifier to check it against the class proficiency table. not_applicable means no category rule applies, for example jewellery. unknown, unsupported and restricted stay distinct and each leaves eligibility unconfirmed. Version 2 content replaces this choice with its own equipment facts (unknown when it has none for the item).";
const SOURCE_HELP = "Where you plan to obtain this item, from the active content's sources for it. Unknown / unselected leaves the acquisition unknown, which prevents a practical recommendation; nothing is chosen for you.";
const unknown = <T,>(): GearObservation<T> => ({ status: "unknown" });
const known = <T,>(value: T): GearObservation<T> => ({ status: "known", value });
const numberObservation = (value: string): GearObservation<number> => value.trim() === "" ? unknown() : known(Number(value));
const itemId = (input: Pick<GearInput, "item">) => `${input.item.item_id} (${input.item.variant})`;
function RankingView({ name, ranking }: { name: string; ranking: GearRanking }) {
  return <Panel title={name} level={3}><p>{ranking.label}. {ranking.scope_note}</p>
    <p>{ranking.top_is_definitive ? "Definitive ordering within the supplied model" : "Ordering is not definitive"}. {ranking.valid.length} legal, {ranking.uncertain.length} uncertain, {ranking.rejected.length} rejected configurations.</p>
    {ranking.valid.slice(0, 5).map((configuration) => <p key={configuration.rank}>#{configuration.rank}: {Object.entries(configuration.configuration.items).map(([slot, item]) => `${slot}: ${item.item.item_id} (${item.item.variant})`).join(", ")}; model score {formatMilli(configuration.score.total)}; {configuration.score.certainty}.</p>)}
    <details><summary>Configuration legality, exclusions and score breakdowns</summary><RankingDetails ranking={ranking} /></details>
  </Panel>;
}
export function GearScreen({ client, refreshKey }: { client: CommandClient; refreshKey: number }) {
  const [profiles, setProfiles] = useState<ProfileSummary[]>([]);
  const [profile, setProfile] = useState("");
  const previousCatalogBinding=useRef("");
  const [catalog, setCatalog] = useState<GearCatalog | null>(null);
  const [authored,setAuthored]=useState<AuthoredSelection|null>(null);
  const [inputs, setInputs] = useState<GearInput[]>([]);
  const [role, setRole] = useState("manual-role");
  const [assumeScoreRole, setAssumeScoreRole] = useState(false);
  const [scoreRole, setScoreRole] = useState("");
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
  const [scores, setScores] = useState<AuthoredScores | null>(null);
  const [error, setError] = useState("");
  const [busy, setBusy] = useState(false);
  const generation = useRef(0);
  const scope = useRef<{ client: CommandClient; profile: string } | null>(null);
  const routeReport = useCallback((value: GearReport | null) => { generation.current++; setReport(value); setScores(null); setReportBasis(value ? "route" : null); }, []);
  const invalidate = useCallback(() => { generation.current++; setReport(null); setScores(null); setBusy(false); }, []);
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
    let live = true; generation.current++; setReport(null); setScores(null); setBusy(false); setCatalog(null);
    const changedScope = scope.current?.client !== client || scope.current?.profile !== profile;
    scope.current = { client, profile };
    if (changedScope) {
      setInputs([]); setAuthored(null); setAssumeScoreRole(false); setScoreRole(""); setSynthetic(false); setWeights("{}"); setRole("manual-role"); setClassName(""); setProficiency("{}"); setMainHand(""); setOffHand("");
      setTimes({ travel: "", formation: "", attempt: "", stopping: "", budget: "", copper: "", attempts: "1" }); setIndependent(false); setPerAttempt(false);
    }
    if (profile) void client.getGearCatalog(profile).then((result) => {
      if (!live) return;
      if (!result.ok) { setError(result.error.message); return; }
      setCatalog(result.value); setError("");
      const binding=JSON.stringify([profile,result.value.revision,result.value.pages]);
      if(result.value.pages&&previousCatalogBinding.current===binding)return;
      previousCatalogBinding.current=binding;
      const availableSlots = new Set([...result.value.items.flatMap((item) => item.slots), ...Object.keys(result.value.equipment)]);
      setMainHand((old) => availableSlots.has(old) ? old : ""); setOffHand((old) => availableSlots.has(old) ? old : "");
      setInputs((old) => old.filter((input) => result.value.items.some((item) => JSON.stringify(item.key) === JSON.stringify(input.item))).map((input) => ({
        ...input, acquisition: input.acquisition && result.value.acquisitions.some((source) => source.id === input.acquisition?.source && source.items.some((item) => JSON.stringify(item) === JSON.stringify(input.item))) ? input.acquisition : null,
      })));
    });
    return () => { live = false; generation.current++; };
  }, [client, profile, refreshKey]);
  const slots = [...new Set([...(catalog?.items.flatMap((i) => i.slots) ?? []), ...Object.keys(catalog?.equipment ?? {})])].sort();
  const itemRef = (key: GearCatalog["items"][number]["key"]) => ({ collection: "items" as const, id: key.item_id, variant: key.variant });
  const scoreOrigins = useRecordOrigins(client, profile ? { kind: "character", profile_id: profile } : null, scores?.request.revision, scores?.candidates.map(candidate => itemRef(candidate.item)) ?? []);
  const itemOrigins = useRecordOrigins(client, profile ? { kind: "character", profile_id: profile } : null, catalog?.revision, catalog?.items.map((item) => itemRef(item.key)) ?? []);
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
    if(authored && authored.revision!==catalog.revision) throw new Error("Authored model revision changed; select it again.");
    if (assumeScoreRole && !scoreRole) throw new Error("Choose a declared role for individual heuristic scores.");
    return {
        ...(assumeScoreRole ? {score_role_assumption:scoreRole} : {}),
        authored_model: authored?.id??null,
        profile_id: profile, snapshot_id: catalog.snapshot_id, revision: catalog.revision,
        model: { role, version: "gear-model-v1", weight_source: { type: synthetic ? "synthetic_test" : "user_preference" }, weights: JSON.parse(weights) as Record<string, number>, known_unmodeled: [] },
        layout: { slots, hands: mainHand || offHand ? { main_hand: mainHand, off_hand: offHand } : null },
        class: className ? known(className) : unknown(), proficiency: { classes: JSON.parse(proficiency) as GearRequest["proficiency"]["classes"] },
        candidates: inputs.map((input) => ({ ...input, acquisition: input.acquisition && !assumeScoreRole ? { source: input.acquisition.source, effort: {
          incremental_travel_ms: numberObservation(times.travel), formation_ms: numberObservation(times.formation), attempt_ms: numberObservation(times.attempt), stopping_ms: numberObservation(times.stopping),
          available_copper: times.copper.trim() ? known(times.copper) : unknown(), session_budget_ms: Number(times.budget), max_attempts: Number(times.attempts), independent_player_attempts: independent, cost_per_attempt: perAttempt,
        } } : null })),
      } satisfies GearRequest;
  }
  async function calculate() {
    if (!catalog?.snapshot_id || !catalog.revision) return;
    const ticket = ++generation.current; setBusy(true); setReport(null); setScores(null); setError("");
    try {
      const request = gearRequest();
      if (inputs.length === 0 || (times.budget.trim() === "" && !(authored && assumeScoreRole)) || (!authored && !Object.keys(request.model.weights).length)) throw new Error("Select candidates and explicitly supply weights and session budget.");
      const result = await client.compareGear(JSON.stringify(request));
      if (ticket !== generation.current) return;
      if (!result.ok) setError(result.error.message); else if (result.value.mode === "authored_score_only_v1") { setScores(result.value); } else { setReport(result.value); setReportBasis("manual"); }
    } catch (failure) { if (ticket === generation.current) setError(failure instanceof Error ? failure.message : "Invalid manual inputs"); }
    finally { if (ticket === generation.current) setBusy(false); }
  }
  return <section aria-label="Gear" className="play-screen"><h2>Gear comparisons</h2>
    <p>Model scores use your explicit assumptions. Select an installed authored model or supply a manual model. Class is an explicit assumption because imported snapshots do not contain class facts. Unknown acquisition or baseline inputs prevent a practical recommendation.</p>
    {catalog?.fixture && <p role="note" className="play-note">SYNTHETIC — engineering fixture, not live game advice.</p>}
    {error && <p role="alert">{error}</p>}
    <Field label="Gear profile" help="The imported character whose equipment snapshot is the baseline. Changing it discards the current comparison.">{(c) => <select {...c} value={profile} onChange={(e) => { invalidate(); setProfile(e.target.value); }}>{profiles.map((p) => <option key={p.id} value={p.id}>{p.display_name}</option>)}</select>}</Field>
    <p>Snapshot: {catalog?.snapshot_id ?? "none"}; revision: {catalog?.revision ?? "none — activate Content first"}</p>
    <AuthoredContentPicker client={client} kind="models" subject={{kind:"character",profile_id:profile}} refreshKey={refreshKey} value={authored} onChange={v=>{invalidate();setAuthored(v);setAssumeScoreRole(false);setScoreRole("");}}/>
    {authored&&<p>Authored weights are documented heuristics. A matching assumed class, fresh level and applicable role are required. When source layout or proficiency is unavailable, only individual scores are shown; equipment eligibility and acquisition remain unresolved.</p>}
    {authored && <fieldset><legend>Role assumption for individual scores</legend>
      <label><input type="checkbox" checked={assumeScoreRole} onChange={event=>{invalidate();setAssumeScoreRole(event.target.checked);}}/>Use a declared role for individual heuristic scores</label>
      <Field label="Declared role for individual scores" help="A separate hypothetical role for arithmetic only. It does not change the imported role or enable equipment or practical recommendations.">{c=><select {...c} disabled={!assumeScoreRole} value={scoreRole} onChange={event=>{invalidate();setScoreRole(event.target.value);}}><option value="">Choose a role explicitly</option><option value="melee_damage">Melee damage</option><option value="ranged_damage">Ranged damage</option><option value="healer">Healer</option><option value="tank">Tank</option></select>}</Field>
      <p>Model selection alone never supplies a captured role. This assumption is used only for individual scores; imported observations remain unchanged. A matching assumed class and fresh level are still required.</p>
    </fieldset>}
    <RouteAcquisitionPanel client={client} profileId={profile} refreshKey={refreshKey} scopeKey={JSON.stringify({authored,weights,role,assumeScoreRole,scoreRole,synthetic,className,proficiency,mainHand,offHand,inputs,times,independent,perAttempt})}
      gearAssumptions={() => { const request=gearRequest(); if(request.score_role_assumption) throw new Error("Declared-role assumptions apply only to individual heuristic scores. Turn it off for route or practical comparisons."); if(!inputs.length || (!authored && !Object.keys(request.model.weights).length)) throw new Error("Select candidates and supply explicit model weights first."); return request; }}
      onGearReport={routeReport} onBusyChange={setBusy} externalBusy={busy} opportunities={inputs.flatMap(input=>input.acquisition?[{source:input.acquisition.source,target:input.item,formation_ms:numberObservation(times.formation),attempt_ms:numberObservation(times.attempt),available_copper:times.copper.trim()?known(times.copper):unknown<string>(),max_attempts:Number(times.attempts),independent_player_attempts:independent,cost_per_attempt:perAttempt}]:[])} />
    <Button disabled={!catalog?.fixture} onClick={example}>Load labelled synthetic example</Button>
    {synthetic && <p>Engineering example: all readable attributes have equal weight 1; categories are explicitly assumed inapplicable, effort is fictional and sources remain unselected. Edit each assumption before use.</p>}
    <fieldset><legend>Manual model</legend>
      <Field label="Role identifier" help="A name you choose for this weighting. It labels the model; it does not change any score. Ignored while an authored model is selected; the model brings its own role.">{(c) => <input {...c} value={role} onChange={(e) => { invalidate(); setRole(e.target.value); }} />}</Field>
      <Field label="Attribute weights (JSON, integer thousandths)" help="Maps attribute ids to weights in thousandths, so 1000 means a weight of 1. An empty map blocks a manual comparison. While an authored model is selected its weights replace these, but the text must still be valid JSON.">{(c) => <textarea {...c} value={weights} onChange={(e) => { invalidate(); setWeights(e.target.value); }} rows={4} />}</Field>
      <Field label="Assumed class identifier" help="Imported snapshots hold no class facts, so the class is your assumption. Leave it empty to keep the class unknown. It is still used with an authored model: it must match that model's class or the comparison is refused.">{(c) => <input {...c} value={className} onChange={(e) => { invalidate(); setClassName(e.target.value); }} />}</Field>
      <Field label="Class proficiency map (JSON allowed/disallowed categories)" help="Used to judge equipment eligibility. Leave it as {} to supply no proficiency facts; eligibility then stays unconfirmed. While an authored model is selected its proficiency table replaces this, but the text must still be valid JSON.">{(c) => <textarea {...c} value={proficiency} onChange={(e) => { invalidate(); setProficiency(e.target.value); }} rows={3} />}</Field>
      {(["Main-hand slot", "Off-hand slot"] as const).map((label, index) => <Field key={label} label={label} help="The slot that holds this hand's weapon. Choose Relationship unknown / inapplicable when you do not know; nothing is inferred. Ignored while an authored model is selected; the model brings its own slot layout.">{(c) => <select {...c} value={index === 0 ? mainHand : offHand} onChange={(e) => { invalidate(); if (index === 0) setMainHand(e.target.value); else setOffHand(e.target.value); }}><OffPageOptions selected={[mainHand,offHand]} visible={slots}/><option value="">Relationship unknown / inapplicable</option>{slots.map((slot) => <option key={slot}>{slot}</option>)}</select>}</Field>)}
    </fieldset>
    <fieldset><legend>Acquisition assumptions for each selected source</legend><p>Empty effort fields remain unknown. Spending ceiling is your declared budget. Probabilities come from active content; player trial semantics require your explicit assertion.</p>
      {(Object.keys(times) as (keyof typeof times)[]).map((key) => <Field key={key} label={key === "copper" ? "Spending ceiling (copper)" : key === "attempts" ? "Maximum trials (1–1024)" : `${key} (ms)`} help={TIME_HELP[key]}>{(c) => <input {...c} type="number" min="0" step="1" value={times[key]} onChange={(e) => { invalidate(); setTimes((old) => ({ ...old, [key]: e.target.value })); }} />}</Field>)}
      <span className="play-inline"><label><input type="checkbox" checked={independent} onChange={(e) => { invalidate(); setIndependent(e.target.checked); }} />Assume independent identical player acquisition trials</label><HelpTip>Tick this only if every attempt is a fresh, identical chance. Unticked, repeated attempts are not combined into one probability.</HelpTip></span>
      <span className="play-inline"><label><input type="checkbox" checked={perAttempt} onChange={(e) => { invalidate(); setPerAttempt(e.target.checked); }} />Pay source cost per attempt</label><HelpTip>Tick this if each attempt costs the source price again. Unticked, the price counts once.</HelpTip></span>
    </fieldset>
    <CatalogPager disabled={busy} client={client} subject={{kind:"character",profile_id:profile}} page={catalog?.pages?.items} label="items" onPage={p=>{setCatalog(old=>old?{...old,items:p.records as NonNullable<typeof catalog>["items"],pages:{...old.pages,items:p}}:old);}}/>
<CatalogPager disabled={busy} client={client} subject={{kind:"character",profile_id:profile}} page={catalog?.pages?.acquisitions} label="acquisitions" onPage={p=>{setCatalog(old=>old?{...old,acquisitions:p.records as NonNullable<typeof catalog>["acquisitions"],pages:{...old.pages,acquisitions:p}}:old);}}/>
<p>Selected candidates: {inputs.length} / 128</p>{inputs.filter(i=>!catalog?.items.some(item=>JSON.stringify(item.key)===JSON.stringify(i.item))).map(i=><p key={JSON.stringify(i.item)}>{i.item.item_id} (another page) <Button variant="ghost" onClick={()=>{invalidate();setInputs(old=>old.filter(v=>v!==i));}}>Remove {i.item.item_id}</Button></p>)}
    <h3>Active candidates ({catalog?.items.length ?? 0})</h3>
    {catalog?.items.length === 0 && <p>No items in active content. Real catalogs require explicit authoring.</p>}
    {catalog?.items.map((item) => {
      const index = inputs.findIndex((i) => JSON.stringify(i.item) === JSON.stringify(item.key)); const selected = inputs[index];
      const update = (patch: Partial<GearInput>) => { invalidate(); setInputs((old) => old.map((input,i) => i === index ? { ...input, ...patch } : input)); };
      return <fieldset key={JSON.stringify(item.key)}><legend>{item.key.item_id} ({item.key.variant})</legend><OriginMarker marks={itemOrigins.get(originKey(itemRef(item.key)))} />
        <label><input type="checkbox" checked={!!selected} disabled={!selected&&inputs.length>=128} onChange={(e) => { invalidate(); setInputs((old) => e.target.checked ? [...old, { item: item.key, category: unknown(), acquisition: null }] : old.filter((_,i) => i !== index)); }} />Compare {item.key.item_id}</label>
        {selected && <><span className="play-inline"><label>Category status for {item.key.item_id}<select value={selected.category.status} onChange={(e) => update({ category: e.target.value === "known" ? known("") : { status: e.target.value as "unknown" | "unsupported" | "restricted" | "not_applicable" } })}>{["unknown","unsupported","restricted","not_applicable","known"].map((status) => <option key={status}>{status}</option>)}</select></label><HelpTip label={`Category status for ${item.key.item_id}`}>{CATEGORY_HELP}</HelpTip></span>
          {selected.category.status === "known" && <label>Category identifier for {item.key.item_id}<input value={selected.category.value} onChange={(e) => update({ category: known(e.target.value) })} /></label>}
          <span className="play-inline"><label>Acquisition source for {item.key.item_id}<select value={selected.acquisition?.source ?? ""} onChange={(e) => update({ acquisition: e.target.value ? { source: e.target.value, effort: { incremental_travel_ms: unknown(), formation_ms: unknown(), attempt_ms: unknown(), stopping_ms: unknown(), available_copper: unknown(), session_budget_ms: 0, max_attempts: 1, independent_player_attempts: false, cost_per_attempt: false } } : null })}><OffPageOptions selected={[selected.acquisition?.source??""]} visible={catalog.acquisitions.map(a=>a.id)}/><option value="">Unknown / unselected</option>{catalog.acquisitions.filter((source) => source.items.some((key) => JSON.stringify(key) === JSON.stringify(item.key))).map((source) => <option key={source.id}>{source.id}</option>)}</select></label><HelpTip label={`Acquisition source for ${item.key.item_id}`}>{SOURCE_HELP}</HelpTip></span>
        </>}
      </fieldset>;
    })}
    <Button variant="primary" disabled={busy || !catalog?.snapshot_id || !catalog.revision} onClick={() => void calculate()}>Compare gear</Button>
    <details><summary>Advanced captured-equipment data</summary><pre>{JSON.stringify(catalog?.equipment ?? {}, null, 2)}</pre></details>
    {scores && <section aria-label="Individual heuristic scores"><h3>Individual heuristic scores</h3>
      <p>These scores compare only the attributes in the authored model. They do not establish whether you can equip or obtain an item. Equipment configurations, baseline gains and practical recommendations are unavailable.</p>
      <p>Imported role: {describeObservation(scores.imported_role)}. {scores.role_assumption ? `Declared role assumption: ${scores.role_assumption} (individual scores only).` : "No declared role assumption was used."}</p>
      <p>Source equipment layout: {describeObservation(scores.source_layout, () => "recorded")}; source class proficiency: {describeObservation(scores.source_proficiency, () => "recorded")}.</p>
      <p>Equipment eligibility: not evaluated. Acquisition availability: not evaluated.</p>
      {scores.candidates.map(candidate => <section key={JSON.stringify(candidate.item)} aria-label={`Heuristic score ${candidate.item.item_id}`}><h4>{candidate.item.item_id} ({candidate.item.variant})<OriginMarker marks={scoreOrigins.get(originKey(itemRef(candidate.item)))} /></h4><p>Heuristic score {formatMilli(candidate.score.total)}; certainty {candidate.score.certainty}.</p><ScoreDetails score={candidate.score} /></section>)}
      <ul>{scores.assumptions.map(assumption => <li key={assumption}>{assumption}</li>)}</ul>
    </section>}
    {report && <section aria-label="Gear result"><h3>Current baseline</h3><p>Model score {formatMilli(report.current_score.total)}; {report.current_score.certainty}.</p>
      <ul>{report.baseline_gaps.map((gap) => <li key={gap}>{gap}</li>)}</ul><BaselineSlots current={report.current} /><ScoreDetails score={report.current_score} />
      <p>{reportBasis === "route" ? "Gear comparison uses backend-calculated route effort and its remaining budget." : "Gear comparison uses your manual effort assumptions."}</p>
      <RankingView name="Theoretical comparison" ranking={report.theoretical} /><RankingView name="Equipment eligibility" ranking={report.equip_eligible} /><RankingView name="Guaranteed individual acquisition" ranking={report.obtainable} />
      <h3>Practical single-acquisition estimates ({report.practical_model_version})</h3><p>Conditional expected positive model gain per reserved effort. No combined acquisition probabilities or useful lifespan.</p>
      {report.practical.length === 0 && <p>No definite practical recommendation. Inspect the missing inputs and rejection reasons below.</p>}
      <ol>{report.practical.map((upgrade) => <li key={`${JSON.stringify(upgrade.item)}:${upgrade.slot}`}>{upgrade.item.item_id}: {upgrade.conditional ? "Conditional estimate" : "Guaranteed acquisition under supplied assumptions"}; positive gain {formatMilli(upgrade.gain_milli)}; expected gain {formatMilli(upgrade.conditional_expected_gain_milli)}; reserved effort {upgrade.reserved_effort_ms} ms; probability {upgrade.probability_millionths / 1000000}.</li>)}</ol>
      {report.candidates.map((candidate) => <section key={JSON.stringify(candidate.item)} aria-label={`Comparison ${candidate.item.item_id}`}><h4>{itemId(candidate)}</h4><p>Eligibility: {candidate.eligibility.outcome}; acquisition: {candidate.acquisition?.status ?? "unknown"}; model score {formatMilli(candidate.score.total)} ({candidate.score.certainty}).</p><ul>{candidate.practical_reasons.map((reason) => <li key={reason}>{reason}</li>)}</ul><details><summary>Score, progression and acquisition details</summary><EligibilityDetails eligibility={candidate.eligibility} /><ScoreDetails score={candidate.score} /><AcquisitionDetails acquisition={candidate.acquisition} /></details></section>)}
      <ul>{report.assumptions.map((assumption) => <li key={assumption}>{assumption}</li>)}</ul>
    </section>}
  </section>;
}
