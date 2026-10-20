import {test} from 'node:test';import {readFileSync} from 'node:fs';import assert from 'node:assert/strict';
import {manualSnapshot,scenario,planningRequest,assertActive,WORKLOAD,routeEdge} from './stable-workload-lib.mjs';
const context={build:{edition:'wow_forever',build:'1.60.1.70450',interface:'16001'},namespace:'reference'};
const catalogDto=JSON.parse(readFileSync(new URL('../../crates/app/tests/support/stable_catalog_dto.json',import.meta.url),'utf8'));
const objective={id:'quest-stage-source',requirements:{status:'known',value:{op:'all',predicates:[{op:'level_at_least',level:10},{op:'quest_state',quest:'prerequisite',state:'turned_in',value:true}]}},evidence:['source-quest'],targets:[{kind:'quest_stage',quest:'quest-source',stage:'stage-source'}]};
test('manual engineering observations never claim addon or live provenance',()=>{
 const selected=scenario(objective,{id:'source-node'}),snapshot=manualSnapshot(context,1000,2,selected.assertions);
 assert.equal(snapshot.fixture,false);assert.equal(snapshot.provenance.source,'manual_entry');assert.equal(snapshot.producer.component,'desktop');assert.equal(snapshot.payload.quests.prerequisite.turned_in.value,true);assert.equal(snapshot.payload.quests.prerequisite.accepted.status,'unknown');assert.equal(snapshot.payload.objectives['stage-source'].value,false);assert.match(WORKLOAD,/no live-client/);assert.doesNotMatch(JSON.stringify(snapshot),/addon_snapshot|synthetic_fixture|demo-/);
 const request=planningRequest('profile',{fixture:false,snapshot_id:'s',revision:'r'},selected);assert.equal(request.candidates[0].node,'source-node');
 assert.throws(()=>manualSnapshot({...context,namespace:'demo'},1000));assert.throws(()=>manualSnapshot(context,1.1));assert.throws(()=>manualSnapshot(context,1000,0));assert.throws(()=>planningRequest('p',{fixture:true},selected));
});
test('unresolved, unsupported, contradictory and absent source scenarios fail closed',()=>{
 for(const value of [{...objective,evidence:[]},{...objective,id:'authored'},{...objective,requirements:{status:'unknown'}},{...objective,targets:[]},{...objective,requirements:{status:'known',value:{op:'level_at_least',level:61}}},{...objective,requirements:{status:'known',value:{op:'quest_state',quest:'quest-source',state:'accepted',value:false}}},{...objective,requirements:{status:'known',value:{op:'unsupported'}}}])assert.throws(()=>scenario(value,{id:'node'}));
 assert.throws(()=>scenario(objective,{}));
 const changed=structuredClone(objective);changed.requirements.value={op:'objective_state',objective:'stage-source',completed:true};assert.throws(()=>scenario(changed,{id:'node'}));
});
test('active scope and version defaults must match the signed installed bundle',()=>{
 const active={key:{scope:{fixture:false,context},revision:'r'},bundle_defaults:{app_version:'1.0.0',previous_app_version:null,revision:'r'},accepts_inferred_travel:true};assert.equal(assertActive(active,context,'1.0.0'),'r');
 for(const value of [null,{...active,accepts_inferred_travel:false},{...active,bundle_defaults:{app_version:'old'}},{...active,key:{scope:{fixture:true,context}}}])assert.throws(()=>assertActive(value,context,'1.0.0'));
 assert.throws(()=>assertActive(active,{...context,namespace:'other'},'1.0.0'));
 assert.equal(assertActive({...active,accepts_inferred_travel:false},context,'1.0.0',false),'r');
});

