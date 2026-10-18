import "./dataScreens.css";
import {ResearchDeliveryPanel} from "./ResearchDeliveryPanel";
import {useEffect,useRef,useState} from "react";
import type {CommandClient} from "../commands";
import type {LocalDataInventoryPage,LocalDataJob,LocalDataPlan,LocalDataPreset,LocalInstallation,LocalDataActivation,SavedLocalInstallation} from "../commands/localData";
import { Button, Field, Panel } from "../ui";

export function LocalDataPanel({client}:{client:CommandClient}) {
  const [selection,setSelection]=useState<LocalInstallation|null>(null);
  const [preset,setPreset]=useState<LocalDataPreset>("max");
  const [saved,setSaved]=useState<SavedLocalInstallation[]>([]);
  const [jobs,setJobs]=useState<LocalDataJob[]>([]);
  const [activation,setActivation]=useState<LocalDataActivation|null>(null);
  const [plan,setPlan]=useState<LocalDataPlan|null>(null);
  const [job,setJob]=useState<LocalDataJob|null>(null);
  const [page,setPage]=useState<LocalDataInventoryPage|null>(null);
  const [busy,setBusy]=useState(false);const [message,setMessage]=useState("");const serial=useRef(0);
  useEffect(()=>()=>{serial.current++;},[client]);
  useEffect(()=>{
    if(job?.state!=="running"&&job?.state!=="preparing")return;
    let current=true;
    const timer=setTimeout(()=>{
      void client.getLocalDataJob(job.job_id).then(result=>{
        if(!current)return;if(result.ok)setJob(result.value);else{setMessage(`Extraction status unavailable: ${result.error.code}.`);setJob({...job,state:"failed"});}
      });
    },1000);
    return()=>{current=false;clearTimeout(timer);};
  },[client,job]);
  async function choose(){
    const ticket=++serial.current;setBusy(true);setMessage("");
    const result=await client.selectLocalInstallation();if(ticket!==serial.current)return;setBusy(false);
    if(result.ok&&result.value){setSelection(result.value);setPreset(result.value.preset??"max");setActivation(null);setPlan(null);setPage(null);setJob(null);}else if(!result.ok)setMessage(`Installation selection unavailable: ${result.error.code}.`);
  }
  async function preview(){
    if(!selection)return;const ticket=++serial.current;setBusy(true);setMessage("");
    const result=await client.previewLocalData(selection.selection_id,preset);if(ticket!==serial.current)return;setBusy(false);
    if(result.ok)setPlan(result.value);else setMessage(`Extraction preview unavailable: ${result.error.code}.`);
  }
  async function start(){
    if(!selection||!plan)return;const ticket=++serial.current;setBusy(true);setMessage("");setPage(null);setActivation(null);
    const result=await client.startLocalData(selection.selection_id,preset);if(ticket!==serial.current)return;setBusy(false);
    if(result.ok)setJob(result.value);else setMessage(`Extraction could not start: ${result.error.code}.`);
  }
  async function cancel(){if(!job)return;const ticket=++serial.current;const result=await client.cancelLocalDataJob(job.job_id);if(ticket!==serial.current)return;if(result.ok)setJob(result.value);else setMessage(`Cancellation unavailable: ${result.error.code}.`);}
  async function savedState(){
    const ticket=++serial.current;setBusy(true);setMessage("");
    const [selections,history]=await Promise.all([client.listLocalInstallations(),client.listLocalDataJobs()]);
    if(ticket!==serial.current)return;setBusy(false);
    if(selections.ok&&history.ok){setSaved(selections.value);setJobs(history.value);}
    else setMessage("Saved extraction state is unavailable.");
  }
  async function restore(id:string){
    const ticket=++serial.current;setBusy(true);setMessage("");
    const result=await client.restoreLocalInstallation(id);if(ticket!==serial.current)return;setBusy(false);
    if(result.ok){setSelection(result.value);setPreset(result.value.preset??"max");setPlan(null);setJob(null);setPage(null);setActivation(null);}
    else setMessage(`Saved installation unavailable: ${result.error.code}.`);
  }
  async function resume(){
    if(!job)return;const ticket=++serial.current;setBusy(true);setMessage("");setActivation(null);
    const result=await client.resumeLocalData(job.job_id);if(ticket!==serial.current)return;setBusy(false);
    if(result.ok)setJob(result.value);else setMessage(`Resume refused: ${result.error.code}. Re-select changed sources and start a new extraction.`);
  }
  async function forget(){
    if(!job)return;const ticket=++serial.current;setBusy(true);setMessage("");
    const result=await client.forgetLocalDataJob(job.job_id);if(ticket!==serial.current)return;setBusy(false);
    if(result.ok){setJobs(items=>items.filter(item=>item.job_id!==job.job_id));setJob(null);setPage(null);setActivation(null);}
    else setMessage(`Job history could not be forgotten: ${result.error.code}.`);
  }
  async function researchActivation(change:boolean,rollback=false){
    if(!job?.summary)return;const ticket=++serial.current;setBusy(true);setMessage("");
    const revision=job.summary.catalog_sha256;
    const result=change&&activation?await client.activateLocalData(revision,activation.generation,rollback):await client.getLocalDataActivation(revision);
    if(ticket!==serial.current)return;setBusy(false);
    if(result.ok)setActivation(result.value);else {setActivation(null);setMessage(`Research activation refused: ${result.error.code}. Review the current revision again.`);}
  }
  async function inventory(offset:number){if(!job?.summary)return;const ticket=++serial.current;setBusy(true);
    const result=await client.getLocalDataInventory(job.summary.catalog_sha256,offset);if(ticket!==serial.current)return;setBusy(false);
    if(result.ok)setPage(result.value);else setMessage(`Catalog verification unavailable: ${result.error.code}.`);
  }
  const running=job?.state==="running"||job?.state==="preparing";
  return <Panel title="Local Data" level={3}>
    <p>Read installed client tables into private research storage. Client records do not prove playable availability. Research catalogs cannot generate routes or recommendations.</p>
    <Button disabled={busy||running} onClick={()=>void choose()}>Select game installation</Button>
    <Button disabled={busy||running} onClick={()=>void savedState()}>Review saved extraction state</Button>
    {saved.map(item=><p key={item.selection_id}>{item.product}, saved preset {item.preset}. <Button disabled={busy||running} onClick={()=>void restore(item.selection_id)}>Restore installation {item.selection_id.slice(0,8)}</Button></p>)}
    {jobs.map(item=><p key={item.job_id}>{item.job_id}: {item.state}, {item.completed} of {item.total}. <Button disabled={busy||running} onClick={()=>{serial.current++;setJob(item);setPage(null);setActivation(null);setMessage("");}}>Inspect saved job {item.job_id}</Button></p>)}
    {selection&&<p>Selected product {selection.inspection.metadata.product}, version {selection.inspection.metadata.version}; declared locales {selection.inspection.metadata.declared_locales.join(", ")||"unknown"}. The post-launch release target is still pending.</p>}
    <Field label="Extraction resources" help="Upper limits for workers, memory and storage. Extraction currently uses one worker whichever preset you pick.">{(c)=><select {...c} value={preset} disabled={busy||running} onChange={event=>{setPreset(event.target.value as LocalDataPreset);setPlan(null);}}>
      <option value="small">Small — 1 worker, 1 GiB memory, 8 GiB storage</option><option value="balanced">Balanced — 2 workers, 2 GiB memory, 32 GiB storage</option><option value="large">Large — 4 workers, 4 GiB memory, 64 GiB storage</option><option value="max">Max — 8 workers, 8 GiB memory, 128 GiB storage</option>
    </select>}</Field>
    <Button disabled={!selection||busy||running} onClick={()=>void preview()}>Review extraction limits</Button>
    {plan&&<div><p>Storage stays in this application's private local-data area. Limits include retained originals, decoded data, indexes and staging. Extraction currently uses one worker. Required storage: {plan.required_storage_bytes===null?"unknown":`${plan.required_storage_bytes} bytes`}; available storage: {plan.available_storage_bytes===null?"unverified":`${plan.available_storage_bytes} bytes`}.</p>
      <p>Scope: reviewed table inventory and supported research decoding. Media and hotfix integration are pending; gameplay projection validation is separate.</p>
      <Button variant="primary" disabled={busy||running} onClick={()=>void start()}>Extract research tables</Button></div>}
    {job&&<div><p role="status">Extraction {job.state}: {job.completed} of {job.total||"unknown"} tables processed.</p>{running&&<Button onClick={()=>void cancel()}>Cancel extraction</Button>}
      {!running&&job.state!=="complete"&&<Button disabled={busy} onClick={()=>void resume()}>Resume unchanged extraction</Button>}
      {!running&&<Button disabled={busy} onClick={()=>void forget()}>Forget saved job</Button>}
      {job.error&&<p role="alert">Extraction failed: {job.error}</p>}
      {job.summary&&<><p>Build {job.summary.build}: {job.summary.tables_decoded} decoded of {job.summary.tables_processed} candidates. Research only; no gameplay catalog activated.</p><Button disabled={busy} onClick={()=>void inventory(0)}>Verify and inspect inventory</Button><Button disabled={busy} onClick={()=>void researchActivation(false)}>Review research activation</Button></>}
      {activation&&<div><p>Research revision: {activation.catalog_sha256??"none"}; generation {activation.generation}. No gameplay availability is established.</p><Button disabled={busy||running} onClick={()=>void researchActivation(true)}>Activate research revision</Button><Button disabled={busy||running||activation.generation===0} onClick={()=>void researchActivation(true,true)}>Restore previously active research revision</Button></div>}
    </div>}
    {page&&<><p>Catalog {page.revision}; {page.total} table and locale candidates.</p><table><thead><tr><th>Table</th><th>Locale</th><th>Disposition</th><th>Details</th></tr></thead><tbody>{page.rows.map(row=><tr key={`${row.table}:${row.locale}`}><td>{row.table}</td><td>{row.locale??"unknown"}</td><td>{row.disposition.replaceAll("_"," ")}</td><td>{row.reason.replaceAll("_"," ")}</td></tr>)}</tbody></table>{page.next_offset!==null&&<Button disabled={busy} onClick={()=>{if(page.next_offset!==null)void inventory(page.next_offset);}}>Next inventory page</Button>}</>}
    {job?.summary&&<ResearchDeliveryPanel client={client} catalog={job.summary.catalog_sha256}/>}
    {busy&&<p role="status">Reading local data…</p>}{message&&<p role="alert">{message}</p>}
  </Panel>;
}
