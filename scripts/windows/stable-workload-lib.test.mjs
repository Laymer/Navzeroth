import {test} from 'node:test';import {readFileSync} from 'node:fs';import assert from 'node:assert/strict';
import {manualSnapshot,scenario,planningRequest,assertActive,WORKLOAD,routeEdge} from './stable-workload-lib.mjs';
const context={build:{edition:'wow_forever',build:'1.60.1.70450',interface:'16001'},namespace:'reference'};
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
 const active={key:{scope:{fixture:false,context},revision:'r'},bundle_defaults:{app_version:'1.0.0'},accepts_inferred_travel:true};assert.equal(assertActive(active,context,'1.0.0'),'r');
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
 const active={key:{scope:{fixture:false,context},revision:'actual-r'},bundle_defaults:{app_version:'1.0.0'},accepts_inferred_travel:true};
 const page=collection=>({revision:'actual-r',catalog_sha256:'a'.repeat(64),collection,offset:0,total:1,next_offset:null});
 const calls=[];
 const call=async(command,args)=>{
  calls.push({command,args});
  switch(command){
   case 'import_snapshot_text':sequence++;return {profile_id:'manual-profile'};
   case 'select_profile':case 'check_database':case 'create_database_backup':return;
   case 'get_app_settings':return {revision,settings:structuredClone(settings)};
   case 'update_app_settings':settings=JSON.parse(args.text).settings;revision++;return {revision,settings:structuredClone(settings)};
   case 'get_active_content':return active;
   case 'list_profiles':return change.missingProfile?[]:[{id:'manual-profile',snapshot_count:sequence}];
   case 'inspect_database_backup':return {candidate_id:'candidate'};
   case 'restore_database_backup':if(change.corruptRecovery)settings.accessibility.text_scale_percent=100;return {};
   case 'get_planning_catalog':return {fixture:change.fixture??false,snapshot_id:`s-${sequence}`,revision:'actual-r',nodes:[{id:'source-node'}],objectives:change.unresolved?[{...objective,requirements:{status:'unknown'}}]:[objective],pages:{quests:page('quests'),items:page('items'),objectives:page('objectives'),edges:page('edges')}};
   case 'get_catalog_page':{const request=JSON.parse(args.text);return {...page(request.collection),records:change.emptyBrowse?[]:request.collection==='edges'?[{id:'source-edge',from:{id:'source-start'},to:{id:'source-node'},evidence:['source-edge-evidence'],requirements:{status:'known',value:{op:'all',predicates:[]}},expected_ms:{status:'known',value:30000}}]:[{id:'actual-source-record'}]};}
   case 'plan_session':return {fixture:false,result:{selected:change.emptyPlan?[]:[{candidate:JSON.parse(args.text).candidates[0].objective,travel:{hops:[{edge:'source-edge'}]}}]},filtered:[],assumptions:[]};
   default:throw new Error('Unexpected IPC '+command);
  }
 };
 return {call,calls};
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
