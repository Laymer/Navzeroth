// Engineering intent only: never addon, official or live observation evidence.
export const WORKLOAD = 'Synthetic engineering workload: source-backed quest stage and declared travel edge; manual progress and candidate-location/completion/utility estimates; no live-client verification';
export function manualSnapshot(context, now, sequence = 1, assertions = {}) {
  if (context.namespace !== 'reference' || !Number.isSafeInteger(now) || !Number.isSafeInteger(sequence) || sequence < 1) throw new Error('Invalid engineering snapshot context/clock');
  const session = 'stable-engineering-session';
  const known = value => ({status:'known',value,source:'manual_entry',clock_trust:'trusted',observed_at_utc_ms:now,session_id:session,sequence});
  const progress = value => value === undefined ? {status:'unknown',source:'manual_entry'} : known(value);
  const quests = Object.fromEntries(Object.entries(assertions.quests ?? {}).map(([quest, states]) => [quest,{quest,context,accepted:progress(states.accepted),completed:progress(states.completed),turned_in:progress(states.turned_in),objectives:{}}]));
  return {format:'azeroth-navigator',kind:'character_snapshot',schema_version:1,fixture:false,
    producer:{component:'desktop',version:'1.0.0',session_id:session,sequence},
    game:{...context.build,locale:'enUS'},namespace:context.namespace,
    provenance:{source:'manual_entry',clock_trust:'trusted',created_at_utc_ms:now},
    payload:{schema_version:1,id:`stable-engineering-${sequence}`,character:'synthetic-engineering-workload',context,
      capture:{source:'manual_entry',clock_trust:'trusted',observed_at_utc_ms:now,session_id:session,sequence},source_revision:'manual-engineering-v1',level:known(60),
      role:{status:'unknown',source:'manual_entry'},location:{status:'unknown',source:'manual_entry'},preferences:{},equipment:{},quests,
      objectives:Object.fromEntries(Object.entries(assertions.objectives ?? {}).map(([id,value])=>[id,known(value)]))}};
}
function assign(states, key, value) {
  if (Object.hasOwn(states,key) && states[key] !== value) throw new Error('Conflicting engineering assertion');
  states[key] = value;
}
function predicate(value, assertions, depth=0) {
  if (!value || depth > 32) throw new Error('Unsupported engineering predicate');
  switch(value.op) {
    case 'all': for(const child of value.predicates) predicate(child,assertions,depth+1); break;
    case 'level_at_least': if(value.level > 60) throw new Error('Engineering level cannot satisfy source eligibility'); break;
    case 'quest_state': if(!['accepted','completed','turned_in'].includes(value.state)||typeof value.value!=='boolean')throw new Error('Unsupported manual quest state'); assertions.quests[value.quest] ??= Object.create(null); assign(assertions.quests[value.quest],value.state,value.value); break;
    case 'objective_state': assign(assertions.objectives,value.objective,value.completed); break;
    default: throw new Error('Unsupported engineering predicate');
  }
}
export function routeEdge(edge) {
 if(!edge?.id||!edge.from?.id||!edge.to?.id||edge.from.id===edge.to.id||!edge.evidence?.length||edge.requirements?.status!=='known'||edge.expected_ms?.status!=='known'||!Number.isFinite(edge.expected_ms.value)||edge.expected_ms.value<0)throw new Error('No supported declared travel edge');
 predicate(edge.requirements.value,{quests:Object.create(null),objectives:Object.create(null)});
 return edge;
}
export function scenario(objective, node, edge) {
  if (!objective?.id?.startsWith('quest-stage-') || objective.requirements?.status !== 'known' || !objective.evidence?.length || objective.targets?.length !== 1 || objective.targets[0].kind !== 'quest_stage' || !node?.id) throw new Error('No source-backed supported quest-stage scenario');
  const assertions = {quests:Object.create(null),objectives:Object.create(null)};
  predicate(objective.requirements.value,assertions);
  if(edge)predicate(routeEdge(edge).requirements.value,assertions);
  const {quest,stage}=objective.targets[0]; assertions.quests[quest] ??= Object.create(null);
  for(const [state,value] of [['accepted',true],['completed',false],['turned_in',false]]) assign(assertions.quests[quest],state,value);
  assign(assertions.objectives,stage,false);
  return {assertions,objective:objective.id,node:edge?.to.id??node.id,origin:edge?.from.id??node.id,route_edge:edge,evidence:objective.evidence,quest,stage,disclosure:WORKLOAD};
}
export function planningRequest(profile, catalog, selected) {
  if (catalog.fixture || !catalog.snapshot_id || !catalog.revision) throw new Error('Planning catalog is not the active admitted scope');
  return {profile_id:profile,snapshot_id:catalog.snapshot_id,revision:catalog.revision,goal_profile:'manual-engineering-v1',origin:selected.origin??selected.node,stopping_node:selected.node,budget_ms:86400000,stopping_service_ms:0,
    candidates:[{objective:selected.objective,node:selected.node,utility:10,completion_ms:1000,service_ms:0}],excluded:[]};
}
export function assertActive(active, context, version, requireDefaults = true) {
  if (!active || active.key.scope.fixture || JSON.stringify(active.key.scope.context) !== JSON.stringify(context) || (requireDefaults && (active.bundle_defaults?.app_version !== version || active.accepts_inferred_travel !== true))) throw new Error('Verified bundled defaults are not active for the engineering scope');
  return active.key.revision;
}

