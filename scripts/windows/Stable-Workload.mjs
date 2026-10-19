// Actual installed native application, disposable hosted runner only. This is
// engineering acceptance over admitted resources, never live-client evidence.
import {spawn,spawnSync} from 'node:child_process';
import {readFileSync,writeFileSync,mkdirSync,statSync} from 'node:fs';
import {dirname,join,resolve} from 'node:path';
import {fileURLToPath} from 'node:url';
import {createRequire} from 'node:module';
import {parseArgs,invokeNative,waitForDatabaseReady,policyAddArgs,gracefulCloseArgs} from './native-smoke-lib.mjs';
import {WORKLOAD,exerciseInstalled} from './stable-workload-lib.mjs';
const options=parseArgs(process.argv.slice(2),['mode','exe','manifest','state','out','version','port','backup']);
if(process.platform!=='win32'||process.env.GITHUB_ACTIONS!=='true'||process.env.RUNNER_ENVIRONMENT!=='github-hosted')throw new Error('Stable workload requires a disposable hosted Windows runner');
const {mode,exe,manifest,state,out,version}=options;
if(!['baseline','fresh','restart','recover','offline'].includes(mode)||![exe,manifest,state,out,version].every(Boolean))throw new Error('Missing stable workload parameters');
const MAX=20*60*1000,port=Number(options.port??9333),started=Date.now();
if(!Number.isInteger(port)||port<1024||port>65535)throw new Error('Invalid debugging port');
function json(path,limit=4*1024*1024){const metadata=statSync(path);if(!metadata.isFile()||metadata.size>limit)throw new Error('Bounded workload input refused');return JSON.parse(readFileSync(path,'utf8'));}
const bundle=json(manifest),context=bundle.packs[0].key.scope.context;
const require=createRequire(join(resolve(dirname(fileURLToPath(import.meta.url)),'../..'),'apps/desktop/package.json'));
const {chromium}=require('@playwright/test');
const evidence={schema_version:1,mode,workload:WORKLOAD,started_at:new Date().toISOString(),version,bundle_target:bundle.target,passed:false};
let child,browser,page,policy=false,exited=false,exitResult;
const sleep=ms=>new Promise(r=>setTimeout(r,ms));
async function bounded(action){let timer;try{return await Promise.race([action(),new Promise((_,reject)=>{timer=setTimeout(()=>reject(new Error('Native operation exceeded 20 minute bound')),MAX);})]);}finally{clearTimeout(timer);}}
async function call(command,args){return bounded(()=>invokeNative(page,command,args));}
function save(path,value){mkdirSync(dirname(resolve(path)),{recursive:true});writeFileSync(path,JSON.stringify(value,null,2)+'\n',{flag:'w'});}
try{
 const queried=spawnSync('reg',['query','HKLM\\SOFTWARE\\Policies\\Microsoft\\Edge\\WebView2\\AdditionalBrowserArguments','/v',exe.split(/[\\/]/).pop()],{encoding:'utf8',timeout:15000});
 if(queried.status===0)throw new Error('Existing debugging policy value must be preserved');
 const added=spawnSync('reg',policyAddArgs(exe.split(/[\\/]/).pop(),port),{encoding:'utf8',timeout:15000});
 if(added.error||added.status!==0)throw new Error('Cannot configure temporary native debugging policy');policy=true;
 child=spawn(exe,[],{stdio:'ignore',windowsHide:false});child.on('exit',(code,signal)=>{exited=true;exitResult={code,signal};});child.on('error',()=>{exited=true;});evidence.pid=child.pid;
 let endpoint;
 while(Date.now()-started<MAX){
  if(exited)throw new Error('Installed application exited before becoming ready');
  try{const response=await fetch(`http://127.0.0.1:${port}/json/version`,{signal:AbortSignal.timeout(3000)});endpoint=(await response.json()).webSocketDebuggerUrl;if(endpoint)break;}catch{}
  await sleep(250);
 }
 if(!endpoint)throw new Error('Installed WebView endpoint did not become ready');
 browser=await chromium.connectOverCDP(endpoint,{timeout:30000});
 while(Date.now()-started<MAX&&!page){page=browser.contexts().flatMap(c=>c.pages()).find(p=>p.url()&&p.url()!=='about:blank'&&!p.url().includes('#reference'));if(!page)await sleep(250);}
 if(!page)throw new Error('Installed application main window unavailable');
 await waitForDatabaseReady(()=>call('get_database_status'),MAX,200);
 const status=await call('get_database_status');
 evidence.database_state=status.state;
 const reference=await call('get_reference_view');if(reference.app_version!==version)throw new Error('Installed native app version differs from expected installer');
 if(mode!=='baseline'){
  if(!status.startup_measurements||!['load_ms','activation_ms'].every(k=>Number.isSafeInteger(status.startup_measurements[k])&&status.startup_measurements[k]>=0))throw new Error('Successful measured verified bundle startup is required');
  evidence.startup_measurements=status.startup_measurements;
 }
 const result=await exerciseInstalled(call,{context,version,mode,saved:['fresh','baseline'].includes(mode)?undefined:json(state,65536),backup:options.backup});
 Object.assign(evidence,result.evidence);save(state,result.state);
 const closed=spawnSync('powershell.exe',gracefulCloseArgs(child.pid),{encoding:'utf8',timeout:15000,windowsHide:true});
 if(closed.error||closed.status!==0)throw new Error('Cannot request normal native shutdown');
 const deadline=Date.now()+15000;while(!exited&&Date.now()<deadline)await sleep(100);if(!exited||exitResult?.code!==0||exitResult?.signal!==null)throw new Error('Native application did not close normally');
 evidence.passed=true;
}catch(error){evidence.error=String(error.message).slice(0,2048);process.exitCode=1;}
finally{
 if(browser)await browser.close().catch(()=>{});
 if(child&&!exited)spawnSync('taskkill',['/PID',String(child.pid),'/T','/F'],{timeout:15000,stdio:'ignore'});
 if(policy){const deleted=spawnSync('reg',['delete','HKLM\\SOFTWARE\\Policies\\Microsoft\\Edge\\WebView2\\AdditionalBrowserArguments','/v',exe.split(/[\\/]/).pop(),'/f'],{timeout:15000,stdio:'ignore'});if(deleted.error||deleted.status!==0){evidence.passed=false;evidence.error='Temporary debugging policy cleanup failed';process.exitCode=1;}}
 evidence.elapsed_ms=Date.now()-started;evidence.finished_at=new Date().toISOString();save(out,evidence);
}
