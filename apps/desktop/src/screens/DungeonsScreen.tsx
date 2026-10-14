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
  const [text,setText]=useState("");
  const [origin,setOrigin]=useState("");
  const [error,setError]=useState("");
  const [busy,setBusy]=useState<number|null>(null);
  const [report,setReport]=useState<{ticket:number;value:PreparationReport}|null>(null);
  const epoch=useRef(0);
  const owner=useRef({client,refreshKey,kind,selected,text,origin});
  if(owner.current.client!==client || owner.current.refreshKey!==refreshKey || owner.current.kind!==kind || owner.current.selected!==selected || owner.current.text!==text || owner.current.origin!==origin) {
    owner.current={client,refreshKey,kind,selected,text,origin};epoch.current++;
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
    const result=await client.getDungeonPreparation(JSON.stringify({schema_version:1,subject,package:parsed,origin:origin.trim()||null}));
    if(ticket!==epoch.current)return;
    if(result.ok)setReport({ticket,value:result.value});else setError(result.error.message);
    if(ticket===epoch.current)setBusy(null);
  }
  return <section className="dungeon-preparation" aria-labelledby="dungeons-heading">
    <h2 id="dungeons-heading">Dungeon preparation</h2>
    <p>No verified Deadmines package is installed. Local references and synthetic examples do not establish dungeon access, useful rewards or encounter facts.</p>
    <label>Preparation members <select value={kind} onChange={e=>{const next=e.target.value as "character"|"party";setKind(next);setSelected(next==="party"?parties[0]??"":profiles.find(p=>p.data_mode==="stored")?.id??"");setError("");}}><option value="character">Imported character</option><option value="party">Local party</option></select></label>
    <label>Selected {kind} <select value={selected} onChange={e=>{setSelected(e.target.value);setError("");}}><option value="">Select stored {kind}</option>{kind==="party"?parties.map(id=><option key={id} value={id}>{id}</option>):profiles.filter(p=>p.data_mode==="stored").map(p=><option key={p.id} value={p.id}>{p.display_name}</option>)}</select></label>
    {!stored&&<p>Import a character snapshot and optionally create a local party. Browser demo profiles cannot supply current preparation assessments.</p>}
    <details><summary>Advanced local reference editor</summary><p>Local declarations require exact content references. Loading an example does not install verified dungeon coverage.</p>
    <label>Local preparation package JSON <textarea value={text} maxLength={1048576} rows={8} onChange={e=>{setText(e.target.value);setError("");}}/></label>
    <button type="button" onClick={()=>{setText(JSON.stringify(example,null,2));setOrigin("demo-node-1");setError("");}}>Load synthetic preparation reference</button>
    <button type="button" onClick={()=>{setText("");setError("");}}>Clear package</button>
    <label>Manually declared shared origin node <input value={origin} onChange={e=>{setOrigin(e.target.value);setError("");}}/></label>
    </details>
    <button type="button" disabled={!stored||pending} onClick={()=>void inspect()}>Inspect preparation gaps</button>
    {pending&&<p role="status">Inspecting imported preparation inputs…</p>}
    {error&&<p role="alert">{error}</p>}
    {visible&&<Preparation value={visible} key={report?.ticket}/>}
  </section>;
}
function Condition({value}:{value:ConditionAssessment|null}) {
  if(!value)return <>Unavailable</>;
  return <>{value.availability}{value.evaluation&&<>; condition {value.evaluation.truth}
    {value.evaluation.reasons.length>0&&<details><summary>Observation gaps ({value.evaluation.reasons.length})</summary><pre>{JSON.stringify(value.evaluation.reasons.slice(0,8),null,2)}</pre>{value.evaluation.reasons.length>8&&<p>First eight reasons shown.</p>}</details>}
  </>}</>;
}
function List({title,section,checklist=false}:{title:string;section:Declared<string[]>;checklist?:boolean}) {
  return <section><h4>{title}</h4>{section.observation.status==="known"?<ul>{section.observation.value.map((text,index)=><li key={index}>{checklist?<label><input type="checkbox"/>{text}</label>:text}</li>)}</ul>:<p>{describeObservation(section.observation)}</p>}
    {checklist&&<p>Manual local checklist; checking a box does not verify imported state.</p>}</section>;
}
function Preparation({value}:{value:PreparationReport}) {
  const packageValue=value.package;
  return <div>
    {value.fixture&&<p className="demo-label">SYNTHETIC — engineering reference, not verified gameplay coverage</p>}
    <p>Context: {value.context.build.edition}/{value.context.build.build}/{value.context.build.interface}; {value.context.namespace}. Content: {value.revision??"unavailable"}.</p>
    {packageValue&&<><h3>{packageValue.title}</h3><p>Explicit local declaration; cited evidence categories do not independently verify this package.</p>
      <p>Declared entrance: {packageValue.entrance.observation.status==="known"?packageValue.entrance.observation.value.id:packageValue.entrance.observation.status}.</p>
      <p>Declared group formation: {describeObservation(packageValue.formation_ms.observation,v=>`${v.min_ms} to ${v.max_ms} ms`)}. Declared run estimate: {describeObservation(packageValue.run_ms.observation,v=>`${v.min_ms} to ${v.max_ms} ms`)}. These are separate from measured personal history.</p>
      <List title="Before departure" section={packageValue.before_departure} checklist/><List title="Before entry" section={packageValue.before_entry} checklist/><List title="Reference encounter notes" section={packageValue.encounter_notes}/>
    </>}
    <h3>Coverage and evidence gaps</h3><ul>{value.gaps.map((gap,index)=><li key={index}>{gap.section}: {gap.code.replaceAll("_"," ")}{gap.availability?` (${gap.availability})`:""}</li>)}</ul>
    <h3>Imported member assessments</h3>{value.members.map(member=><details key={member.profile_id}><summary>{member.character} — capture {member.freshness?.status??"missing"}</summary>
      <p>Snapshot: {member.snapshot_id??"missing"}. This is imported state.</p><p>Declared access: <Condition value={member.access}/></p>
      <h4>Quest eligibility and prerequisites</h4><ul>{member.quests.map(q=><li key={q.quest}>{q.quest}: {q.assessment?<>acceptance {q.assessment.acceptance}; progress {q.assessment.progress}; turn-in {q.assessment.turn_in}<ul>{q.assessment.reasons.slice(0,8).map((r,i)=><li key={i}>{r.phase}: {r.message}</li>)}</ul>{q.assessment.catch_up&&<details><summary>Catch-up prerequisites</summary><pre>{JSON.stringify(q.assessment.catch_up,null,2)}</pre></details>}</>:"Unavailable: missing or nonfresh imported capture"}</li>)}</ul>
      <h4>Declared role, consumable and equipment conditions</h4><ul>{member.checks.map(check=><li key={check.check}>{packageValue?.checks.observation.status==="known"?packageValue.checks.observation.value.find(c=>c.id===check.check)?.label??check.check:check.check}: <Condition value={check.condition}/></li>)}</ul>
      <p>Consumable inventory is unavailable in current snapshots; possession is not inferred.</p>
      <h4>Declared reward references</h4><ul>{member.rewards.map(reward=><li key={`${reward.source}-${reward.item.item_id}`}>{reward.item.item_id} via {reward.source}; item requirements <Condition value={reward.item_requirement}/>; source requirements <Condition value={reward.acquisition_requirement}/>; probability {describeObservation(reward.probability)}; cost {describeObservation(reward.cost)}. No drop guarantee or role benefit is inferred.</li>)}</ul>
    </details>)}
    <h3>Declared entrance travel</h3>{value.route?.outcome.status==="complete"?<p>Estimated shared stateless route: {value.route.outcome.route?.total_ms} ms. Entrance association remains a local declaration.</p>:<p>No actionable supported route is available.</p>}
    <ul>{value.route_exclusions.map((e,index)=><li key={index}>{e.edge}: {e.character??"shared route"}; {e.reason.replaceAll("_"," ")} {e.condition&&<Condition value={e.condition}/>}</li>)}</ul>
    <h3>Referenced evidence and source policy</h3><ul>{value.evidence.map(e=><li key={e.id}>{e.id}: {e.confidence}; source {e.source}; license {describeObservation(e.license)}; redistribution {describeObservation(e.redistribution)}; offline use {describeObservation(e.offline_usable)}.</li>)}</ul>
  </div>;
}
