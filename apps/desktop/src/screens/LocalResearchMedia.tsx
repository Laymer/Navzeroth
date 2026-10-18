import {useEffect,useRef,useState} from "react";
import type {CommandClient} from "../commands";
import type {LocalDataPreset,LocalMediaDiscovery,LocalMediaManifest,LocalMediaPage} from "../commands/localData";
import {Button,Field} from "../ui";
export function LocalResearchMedia({client,catalog,selectionId,preset}:{client:CommandClient;catalog:string;selectionId:string;preset:LocalDataPreset}) {
 const [locale,setLocale]=useState("");const [discovery,setDiscovery]=useState<LocalMediaDiscovery|null>(null);const [ids,setIds]=useState<number[]>([]);
 const [revision,setRevision]=useState<string|null>(null);const [manifest,setManifest]=useState<LocalMediaManifest|null>(null);const [page,setPage]=useState<LocalMediaPage|null>(null);
 const [busy,setBusy]=useState(false);const [message,setMessage]=useState("");const serial=useRef(0);
 useEffect(()=>{serial.current++;setDiscovery(null);setIds([]);setRevision(null);setManifest(null);setPage(null);setMessage("");setBusy(false);return()=>{serial.current++;};},[client,catalog,selectionId]);
 async function discover(offset:number){const ticket=++serial.current;setBusy(true);setMessage("");const result=await client.discoverLocalMedia(catalog,locale,offset);if(ticket!==serial.current)return;setBusy(false);
  if(result.ok&&result.value.catalog_sha256===catalog&&result.value.locale===locale&&result.value.offset===offset&&(offset===0||result.value.revision===discovery?.revision)){
   setDiscovery(result.value);if(offset===0){setIds([]);setRevision(null);setManifest(null);setPage(null);}
  }else setMessage("Media discovery changed or is unavailable; discover again from the first page.");
 }
 async function extract(){if(!ids.length||!discovery)return;const ticket=++serial.current;setBusy(true);setMessage("");setPage(null);setManifest(null);setRevision(null);
  const result=await client.extractLocalMedia(selectionId,catalog,locale,ids,preset);if(ticket!==serial.current)return;
  if(!result.ok){setBusy(false);setMessage(`Selected media extraction refused: ${result.error.code}.`);return;}
  const candidate=result.value;const [metadata,assets]=await Promise.all([client.getLocalMediaManifest(candidate),client.getLocalMediaPage(candidate,0)]);
  if(ticket!==serial.current)return;setBusy(false);
  if(metadata.ok&&assets.ok&&metadata.value.catalog_sha256===catalog&&metadata.value.locale===locale&&assets.value.revision===candidate&&assets.value.offset===0){setRevision(candidate);setManifest(metadata.value);setPage(assets.value);}
  else setMessage("Media verification failed; no preview is available.");
 }
 async function next(offset:number){if(!revision)return;const ticket=++serial.current;setBusy(true);const result=await client.getLocalMediaPage(revision,offset);if(ticket!==serial.current)return;setBusy(false);if(result.ok&&result.value.revision===revision&&result.value.offset===offset)setPage(result.value);else setMessage("Media page is stale or unavailable.");}
 function safeUrl(id:string,url:string){return !!revision&&/^[a-f0-9]{64}$/.test(revision)&&/^[a-f0-9]{64}$/.test(id)&&["atlas://localhost","http://atlas.localhost"].some(origin=>url===`${origin}/research/${revision}/${id}`);}
 return <section aria-label="Private research media"><h4>Private research media</h4><p>Discover map and icon candidates from verified tables. Select at most 128 file IDs. Extraction uses an isolated bounded worker. Coordinates remain uncalibrated; these images do not establish gameplay availability.</p>
  <Field label="Media content locale">{props=><input {...props} value={locale} disabled={busy} onChange={event=>{serial.current++;setLocale(event.target.value);setDiscovery(null);setIds([]);setRevision(null);setManifest(null);setPage(null);}}/>}</Field>
  <Button disabled={busy||!locale} onClick={()=>void discover(0)}>Discover private media candidates</Button>
  {discovery&&<><p>{discovery.total} candidates; {ids.length} unique file IDs selected.</p>{discovery.gaps.map(gap=><p key={gap}>{gap.replaceAll("_"," ")}</p>)}
   {discovery.candidates.map((candidate,index)=><label key={`${discovery.offset+index}`}><input type="checkbox" checked={ids.includes(candidate.file_data_id)} disabled={busy||(!ids.includes(candidate.file_data_id)&&ids.length>=128)} onChange={event=>setIds(values=>event.target.checked?[...new Set([...values,candidate.file_data_id])].sort((a,b)=>a-b):values.filter(id=>id!==candidate.file_data_id))}/>{candidate.table} {candidate.row}, {candidate.field}: {candidate.file_data_id}</label>)}
   {discovery.next_offset!==null&&<Button disabled={busy} onClick={()=>{if(discovery.next_offset!==null)void discover(discovery.next_offset);}}>Next media discovery page</Button>}
   <Button disabled={busy||ids.length===0} onClick={()=>void extract()}>Extract selected private media</Button></>}
  {manifest&&<><p>Media {revision}; {manifest.fixture?"synthetic fixture":"selected installed source"}. Local only, uncalibrated, unavailable for gameplay advice.</p><table><thead><tr><th>File ID</th><th>Disposition</th><th>Read refusal</th></tr></thead><tbody>{manifest.assets.map(asset=><tr key={asset.file_data_id}><td>{asset.file_data_id}</td><td>{asset.disposition}</td><td>{asset.read_refusal??"none recorded"}</td></tr>)}</tbody></table></>}
  {page&&<><p>{page.total} decoded previews.</p>{page.assets.map(asset=>safeUrl(asset.id,asset.url)?<figure key={asset.id}><img src={asset.url} alt={`Uncalibrated private research asset ${asset.id.slice(0,8)}`} width={Math.min(asset.width,320)} height={Math.round(asset.height*Math.min(asset.width,320)/asset.width)} loading="lazy" onError={()=>setMessage("Private image unavailable; verify its revision again.")}/><figcaption>{asset.width} × {asset.height}; uncalibrated.</figcaption></figure>:<p key={asset.id}>Invalid private asset reference.</p>)}{page.next_offset!==null&&<Button disabled={busy} onClick={()=>{if(page.next_offset!==null)void next(page.next_offset);}}>Next decoded media page</Button>}</>}
  {busy&&<p role="status">Processing selected private media…</p>}{message&&<p role="status">{message}</p>}
 </section>;
}