test('manual snapshot shape matches the independently decoded Rust contract fixture',()=>{
 const selected=scenario({...objective,requirements:{status:'known',value:{op:'all',predicates:[{op:'quest_state',quest:'prerequisite',state:'turned_in',value:true}]}}},{id:'source-node'});
 const expected=JSON.parse(readFileSync(new URL('../../crates/app/tests/support/stable_engineering_snapshot.json',import.meta.url),'utf8'));assert.deepEqual(manualSnapshot(context,1000,2,selected.assertions),expected);
});
import {exerciseInstalled} from './stable-workload-lib.mjs';
function nativeMock(change={}){
 let sequence=0,settings={accessibility:{text_scale_percent:100}},revision=0;
 const active={key:{scope:{fixture:false,context},revision:'actual-r'},bundle_defaults:{app_version:'1.0.0',previous_app_version:null,revision:'actual-r'},accepts_inferred_travel:true};
 const page=(collection,offset=0)=>{const total=change.paged&&['edges','objectives'].includes(collection)?201:1;return {revision:'actual-r',catalog_sha256:collection==='objectives'?'b'.repeat(64):'a'.repeat(64),collection,offset,total,next_offset:offset+200<total?offset+200:null};};
 const calls=[];
 const call=async(command,args)=>{
  calls.push({command,args});
  switch(command){
   case 'import_snapshot_text':sequence++;return {profile_id:'manual-profile'};
   case 'select_profile':case 'check_database':case 'create_database_backup':return;
   case 'get_app_settings':return {revision,settings:structuredClone(settings)};
   case 'update_app_settings':settings=JSON.parse(args.text).settings;revision++;return {revision,settings:structuredClone(settings)};
   case 'get_active_content':return structuredClone(active);
   case 'activate_content_set':if(!change.ignoreChoice)active.accepts_inferred_travel=args.accept_inferred_travel;if(change.toggleRevision)active.key.revision='other';if(change.toggleDefaults)active.bundle_defaults.previous_app_version='changed';return {};
   case 'list_profiles':return change.missingProfile?[]:[{id:'manual-profile',snapshot_count:sequence}];
   case 'inspect_database_backup':return {candidate_id:'candidate'};
   case 'restore_database_backup':if(change.corruptRecovery)settings.accessibility.text_scale_percent=100;if(change.recoveryActive)Object.assign(active,structuredClone(change.recoveryActive));return Object.hasOwn(change,'restoreReceipt')?structuredClone(change.restoreReceipt):{pending_marker_cleanup_required:false,original_files_retained:true,recovery_directory:'synthetic-retained-originals'};
   case 'get_database_status':return Object.hasOwn(change,'restoredStatus')?structuredClone(change.restoredStatus):{state:'ready',error:null};
   case 'get_planning_catalog':return {fixture:change.fixture??false,snapshot_id:`s-${sequence}`,revision:'actual-r',nodes:[{id:'source-node'}],objectives:change.paged?Array.from({length:200},()=>({...objective,requirements:{status:'unknown'}})):change.unresolved?[{...objective,requirements:{status:'unknown'}}]:[objective],pages:change.missingNodes?{objectives:page('objectives')}:{...structuredClone(catalogDto.planning_catalog.pages),objectives:page('objectives')}};
   case 'get_catalog_page':{
    const request=JSON.parse(args.text);
    if(request.revision!=='actual-r'||request.catalog_sha256!==catalogDto.planning_catalog.pages[request.collection==='objectives'?'objectives':'nodes'].catalog_sha256)throw new Error('Wrong request pin');
    const edge={id:'source-edge',from:{id:'source-start'},to:{id:'source-node'},evidence:['source-edge-evidence'],requirements:{status:'known',value:{op:'all',predicates:[]}},expected_ms:{status:'known',value:30000}};
    const rows=change.emptyBrowse?[]:request.collection==='edges'?(change.paged&&request.offset===0?Array.from({length:200},()=>({...edge,requirements:{status:'unknown'}})):[edge]):request.collection==='objectives'?[objective]:[{id:'actual-source-record'}];
    return {...page(request.collection,request.offset),...change.pageOverride,records:rows};
   }
   case 'plan_session':return {fixture:false,result:{selected:change.emptyPlan?[]:[{candidate:JSON.parse(args.text).candidates[0].objective,travel:{hops:[{edge:'source-edge'}]}}]},filtered:[],assumptions:[]};
   default:throw new Error('Unexpected IPC '+command);
  }
 };
 return {call,calls,active};
}
test('native engineering workflow browses, plans, restarts and restores actual IPC-shaped state',async()=>{
 const mock=nativeMock();
 const fresh=await exerciseInstalled(mock.call,{context,version:'1.0.0',mode:'fresh'});assert.equal(fresh.evidence.plan.selected[0].candidate,objective.id);assert.equal(fresh.state.active_revision,'actual-r');assert.equal(fresh.state.settings.accessibility.text_scale_percent,125);assert.ok(fresh.evidence.browse_ms>=0);
 const restart=await exerciseInstalled(mock.call,{context,version:'1.0.0',mode:'restart',saved:fresh.state});assert.equal(restart.evidence.restart_verified,true);
 const restored=await exerciseInstalled(mock.call,{context,version:'1.0.0',mode:'recover',saved:restart.state,backup:'new-selected-backup.sqlite3'});assert.equal(restored.evidence.recovery_verified,true);
 assert.ok(mock.calls.some(c=>c.command==='check_database'));assert.ok(mock.calls.some(c=>c.command==='get_catalog_page'));
 const baseline=await exerciseInstalled(nativeMock().call,{context,version:'1.0.0-rc.1',mode:'baseline'});assert.equal(baseline.state.baseline_version,'1.0.0-rc.1');
});
test('actual workflow cannot promote missing profiles, unresolved source goals, empty plans or changed recovery settings',async()=>{
 for(const change of [{unresolved:true},{emptyPlan:true},{fixture:true},{emptyBrowse:true},{missingProfile:true}])await assert.rejects(exerciseInstalled(nativeMock(change).call,{context,version:'1.0.0',mode:'fresh'}));
 const mock=nativeMock({corruptRecovery:true}),fresh=await exerciseInstalled(mock.call,{context,version:'1.0.0',mode:'fresh'});
 await assert.rejects(exerciseInstalled(mock.call,{context,version:'1.0.0',mode:'recover',saved:fresh.state,backup:'new.sqlite3'}),/retain profiles and settings/);
 const healthy=nativeMock(),ready=await exerciseInstalled(healthy.call,{context,version:'1.0.0',mode:'fresh'});
 await assert.rejects(exerciseInstalled(healthy.call,{context,version:'1.0.0',mode:'recover',saved:ready.state}),/backup path/);
 await assert.rejects(exerciseInstalled(mock.call,{context,version:'1.0.0',mode:'unknown'}),/Unknown/);
});
import {spawnSync} from 'node:child_process';
import {fileURLToPath} from 'node:url';
test('native driver refuses unsupported environment or missing explicit inputs before side effects',()=>{
 const result=spawnSync(process.execPath,[fileURLToPath(new URL('./Stable-Workload.mjs',import.meta.url))],{encoding:'utf8',timeout:10000});
 assert.notEqual(result.status,0);assert.match(result.stderr,/requires a disposable hosted Windows runner|Missing stable workload parameters/);
});

