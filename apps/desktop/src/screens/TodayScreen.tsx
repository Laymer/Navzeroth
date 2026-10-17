import "./play.css";
import { Button } from "../ui";
import {useEffect,useRef,useState} from "react";
import type {TodayClient,TodayView} from "../commands/today";
import {describeAge} from "../atlas/describe";
import {SavedPlanCard} from "./SavedPlanCard";
export function TodayScreen({client,refreshKey=0}:{client:TodayClient;refreshKey?:number}) {
 const epoch=useRef(0);const identity=useRef({client,refreshKey});if(identity.current.client!==client||identity.current.refreshKey!==refreshKey){identity.current={client,refreshKey};epoch.current++;}
 const [view,setView]=useState<{epoch:number;data:TodayView}|null>(null);const [error,setError]=useState("");const [busy,setBusy]=useState<number|null>(null);
 async function refresh(){const ticket=++epoch.current;setBusy(ticket);setError("");const result=await client.getTodayView();if(ticket!==epoch.current)return;setBusy(null);if(result.ok&&result.value.schema_version===1&&["empty","historical","current"].includes(result.value.current_plan.state)&&Number.isSafeInteger(result.value.current_plan.revision)){setView({epoch:ticket,data:result.value});}else{setView(null);setError(result.ok?"Unsupported Today response":result.error.message);}}
 useEffect(()=>{void refresh();const timer=window.setInterval(()=>void refresh(),60000);return()=>{clearInterval(timer);epoch.current++;};},[client,refreshKey]);
 const current=view?.epoch===epoch.current?view.data:null;
 async function clear(){if(!current)return;const expected=current.current_plan.revision;const ticket=++epoch.current;setBusy(ticket);const result=await client.clearCurrentPlan(expected);if(ticket!==epoch.current)return;setBusy(null);if(!result.ok){setError(result.error.message);setView(null);}else await refresh();}
 return <section className="today-screen play-screen" aria-labelledby="today-heading"><h2 id="today-heading">Today</h2><Button disabled={busy===epoch.current} onClick={()=>void refresh()}>Refresh imported advice</Button>
 {busy===epoch.current&&<p role="status">Checking saved inputs…</p>}{error&&<p role="alert">{error}</p>}
 {current&&<><p>Selected {current.selected_subject?.kind??"subject"}: {current.selected_subject?.kind==="character"?current.members[0]?.character??"missing character":current.selected_subject?.kind==="party"?current.selected_subject.party_id:"none"}.</p>
 {!current.selection_available&&<p>Select an available character or party in Settings. Deleted or missing subjects remain unavailable.</p>}
 <p>Current plan goal: {current.current_plan.state==="current"?current.current_plan.summary?.goal_profile??"unavailable":"no current plan goal"}.</p>
 <p>Current manual goal: {current.manual_goal??"not configured"}.</p><ul>{current.members.map(member=><li key={member.profile_id}>{member.character}: {member.freshness?describeAge(member.freshness):"No imported current snapshot"}.</li>)}</ul>
 <SavedPlanCard plan={current.current_plan}/><Button variant="danger" disabled={current.current_plan.state==="empty"} onClick={()=>void clear()}>Clear saved plan</Button>
 <p>Choose installation, character/party and local defaults in Settings.</p></>}
 </section>;
}
