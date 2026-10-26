// Dev-only actual React screens with synthetic IPC; never installer acceptance.
import {useState} from 'react';
import {createRoot} from 'react-dom/client';
import {ContentScreen} from '../../src/screens/ContentScreen';
import {PlannerScreen} from '../../src/screens/PlannerScreen';
import {SettingsScreen} from '../../src/screens/SettingsScreen';
import {TodayScreen} from '../../src/screens/TodayScreen';
import {createClient,type CommandAdapter} from '../../src/commands';
import {defaultSettings} from '../../src/commands/today';
import golden from '../../../../fixtures/desktop/content-governance/synthetic-flow.json';
const context={build:{edition:'wow_forever',build:'1.60.1.70450',interface:'16001'},namespace:'reference'};
const profile='stored-'+ 'a'.repeat(64),revision='draft-ui-admitted-set';
let exists=false,snapshot='draft-ui-engineering-1',settings={revision:0,settings:defaultSettings()},accepts=true;
const calls:string[]=[];let failure="";
const objective={id:'quest-stage-ui-source',requirements:{status:'known',value:{op:'level_at_least',level:10}},expected_ms:{status:'unknown'},evidence:['mock-source-proof'],targets:[{kind:'quest_stage',quest:'quest-ui',stage:'stage-ui'}]};
const unknowns=Array.from({length:200},(_,i)=>({...objective,id:`quest-stage-unknown-${i}`,requirements:{status:'unknown'}}));
function pin(collection:string,offset=0){const total=collection==='objectives'?201:1;return {collection,revision,catalog_sha256:(collection==='objectives'?'b':'a').repeat(64),offset,total,next_offset:offset+200<total?offset+200:null};}
function inspection(name=revision){return {...golden.inspection,key:{revision:name,scope:{context,fixture:false}},bundle_defaults:{app_version:'1.0.0',previous_app_version:null,revision},accepts_inferred_travel:accepts};}
function catalog(){return {fixture:false,revision,snapshot_id:snapshot,nodes:[{id:'ui-declared-node'}],objectives:unknowns,pages:{nodes:pin('nodes'),objectives:pin('objectives')}};}
type Request={profile_id:string;snapshot_id:string;revision:string;goal_profile:string;origin:string;stopping_node:string;budget_ms:number;stopping_service_ms:number;candidates:{objective:string;node:string;utility:number;completion_ms:number;service_ms:number}[];excluded:string[]};
let request:Request|null=null;
function report(){if(!request||request.candidates.length!==1)throw Error('No actual UI request');const selected=request.candidates[0];if(!selected)throw Error('Missing considered UI objective');return {fixture:false,context,request,result:{selected:failure==='empty-ui-plan'?[]:[{candidate:selected.objective,utility:selected.utility,completion_ms:selected.completion_ms,service_ms:selected.service_ms,expected_session_ms:1000,travel:{total_ms:0}}],expected_total_ms:1000,stopping:{total_ms:0},alternatives:[],trace:[]},filtered:[],assumptions:['Mock same-node engineering scenario only']};}
function newPlan(){if(!request)throw Error('No UI request to pin');const selected=request.candidates[0];if(!selected)throw Error('No UI candidate');return {revision:1,state:'current',fixture:false,context,dataset_revision:revision,subject:{kind:'character',profile_id:failure==='wrong-native-plan'?'wrong-profile':profile},input_snapshots:[{profile_id:profile,snapshot_id:snapshot}],saved_at_utc_ms:Date.now(),evaluated_at_utc_ms:Date.now(),age_ms:0,valid_from_utc_ms:Date.now()-1000,valid_until_utc_ms:Date.now()+3600000,original_step_count:1,summary_truncated:false,reason:null,summary:{schema_version:1,goal_profile:request.goal_profile,steps:[{objective:selected.objective,node:selected.node,completion_ms:selected.completion_ms,service_ms:selected.service_ms,expected_session_ms:1000}],expected_total_ms:1000,termination:'no_feasible_candidate'}};}
let plan:ReturnType<typeof newPlan>|null=null;
async function dispatch(command:string,input:unknown){
 calls.push(command);const args=input as {text?:string;path?:string;accept_inferred_travel?:boolean;revisions?:string[]};
 const text=()=>{if(typeof args.text!=='string')throw Error('Missing mock text argument');return args.text;};
 switch(command){
 case 'get_app_status':return {data_mode:'stored'};
 case 'import_snapshot_text':{const value=JSON.parse(text().slice(6)) as {payload:{id:string}};snapshot=value.payload.id;exists=true;return {profile_id:profile};}
 case 'select_profile':return null;
 case 'list_profiles':return exists?[{id:profile,display_name:'Draft UI engineering',fixture:false,context}]:[];
 case 'list_parties':return [];
 case 'get_app_settings':return structuredClone(settings);
 case 'update_app_settings':{const update=JSON.parse(text()) as {expected_revision:number;settings:ReturnType<typeof defaultSettings>};if(update.expected_revision!==settings.revision)throw Error('Stale settings');settings={revision:settings.revision+1,settings:update.settings};return structuredClone(settings);}
 case 'get_local_workflow':return {format_version:1,revision:0,preferences:{first_run_completed:true,startup_mode:'later'}};
 case 'get_active_content':return inspection();
 case 'get_content_dispositions':return {schema_version:1,scope:{context,fixture:false},active:null,revisions:[]};
 case 'list_local_submissions':return {rows:[],next:null,total:0};
 case 'stage_verified_content_bundle':return inspection(args.path?.endsWith('authored-v2')?'authored-ui':'world-ui');
 case 'activate_content_set':if(args.revisions?.join(',')!=='authored-ui,world-ui')throw Error('Not both UI-staged packs');accepts=args.accept_inferred_travel===true;return inspection();
 case 'get_planning_catalog':return catalog();
 case 'get_record_origins':return {revision,origins:[]};
 case 'get_catalog_page':{const q=JSON.parse(text()) as {collection:string;offset:number};if(q.collection!=='objectives'||q.offset!==200)throw Error('Wrong page');return {...pin('objectives',200),records:[objective]};}
 case 'start_planning_job':request=JSON.parse(text()) as Request;if(request.profile_id!==profile||request.snapshot_id!==snapshot||request.revision!==revision)throw Error('UI request binding mismatch');return {job_schema_version:1,job_id:'ui-job'};
 case 'get_planning_job':return {job_schema_version:1,job_id:'ui-job',kind:'solo',state:'succeeded',phase:'selection',work_units:1,cancel_requested:false,result_available:true,error:null};
 case 'take_planning_job_result':return {job_schema_version:1,job_id:'ui-job',kind:'solo',report_json:JSON.stringify(report())};
 case 'get_today_view':return {schema_version:1,settings_revision:settings.revision,selected_subject:settings.settings.selected_subject,members:exists?[{profile_id:profile,character:'draft-ui-engineering-workload',snapshot_id:snapshot,freshness:null}]:[],selection_available:exists,manual_goal:null,current_plan:plan??{revision:0,state:'empty',subject:null,fixture:null,context:null,saved_at_utc_ms:null,evaluated_at_utc_ms:null,age_ms:null,reason:null,summary:null}};
 case 'pin_current_plan':plan=newPlan();return 1;
 default:throw Error(`Unexpected mock command ${command}`);
 }
}
const adapter:CommandAdapter={invoke(command,input){return dispatch(command,input).then(value=>value as never);}};
const client=createClient(adapter);
Object.defineProperty(window,'__draftUiRegression',{value:{call:dispatch,calls,context,profile,setFailure:(value:string)=>{failure=value;},identity:{version:'1.0.0',source_commit:'c'.repeat(40),installer_sha256:'d'.repeat(64)},packDirectories:[{path:'/installed/content/release-packs/authored-v2',revision:'authored-ui'},{path:'/installed/content/release-packs/world-v2',revision:'world-ui'}],request:()=>request}});
function Harness(){const [screen,setScreen]=useState('Today'),[refresh,setRefresh]=useState(0);const changed=()=>setRefresh(n=>n+1);return <><p>DEV MOCK ONLY — not installed acceptance</p><nav aria-label="Primary">{['Content','Planner','Settings','Today'].map(name=><button key={name} onClick={()=>setScreen(name)}>{name}</button>)}</nav>{screen==='Content'?<ContentScreen client={client} onChanged={changed} refreshKey={refresh}/>:screen==='Planner'?<PlannerScreen client={client} refreshKey={refresh}/>:screen==='Settings'?<SettingsScreen client={client} refreshKey={refresh} onChanged={changed}/>:<TodayScreen client={client} refreshKey={refresh}/>}</>;}
const root=document.getElementById('root');if(!root)throw Error('Missing mock root');createRoot(root).render(<Harness/>);
