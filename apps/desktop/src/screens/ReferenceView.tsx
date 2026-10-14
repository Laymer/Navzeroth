import {SavedPlanCard} from "./SavedPlanCard";
import {useEffect,useRef,useState} from "react";
import type {ReferenceClient,ReferenceViewData} from "../commands/reference";
import {describeObservation} from "../observation";
import {describeAge} from "../atlas/describe";
import "./reference.css";
export function ReferenceView({client}:{client:Pick<ReferenceClient,"getReferenceView">}) {
 const [view,setView]=useState<ReferenceViewData|null>(null);const [error,setError]=useState("");const [busy,setBusy]=useState(false);const generation=useRef(0);const owner=useRef(client);owner.current=client;const acceptedOwner=useRef(client);
 async function refresh(){const ticket=++generation.current;const expected=client;setBusy(true);setError("");try{const result=await client.getReferenceView();if(ticket!==generation.current||owner.current!==expected)return;if(!result.ok){setView(null);setError(result.error.code);}else if(result.value.schema_version!==1){setView(null);setError("UNSUPPORTED_SCHEMA");}else {acceptedOwner.current=client;setView(result.value);const a=result.value.accessibility;if(a){document.documentElement.style.fontSize=`${a.text_scale_percent}%`;document.documentElement.dataset.contrast=a.contrast;document.documentElement.dataset.motion=a.motion;}}}catch{if(ticket===generation.current&&owner.current===expected){setView(null);setError("INTERNAL");}}finally{if(ticket===generation.current&&owner.current===expected)setBusy(false);}}
 useEffect(()=>{setView(null);void refresh();const timer=window.setInterval(()=>void refresh(),60000);return()=>{generation.current++;window.clearInterval(timer);};},[client]);
 const displayed=acceptedOwner.current===client?view:null;const snapshot=displayed?.snapshot;
 return <section className="reference-view" aria-label="Imported reference"><h2>Imported reference</h2><p>Read-only imported snapshot facts. This window has no continuous game-state feed. The in-game Lua HUD remains the primary live guidance surface.</p>
 <button type="button" disabled={busy} onClick={()=>void refresh()}>Refresh imported reference</button>
 {busy&&<p role="status">Checking imported state…</p>}{error&&<p role="alert">Imported reference unavailable: {error}. Use the normal desktop or recovery controls.</p>}
 {displayed&&<p>App {displayed.app_version}. Ages are checked by the desktop, including a local refresh every minute.</p>}
 {displayed&&snapshot===null&&<p>No selected imported snapshot.</p>}
 {snapshot&&<><p>{snapshot.fixture||snapshot.context.namespace==="demo"?"SYNTHETIC — fixture reference, not from your game.":"Imported snapshot state; these facts can age or be replaced."}</p><dl>
 <dt>Character</dt><dd>{snapshot.character}</dd><dt>Snapshot</dt><dd>{snapshot.snapshot_id}</dd><dt>Recorded build</dt><dd>{snapshot.context.build.edition} / {snapshot.context.build.build}; interface {snapshot.context.build.interface}</dd>
 <dt>Capture age</dt><dd>{describeAge(snapshot.capture_freshness)}</dd><dt>Level</dt><dd>{describeObservation(snapshot.level)}; {describeAge(snapshot.level_freshness)}</dd>
 <dt>Role</dt><dd>{describeObservation(snapshot.role)}; {describeAge(snapshot.role_freshness)}</dd><dt>Location</dt><dd>{describeObservation(snapshot.location,p=>`${p.map.map_id} / floor ${p.map.floor} (${p.x}, ${p.y})`)}; {describeAge(snapshot.location_freshness)}</dd></dl></>}
 <p>This is an ordinary desktop window. Visibility in exclusive fullscreen is unsupported by this experiment; borderless, focus, DPI and multiple-monitor behavior remain unverified. Use the normal desktop reference if this window is unsuitable.</p>
 {displayed?.current_plan&&!busy&&<SavedPlanCard plan={displayed.current_plan}/>}
 </section>;
}