test('full-pack route scenario keeps declared estimates and distinct source nodes',()=>{
 const edge={id:'walk-source',from:{id:'source-start'},to:{id:'source-node'},requirements:{status:'known',value:{op:'all',predicates:[]}},expected_ms:{status:'known',value:30000},evidence:['derived-source']};
 assert.equal(routeEdge(edge),edge);const selected=scenario(objective,{id:'source-node'},edge);assert.equal(selected.origin,'source-start');assert.equal(selected.node,'source-node');assert.equal(selected.route_edge.expected_ms.value,30000);
 for(const value of [{...edge,expected_ms:{status:'unknown'}},{...edge,evidence:[]},{...edge,to:edge.from},{...edge,requirements:{status:'unknown'}}])assert.throws(()=>routeEdge(value));
});


test('saved state binds false inferred choice and exact defaults across restart, recovery and offline work',async()=>{
 const mock=nativeMock(),fresh=await exerciseInstalled(mock.call,{context,version:'1.0.0',mode:'fresh'});
 assert.equal(fresh.state.schema_version,2);
 assert.equal(fresh.evidence.initial_content.accepts_inferred_travel,true);
 assert.equal(fresh.state.accepts_inferred_travel,false);
 assert.deepEqual(fresh.state.bundle_defaults,{app_version:'1.0.0',previous_app_version:null,revision:'actual-r'});
 for(const mode of ['restart','recover','offline']){
  const start=mock.calls.length;
  const result=await exerciseInstalled(mock.call,{context,version:'1.0.0',mode,saved:fresh.state,backup:'new.sqlite3'});
  assert.equal(result.evidence.initial_content.accepts_inferred_travel,false);
  assert.equal(result.state.accepts_inferred_travel,false);
  assert.deepEqual(result.state.bundle_defaults,fresh.state.bundle_defaults);
  const calls=mock.calls.slice(start),toggle=calls.filter(c=>c.command==='activate_content_set');
  assert.deepEqual(toggle.map(c=>c.args),[true,false].map(accept=>({profile_id:'manual-profile',revisions:['actual-r'],accept_inferred_travel:accept})));
  assert.ok(calls.findIndex(c=>c.command==='get_active_content')<calls.findIndex(c=>c.command==='activate_content_set'));
  assert.ok(calls.findIndex(c=>c.command==='plan_session')<calls.lastIndexOf(toggle[1]));
  assert.equal(result.evidence.planning_inferred_travel,true);
 }
});
test('same-version lost or reset choice, defaults markers and active revision cannot pass restart',async()=>{
 for(const mutate of [
  active=>{active.accepts_inferred_travel=true;},
  active=>{delete active.accepts_inferred_travel;},
  active=>{active.bundle_defaults.app_version='old';},
  active=>{active.bundle_defaults.previous_app_version='other';},
  active=>{active.bundle_defaults.revision='other';},
  active=>{delete active.bundle_defaults;},
  active=>{active.key.revision='other';},
 ]){
  const mock=nativeMock(),fresh=await exerciseInstalled(mock.call,{context,version:'1.0.0',mode:'fresh'});
  mutate(mock.active);const start=mock.calls.length;
  await assert.rejects(exerciseInstalled(mock.call,{context,version:'1.0.0',mode:'restart',saved:fresh.state}),/persist|defaults/);
  assert.ok(!mock.calls.slice(start).some(c=>c.command==='activate_content_set'));
 }
});
test('backup restore must retain false choice, exact revision and full defaults marker',async()=>{
 for(const recoveryActive of [
  {accepts_inferred_travel:true},
  {key:{scope:{fixture:false,context},revision:'other'}},
  {bundle_defaults:{app_version:'1.0.0',previous_app_version:'old',revision:'actual-r'}},
  {bundle_defaults:{app_version:'old',previous_app_version:null,revision:'actual-r'}},
 ]){
  const mock=nativeMock({recoveryActive}),fresh=await exerciseInstalled(mock.call,{context,version:'1.0.0',mode:'fresh'});
  const start=mock.calls.length;
  await assert.rejects(exerciseInstalled(mock.call,{context,version:'1.0.0',mode:'recover',saved:fresh.state,backup:'new.sqlite3'}),/persist|defaults/);
  assert.ok(!mock.calls.slice(start).some(c=>c.command==='activate_content_set'));
 }
});
test('fresh and retained RC upgrade require true version defaults before selecting false',async()=>{
 const mock=nativeMock();
 const baseline=await exerciseInstalled(mock.call,{context,version:'1.0.0-rc.1',mode:'baseline'});
 const upgraded=await exerciseInstalled(mock.call,{context,version:'1.0.0',mode:'restart',saved:baseline.state});
 assert.equal(upgraded.evidence.initial_content.accepts_inferred_travel,true);
 assert.equal(upgraded.state.baseline_version,'1.0.0-rc.1');
 for(const mode of ['fresh','restart']){
  const changed=nativeMock();
  const retained=await exerciseInstalled(changed.call,{context,version:'1.0.0-rc.1',mode:'baseline'});
  changed.active.accepts_inferred_travel=false;
  await assert.rejects(exerciseInstalled(changed.call,{context,version:'1.0.0',mode,saved:retained.state}),/defaults/);
 }
});
test('state v1 and missing saved preference cannot silently fall back to fresh acceptance',async()=>{
 const mock=nativeMock(),fresh=await exerciseInstalled(mock.call,{context,version:'1.0.0',mode:'fresh'});
 for(const mutate of [s=>{s.schema_version=1;},s=>{delete s.accepts_inferred_travel;},s=>{delete s.bundle_defaults;},s=>{delete s.active_revision;},s=>{s.phase='baseline';}]){
  const saved=structuredClone(fresh.state);mutate(saved);
  await assert.rejects(exerciseInstalled(mock.call,{context,version:'1.0.0',mode:'restart',saved}),/persist|versioned/);
 }
});
test('ignored preference writes, revision or default changes fail closed and plan failure restores false',async()=>{
 for(const change of [{ignoreChoice:true},{toggleRevision:true},{toggleDefaults:true}]){
  await assert.rejects(exerciseInstalled(nativeMock(change).call,{context,version:'1.0.0',mode:'fresh'}),/persist/);
 }
 const mock=nativeMock(),fresh=await exerciseInstalled(mock.call,{context,version:'1.0.0',mode:'fresh'});
 const failing=async(command,args)=>command==='plan_session'?Promise.reject(new Error('controlled plan failure')):mock.call(command,args);
 await assert.rejects(exerciseInstalled(failing,{context,version:'1.0.0',mode:'restart',saved:fresh.state}),/controlled plan failure/);
 assert.equal(mock.active.accepts_inferred_travel,false);
});

