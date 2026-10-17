import "./play.css";
import { Button, Field, HelpTip } from "../ui";
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
  return <section aria-labelledby="reminder-title" className="play-screen">
    <h2 id="reminder-title">Reminders</h2>
    <p>Advice uses imported snapshots. Combat, loading and cutscene state are unavailable here; reminders stay in this quiet inspection view.</p>
    {error&&<p role="alert">Error {error.code}: {error.message}</p>}
    <Field label="Reminder profile" help="Reminders are evaluated against this character's latest imported snapshot only.">{(c) => <select {...c} disabled={busy} value={profile} onChange={event=>{generation.current++;setView(null);setProfile(event.target.value);}}>{profiles.map(p=><option key={p.id} value={p.id}>{p.display_name}</option>)}</select>}</Field>
    {!profiles.length&&<p>Import a snapshot to inspect reminders.</p>}
    {view&&<>
      {view.fixture&&<p className="demo-label play-note">SYNTHETIC — these rules demonstrate engineering behavior.</p>}
      <span className="play-inline"><label><input type="checkbox" checked={view.enabled} disabled={busy} onChange={event=>void apply({kind:"preferences",enabled:event.target.checked})}/> Enable reminders for this profile</label><HelpTip>Turns reminders on or off for this profile only. Reminders use the latest imported snapshot; nothing is read from the running game.</HelpTip></span>
      <h3>Optional manual checklist</h3>
      <span className="play-inline"><label><input type="checkbox" checked={view.bag_checklist_enabled} disabled={busy} onChange={event=>void apply({kind:"bag_checklist",enabled:event.target.checked,checked:view.bag_checklist_checked})}/> Add a bag-space checklist</label><HelpTip>Bag space is not measured. This is a reminder you tick yourself, and it does not reset on its own.</HelpTip></span>
      {view.bag_checklist_enabled&&<label><input type="checkbox" checked={view.bag_checklist_checked} disabled={busy} onChange={event=>void apply({kind:"bag_checklist",enabled:true,checked:event.target.checked})}/> I checked bag space manually</label>}
      <p>This checklist is your declaration. It does not measure slots or reset automatically; clear the check when you want to repeat it.</p>
      <details><summary>Required inputs for reminder categories</summary><ul>{view.gaps.map(gap=><li key={gap.category}>{gap.explanation}</li>)}</ul></details>
      {view.rules.some(r=>r.delivery!=="verified"&&r.delivery!=="synthetic")&&<><h3>Evidence-gated rules</h3><ul>{view.rules.filter(r=>r.delivery!=="verified"&&r.delivery!=="synthetic").map(r=><li key={r.id}>{r.id}: {r.delivery.replaceAll("_"," ")}. Advice withheld.</li>)}</ul></>}
      {!view.rules.length&&<p>No authored reminder rules are available in the active dataset.</p>}
      {view.plan&&<><p>Presentation: {view.plan.mode.replaceAll("_"," ")}. Snapshot: {view.snapshot_id}; dataset: {view.revision}.</p>
        <ul>{view.plan.queue.map(item=><li key={item.rule}>{item.kind.kind==="advice"?item.kind.text:"Cannot evaluate this reminder with the available inputs."}
          <Button variant="ghost" disabled={busy} onClick={()=>void apply({kind:"acknowledge",rules:[item.rule]})}>Mark {item.rule} as read</Button>
          <Button variant="ghost" disabled={busy} onClick={()=>void apply({kind:"dismiss",rule:item.rule})}>Hide {item.rule}</Button>
        </li>)}</ul>
        {!view.plan.queue.length&&<p>No new advice. Unknown inputs, dismissals and unchanged outcomes may withhold rules.</p>}
        {Object.entries(view.plan.excluded).length>0&&<details><summary>Withheld rules</summary><ul>{Object.entries(view.plan.excluded).map(([id,reason])=><li key={id}>{id}: {reason.code.replaceAll("_"," ")}</li>)}</ul></details>}
      </>}
      {view.dismissed.length>0&&<><h3>Hidden reminders</h3><ul>{view.dismissed.map(id=><li key={id}>{id}{view.rules.some(r=>r.id===id)?<Button disabled={busy} onClick={()=>void apply({kind:"restore",rule:id})}>Restore {id}</Button>:<span> — rule absent from the active dataset; restoration is unavailable until its rule returns.</span>}</li>)}</ul></>}
    </>}
  </section>;
}