// I/O-independent native workflow; the Windows driver supplies the actual IPC.
export async function exerciseInstalled(call, {context,version,mode,saved,backup}) {
 if(!['baseline','fresh','restart','recover','offline'].includes(mode))throw new Error('Unknown native engineering workload mode');
 const evidence={};
 if(mode==='baseline'){
  const snapshot=manualSnapshot(context,Date.now());
  const receipt=await call('import_snapshot_text',{text:'ANAV1:'+JSON.stringify(snapshot)});
  await call('select_profile',{profile_id:receipt.profile_id});
  const settings=await call('get_app_settings');settings.settings.accessibility.text_scale_percent=125;
  const changed=await call('update_app_settings',{text:JSON.stringify({schema_version:1,expected_revision:settings.revision,settings:settings.settings})});
  saved={schema_version:1,profile_id:receipt.profile_id,context,sequence:1,baseline_version:version,settings:changed.settings,workload:WORKLOAD};
  evidence.profile_id=receipt.profile_id;
 }else{
  saved=mode==='fresh'?{context,sequence:1}:structuredClone(saved);
  if(JSON.stringify(saved.context)!==JSON.stringify(context))throw new Error('Saved manual scope differs from installed bundle');
  let profile=saved.profile_id;
  if(mode==='fresh'){
   const receipt=await call('import_snapshot_text',{text:'ANAV1:'+JSON.stringify(manualSnapshot(context,Date.now(),saved.sequence))});profile=receipt.profile_id;
  }
  await call('select_profile',{profile_id:profile});
  const settings=await call('get_app_settings');
  if(saved.settings && JSON.stringify(saved.settings)!==JSON.stringify(settings.settings))throw new Error('Application settings did not persist');
  if(mode==='fresh'){
   settings.settings.accessibility.text_scale_percent=125;
   const changed=await call('update_app_settings',{text:JSON.stringify({schema_version:1,expected_revision:settings.revision,settings:settings.settings})});saved.settings=changed.settings;
  }
  const active=await call('get_active_content',{profile_id:profile});
  const revision=assertActive(active,context,version,mode==='fresh'||!saved.active_revision);
  if(mode!=='fresh'&&saved.active_revision&&revision!==saved.active_revision)throw new Error('Active revision did not persist');
  const profiles=await call('list_profiles');if(!profiles.some(p=>p.id===profile))throw new Error('Manual engineering profile did not persist');
  if(mode==='recover'){
   if(!backup)throw new Error('Recovery workload requires a new selected backup path');
   await call('create_database_backup',{path:backup});
   const preview=await call('inspect_database_backup',{path:backup});
   await call('restore_database_backup',{candidate_id:preview.candidate_id});
   assertActive(await call('get_active_content',{profile_id:profile}),context,version,false);
   await call('check_database');
   const restoredSettings=await call('get_app_settings'),restoredProfiles=await call('list_profiles');
   if(JSON.stringify(restoredSettings.settings)!==JSON.stringify(saved.settings)||JSON.stringify(restoredProfiles)!==JSON.stringify(profiles))throw new Error('Backup recovery did not retain profiles and settings');
   evidence.recovery_verified=true;
  }
  const browse=performance.now();
  let catalog=await call('get_planning_catalog',{profile_id:profile});
  if(catalog.fixture||!catalog.revision||!catalog.nodes?.length||!catalog.pages?.quests?.total)throw new Error('Actual installed world catalog is empty');
  async function records(collection,offset=0){const pin=catalog.pages[collection];if(!pin)throw new Error('Missing actual catalog collection');return call('get_catalog_page',{text:JSON.stringify({subject:{kind:'character',profile_id:profile},collection,revision:pin.revision,catalog_sha256:pin.catalog_sha256,offset})});}
  const questPage=await records('quests');if(!questPage.records.length)throw new Error('Actual quest browsing returned no records');
  const itemPage=await records('items');if(!itemPage.records.length)throw new Error('Actual item browsing returned no records');
  evidence.browse_ms=performance.now()-browse;evidence.catalog_totals=Object.fromEntries(Object.entries(catalog.pages).map(([k,v])=>[k,v.total]));
  let edge,edgeOffset=0;
  for(let scanned=0;scanned<128&&!edge;scanned++){
   const page=await records('edges',edgeOffset);
   for(const candidate of page.records){try{edge=routeEdge(candidate);break;}catch{}}
   if(edge||page.next_offset===null||page.next_offset===undefined)break;
   edgeOffset=page.next_offset;
  }
  if(!edge)throw new Error('Actual full-pack planning requires a supported declared source travel edge');
  let selected;
  let objectives=catalog.objectives,next=catalog.pages.objectives?.next_offset;
  for(let scanned=0;scanned<128&&!selected;scanned++){
   for(const objective of objectives){try{selected=scenario(objective,catalog.nodes[0],edge);break;}catch{}}
   if(selected||next===null||next===undefined)break;
   const page=await records('objectives',next);objectives=page.records;next=page.next_offset;
  }
  if(!selected)throw new Error('No source-backed quest stage has supported resolved eligibility for this engineering scenario');
  saved.sequence++;
  const receipt=await call('import_snapshot_text',{text:'ANAV1:'+JSON.stringify(manualSnapshot(context,Date.now(),saved.sequence,selected.assertions))});
  if(receipt.profile_id!==profile)throw new Error('Manual update changed profile identity');
  catalog=await call('get_planning_catalog',{profile_id:profile});
  const request=planningRequest(profile,catalog,selected),planning=performance.now();
  const report=await call('plan_session',{text:JSON.stringify(request)});
  evidence.planning_ms=performance.now()-planning;
  if(report.fixture||!report.result?.selected?.some(row=>row.candidate===selected.objective&&row.travel?.hops?.length>0))throw new Error('Actual source-backed objective was not selected by the planner');
  evidence.scenario=selected;evidence.plan={selected:report.result.selected,filtered:report.filtered,assumptions:report.assumptions};
  saved={...saved,profile_id:profile,active_revision:revision,workload:WORKLOAD};
  evidence.profile_id=profile;evidence.active_revision=revision;evidence.restart_verified=mode==='restart';
 }
 return {state:saved,evidence};
}
