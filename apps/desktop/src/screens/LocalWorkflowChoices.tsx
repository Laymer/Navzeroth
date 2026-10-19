import {useEffect,useRef,useState} from "react";
import type {CommandClient} from "../commands";
import type {LocalChangeDetector,LocalDataPreset,LocalWorkflowRecord} from "../commands/localData";
import {Button} from "../ui";

export function LocalWorkflowChoices({client,selectionId,locales,preset,onLocales}:{client:CommandClient;selectionId:string;locales:string[];preset:LocalDataPreset;onLocales:(locales:string[])=>void}) {
  const [record,setRecord]=useState<LocalWorkflowRecord|null>(null);
  const [selected,setSelected]=useState<string[]>([]);
  const [detect,setDetect]=useState(false);
  const [views,setViews]=useState<LocalChangeDetector[]>([]);
  const dirty=useRef(false);
  const [busy,setBusy]=useState(false);
  const [message,setMessage]=useState("");
  const serial=useRef(0);
  // The parent remounts on selection change; every outstanding response is discarded.
  useEffect(()=>{
    let current=true;
    void client.getLocalWorkflow().then(result=>{
      if(!current)return;
      if(result.ok&&result.value?.format_version===1){
        setRecord(result.value);
        const choices=result.value.preferences.selections[selectionId];
        if(choices&&!dirty.current){
          const installed=[...new Set(choices.locales.filter(locale=>locales.includes(locale)))].sort();
          setSelected(installed);onLocales(installed);setDetect(choices.change_detection);
          if(choices.locales.some(locale=>!locales.includes(locale)))setMessage("Some saved content locales are no longer declared by this installation. Review the available choices before extracting; no replacement locale was selected.");
        }
      }else setMessage("Saved choices unavailable. Extraction still requires explicit locale selection.");
    });
    return()=>{current=false;serial.current++;};
  },[client,selectionId,onLocales,locales]);
  async function save(){
    if(!record)return;const ticket=++serial.current;setBusy(true);
    const result=await client.saveLocalWorkflowChoices(selectionId,record.revision,selected,preset,detect);
    if(ticket!==serial.current)return;setBusy(false);
    if(result.ok){setRecord(result.value);setMessage("Choices saved. Build changes require review and explicit extraction.");}
    else {setRecord(null);setMessage(`Choices were not saved: ${result.error.code}. Reopen the installation to reload current choices.`);}
  }
  async function review(){
    const ticket=++serial.current;setBusy(true);const result=await client.getLocalChangeDetectors();
    if(ticket!==serial.current)return;setBusy(false);
    if(result.ok)setViews(result.value.filter(view=>view.selection_id===selectionId));
    else setMessage(`Change status unavailable: ${result.error.code}.`);
  }
  async function acknowledge(candidate:string){
    if(!record)return;const ticket=++serial.current;setBusy(true);
    const result=await client.acknowledgeLocalBuildChange(selectionId,record.revision,candidate);
    if(ticket!==serial.current)return;setBusy(false);
    if(result.ok){setRecord(result.value);setViews([]);setMessage("Reviewed build is the new detection baseline. No extraction or activation was performed.");}
    else setMessage(`Build review refused: ${result.error.code}. Refresh change status before retrying.`);
  }
  return <fieldset className="local-research-pane"><legend>Installed content choices</legend>
    <p>Select content locales explicitly. The desktop interface remains English.</p>
    {locales.map(locale=><label key={locale}><input type="checkbox" checked={selected.includes(locale)} disabled={busy} onChange={event=>{
      dirty.current=true;const next=(event.target.checked?[...selected,locale]:selected.filter(value=>value!==locale)).sort();setSelected(next);onLocales(next);
    }}/>{locale}</label>)}
    {locales.length===0&&<p>Installed content locales are unknown; extraction cannot start.</p>}
    <label><input type="checkbox" checked={detect} disabled={busy} onChange={event=>{dirty.current=true;setDetect(event.target.checked);}}/>Watch selected build metadata while the app is open</label>
    <p>Checks every 500 ms with a 750 ms stability window. Changes stage private metadata for review. Cache files are never watched.</p>
    <Button disabled={!record||busy||selected.length===0} onClick={()=>void save()}>Save content choices</Button>
    <Button disabled={busy} onClick={()=>void review()}>Review detected build changes</Button>
    {views.map(view=><div key={view.selection_id}><p>Build detection: {view.status}</p>{view.candidate_sha256&&<><p>Staged metadata {view.candidate_sha256}. No automatic activation.</p><Button disabled={!record||busy} onClick={()=>{if(view.candidate_sha256)void acknowledge(view.candidate_sha256);}}>Acknowledge staged build metadata</Button></>}</div>)}
    {message&&<p role="status">{message}</p>}
  </fieldset>;
}
