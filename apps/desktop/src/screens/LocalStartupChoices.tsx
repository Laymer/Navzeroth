import {useEffect,useRef,useState} from "react";
import type {CommandClient} from "../commands";
import type {LocalStartupMode,LocalWorkflowRecord} from "../commands/localData";
import {Button,Panel} from "../ui";
export function LocalStartupChoices({client,onNavigate}:{client:CommandClient;onNavigate:(mode:LocalStartupMode)=>void}) {
 const [record,setRecord]=useState<LocalWorkflowRecord|null>(null);
 const [busy,setBusy]=useState(false);const [message,setMessage]=useState("");const serial=useRef(0);
 useEffect(()=>{let current=true;void client.getLocalWorkflow().then(result=>{if(current&&result.ok&&result.value?.format_version===1)setRecord(result.value);});return()=>{current=false;serial.current++;};},[client]);
 async function choose(mode:LocalStartupMode){
  if(!record)return;const ticket=++serial.current;setBusy(true);
  const result=await client.finishLocalStartup(record.revision,mode);
  if(ticket!==serial.current)return;setBusy(false);
  if(result.ok){setRecord(result.value);onNavigate(mode);}else setMessage(`Startup choice was not saved: ${result.error.code}.`);
 }
 if(!record||record.preferences.first_run_completed)return null;
 return <Panel title="Choose how to begin" level={2}>
  <p>Import a permitted addon snapshot, or inspect explicitly selected installed content in private research storage. Research content does not establish playable availability.</p>
  <Button disabled={busy} onClick={()=>void choose("snapshots")}>Begin with addon snapshots</Button>
  <Button disabled={busy} onClick={()=>void choose("local_research")}>Begin with local research content</Button>
  <Button disabled={busy} onClick={()=>void choose("later")}>Choose later</Button>
  <p>Nothing is extracted or activated by this choice. The desktop interface is English.</p>
  {message&&<p role="alert">{message}</p>}
 </Panel>;
}