test('actual serialized planning DTO omits other collections and pages stay globally hash bound',async()=>{
 assert.deepEqual(Object.keys(catalogDto.planning_catalog.pages).sort(),['nodes','objectives']);
 assert.equal(catalogDto.quest_page.page,undefined);assert.equal(catalogDto.quest_page.collection,'quests');
 const mock=nativeMock(),result=await exerciseInstalled(mock.call,{context,version:'1.0.0',mode:'fresh'});
 for(const collection of ['quests','items','edges']){
  const request=JSON.parse(mock.calls.find(c=>c.command==='get_catalog_page'&&JSON.parse(c.args.text).collection===collection).args.text);
  assert.equal(request.catalog_sha256,catalogDto.planning_catalog.pages.nodes.catalog_sha256);
  assert.equal(result.evidence.catalog_totals[collection],1);
 }
 for(const pageOverride of [{revision:'stale'},{catalog_sha256:'c'.repeat(64)},{collection:'other'},{offset:1},{total:0},{next_offset:1}]){
  await assert.rejects(exerciseInstalled(nativeMock({pageOverride}).call,{context,version:'1.0.0',mode:'fresh'}),/exact requested binding/);
 }
});

test('actual flattened pages continue past unsupported first pages with separate objective binding',async()=>{
 const mock=nativeMock({paged:true});
 const result=await exerciseInstalled(mock.call,{context,version:'1.0.0',mode:'fresh'});
 const requests=mock.calls.filter(c=>c.command==='get_catalog_page').map(c=>JSON.parse(c.args.text));
 assert(requests.some(r=>r.collection==='edges'&&r.offset===200&&r.catalog_sha256==='a'.repeat(64)));
 assert(requests.some(r=>r.collection==='objectives'&&r.offset===200&&r.catalog_sha256==='b'.repeat(64)));
 assert.equal(result.evidence.plan.selected[0].candidate,objective.id);
 assert.equal(result.evidence.catalog_totals.edges,201);
});

