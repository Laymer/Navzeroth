// Actual UI actions plus native readback. Pure injected I/O enables dev mocks;
// only the installed Windows driver can produce draft engineering evidence.
import {manualSnapshot,scenario,planningRequest,assertActive,WORKLOAD} from './stable-workload-lib.mjs';
import {waitForSyntheticContentInspection} from './native-smoke-lib.mjs';
export const DRAFT_WORKLOAD=WORKLOAD+'; same-node candidate/origin/stop are explicit manual engineering inputs, not source-backed geometry or route coverage';
export function canonical(value){return JSON.stringify(value,(_k,v)=>v&&typeof v==='object'&&!Array.isArray(v)?Object.fromEntries(Object.entries(v).sort(([a],[b])=>a.localeCompare(b))):v);}
export function draftSnapshot(context,now,sequence=1,assertions={}){
 const value=manualSnapshot(context,now,sequence,assertions);
 value.payload.id=`draft-ui-engineering-${sequence}`;
 value.payload.character='draft-ui-engineering-workload';
 if(new TextEncoder().encode(JSON.stringify(value)).byteLength>65536)throw Error('Draft manual snapshot exceeds bounded capacity');
 return value;
}
export function contentState(active){return {active_revision:active.key.revision,accepts_inferred_travel:active.accepts_inferred_travel,bundle_defaults:structuredClone(active.bundle_defaults)};}
export function assertDraftCatalog(catalog,revision){
 if(catalog?.fixture!==false||catalog.revision!==revision||typeof catalog.snapshot_id!=='string'||!catalog.snapshot_id||!Array.isArray(catalog.nodes)||!catalog.nodes.length||catalog.nodes.length>200||!Array.isArray(catalog.objectives)||catalog.objectives.length>200)throw Error('Draft planning catalog lacks actual admitted source scope');
 for(const collection of ['nodes','objectives']){
  const pin=catalog.pages?.[collection];
  if(pin?.collection!==collection||pin.revision!==revision||!/^[a-f0-9]{64}$/.test(pin.catalog_sha256)||pin.offset!==0||!Number.isSafeInteger(pin.total)||pin.total<catalog[collection].length||pin.total>1000000||catalog[collection].length!==Math.min(200,pin.total)||pin.next_offset!==(catalog[collection].length<pin.total?catalog[collection].length:null))throw Error('Draft planning catalog pin is invalid');
 }
 return catalog;
}
export async function objectivePage(call,profile,catalog,offset){
 const pin=catalog.pages.objectives;
 if(!Number.isSafeInteger(offset)||offset<0||offset>pin.total||offset%200!==0)throw Error('Draft objective offset is invalid');
 const page=await call('get_catalog_page',{text:JSON.stringify({subject:{kind:'character',profile_id:profile},collection:'objectives',revision:pin.revision,catalog_sha256:pin.catalog_sha256,offset})});
 if(page.collection!=='objectives'||page.revision!==pin.revision||page.catalog_sha256!==pin.catalog_sha256||page.offset!==offset||page.total!==pin.total||!Array.isArray(page.records)||page.records.length!==Math.min(200,page.total-offset)||page.records.length>200||page.next_offset!==(offset+page.records.length<page.total?offset+page.records.length:null))throw Error('Draft objective page differs from issued source pin');
 return page;
}
export async function sourceScenario(call,profile,catalog,wanted){
 assertDraftCatalog(catalog,catalog.revision);
 let records=catalog.objectives,offset=0,next=catalog.pages.objectives.next_offset;
 for(let scan=0;scan<128;scan++){
  for(const objective of records){
   if(wanted&&objective.id!==wanted)continue;
   try{return {selected:scenario(objective,catalog.nodes[0]),offset};}catch{}
  }
  if(next===null||next===undefined)break;
  if(!Number.isSafeInteger(next)||next!==offset+200)throw Error('Draft objective pagination is not bounded');
  const page=await objectivePage(call,profile,catalog,next);records=page.records;offset=next;next=page.next_offset;
 }
 throw Error('No source-backed quest stage supports the declared manual draft scenario');
}
export function assertPinned(today,saved,restart=false){
 const plan=today?.current_plan;
 if(today?.schema_version!==1||!plan||!Number.isSafeInteger(plan.revision)||plan.revision<1||!['current','historical'].includes(plan.state)||(!restart&&plan.state!=='current')||plan.fixture!==false||canonical(plan.context)!==canonical(saved.context)||plan.dataset_revision!==saved.active_revision||plan.subject?.kind!=='character'||plan.subject.profile_id!==saved.profile_id||!Array.isArray(plan.input_snapshots)||!plan.input_snapshots.some(x=>x.profile_id===saved.profile_id&&x.snapshot_id===saved.snapshot_id)||plan.summary?.schema_version!==1||plan.summary.goal_profile!=='manual-engineering-v1'||!Array.isArray(plan.summary.steps)||plan.summary.steps.length!==1)throw Error('Rendered plan does not match the actual native pinned source request');
 const step=plan.summary.steps[0];
 if(step.objective!==saved.scenario.objective||step.node!==saved.scenario.node||step.completion_ms!==1000||step.service_ms!==0||!Number.isSafeInteger(plan.summary.expected_total_ms)||plan.summary.expected_total_ms<1000)throw Error('Native pinned plan is empty or differs from manual UI controls');
 if(restart&&(plan.revision!==saved.pinned_plan.revision||canonical(plan.summary)!==canonical(saved.pinned_plan.summary)))throw Error('Native pinned plan did not persist across restart');
 return {revision:plan.revision,summary:structuredClone(plan.summary),state:plan.state,reason:plan.reason};
}
export function assertDraftState(saved,identity,context){
 if(saved?.schema_version!==1||saved.phase!=='draft_ui_saved'||canonical(saved.identity)!==canonical(identity)||canonical(saved.context)!==canonical(context)||typeof saved.profile_id!=='string'||!saved.profile_id||!saved.scenario||!saved.pinned_plan||!saved.settings||saved.accepts_inferred_travel!==false||saved.bundle_defaults?.app_version!==identity.version)throw Error('Draft restart state lacks exact installer identity and persistence evidence');
 return saved;
}
async function selectIfDifferent(locator,value,timeout){if(await locator.inputValue({timeout})!==value)await locator.selectOption(value,{timeout});}
async function navigate(page,name,timeout){await page.getByRole('navigation',{name:'Primary',exact:true}).getByRole('button',{name,exact:true}).click({timeout});}
async function contentUi(page,profile,active,timeout){await navigate(page,'Content',timeout);await waitForSyntheticContentInspection(page,{profile_id:profile,...active.key},timeout);}
async function settingsUi(page,profile,timeout,change){
 await navigate(page,'Settings',timeout);
 const text=page.getByLabel('Text size',{exact:true}),subject=page.getByLabel('Character or party',{exact:true});
 if(change){await selectIfDifferent(text,'125',timeout);await selectIfDifferent(subject,`character:${profile}`,timeout);await page.getByRole('button',{name:'Save local settings',exact:true}).click({timeout});await page.getByText('Local settings saved. Watching and windows remain explicit controls.',{exact:true}).waitFor({timeout});}
 if(await text.inputValue({timeout})!=='125'||await subject.inputValue({timeout})!==`character:${profile}`)throw Error('Rendered local settings did not persist');
}
async function showToday(page,saved,timeout){
 await navigate(page,'Today',timeout);
 const panel=page.getByRole('heading',{name:'Saved session plan',exact:true});await panel.waitFor({timeout});
 const details=page.getByText(/^(Historical original steps|Original steps and stopping estimate)$/);
 await details.click({timeout});
 await page.getByText(`${saved.scenario.objective} at ${saved.scenario.node}; declared completion 1s, service 0s.`,{exact:true}).waitFor({timeout});
}
export async function exerciseDraftUi(page,call,{mode,identity,context,packDirectories,saved,capture=async()=>{},timeout=()=>1200000,onPhase=()=>{}}){
 if(!['fresh','restart'].includes(mode)||identity?.version!=='1.0.0'||!/^[a-f0-9]{40}$/.test(identity.source_commit)||!/^[a-f0-9]{64}$/.test(identity.installer_sha256))throw Error('Invalid draft UI source or installer identity');
 const checks={first_start:false,bundled_activation:false,source_backed_planning:false,restart_persistence:false};
 if(mode==='restart'){
  onPhase('ui_restart_native');assertDraftState(saved,identity,context);
  const profiles=await call('list_profiles');if(!profiles.some(x=>x.id===saved.profile_id))throw Error('Draft profile did not persist');
  const settings=await call('get_app_settings');if(canonical(settings.settings)!==canonical(saved.settings))throw Error('Draft settings did not persist');
  const active=await call('get_active_content',{profile_id:saved.profile_id});assertActive(active,context,identity.version,false);
  if(canonical(contentState(active))!==canonical({active_revision:saved.active_revision,accepts_inferred_travel:saved.accepts_inferred_travel,bundle_defaults:saved.bundle_defaults}))throw Error('Draft active revision or false inferred choice did not persist');
  const pinned=assertPinned(await call('get_today_view'),saved,true);
  onPhase('ui_restart_content');await contentUi(page,saved.profile_id,active,timeout());await page.getByText('Active revision travel estimates: disabled.',{exact:true}).waitFor({timeout:timeout()});await capture('restart-content');
  onPhase('ui_restart_settings');await settingsUi(page,saved.profile_id,timeout(),false);await capture('restart-settings');
  onPhase('ui_restart_today');await showToday(page,saved,timeout());await capture('restart-today');
  checks.first_start=true;checks.restart_persistence=true;
  return {state:saved,evidence:{checks,ui_interaction_verified:true,profile_id:saved.profile_id,active_revision:active.key.revision,persisted_content:contentState(active),pinned_plan:pinned,workload:DRAFT_WORKLOAD}};
 }
 if(!Array.isArray(packDirectories)||packDirectories.length!==2||new Set(packDirectories.map(x=>x.revision)).size!==2)throw Error('Draft UI needs both distinct installed admitted packs');
 const receipt=await call('import_snapshot_text',{text:'ANAV1:'+JSON.stringify(draftSnapshot(context,Date.now()))});
 const profile=receipt.profile_id;if(typeof profile!=='string'||!profile)throw Error('Draft profile import failed');
 await call('select_profile',{profile_id:profile});
 let active=await call('get_active_content',{profile_id:profile});const revision=assertActive(active,context,identity.version,true);checks.first_start=true;
 onPhase('ui_content');await contentUi(page,profile,active,timeout());await capture('fresh-defaults');
 for(const [index,pack] of packDirectories.entries()){
  onPhase(`ui_stage_pack_${index+1}`);
  await page.getByLabel('Verified content pack directory',{exact:true}).fill(pack.path,{timeout:timeout()});
  await page.getByRole('button',{name:'Verify and stage content pack',exact:true}).click({timeout:timeout()});
  const build=context.build;
  await page.getByText(`${pack.revision}: ${build.edition}/${build.build}/${build.interface}, ${context.namespace}`,{exact:true}).waitFor({timeout:timeout()});
 }
 onPhase('ui_activate_set');const inferred=page.getByRole('checkbox',{name:/^Accept inferred travel \(estimates\):/});
 if(!await inferred.isChecked())throw Error('Draft UI did not show the initial inferred default');
 await inferred.uncheck({timeout:timeout()});
 await page.getByRole('button',{name:'Activate all staged packs as one set',exact:true}).click({timeout:timeout()});
 await page.getByText('Active revision travel estimates: disabled.',{exact:true}).waitFor({timeout:timeout()});
 active=await call('get_active_content',{profile_id:profile});assertActive(active,context,identity.version,false);
 if(active.key.revision!==revision||active.accepts_inferred_travel!==false||active.bundle_defaults.revision!==revision)throw Error('UI activation changed the admitted set or lost the selected false choice');
 checks.bundled_activation=true;await capture('fresh-activated-content');
 onPhase('ui_settings');await settingsUi(page,profile,timeout(),true);
 const settings=(await call('get_app_settings')).settings;
 if(settings.accessibility?.text_scale_percent!==125||settings.selected_subject?.profile_id!==profile||settings.selected_subject.kind!=='character')throw Error('UI settings differ from native stored state');
 await capture('fresh-settings');
 onPhase('ui_source_scenario');let catalog=assertDraftCatalog(await call('get_planning_catalog',{profile_id:profile}),revision);
 let found=await sourceScenario(call,profile,catalog);
 const update=await call('import_snapshot_text',{text:'ANAV1:'+JSON.stringify(draftSnapshot(context,Date.now(),2,found.selected.assertions))});if(update.profile_id!==profile)throw Error('Draft manual assertions changed profile identity');
 catalog=assertDraftCatalog(await call('get_planning_catalog',{profile_id:profile}),revision);
 found=await sourceScenario(call,profile,catalog,found.selected.objective);
 const selected=found.selected,request=planningRequest(profile,catalog,selected);
 onPhase('ui_planner_controls');await navigate(page,'Planner',timeout());await selectIfDifferent(page.getByLabel('Planning profile',{exact:true}),profile,timeout());
 await page.getByText(`Snapshot: ${catalog.snapshot_id}; revision: ${revision}`,{exact:true}).waitFor({timeout:timeout()});
 for(let offset=0;offset<found.offset;offset+=200){await page.getByRole('button',{name:'Next objectives',exact:true}).click({timeout:timeout()});await page.getByRole('navigation',{name:'objectives pages',exact:true}).getByText(`objectives: ${offset+201}–${Math.min(offset+400,catalog.pages.objectives.total)} of ${catalog.pages.objectives.total}. Selections on other pages are retained.`,{exact:true}).waitFor({timeout:timeout()});}
 await page.getByLabel('Origin',{exact:true}).selectOption(request.origin,{timeout:timeout()});
 await page.getByLabel('Stopping node',{exact:true}).selectOption(request.stopping_node,{timeout:timeout()});
 await page.getByLabel('Versioned goal identifier',{exact:true}).fill(request.goal_profile,{timeout:timeout()});
 await page.getByLabel('Budget (ms)',{exact:true}).fill(String(request.budget_ms),{timeout:timeout()});
 await page.getByLabel('Stopping service (ms)',{exact:true}).fill('0',{timeout:timeout()});
 const objective=page.getByRole('group',{name:selected.objective,exact:true});
 await objective.getByRole('checkbox',{name:'Consider objective',exact:true}).check({timeout:timeout()});
 await objective.getByRole('combobox',{name:'Objective node',exact:true}).selectOption(selected.node,{timeout:timeout()});
 for(const [label,value] of [['utility',10],['completion_ms',1000],['service_ms',0]])await objective.getByLabel(label,{exact:true}).fill(String(value),{timeout:timeout()});
 onPhase('ui_plan_job');await page.getByRole('button',{name:'Plan session',exact:true}).click({timeout:timeout()});
 const result=page.getByRole('region',{name:'Planning result',exact:true});
 await result.getByText(`${selected.objective}: utility 10; travel 0 ms; completion 1000 ms; service 0 ms`,{exact:true}).waitFor({timeout:timeout()});
 await result.getByRole('button',{name:'Export plan',exact:true}).waitFor({timeout:timeout()});
 if(!await result.getByRole('button',{name:'Export plan',exact:true}).isEnabled())throw Error('Rendered UI plan is empty');
 onPhase('ui_save_plan');await result.getByRole('button',{name:'Save on Today',exact:true}).click({timeout:timeout()});
 await result.getByText('Saved on Today. It will be checked against current imported inputs when read.',{exact:true}).waitFor({timeout:timeout()});
 const state={schema_version:1,phase:'draft_ui_saved',identity,context,profile_id:profile,...contentState(active),settings,scenario:selected,snapshot_id:catalog.snapshot_id};
 state.pinned_plan=assertPinned(await call('get_today_view'),state);
 checks.source_backed_planning=true;await capture('fresh-planning-result');
 onPhase('ui_today');await showToday(page,state,timeout());await capture('fresh-today');
 return {state,evidence:{checks,ui_interaction_verified:true,profile_id:profile,active_revision:revision,persisted_content:contentState(active),pinned_plan:state.pinned_plan,scenario:selected,workload:DRAFT_WORKLOAD}};
}
