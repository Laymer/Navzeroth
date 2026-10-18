import {useEffect,useRef,useState} from "react";
import type {CommandClient} from "../commands";
import type {LocalStartupMode,LocalWorkflowRecord} from "../commands/localData";
import {Button,Field,Panel} from "../ui";
export function LocalStartupChoices({client,onNavigate}:{client:CommandClient;onNavigate:(mode:LocalStartupMode)=>void}) {
 const [record,setRecord]=useState<LocalWorkflowRecord|null>(null);
 const [busy,setBusy]=useState(false);const [message,setMessage]=useState("");const serial=useRef(0);
 useEffect(()=>{let current=true;void client.getLocalWorkflow().then(result=>{if(current&&result.ok&&result.value?.format_version===1){setRecord(result.value);if(result.value.preferences.first_run_completed)onNavigate(result.value.preferences.startup_mode);}});return()=>{current=false;serial.current++;};},[client,onNavigate]);
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

export function LocalStartupSettings({client}:{client:CommandClient}) {
 const [record,setRecord]=useState<LocalWorkflowRecord|null>(null);const [mode,setMode]=useState<LocalStartupMode>("later");const [message,setMessage]=useState("");const [busy,setBusy]=useState(false);const serial=useRef(0);
 useEffect(()=>()=>{serial.current++;},[client]);
 async function load(){const ticket=++serial.current;setBusy(true);const result=await client.getLocalWorkflow();if(ticket!==serial.current)return;setBusy(false);if(result.ok&&result.value?.format_version===1){setRecord(result.value);setMode(result.value.preferences.startup_mode);}else setMessage("Startup preferences are unavailable.");}
 async function save(){if(!record)return;const ticket=++serial.current;setBusy(true);const result=await client.finishLocalStartup(record.revision,mode);if(ticket!==serial.current)return;setBusy(false);if(result.ok){setRecord(result.value);setMessage("Starting workflow saved for the next launch. No import, extraction or activation was started.");}else {setRecord(null);setMessage(`Startup choice was not saved: ${result.error.code}. Review current preferences before retrying.`);}}
 return <section aria-label="Starting workflow preferences"><h3>Starting workflow</h3><p>Choose which workflow opens on the next launch. Data operations remain explicit.</p><Button disabled={busy} onClick={()=>void load()}>Review starting workflow</Button>{record&&<><Field label="Starting workflow selection">{props=><select {...props} value={mode} disabled={busy} onChange={event=>setMode(event.target.value as LocalStartupMode)}><option value="later">Today</option><option value="snapshots">Addon snapshots</option><option value="local_research">Local research and compatibility</option></select>}</Field><Button disabled={busy} onClick={()=>void save()}>Save starting workflow</Button></>}{message&&<p role="status">{message}</p>}</section>;
}