test('failed catalog checks retain bounded diagnostic identities without records',async()=>{
 let diagnostic;
 await assert.rejects(exerciseInstalled(nativeMock({missingNodes:true}).call,{context,version:'1.0.0',mode:'fresh',onCatalog:value=>{diagnostic=value;}}),/lacks its active revision/);
 assert.deepEqual(diagnostic,{fixture:false,revision:'actual-r',snapshot_id:'s-1',node_count:1,objective_count:1,pages:[{collection:'objectives',revision:'actual-r',catalog_sha256:'b'.repeat(64),total:1}]});
 assert(!JSON.stringify(diagnostic).includes('source-quest'));
 assert(JSON.stringify(diagnostic).length<16384);
});

test('recovery requires actual complete receipt and ready error-free status before verification',async()=>{
 const mock=nativeMock(),fresh=await exerciseInstalled(mock.call,{context,version:'1.0.0',mode:'fresh'}),start=mock.calls.length;
 const result=await exerciseInstalled(mock.call,{context,version:'1.0.0',mode:'recover',saved:fresh.state,backup:'synthetic-selected-backup.sqlite3'});
 assert.equal(result.evidence.recovery_verified,true);
 assert.deepEqual(result.evidence.recovery_receipt,{pending_marker_cleanup_required:false,original_files_retained:true,recovery_directory:'synthetic-retained-originals',recovery_directory_truncated:false});
 assert.deepEqual(result.evidence.recovery_database_status,{state:'ready',error_present:false});
 const calls=mock.calls.slice(start).map(c=>c.command);
 assert(calls.indexOf('restore_database_backup')<calls.indexOf('get_database_status'));
 assert(calls.indexOf('get_database_status')<calls.indexOf('check_database'));
 assert(calls.indexOf('check_database')<calls.indexOf('plan_session'));
});
test('pending missing malformed and unretained restore receipts cannot pass recovery',async()=>{
 const good={pending_marker_cleanup_required:false,original_files_retained:true,recovery_directory:'synthetic-originals'};
 const bad=[undefined,null,{},true,...[true,'false',0,null].map(value=>({...good,pending_marker_cleanup_required:value})),...[false,'true',null].map(value=>({...good,original_files_retained:value})),...['','  ',null,1].map(value=>({...good,recovery_directory:value}))];
 for(const field of Object.keys(good)){const missing={...good};delete missing[field];bad.push(missing);}
 for(const restoreReceipt of bad){
  const mock=nativeMock({restoreReceipt}),fresh=await exerciseInstalled(mock.call,{context,version:'1.0.0',mode:'fresh'}),start=mock.calls.length;let diagnostic;
  await assert.rejects(exerciseInstalled(mock.call,{context,version:'1.0.0',mode:'recover',saved:fresh.state,backup:'selected',onRecovery:value=>{diagnostic=value;}}),/restore receipt/);
  assert(diagnostic?.receipt);assert(JSON.stringify(diagnostic).length<2048);
  assert(!mock.calls.slice(start).some(c=>['get_database_status','check_database','plan_session'].includes(c.command)));
 }
});
test('checking failed missing or error-bearing restored database status refuses recovery',async()=>{
 for(const restoredStatus of [undefined,null,{}, {state:'checking',error:null},{state:'recovery_required',error:null},{state:'ready'},{state:'ready',error:{code:'DatabaseError',message:'Synthetic retained marker'}}]){
  const mock=nativeMock({restoredStatus}),fresh=await exerciseInstalled(mock.call,{context,version:'1.0.0',mode:'fresh'}),start=mock.calls.length;let diagnostic;
  await assert.rejects(exerciseInstalled(mock.call,{context,version:'1.0.0',mode:'recover',saved:fresh.state,backup:'selected',onRecovery:value=>{diagnostic=value;}}),/not ready without an error/);
  assert(diagnostic.database_status);
  assert(!mock.calls.slice(start).some(c=>['check_database','plan_session'].includes(c.command)));
 }
});
test('recovery diagnostics retain only bounded actual receipt metadata on failure',async()=>{
 const restoreReceipt={pending_marker_cleanup_required:true,original_files_retained:true,recovery_directory:'x'.repeat(8192),private_dump:'not captured'};
 const mock=nativeMock({restoreReceipt}),fresh=await exerciseInstalled(mock.call,{context,version:'1.0.0',mode:'fresh'});let diagnostic;
 await assert.rejects(exerciseInstalled(mock.call,{context,version:'1.0.0',mode:'recover',saved:fresh.state,backup:'selected',onRecovery:value=>{diagnostic=value;}}),/restore receipt/);
 assert.equal(diagnostic.receipt.recovery_directory.length,512);assert.equal(diagnostic.receipt.recovery_directory_truncated,true);
 assert(!JSON.stringify(diagnostic).includes('private_dump'));
});

