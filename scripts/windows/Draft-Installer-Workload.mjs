// Exact downloaded installed native draft only. No local invocation or live game.
import {spawn,spawnSync} from 'node:child_process';
import {writeFileSync,mkdirSync,lstatSync,existsSync,realpathSync,openSync,closeSync,fstatSync,readSync} from 'node:fs';
import {dirname,join,resolve,basename,isAbsolute,relative} from 'node:path';
import {createHash} from 'node:crypto';
import {fileURLToPath} from 'node:url';
import {createRequire} from 'node:module';
import {parseArgs,invokeNative,waitForDatabaseReady,policyAddArgs,gracefulCloseArgs} from './native-smoke-lib.mjs';
import {createWorkloadProgress} from './stable-workload-progress.mjs';
import {exerciseDraftUi,DRAFT_WORKLOAD} from './draft-installer-ui.mjs';
const options=parseArgs(process.argv.slice(2),['mode','exe','manifest','state','out','version','port','source-commit','installer-sha256']);
if(process.platform!=='win32'||process.env.GITHUB_ACTIONS!=='true'||process.env.RUNNER_ENVIRONMENT!=='github-hosted'||process.env.OS!=='Windows_NT')throw Error('Draft UI smoke requires a disposable hosted Windows runner');
const {mode,exe,manifest,state,out,version}=options,source=options['source-commit'],installer=options['installer-sha256'];
if(!['fresh','restart'].includes(mode)||version!=='1.0.0'||![exe,manifest,state,out].every(x=>typeof x==='string'&&isAbsolute(x)&&x.length<=4096)||!/^[a-f0-9]{40}$/.test(source)||!/^[a-f0-9]{64}$/.test(installer))throw Error('Invalid draft UI parameters or source/installer identity');
const MAX=20*60*1000,started=Date.now(),port=Number(options.port??9335);
if(!Number.isSafeInteger(port)||port<1024||port>65535)throw Error('Invalid draft native debugging port');
function remaining(){const left=MAX-(Date.now()-started);if(left<=0)throw Error('Draft native UI workload exceeded20minute bound');return left;}
async function bounded(action){let timer;try{return await Promise.race([action(),new Promise((_,reject)=>{timer=setTimeout(()=>reject(Error('Draft operation exceeded remaining20minute bound')),remaining());})]);}finally{clearTimeout(timer);}}
function bytes(path,limit){
 const before=lstatSync(path);if(!before.isFile()||before.isSymbolicLink()||before.size>limit)throw Error('Draft bounded regular resource refused');
 const fd=openSync(path,'r');try{
  const opened=fstatSync(fd);if(!opened.isFile()||opened.dev!==before.dev||opened.ino!==before.ino||opened.size>limit)throw Error('Draft resource changed before open');
  const parts=[];let size=0;
  while(size<=limit){const chunk=Buffer.alloc(Math.min(65536,limit+1-size));const count=readSync(fd,chunk,0,chunk.length,null);if(!count)break;parts.push(chunk.subarray(0,count));size+=count;}
  const after=fstatSync(fd),current=lstatSync(path);
  if(size>limit||size!==opened.size||after.size!==opened.size||after.mtimeMs!==opened.mtimeMs||current.isSymbolicLink()||current.dev!==opened.dev||current.ino!==opened.ino)throw Error('Draft resource changed or exceeded bound');
  return Buffer.concat(parts,size);
 }finally{closeSync(fd);}
}
function json(path,limit=4*1024*1024){return JSON.parse(bytes(path,limit).toString('utf8'));}
function samePath(a,b){return realpathSync(a).toLowerCase()===realpathSync(b).toLowerCase();}
const installed=dirname(realpathSync(exe)),bundleRoot=dirname(realpathSync(manifest));
if(!samePath(bundleRoot,join(installed,'bundled-content'))||basename(manifest)!=='manifest.json')throw Error('Draft manifest must be the exact installed bundled-content manifest');
const bundle=json(manifest);
if(bundle.format!=='navzeroth-installed-content'||bundle.schema_version!==1||bundle.app_version!==version||!Array.isArray(bundle.packs)||bundle.packs.length!==2||!Array.isArray(bundle.files))throw Error('Installed draft bundle contract differs');
const context=bundle.packs[0]?.key?.scope?.context;
if(context?.namespace!=='reference'||context.build?.edition!==bundle.target.edition||context.build.build!==`${bundle.target.version}.${bundle.target.build}`||context.build.interface!==String(bundle.target.interface))throw Error('Installed draft target scope differs');
const packDirectories=bundle.packs.map(pack=>{
 if(!['content/release-packs/authored-v2','content/release-packs/world-v2'].includes(pack.directory)||pack.key?.scope?.fixture!==false||JSON.stringify(pack.key.scope.context)!==JSON.stringify(context)||!pack.key.revision||pack.manifest?.path!==pack.directory+'/manifest.json')throw Error('Draft pack is not the admitted installed world/authored content');
 const path=join(bundleRoot,...pack.directory.split('/')),info=lstatSync(path);
 if(!info.isDirectory()||info.isSymbolicLink()||relative(bundleRoot,realpathSync(path)).startsWith('..'))throw Error('Draft installed pack escapes its verified resource root');
 const pinned=bundle.files.filter(f=>f.path===pack.manifest.path);if(pinned.length!==1||pinned[0].bytes!==pack.manifest.bytes||pinned[0].sha256!==pack.manifest.sha256)throw Error('Draft pack manifest pin differs');
 const data=bytes(join(path,'manifest.json'),4*1024*1024);if(data.length!==pack.manifest.bytes||createHash('sha256').update(data).digest('hex')!==pack.manifest.sha256)throw Error('Draft installed pack manifest bytes differ');
 return {path,revision:pack.key.revision};
});
if(new Set(bundle.packs.map(p=>p.directory)).size!==2)throw Error('Draft installed pack identities repeat');
if(mode==='fresh'&&existsSync(state))throw Error('Fresh draft UI state must not overwrite prior evidence');
if(existsSync(out))throw Error('Draft output must not overwrite existing evidence');
const identity={version,source_commit:source,installer_sha256:installer};
const evidence={schema_version:1,phase:'draft',mode,...identity,workload:DRAFT_WORKLOAD,human_performed:false,ui_interaction_verified:false,completed:false,passed:false,started_at:new Date().toISOString(),bundle_target:bundle.target,manifest_sha256:createHash('sha256').update(bytes(manifest,4*1024*1024)).digest('hex'),screenshots:[]};
const progress=createWorkloadProgress(out+'.progress.json',{mode,version});
const require=createRequire(join(resolve(dirname(fileURLToPath(import.meta.url)),'../..'),'apps/desktop/package.json')),{chromium}=require('@playwright/test');
let child,browser,page,policy=false,exited=false,exitResult,result;
const sleep=ms=>new Promise(r=>setTimeout(r,ms));
async function call(command,args){return progress.track(command,()=>bounded(()=>invokeNative(page,command,args)));}
function save(path,value,limit=256*1024){const text=JSON.stringify(value,null,2)+'\n';if(Buffer.byteLength(text)>limit)throw Error('Draft report exceeds bounded capacity');mkdirSync(dirname(resolve(path)),{recursive:true});writeFileSync(path,text,{flag:'w'});}
async function capture(name){
 if(!/^(fresh|restart)-[a-z-]{1,48}$/.test(name)||evidence.screenshots.length>=8)throw Error('Invalid draft screenshot identity');
 const directory=out+'.screenshots';mkdirSync(directory,{recursive:true});const path=join(directory,name+'.png');if(existsSync(path))throw Error('Draft screenshot already exists');
 await bounded(()=>page.screenshot({path,fullPage:false,timeout:remaining()}));
 const image=bytes(path,8*1024*1024);if(image.length<8||!image.subarray(0,8).equals(Buffer.from([137,80,78,71,13,10,26,10])))throw Error('Draft UI screenshot is not a PNG');evidence.screenshots.push({path:relative(dirname(out),path).split('\\').join('/'),bytes:image.length,sha256:createHash('sha256').update(image).digest('hex')});
}
try{
 progress.phase('launch');
 // Refuse a pre-existing endpoint rather than attach to another process.
 let occupied=false;try{const response=await fetch(`http://127.0.0.1:${port}/json/version`,{signal:AbortSignal.timeout(1500)});occupied=true;}catch{}
 if(occupied)throw Error('Draft debugging endpoint already exists');
 const key='HKLM\\SOFTWARE\\Policies\\Microsoft\\Edge\\WebView2\\AdditionalBrowserArguments',name=basename(exe);
 const queried=spawnSync('reg',['query',key,'/v',name],{encoding:'utf8',timeout:15000});if(queried.status===0)throw Error('Existing per-executable debugging policy must be preserved');
 const added=spawnSync('reg',policyAddArgs(name,port),{encoding:'utf8',timeout:15000});if(added.error||added.status!==0)throw Error('Cannot configure temporary draft debugging policy');policy=true;
 child=spawn(exe,[],{stdio:'ignore',windowsHide:false});child.on('exit',(code,signal)=>{exited=true;exitResult={code,signal};});child.on('error',()=>{exited=true;});evidence.pid=child.pid;
 let endpoint;progress.phase('endpoint');
 while(remaining()>0){if(exited)throw Error('Draft installed app exited before startup');try{const response=await fetch(`http://127.0.0.1:${port}/json/version`,{signal:AbortSignal.timeout(3000)});const value=await response.json();if(typeof value.webSocketDebuggerUrl==='string'&&value.webSocketDebuggerUrl.length<2048){const url=new URL(value.webSocketDebuggerUrl);if(url.protocol!=='ws:'||!['127.0.0.1','localhost','[::1]'].includes(url.hostname)||url.port!==String(port))throw Error('Draft CDP endpoint is not the issued local port');endpoint=url.href;break;}}catch{}await sleep(250);}
 browser=await chromium.connectOverCDP(endpoint,{timeout:Math.min(30000,remaining())});
 while(!page){remaining();page=browser.contexts().flatMap(c=>c.pages()).find(p=>/^https?:\/\/tauri\.localhost(?:[:/]|$)|^tauri:\/\/localhost(?:[:/]|$)/.test(p.url())&&!p.url().includes('#reference'));if(!page)await sleep(250);}
 progress.phase('startup');await waitForDatabaseReady(()=>call('get_database_status'),remaining(),200);
 const status=await call('get_database_status'),reference=await call('get_reference_view'),app=await call('get_app_status');
 if(status.state!=='ready'||status.error!==null||reference.app_version!==version||app.data_mode!=='stored')throw Error('Exact installed native app did not reach verified stored mode');
 evidence.database_state=status.state;evidence.startup_measurements=status.startup_measurements;
 progress.phase('ui_exercise');result=await bounded(()=>exerciseDraftUi(page,call,{mode,identity,context,packDirectories,saved:mode==='restart'?json(state,65536):undefined,capture,timeout:remaining,onPhase:phase=>progress.phase(phase)}));Object.assign(evidence,result.evidence);
 progress.phase('shutdown');const closed=spawnSync('powershell.exe',gracefulCloseArgs(child.pid),{encoding:'utf8',timeout:15000,windowsHide:true});if(closed.error||closed.status!==0)throw Error('Cannot request draft native normal shutdown');
 const closeDeadline=Date.now()+Math.min(15000,remaining());while(!exited&&Date.now()<closeDeadline)await sleep(100);
 if(!exited||exitResult?.code!==0||exitResult?.signal!==null)throw Error('Draft native application did not close normally');
 evidence.passed=true;
}catch(error){evidence.error=String(error.message).slice(0,2048);process.exitCode=1;if(page){try{await capture(mode+'-failure');}catch{}}}
finally{
 try{progress.phase('cleanup');}catch{evidence.passed=false;evidence.error??='Draft progress cleanup failed';process.exitCode=1;}
 if(browser){let timer;try{await Promise.race([browser.close(),new Promise((_,reject)=>{timer=setTimeout(()=>reject(Error('Draft browser cleanup timed out')),10000);})]);}catch{evidence.passed=false;evidence.error??='Draft browser cleanup failed';process.exitCode=1;}finally{clearTimeout(timer);}}
 if(child&&!exited)spawnSync('taskkill',['/PID',String(child.pid),'/T','/F'],{timeout:15000,stdio:'ignore'});
 if(policy){const deleted=spawnSync('reg',['delete','HKLM\\SOFTWARE\\Policies\\Microsoft\\Edge\\WebView2\\AdditionalBrowserArguments','/v',basename(exe),'/f'],{timeout:15000,stdio:'ignore'});if(deleted.error||deleted.status!==0){evidence.passed=false;evidence.error='Draft per-executable policy cleanup failed';process.exitCode=1;}}
 try{progress.phase('finished');}catch{evidence.passed=false;evidence.error??='Draft final progress checkpoint failed';process.exitCode=1;}
 evidence.completed=evidence.passed===true;evidence.ui_interaction_verified=evidence.passed===true&&evidence.ui_interaction_verified===true;
 if(evidence.passed){try{save(state,result.state,65536);}catch{evidence.passed=false;evidence.completed=false;evidence.error='Draft state could not be saved';process.exitCode=1;}}
 evidence.elapsed_ms=Date.now()-started;evidence.finished_at=new Date().toISOString();save(out,evidence);
}
