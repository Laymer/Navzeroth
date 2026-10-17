import {useEffect,useState} from "react";
import {Panel,StatusBadge} from "../ui";
import type {CurrentPlanView} from "../commands/today";
function utc(value:number|null):string {if(value===null)return "time unknown";const date=new Date(value);return Number.isFinite(date.getTime())?date.toISOString():`UTC milliseconds ${value} (calendar display unavailable)`;}
export function SavedPlanCard({plan}:{plan:CurrentPlanView}) {
 const [now,setNow]=useState(()=>Date.now());useEffect(()=>{const timer=window.setInterval(()=>setNow(Date.now()),1000);return()=>clearInterval(timer);},[]);
 const current=plan.state==="current"&&plan.valid_from_utc_ms!=null&&plan.valid_until_utc_ms!=null&&now>=plan.valid_from_utc_ms&&now<=plan.valid_until_utc_ms;
 return <Panel title="Saved session plan" level={3}>
  {plan.state==="empty"?<p>No session plan selected. Use “Save on Today” after calculating a plan.</p>:<>
   <p><StatusBadge tone={current?"ok":"warn"}>{current?"Matches inputs":"Historical"}</StatusBadge> {current?"Current against imported inputs at this check.":"Historical saved advice — inputs or freshness changed. Calculate a fresh plan before using it."}</p>
   {plan.reason==="recovered_reference_requires_new_plan"&&<p>Recovered local reference. Calculate and explicitly save a new plan before using this advice.</p>}
   {plan.fixture&&<p role="note" className="play-note">SYNTHETIC — fixture plan, not verified gameplay advice.</p>}
   {plan.context&&<p>Recorded context: {plan.context.build.edition} / {plan.context.build.build}; interface {plan.context.build.interface}; {plan.context.namespace}. Dataset revision: {plan.dataset_revision??"unavailable"}.</p>}
   {plan.input_snapshots&&<details><summary>Imported inputs used</summary><ul>{plan.input_snapshots.map(input=><li key={input.profile_id}>{input.profile_id}: snapshot {input.snapshot_id}.</li>)}</ul></details>}
   <p>Plan subject: {plan.subject?.kind==="party"?`party ${plan.subject.party_id}`:plan.subject?.kind==="character"?`character ${plan.subject.profile_id}`:"unavailable"}.</p><p>Saved {utc(plan.saved_at_utc_ms)}. Age {plan.age_ms===null?"unknown":`${Math.floor(plan.age_ms/1000)} seconds`}.</p>
   {plan.summary&&<><p>Declared goal: {plan.summary.goal_profile}. Expected session: {plan.summary.expected_total_ms/1000} seconds (manual estimates).</p>
    {current&&<p>Next action: {plan.summary.steps[0]?.objective??"No feasible objective selected"}</p>}
    <details><summary>{!current?"Historical original steps":"Original steps and stopping estimate"}</summary><p>{plan.summary_truncated?`Showing ${plan.summary.steps.length} of ${plan.original_step_count} original steps.`:""}</p><ol>{plan.summary.steps.map(step=><li key={step.objective}>{step.objective} at {step.node}; declared completion {step.completion_ms/1000}s, service {step.service_ms/1000}s.</li>)}</ol><p>Search stopped: {plan.summary.termination.replaceAll("_"," ")}.</p></details></>}
   <p>Desktop imported reference. This does not show addon progress or participant acceptance.</p>
  </>}
 </Panel>;
}
