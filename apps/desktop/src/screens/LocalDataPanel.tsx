import {useEffect,useRef,useState} from "react";
import type {CommandClient} from "../commands";
import type {LocalDataInventoryPage,LocalDataJob,LocalDataPlan,LocalDataPreset,LocalInstallation} from "../commands/localData";
import { Button, Field, Panel } from "../ui";

export function LocalDataPanel({client}:{client:CommandClient}) {
  const [selection,setSelection]=useState<LocalInstallation|null>(null);
  const [preset,setPreset]=useState<LocalDataPreset>("max");
  const [plan,setPlan]=useState<LocalDataPlan|null>(null);
  const [job,setJob]=useState<LocalDataJob|null>(null);
  const [page,setPage]=useState<LocalDataInventoryPage|null>(null);
  const [busy,setBusy]=useState(false);const [message,setMessage]=useState("");const serial=useRef(0);
  useEffect(()=>()=>{serial.current++;},[client]);
  useEffect(()=>{
    if(job?.state!=="running")return;
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
    if(result.ok&&result.value){setSelection(result.value);setPlan(null);setPage(null);setJob(null);}else if(!result.ok)setMessage(`Installation selection unavailable: ${result.error.code}.`);
  }
  async function preview(){
    if(!selection)return;const ticket=++serial.current;setBusy(true);setMessage("");
    const result=await client.previewLocalData(selection.selection_id,preset);if(ticket!==serial.current)return;setBusy(false);
    if(result.ok)setPlan(result.value);else setMessage(`Extraction preview unavailable: ${result.error.code}.`);
  }
  async function start(){
    if(!selection||!plan)return;const ticket=++serial.current;setBusy(true);setMessage("");setPage(null);
    const result=await client.startLocalData(selection.selection_id,preset);if(ticket!==serial.current)return;setBusy(false);
    if(result.ok)setJob(result.value);else setMessage(`Extraction could not start: ${result.error.code}.`);
  }
  async function cancel(){if(!job)return;const result=await client.cancelLocalDataJob(job.job_id);if(result.ok)setJob(result.value);else setMessage(`Cancellation unavailable: ${result.error.code}.`);}
  async function inventory(offset:number){if(!job?.summary)return;const ticket=++serial.current;setBusy(true);
    const result=await client.getLocalDataInventory(job.summary.catalog_sha256,offset);if(ticket!==serial.current)return;setBusy(false);
    if(result.ok)setPage(result.value);else setMessage(`Catalog verification unavailable: ${result.error.code}.`);
  }
  const running=job?.state==="running";
  return <Panel title="Local Data" level={3}>
    <p>Read installed client tables into private research storage. Client records do not prove playable availability. Research catalogs cannot generate routes or recommendations.</p>
    <Button disabled={busy||running} onClick={()=>void choose()}>Select game installation</Button>
    {selection&&<p>Selected product {selection.inspection.metadata.product}, version {selection.inspection.metadata.version}; declared locales {selection.inspection.metadata.declared_locales.join(", ")||"unknown"}. The release target remains build 70374.</p>}
    <Field label="Extraction resources" help="Upper limits for workers, memory and storage. Extraction currently uses one worker whichever preset you pick.">{(c)=><select {...c} value={preset} disabled={busy||running} onChange={event=>{setPreset(event.target.value as LocalDataPreset);setPlan(null);}}>
      <option value="small">Small — 1 worker, 1 GiB memory, 8 GiB storage</option><option value="balanced">Balanced — 2 workers, 2 GiB memory, 32 GiB storage</option><option value="large">Large — 4 workers, 4 GiB memory, 64 GiB storage</option><option value="max">Max — 8 workers, 8 GiB memory, 128 GiB storage</option>
    </select>}</Field>
    <Button disabled={!selection||busy||running} onClick={()=>void preview()}>Review extraction limits</Button>
    {plan&&<div><p>Storage stays in this application's private local-data area. Limits include retained originals, decoded data, indexes and staging. Extraction currently uses one worker. Required storage: {plan.required_storage_bytes===null?"unknown":`${plan.required_storage_bytes} bytes`}; available storage: {plan.available_storage_bytes===null?"unverified":`${plan.available_storage_bytes} bytes`}.</p>
      <p>Scope: reviewed table inventory and supported research decoding. Media, hotfix integration and gameplay projections are pending.</p>
      <Button variant="primary" disabled={busy||running} onClick={()=>void start()}>Extract research tables</Button></div>}
    {job&&<div><p role="status">Extraction {job.state}: {job.completed} of {job.total||"unknown"} tables processed.</p>{running&&<Button onClick={()=>void cancel()}>Cancel extraction</Button>}
      {job.error&&<p role="alert">Extraction failed: {job.error}</p>}
      {job.summary&&<><p>Build {job.summary.build}: {job.summary.tables_decoded} decoded of {job.summary.tables_processed} candidates. Research only; no gameplay catalog activated.</p><Button disabled={busy} onClick={()=>void inventory(0)}>Verify and inspect inventory</Button></>}
    </div>}
    {page&&<><p>Catalog {page.revision}; {page.total} table and locale candidates.</p><table><thead><tr><th>Table</th><th>Locale</th><th>Disposition</th><th>Details</th></tr></thead><tbody>{page.rows.map(row=><tr key={`${row.table}:${row.locale}`}><td>{row.table}</td><td>{row.locale??"unknown"}</td><td>{row.disposition.replaceAll("_"," ")}</td><td>{row.reason.replaceAll("_"," ")}</td></tr>)}</tbody></table>{page.next_offset!==null&&<Button disabled={busy} onClick={()=>{if(page.next_offset!==null)void inventory(page.next_offset);}}>Next inventory page</Button>}</>}
    {busy&&<p role="status">Reading local data…</p>}{message&&<p role="alert">{message}</p>}
  </Panel>;
}