test('bounded progress retains only command/phase timing and never acceptance success', async () => {
 const {createWorkloadProgress}=await import('./stable-workload-progress.mjs');
 const {mkdtempSync,readFileSync,rmSync}=await import('node:fs');
 const {tmpdir}=await import('node:os');const {join}=await import('node:path');
 const dir=mkdtempSync(join(tmpdir(),'anav-progress-')),path=join(dir,'partial.json');let now=100;
 try{
  const progress=createWorkloadProgress(path,{mode:'recover',version:'1.0.0',now:()=>now++});
  progress.phase('exercise');
  for(let n=0;n<100;n++)assert.equal(await progress.track('get_database_status',async()=>42),42);
  const error=new Error('PRIVATE_ERROR_NOT_RETAINED');
  await assert.rejects(progress.track('restore_database_backup',async()=>{throw error;}),e=>e===error);
  const raw=readFileSync(path,'utf8'),value=JSON.parse(raw);
  assert.equal(value.passed,false);assert.equal(value.completed,false);assert.equal(value.history.length,32);
  assert.equal(value.active_command,'restore_database_backup');assert.equal(value.history.at(-1).stage,'error');
  assert.ok(Buffer.byteLength(raw)<16384);assert.ok(!raw.includes('PRIVATE_ERROR'));
  assert.throws(()=>progress.phase('bad\nlabel'),/Invalid/);
 }finally{rmSync(dir,{recursive:true,force:true});}
});

