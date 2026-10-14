import {useEffect,useRef,useState} from "react";
import type {ReferenceControlClient,ReferenceControlReceipt,ReferenceWindowState,InteractionMode} from "../commands/reference";
import {ReferenceView} from "./ReferenceView";
import {referenceShortcut} from "./referenceShortcut";
export function ReferenceWindowControls({client,initialTopmost=false}:{client:ReferenceControlClient;initialTopmost?:boolean}) {
 const [state,setState]=useState<ReferenceWindowState|null>(null);const [desiredTopmost,setDesiredTopmost]=useState(false);const [mode,setMode]=useState<InteractionMode>("interactive");const [shortcut,setShortcut]=useState(false);const [receipt,setReceipt]=useState<ReferenceControlReceipt|null>(null);const [error,setError]=useState("");const [busy,setBusy]=useState(false);const [fallback,setFallback]=useState(false);const generation=useRef(0);const owner=useRef(client);owner.current=client;const acceptedOwner=useRef(client);const pending=useRef(false);
 async function run(action:"inspect"|"toggle"|"hide"|"mode"){
  if(pending.current||owner.current!==client)return;pending.current=true;const ticket=++generation.current;const expected=client;setBusy(true);setError("");
  try{
   if(action==="toggle"||action==="mode"){
    const result=action==="toggle"?await client.toggleReferenceWindow(desiredTopmost,mode):await client.setReferenceInteraction(mode);
    if(ticket!==generation.current||owner.current!==expected)return;
    if(!result.ok){setState(null);setReceipt(null);setError(result.error.code);}
    else if(result.value.schema_version!==1){setState(null);setReceipt(null);setError("UNSUPPORTED_SCHEMA");}
    else{acceptedOwner.current=expected;setReceipt(result.value);setState(result.value.window);if(result.value.window)setDesiredTopmost(result.value.window.always_on_top);}
   }else{
    const result=action==="inspect"?await client.getReferenceWindowState():await client.setReferenceWindow(false,false);
    if(ticket!==generation.current||owner.current!==expected)return;
    if(!result.ok){setState(null);setReceipt(null);setError(result.error.code);}
    else if(result.value.schema_version!==1){setState(null);setReceipt(null);setError("UNSUPPORTED_SCHEMA");}
    else{acceptedOwner.current=expected;setState(result.value);setReceipt(null);if(action==="hide")setDesiredTopmost(result.value.always_on_top);}
   }
  }catch{if(ticket===generation.current&&owner.current===expected){setState(null);setReceipt(null);setError("INTERNAL");}}
  finally{if(ticket===generation.current&&owner.current===expected){pending.current=false;setBusy(false);}}
 }
 useEffect(()=>{generation.current++;pending.current=false;setState(null);setReceipt(null);setShortcut(false);setMode("interactive");setDesiredTopmost(initialTopmost);void run("inspect");return()=>{generation.current++;pending.current=false;};},[client,initialTopmost]);
 useEffect(()=>{const onKey=(event:KeyboardEvent)=>{if(owner.current===client&&referenceShortcut(event,shortcut,pending.current,document.hasFocus())){event.preventDefault();void run("toggle");}};window.addEventListener("keydown",onKey);return()=>window.removeEventListener("keydown",onKey);},[client,shortcut,mode,desiredTopmost]);
 const displayed=acceptedOwner.current===client?state:null;const shownReceipt=acceptedOwner.current===client?receipt:null;
 return <section className="reference-controls" aria-label="Reference window controls"><h2>Ordinary reference window</h2><p>Optional, decorated and resizable. It starts closed. Showing it creates no game connection and does not enable any game actions.</p>
 <label><input type="checkbox" disabled={busy} checked={desiredTopmost} onChange={e=>setDesiredTopmost(e.target.checked)}/>Keep on top when shown (explicit preference)</label>
 <label>Requested reference interaction <select disabled={busy} value={mode} onChange={e=>{setMode(e.target.value as InteractionMode);setReceipt(null);}}><option value="interactive">Interactive ordinary window</option><option value="pass_through">Pass through mouse; request nonfocusable</option></select></label>
 <p>Both modes show read-only imported facts. Pass-through requests ignore mouse input, including window controls; use the main desktop to hide or change it. It does not move focus to the game. API acceptance is not an observation of focus or hit-testing.</p>
 <button type="button" disabled={busy||displayed===null||displayed.visible} onClick={()=>void run("toggle")}>Show ordinary reference</button>
 <button type="button" disabled={busy||displayed===null||!displayed.exists} onClick={()=>void run("hide")}>Hide ordinary reference</button>
 <button type="button" disabled={busy||displayed===null||!displayed.exists} onClick={()=>void run("mode")}>Apply requested interaction</button>
 <button type="button" disabled={busy} onClick={()=>void run("inspect")}>Check reference window state</button>
 <label><input type="checkbox" disabled={busy} checked={shortcut} onChange={e=>setShortcut(e.target.checked)}/>Enable Ctrl+Shift+F8 in this main desktop session</label><p>The shortcut works only while this desktop is focused and outside text editors; it is off after restart. There is no global or game shortcut. Native control requires actual main-window focus.</p>
 {displayed&&<p>Last observed window: {displayed.exists?(displayed.visible?"shown":"hidden"):"closed"}; keep on top: {displayed.always_on_top?"on":"off"}. Hide turns keep-on-top off.</p>}
 {shownReceipt&&<p role="status">Requested interaction: {shownReceipt.requested_interaction==="interactive"?"interactive":"pass through"}; dispatch {shownReceipt.dispatch.replaceAll("_"," ")}{shownReceipt.reason?` (${shownReceipt.reason.replaceAll("_"," ")})`:""}. Focusability request: {shownReceipt.operations.focusability.replaceAll("_"," ")}; pointer request: {shownReceipt.operations.pointer_events.replaceAll("_"," ")}. This does not verify native interaction behavior.</p>}
 {error&&<p role="status">Reference window unavailable: {error}. The normal desktop reference remains available.</p>}
 <button type="button" onClick={()=>setFallback(old=>!old)}>{fallback?"Close normal desktop reference":"Open normal desktop reference"}</button>
 {fallback&&<ReferenceView client={client}/>}
 </section>;
}
