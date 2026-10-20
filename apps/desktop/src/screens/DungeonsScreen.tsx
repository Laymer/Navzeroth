import "./play.css";
import { Button, Field, OriginMarker } from "../ui";
import { useRecordOrigins, type Origins } from "./useRecordOrigins";
import { originKey, type OriginClient, type OriginRef } from "../commands/world";
import type { CatalogSubject } from "../commands/catalogPages";
import {AuthoredContentPicker} from "./AuthoredContentPicker";
import type {AuthoredSelection} from "../commands/authoredContent";
import { useEffect, useRef, useState } from "react";
import type { CommandClient } from "../commands";
import type { ProfileSummary } from "../commands/contract";
import type { ConditionAssessment, Declared, PreparationReport } from "../commands/dungeonPreparation";
import { describeObservation } from "../observation";
import example from "../../../../fixtures/desktop/preparation/synthetic-package.json";
import "./dungeonPreparation.css";

export function DungeonsScreen({client,refreshKey}:{client:CommandClient;refreshKey:number}) {
  const [profiles,setProfiles]=useState<ProfileSummary[]>([]);
  const [parties,setParties]=useState<string[]>([]);
  const [kind,setKind]=useState<"character"|"party">("character");
  const [selected,setSelected]=useState("");
  const [authored,setAuthored]=useState<AuthoredSelection|null>(null);
  const [text,setText]=useState("");
  const [origin,setOrigin]=useState("");
  const [error,setError]=useState("");
  const [busy,setBusy]=useState<number|null>(null);
  const [report,setReport]=useState<{ticket:number;value:PreparationReport}|null>(null);
  const epoch=useRef(0);
  const owner=useRef({client,refreshKey,kind,selected,text,origin,authored});
  if(owner.current.client!==client || owner.current.refreshKey!==refreshKey || owner.current.kind!==kind || owner.current.selected!==selected || owner.current.text!==text || owner.current.origin!==origin || owner.current.authored!==authored) {
    owner.current={client,refreshKey,kind,selected,text,origin,authored};epoch.current++;
  }
  useEffect(()=>{let live=true;
    void Promise.all([client.listProfiles(),client.listParties()]).then(([p,g])=>{
      if(!live)return;
      if(p.ok)setProfiles(p.value);else setError(p.error.message);
      if(g.ok)setParties(g.value);else setError(g.error.message);
const options=owner.current.kind==="party"?(g.ok?g.value:[]):(p.ok?p.value.filter(v=>v.data_mode==="stored").map(v=>v.id):[]);
setSelected(old=>options.includes(old)?old:options[0]??"");
    });return()=>{live=false;epoch.current++;};
  },[client,refreshKey]);
  const visible=report?.ticket===epoch.current?report.value:null;
  const pending=busy===epoch.current;
  const stored=kind==="party"?parties.includes(selected):profiles.some(p=>p.id===selected&&p.data_mode==="stored");
  async function inspect() {
    let parsed:unknown=null;
    if(text.trim()){try{parsed=JSON.parse(text);}catch{setError("Preparation package JSON is invalid.");return;}}
    const ticket=++epoch.current;setBusy(ticket);setReport(null);setError("");
    const subject=kind==="party"?{kind,party_id:selected}:{kind,profile_id:selected};
    const result=await client.getDungeonPreparation(JSON.stringify({schema_version:1,subject,package:parsed,authored,origin:origin.trim()||null}));
    if(ticket!==epoch.current)return;
    if(result.ok)setReport({ticket,value:result.value});else setError(result.error.message);
    if(ticket===epoch.current)setBusy(null);
  }
  return <section className="dungeon-preparation play-screen" aria-labelledby="dungeons-heading">
    <h2 id="dungeons-heading">Dungeon preparation</h2>
    <p>Select an installed authored dungeon or inspect a local reference. Missing sections remain unavailable; synthetic examples do not establish gameplay facts.</p>
    <Field label="Preparation members" help="Inspect one imported character or a local party you created in Party.">{(c) => <select {...c} value={kind} onChange={e=>{const next=e.target.value as "character"|"party";setKind(next);setSelected(next==="party"?parties[0]??"":profiles.find(p=>p.data_mode==="stored")?.id??"");setError("");}}><option value="character">Imported character</option><option value="party">Local party</option></select>}</Field>
    <Field label={`Selected ${kind}`} help="Only stored imports can be inspected; demo entries have no imported state.">{(c) => <select {...c} value={selected} onChange={e=>{setSelected(e.target.value);setError("");}}><option value="">Select stored {kind}</option>{kind==="party"?parties.map(id=><option key={id} value={id}>{id}</option>):profiles.filter(p=>p.data_mode==="stored").map(p=><option key={p.id} value={p.id}>{p.display_name}</option>)}</select>}</Field>
    {!stored&&<p>Import a character snapshot and optionally create a local party. Browser demo profiles cannot supply current preparation assessments.</p>}
    <AuthoredContentPicker client={client} kind="dungeons" subject={kind==="party"?{kind,party_id:selected}:{kind,profile_id:selected}} refreshKey={refreshKey} value={authored} onChange={v=>{setAuthored(v);setText("");setError("");}}/>
    <details><summary>Advanced local reference editor</summary><p>Local declarations require exact content references. Loading an example does not install verified dungeon coverage.</p>
    <Field label="Local preparation package JSON" help="A local reference you write or load. It is your declaration, not verified dungeon data, and is limited to 1 MiB.">{(c) => <textarea {...c} value={text} maxLength={1048576} rows={8} onChange={e=>{setText(e.target.value);setError("");}}/>}</Field>
    <Button onClick={()=>{setText(JSON.stringify(example,null,2));setOrigin("demo-node-1");setError("");}}>Load synthetic preparation reference</Button>
    <Button variant="ghost" onClick={()=>{setText("");setError("");}}>Clear package</Button>
    <Field label="Manually declared shared origin node" help="Where the group starts, used only to estimate shared travel to the entrance. Leave empty if you declare none.">{(c) => <input {...c} value={origin} onChange={e=>{setOrigin(e.target.value);setError("");}}/>}</Field>
    </details>
    <Button variant="primary" disabled={!stored||pending} onClick={()=>void inspect()}>Inspect preparation gaps</Button>
    <p>Complete reports are limited to 4 MiB. Larger assessments are refused; reference lists are never cut to fit.</p>
    {pending&&<p role="status">Inspecting imported preparation inputs…</p>}
    {error&&<p role="alert">{error}</p>}
    {visible&&<Preparation value={visible} key={report?.ticket} client={client} subject={kind==="party"?{kind,party_id:selected}:{kind,profile_id:selected}}/>}
  </section>;
}
function Condition({value}:{value:ConditionAssessment|null}) {
  if(!value)return <>Unavailable</>;
  if(value.availability==="known"&&!value.evaluation)return <>Known declared requirement; current assessment unavailable</>;
  return <>{value.availability}{value.evaluation&&<>; condition {value.evaluation.truth}
    {value.evaluation.reasons.length>0&&<details><summary>Observation gaps ({value.evaluation.reasons.length})</summary><pre>{JSON.stringify(value.evaluation.reasons.slice(0,8),null,2)}</pre>{value.evaluation.reasons.length>8&&<p>First eight reasons shown.</p>}</details>}
  </>}</>;
}
function List({title,section,checklist=false}:{title:string;section:Declared<string[]>;checklist?:boolean}) {
  return <section><h4>{title}</h4>{section.observation.status==="known"?<ul>{section.observation.value.map((text,index)=><li key={index}>{checklist?<label><input type="checkbox"/>{text}</label>:text}</li>)}</ul>:<p>{describeObservation(section.observation)}</p>}
    {checklist&&<p>Manual local checklist; checking a box does not verify imported state.</p>}</section>;
}
/** Records this report shows whose origin can be marked: the authored dungeon, quests, reward sources and travel edges. */
function preparationRefs(value:PreparationReport):OriginRef[] {
  return [...(value.authored?[{collection:"dungeons" as const,id:value.authored.id}]:[]),
    ...(value.authored_travel?.edges.map(e=>({collection:"edges" as const,id:e.id}))??[])];
}
const mark=(origins:Origins,ref:OriginRef)=><OriginMarker marks={origins.get(originKey(ref))}/>;
function Preparation({value,client,subject}:{value:PreparationReport;client?:Partial<OriginClient>;subject?:CatalogSubject}) {
  const packageValue=value.package;
  const origins=useRecordOrigins(client,subject??null,value.revision,preparationRefs(value));
  return <div>
    {value.fixture&&<p className="demo-label play-note">SYNTHETIC — engineering reference, not verified gameplay coverage</p>}
    <p>Context: {value.context.build.edition}/{value.context.build.build}/{value.context.build.interface}; {value.context.namespace}. Content: {value.revision??"unavailable"}.</p>
    {packageValue&&<><h3>{packageValue.title}{value.authored&&mark(origins,{collection:"dungeons",id:value.authored.id})}</h3><p>{value.authored?"Authored record resolved from the exact active revision; review cited evidence and missing sections.":"Explicit local declaration; cited evidence categories do not independently verify this package."}</p>
      <p>Declared entrance: {packageValue.entrance.observation.status==="known"?packageValue.entrance.observation.value.id:packageValue.entrance.observation.status}.</p>
      <p>Declared group formation: {describeObservation(packageValue.formation_ms.observation,v=>`${v.min_ms} to ${v.max_ms} ms`)}. Declared run estimate: {describeObservation(packageValue.run_ms.observation,v=>`${v.min_ms} to ${v.max_ms} ms`)}. These are separate from measured personal history.</p>
      <List title="Before departure" section={packageValue.before_departure} checklist/><List title="Before entry" section={packageValue.before_entry} checklist/><List title="Reference encounter notes" section={packageValue.encounter_notes}/>
    </>}
    <h3>Coverage and evidence gaps</h3><ul>{value.gaps.map((gap,index)=><li key={index}>{gap.section}: {gap.code.replaceAll("_"," ")}{gap.availability?` (${gap.availability})`:""}</li>)}</ul>
    <h3>Imported member assessments</h3>{value.members.map(member=><MemberPreparation key={member.profile_id} member={member} value={value} client={client} subject={subject}/>)}
    {value.authored_travel&&<section aria-label="Authored travel dependencies"><h3>Authored travel dependencies</h3><p>Availability: {value.authored_travel.declaration.observation.status}. These directed connections are reference facts, not an ordered route or proof of current member access.</p>{value.authored_travel.declaration.observation.status==="known"&&value.authored_travel.edges.length===0&&<p>The authored list is explicitly empty.</p>}<ul>{value.authored_travel.edges.map(edge=><li key={edge.id}>{edge.id}: {edge.from.id} → {edge.to.id} ({edge.mode}){mark(origins,{collection:"edges",id:edge.id})}</li>)}</ul><details><summary>Travel facts and provenance</summary><pre>{JSON.stringify(value.authored_travel,null,2)}</pre></details></section>}
    <h3>Declared entrance travel</h3>{value.route?.outcome.status==="complete"?<p>Estimated shared stateless route: {value.route.outcome.route?.total_ms} ms. Entrance association remains a local declaration.</p>:<p>No actionable supported route is available.</p>}
    <ul>{value.route_exclusions.map((e,index)=><li key={index}>{e.edge}: {e.character??"shared route"}; {e.reason.replaceAll("_"," ")} {e.condition&&<Condition value={e.condition}/>}</li>)}</ul>
    <h3>Referenced evidence and source policy</h3><ul>{value.evidence.map(e=><li key={e.id}>{e.id}: {e.confidence}; source {e.source}; license {describeObservation(e.license)}; redistribution {describeObservation(e.redistribution)}; offline use {describeObservation(e.offline_usable)}.</li>)}</ul>
  </div>;
}
function RewardRole({packageValue,reward}:{packageValue:PreparationReport["package"];reward:PreparationReport["members"][number]["rewards"][number]}) {
  const reference=packageValue?.rewards.observation.status==="known"?packageValue.rewards.observation.value.find(r=>r.source===reward.source&&r.item.item_id===reward.item.item_id&&r.item.variant===reward.item.variant):null;
  return <>{reference?describeObservation(reference.declared_role):"Unavailable"}</>;
}