test('forced child kill retains the complete pending IPC checkpoint without finally', async () => {
 const {spawn}=await import('node:child_process');const {once}=await import('node:events');
 const {mkdtempSync,readFileSync,rmSync}=await import('node:fs');
 const {tmpdir}=await import('node:os');const {join}=await import('node:path');
 const dir=mkdtempSync(join(tmpdir(),'anav-progress-kill-')),path=join(dir,'partial.json');
 const moduleUrl=new URL('./stable-workload-progress.mjs',import.meta.url).href;
 const script=`import {createWorkloadProgress} from ${JSON.stringify(moduleUrl)};const p=createWorkloadProgress(process.argv[1],{mode:'recover',version:'1.0.0'});p.phase('exercise');await p.track('inspect_database_backup',async()=>{process.stdout.write('ready');setInterval(()=>{},1000);await new Promise(()=>{});});`;
 const child=spawn(process.execPath,['--input-type=module','-e',script,path],{stdio:['ignore','pipe','pipe']});
 try{
  await Promise.race([once(child.stdout,'data'),new Promise((_,reject)=>setTimeout(()=>reject(Error('Synthetic child did not checkpoint')),5000).unref())]);
  const exit=once(child,'exit');child.kill('SIGKILL');await exit;
  const partial=JSON.parse(readFileSync(path,'utf8'));
  assert.equal(partial.active_command,'inspect_database_backup');assert.equal(partial.phase,'exercise');
  assert.equal(partial.history.at(-1).stage,'before');assert.equal(partial.passed,false);assert.equal(partial.completed,false);
 }finally{if(child.exitCode===null&&child.signalCode===null)child.kill('SIGKILL');rmSync(dir,{recursive:true,force:true});}
});
