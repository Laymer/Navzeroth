import { useEffect, useRef, useState } from "react";
import type { AppError, CommandClient, ProfileSummary } from "../commands";
import type { ReminderAction, ReminderView } from "../commands/reminders";

export function ReminderScreen({client,refreshKey}:{client:CommandClient;refreshKey:number}) {
  const [profiles,setProfiles]=useState<ProfileSummary[]>([]);
  const [profile,setProfile]=useState("");
  const [view,setView]=useState<ReminderView|null>(null);
  const [error,setError]=useState<AppError|null>(null);
  const [busy,setBusy]=useState(false);
  const generation=useRef(0);
  const mutation=useRef(false);
  useEffect(()=> {let current=true; void client.listProfiles().then(result=> {if(!current)return;if(result.ok){setProfiles(result.value);setProfile(old=>result.value.some(p=>p.id===old)?old:result.value[0]?.id??"");}else setError(result.error);});return()=>{current=false;};},[client,refreshKey]);
  useEffect(()=> {let current=true;generation.current++;setView(null);setError(null);if(profile)void client.reminderView(profile).then(result=>{if(!current)return;if(result.ok)setView(result.value);else setError(result.error);});return()=>{current=false;generation.current++;};},[client,profile,refreshKey]);
  async function apply(action:ReminderAction) {
    if(!view||mutation.current)return;
    mutation.current=true;setBusy(true);setError(null);
    const request=generation.current;
    const result=await client.mutateReminders({profile_id:profile,snapshot_id:view.snapshot_id,revision:view.revision,action});
    mutation.current=false;setBusy(false);
    if(request!==generation.current)return;
    if(result.ok)setView(result.value);else setError(result.error);
  }
  return <section aria-labelledby="reminder-title">
    <h2 id="reminder-title">Reminders</h2>
    <p>Advice uses imported snapshots. Combat, loading and cutscene state are unavailable here; reminders stay in this quiet inspection view.</p>
    {error&&<p role="alert">Error {error.code}: {error.message}</p>}
    <label>Reminder profile <select disabled={busy} value={profile} onChange={event=>{generation.current++;setView(null);setProfile(event.target.value);}}>{profiles.map(p=><option key={p.id} value={p.id}>{p.display_name}</option>)}</select></label>
    {!profiles.length&&<p>Import a snapshot to inspect reminders.</p>}
    {view&&<>
      {view.fixture&&<p className="demo-label">SYNTHETIC — these rules demonstrate engineering behavior.</p>}
      <label><input type="checkbox" checked={view.enabled} disabled={busy} onChange={event=>void apply({kind:"preferences",enabled:event.target.checked})}/> Enable reminders for this profile</label>
      {!view.rules.length&&<p>No authored reminder rules are available in the active dataset.</p>}
      {view.plan&&<><p>Presentation: {view.plan.mode.replaceAll("_"," ")}. Snapshot: {view.snapshot_id}; dataset: {view.revision}.</p>
        <ul>{view.plan.queue.map(item=><li key={item.rule}>{item.kind.kind==="advice"?item.kind.text:"Cannot evaluate this reminder with the available inputs."}
          <button disabled={busy} onClick={()=>void apply({kind:"acknowledge",rules:[item.rule]})}>Mark {item.rule} as read</button>
          <button disabled={busy} onClick={()=>void apply({kind:"dismiss",rule:item.rule})}>Hide {item.rule}</button>
        </li>)}</ul>
        {!view.plan.queue.length&&<p>No new advice. Unknown inputs, dismissals and unchanged outcomes may withhold rules.</p>}
        {Object.entries(view.plan.excluded).length>0&&<details><summary>Withheld rules</summary><ul>{Object.entries(view.plan.excluded).map(([id,reason])=><li key={id}>{id}: {reason.code.replaceAll("_"," ")}</li>)}</ul></details>}
      </>}
      {view.dismissed.length>0&&<><h3>Hidden reminders</h3><ul>{view.dismissed.map(id=><li key={id}>{id}<button disabled={busy} onClick={()=>void apply({kind:"restore",rule:id})}>Restore {id}</button></li>)}</ul></>}
    </>}
  </section>;
}