const PREPARATION_PAGE = 24;
function PageControls({label,offset,total,change}:{label:string;offset:number;total:number;change:(offset:number)=>void}) {
  if(total<=PREPARATION_PAGE)return null;
  return <nav aria-label={`${label} pages`}><p>Showing {offset+1}–{Math.min(offset+PREPARATION_PAGE,total)} of {total} {label}. Every reference is retained; pages do not establish eligibility or a drop guarantee.</p><Button disabled={offset===0} onClick={()=>change(Math.max(0,offset-PREPARATION_PAGE))}>Previous {label}</Button><Button disabled={offset+PREPARATION_PAGE>=total} onClick={()=>change(offset+PREPARATION_PAGE)}>Next {label}</Button></nav>;
}
function MemberPreparation({member,value,client,subject}:{member:PreparationReport["members"][number];value:PreparationReport;client:Partial<OriginClient>|undefined;subject:CatalogSubject|undefined}) {
  const [questOffset,setQuestOffset]=useState(0);
  const [rewardOffset,setRewardOffset]=useState(0);
  const quests=member.quests.slice(questOffset,questOffset+PREPARATION_PAGE);
  const rewards=member.rewards.slice(rewardOffset,rewardOffset+PREPARATION_PAGE);
  const packageValue=value.package;
  const refs:OriginRef[]=[...quests.map(q=>({collection:"quests" as const,id:q.quest})),...rewards.map(r=>({collection:"acquisitions" as const,id:r.source}))];
  const origins=useRecordOrigins(client,subject??null,value.revision,refs);
  return <details><summary>{member.character} — capture {member.freshness?.status??"missing"}</summary>
      <p>Snapshot: {member.snapshot_id??"missing"}. This is imported state.</p><p>Declared access: <Condition value={member.access}/></p>
      <h4>Quest eligibility and prerequisites</h4><PageControls label="quests" offset={questOffset} total={member.quests.length} change={setQuestOffset}/><ul>{quests.map(q=><li key={q.quest}>{q.quest}{mark(origins,{collection:"quests",id:q.quest})}: {q.assessment?<>acceptance {q.assessment.acceptance}; progress {q.assessment.progress}; turn-in {q.assessment.turn_in}<ul>{q.assessment.reasons.slice(0,8).map((r,i)=><li key={i}>{r.phase}: {r.message}</li>)}</ul>{q.assessment.catch_up&&<details><summary>Catch-up prerequisites</summary><pre>{JSON.stringify(q.assessment.catch_up,null,2)}</pre></details>}</>:"Unavailable: missing or nonfresh imported capture"}</li>)}</ul>
      <h4>Declared role, consumable and equipment conditions</h4><ul>{member.checks.map(check=><li key={check.check}>{packageValue?.checks.observation.status==="known"?packageValue.checks.observation.value.find(c=>c.id===check.check)?.label??check.check:check.check}: <Condition value={check.condition}/></li>)}</ul>
      <p>Consumable inventory is unavailable in current snapshots; possession is not inferred.</p>
      <h4>Declared reward references</h4><PageControls label="rewards" offset={rewardOffset} total={member.rewards.length} change={setRewardOffset}/><ul>{rewards.map(reward=><li key={`${reward.source}-${reward.item.item_id}`}>{reward.item.item_id} via {reward.source}{mark(origins,{collection:"acquisitions",id:reward.source})}; declared role label <RewardRole packageValue={packageValue} reward={reward}/>; item requirements <Condition value={reward.item_requirement}/>; source requirements <Condition value={reward.acquisition_requirement}/>; source probability {describeObservation(reward.probability)}; cost {describeObservation(reward.cost)}. No drop guarantee or role benefit is inferred.</li>)}</ul>
    </details>;
}
