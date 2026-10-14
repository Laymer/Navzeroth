import {useEffect,useRef,useState} from "react";
import type {ReferenceClient,ReferenceWindowState} from "../commands/reference";
import {ReferenceView} from "./ReferenceView";
export function ReferenceWindowControls({client,initialTopmost=false}:{client:ReferenceClient;initialTopmost?:boolean}) {
 const [state,setState]=useState<ReferenceWindowState|null>(null);const [desiredTopmost,setDesiredTopmost]=useState(false);const [error,setError]=useState("");const [busy,setBusy]=useState(false);const [fallback,setFallback]=useState(false);const generation=useRef(0);const owner=useRef(client);owner.current=client;const acceptedOwner=useRef(client);
 async function run(visible?:boolean){const ticket=++generation.current;const expected=client;setBusy(true);setError("");try{const result=visible===undefined?await client.getReferenceWindowState():await client.setReferenceWindow(visible,visible&&desiredTopmost);if(ticket!==generation.current||owner.current!==expected)return;if(!result.ok){setState(null);setError(result.error.code);}else if(result.value.schema_version!==1){setState(null);setError("UNSUPPORTED_SCHEMA");}else{acceptedOwner.current=client;setState(result.value);if(visible!==undefined)setDesiredTopmost(result.value.always_on_top);}}catch{if(ticket===generation.current&&owner.current===expected){setState(null);setError("INTERNAL");}}finally{if(ticket===generation.current&&owner.current===expected)setBusy(false);}}
 useEffect(()=>{setState(null);setDesiredTopmost(initialTopmost);void run();return()=>{generation.current++;};},[client,initialTopmost]);
 const displayed=acceptedOwner.current===client?state:null;
 return <section aria-label="Reference window controls"><h2>Ordinary reference window</h2><p>Optional, decorated and resizable. It starts closed. Showing it creates no game connection and does not enable any game actions.</p>
 <label><input type="checkbox" disabled={busy} checked={desiredTopmost} onChange={e=>{generation.current++;setDesiredTopmost(e.target.checked);}}/>Keep on top when shown (explicit preference)</label>
 <button type="button" disabled={busy||displayed===null} onClick={()=>void run(true)}>Show ordinary reference</button>
 <button type="button" disabled={busy||displayed===null||!displayed.exists} onClick={()=>void run(false)}>Hide ordinary reference</button>
 <button type="button" disabled={busy} onClick={()=>void run()}>Check reference window state</button>
 {displayed&&<p>Last observed window: {displayed.exists?(displayed.visible?"shown":"hidden"):"closed"}; keep on top: {displayed.always_on_top?"on":"off"}. Hide turns keep-on-top off.</p>}
 {error&&<p role="status">Reference window unavailable: {error}. The normal desktop reference remains available.</p>}
 <button type="button" onClick={()=>setFallback(old=>!old)}>{fallback?"Close normal desktop reference":"Open normal desktop reference"}</button>
 {fallback&&<ReferenceView client={client}/>}
 </section>;
}
