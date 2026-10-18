import {ResearchFactsTools} from "./ResearchFactsTools";
import {ResearchReferenceHistory} from "./ResearchReferenceHistory";
import {ResearchBindings,type BindingSelection} from "./ResearchBindings";
import {useEffect,useRef,useState} from "react";
import type {CommandClient} from "../commands";
import type {ResearchDeliveryActivation,ResearchDeliveryDiff,ResearchDeliveryPage,LocalDataPreset} from "../commands/localData";
import {Button,Field} from "../ui";

export function ResearchDeliveryPanel({client,catalog,preset="max"}:{client:CommandClient;catalog:string;preset?:LocalDataPreset}) {
  const [bindings,setBindings]=useState<BindingSelection>({media:[],inputs:[],guide_facts:false});
  const [locale,setLocale]=useState("");const [revision,setRevision]=useState<string|null>(null);
  const [activation,setActivation]=useState<ResearchDeliveryActivation|null>(null);
  const [page,setPage]=useState<ResearchDeliveryPage|null>(null);const [domain,setDomain]=useState("equipment");
  const [comparison,setComparison]=useState<ResearchDeliveryDiff|null>(null);const [rollback,setRollback]=useState("");
  const [busy,setBusy]=useState(false);const [message,setMessage]=useState("");const serial=useRef(0);
  useEffect(()=>{serial.current++;setBindings({media:[],inputs:[],guide_facts:false});setRevision(null);setActivation(null);setPage(null);setComparison(null);setMessage("");setBusy(false);return()=>{serial.current++;};},[client,catalog]);
  async function prepare(){
    const ticket=++serial.current;setBusy(true);setMessage("");setComparison(null);setActivation(null);setPage(null);setRevision(null);
    const result=await client.prepareResearchDelivery(catalog,locale,{format_version:1,...bindings,preset});
    if(ticket!==serial.current)return;
    if(!result.ok){setBusy(false);setMessage(`Reference preparation refused: ${result.error.code}.`);return;}
    const candidate=result.value;
    const [current,records]=await Promise.all([client.getResearchDeliveryActivation(candidate),client.getResearchDeliveryPage(candidate,domain,0)]);
    if(ticket!==serial.current)return;setBusy(false);
    if(!current.ok||!records.ok||records.value.revision!==candidate||records.value.domain!==domain){setMessage("Reference verification failed; prepare the reference again.");return;}
    setRevision(candidate);setActivation(current.value);setPage(records.value);
    if(current.value.delivery_sha256&&current.value.delivery_sha256!==candidate){
      setBusy(true);const diff=await client.diffResearchDelivery(current.value.delivery_sha256,candidate);
      if(ticket!==serial.current)return;setBusy(false);if(diff.ok&&diff.value.before===current.value.delivery_sha256&&diff.value.after===candidate)setComparison(diff.value);else setMessage("Reference comparison failed; activation is unavailable.");
    }
  }
  async function inspectStored(candidate:string){
    const ticket=++serial.current;setBusy(true);setRevision(null);setActivation(null);setPage(null);setComparison(null);setMessage("");
    const [current,records]=await Promise.all([client.getResearchDeliveryActivation(candidate),client.getResearchDeliveryPage(candidate,domain,0)]);
    if(ticket!==serial.current)return;setBusy(false);
    if(!current.ok||!records.ok||records.value.revision!==candidate||records.value.domain!==domain){setMessage("Stored reference is stale or unavailable.");return;}
    setRevision(candidate);setActivation(current.value);setPage(records.value);
    if(current.value.delivery_sha256&&current.value.delivery_sha256!==candidate){
      setBusy(true);const diff=await client.diffResearchDelivery(current.value.delivery_sha256,candidate);
      if(ticket!==serial.current)return;setBusy(false);if(diff.ok&&diff.value.before===current.value.delivery_sha256&&diff.value.after===candidate)setComparison(diff.value);else setMessage("Stored reference comparison failed; activation is unavailable.");
    }
  }
  async function lookup(offset:number){
    if(!revision)return;const ticket=++serial.current;setBusy(true);setMessage("");setPage(null);
    const result=await client.getResearchDeliveryPage(revision,domain,offset);
    if(ticket!==serial.current)return;setBusy(false);
    if(result.ok&&result.value.revision===revision&&result.value.domain===domain&&result.value.offset===offset)setPage(result.value);else setMessage("Reference page changed or is unavailable.");
  }
  async function reviewRollback(){
    if(!activation?.delivery_sha256)return;const ticket=++serial.current;setBusy(true);setMessage("");setComparison(null);
    const result=await client.diffResearchDelivery(activation.delivery_sha256,rollback);
    if(ticket!==serial.current)return;setBusy(false);
    if(result.ok&&result.value.before===activation.delivery_sha256&&result.value.after===rollback&&result.value.same_scope)setComparison(result.value);else setMessage("Rollback comparison refused for this build and locale.");
  }
  async function activate(isRollback:boolean){
    if(!activation||!revision)return;
    const target=isRollback?comparison?.after:revision;if(!target)return;
    const ticket=++serial.current;setBusy(true);setMessage("");
    const result=await client.activateResearchDelivery(target,activation.generation,isRollback);
    if(ticket!==serial.current)return;setBusy(false);setComparison(null);
    if(result.ok){setActivation(result.value);setMessage("Research reference selected. Advice remains unavailable.");}
    else{setActivation(null);setMessage(`Reference activation refused: ${result.error.code}. Prepare again to refresh its generation.`);}
  }
  const compared=comparison?.same_scope&&comparison.before===activation?.delivery_sha256&&comparison.after===revision;
  const canActivate=!!activation&&(!activation.delivery_sha256||activation.delivery_sha256===revision||compared);
  const canRollback=!!comparison?.same_scope&&comparison.before===activation?.delivery_sha256&&comparison.after===rollback;
  return <section className="research-delivery" aria-label="Research reference delivery">
    <h4>Research reference</h4><p>Inspect partial client facts. Selecting a reference does not enable routes, gear advice or acceptance readiness.</p>
    <Field label="Installed content locale" help="Enter an explicitly selected locale declared by this installation, such as enUS.">{c=><input {...c} value={locale} disabled={busy} onChange={e=>{serial.current++;setLocale(e.target.value);setRevision(null);setPage(null);setActivation(null);setComparison(null);}}/>}</Field>
    <ResearchReferenceHistory client={client} catalog={catalog} disabled={busy} onReference={candidate=>void inspectStored(candidate)}/>
    <ResearchBindings client={client} value={bindings} disabled={busy} onChange={value=>{serial.current++;setBindings(value);setRevision(null);setActivation(null);setPage(null);setComparison(null);}}/>
    <Button disabled={busy||!locale} onClick={()=>void prepare()}>Prepare research reference</Button>
    {revision&&<><p>Reference {revision}. Development match: {page?.guard.development_match??"unverified"}. Advice and acceptance readiness remain unavailable.</p>
      <Field label="Reference domain">{c=><select {...c} value={domain} disabled={busy} onChange={e=>{serial.current++;setDomain(e.target.value);setPage(null);}}>{[["equipment","Equipment"],["geography","Maps"],["travel","Flight nodes"],["SpellName","Spell names"],["Talent","Talents"],["QuestV2","Quest identities"]].map(([id,label])=><option key={id} value={id}>{label}</option>)}</select>}</Field>
      <Button disabled={busy} onClick={()=>void lookup(0)}>Inspect reference facts</Button>
      {activation&&<><p>Selected reference: {activation.delivery_sha256??"none"}; generation {activation.generation}.</p><Button disabled={busy||!canActivate} onClick={()=>void activate(false)}>Select this research reference</Button>
        <Field label="Previously selected reference revision">{c=><input {...c} value={rollback} disabled={busy} onChange={e=>{setRollback(e.target.value);setComparison(null);}}/>}</Field>
        <Button disabled={busy||!activation.delivery_sha256||!rollback} onClick={()=>void reviewRollback()}>Review reference rollback</Button>
        {comparison&&<p>Changed bindings: {comparison.changed.join(", ")||"none"}. Same build and locale: {comparison.same_scope?"yes":"no"}.</p>}
        <Button disabled={busy||!canRollback} onClick={()=>void activate(true)}>Restore reviewed reference</Button></>}
      {page&&<><p>{page.total===null?"Record count unavailable":`${page.total} imported records`}; completeness unverified.</p><ul>{page.sources.map(source=><li key={source.table}>{source.table}: {source.state}, {source.reason}</li>)}</ul><table><thead><tr><th>Record ID</th><th>Research facts</th></tr></thead><tbody>{page.rows.map(row=><tr key={row.id}><td>{row.id}</td><td><pre>{JSON.stringify(row.facts,null,2)}</pre></td></tr>)}</tbody></table>{page.next_offset!==null&&<Button disabled={busy} onClick={()=>{if(page.next_offset!==null)void lookup(page.next_offset);}}>Next reference page</Button>}</>}
    </>}
    {revision&&<ResearchFactsTools key={revision} client={client} revision={revision} catalog={catalog} preset={preset}/>}
    {busy&&<p role="status">Verifying research reference…</p>}{message&&<p role="status">{message}</p>}
  </section>;
}
