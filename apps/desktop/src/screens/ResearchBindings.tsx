import {useEffect,useRef,useState} from "react";
import type {CommandClient} from "../commands";
import type {LocalArtifactPage} from "../commands/localData";
import {Button,Field} from "../ui";
export interface BindingSelection {media:string[];inputs:string[];guide_facts:boolean}
export function ResearchBindings({client,value,onChange,disabled}:{client:CommandClient;value:BindingSelection;onChange:(value:BindingSelection)=>void;disabled:boolean}) {
 const [namespace,setNamespace]=useState<"media"|"inputs">("inputs");const [page,setPage]=useState<LocalArtifactPage|null>(null);
 const [busy,setBusy]=useState(false);const [message,setMessage]=useState("");const serial=useRef(0);
 useEffect(()=>()=>{serial.current++;},[client]);
 async function list(offset:number){const ticket=++serial.current;setBusy(true);setMessage("");const result=await client.getLocalArtifactPage(namespace,offset);if(ticket!==serial.current)return;setBusy(false);if(result.ok&&result.value.namespace===namespace&&result.value.offset===offset&&(offset===0||result.value.revision===page?.revision))setPage(result.value);else{setPage(null);setMessage("Revision list changed or is unavailable; review its first page again.");}}
 return <fieldset><legend>Explicit reference dependencies</legend><p>Choose up to 16 media and 16 input revisions. Listed digests are candidates, not verified completeness. Preparing a reference reproduces and validates every selected dependency against its exact build and locale.</p>
  <label><input type="checkbox" checked={value.guide_facts} disabled={disabled||busy} onChange={event=>onChange({...value,guide_facts:event.target.checked})}/>Build and bind research guide facts from the selected inputs</label>
  <Field label="Dependency kind">{props=><select {...props} value={namespace} disabled={disabled||busy} onChange={event=>{serial.current++;setNamespace(event.target.value as "media"|"inputs");setPage(null);}}><option value="inputs">Cache and observation inputs</option><option value="media">Private media</option></select>}</Field>
  <Button disabled={disabled||busy} onClick={()=>void list(0)}>Review retained dependency revisions</Button>
  <p>{value.inputs.length} input and {value.media.length} media revisions selected.</p>
  {page&&<><p>{page.total} retained {page.namespace} revisions; integrity and source compatibility checked during preparation. {page.staging_present?"A staging write is in progress.":""}</p>{page.rows.map(id=><label key={id}><input type="checkbox" checked={value[namespace].includes(id)} disabled={disabled||busy||(!value[namespace].includes(id)&&value[namespace].length>=16)} onChange={event=>onChange({...value,[namespace]:event.target.checked?[...new Set([...value[namespace],id])].sort():value[namespace].filter(selected=>selected!==id)})}/>{id}</label>)}{page.next_offset!==null&&<Button disabled={disabled||busy} onClick={()=>{if(page.next_offset!==null)void list(page.next_offset);}}>Next dependency revision page</Button>}</>}
  <Button disabled={disabled||busy} onClick={()=>onChange({media:[],inputs:[],guide_facts:false})}>Clear reference dependencies</Button>
  {message&&<p role="status">{message}</p>}
 </fieldset>;
}
