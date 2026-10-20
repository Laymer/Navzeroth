// Native first-party instrumentation. Every process is repository-owned or a
// declared synthetic workload; this collector never selects game/private inputs.
import fs from 'node:fs';import path from 'node:path';import crypto from 'node:crypto';
import {createRequire} from 'node:module';import {fileURLToPath} from 'node:url';
import {verifyBuildConfiguration} from '../measure-continuation.mjs';
import {prepareNativeBundle,stageNativeBundle,instrumentedBundleRun} from './native-bundle.mjs';
import {loggedProcess} from './process-output.mjs';
import {assertCleanCommittedSource} from './clean-source.mjs';
import {ownedProfileOutput} from './profile-output.mjs';
import {inventory} from './inventory.mjs';import {coverageEnvironment,compilerIdentity,requireMatchingLlvm} from './rust-environment.mjs';
const root=path.resolve(path.dirname(fileURLToPath(import.meta.url)),'../..');
const hash=bytes=>crypto.createHash('sha256').update(bytes).digest('hex');
const require=createRequire(path.join(root,'apps/desktop/package.json'));
function snapshot(){
 assertCleanCommittedSource(root);
 return JSON.stringify(inventory(root),null,2)+'\n';
}
export function collectRust(destination){
 if(!['linux','win32'].includes(process.platform))throw new Error('Unsupported native platform');
 verifyBuildConfiguration(process.env,root);
 if(process.env.NAVZEROTH_BUNDLE_IDENTITY_JSON)throw new Error('Inherited native bundle identity is unsupported');
 if(Object.keys(process.env).some(key=>process.env[key]&&/^(?:CARGO_LLVM_COV|LLVM_PROFILE)|^(?:LLVM_COV|LLVM_PROFDATA)$/.test(key)))throw new Error('Inherited coverage configuration is unsupported');
 if(process.platform==='win32'&&process.env.GITHUB_ACTIONS!=='true')throw new Error('Native Windows smoke requires an ephemeral hosted runner');
 if(fs.existsSync(destination)||path.resolve(destination).startsWith(root+path.sep))throw new Error('Use a new native coverage directory outside the checkout');
 const before=snapshot(),indexed=JSON.parse(before);fs.mkdirSync(destination,{recursive:true});
 fs.writeFileSync(path.join(destination,'inventory.json'),before,{flag:'wx'});
 const target=path.join(destination,'build'),logs=path.join(destination,'logs');fs.mkdirSync(logs);
 const environment={...process.env,CARGO_TARGET_DIR:target,CARGO_LLVM_COV_TARGET_DIR:target,CARGO_INCREMENTAL:'0',CARGO_PROFILE_DEV_DEBUG:'0',CARGO_PROFILE_TEST_DEBUG:'0',
  RUSTFLAGS:process.platform==='linux'?'-C llvm-args=-runtime-counter-relocation':''};
 for(const key of ['CARGO_ENCODED_RUSTFLAGS','RUSTC_WORKSPACE_WRAPPER','RUSTC_WRAPPER','LLVM_PROFILE_FILE','CARGO_BUILD_RUSTFLAGS'])delete environment[key];
 // Ordinary cargo-run children can reuse instrumented dependencies: give their
 // profiling runtime the same owned output instead of a default in root cwd.
 Object.assign(environment,ownedProfileOutput(target,process.platform));
 const commands=[];let instrumented=environment;
 function run(name,exe,args,{env=environment,cwd=root,expected=0,timeout=2400000,stdoutOnly=false}={}){
  console.log(`Native coverage: ${name}`);
  const {process:result,text}=loggedProcess(exe,args,{log:path.join(logs,name+'.log'),cwd,env,timeout,windowsHide:true,stdoutOnly});
  commands.push({name,executable:path.basename(exe),arguments:args,expected_exit_code:expected,exit_code:result.status,signal:result.signal,error:result.error?.code??null});
  fs.writeFileSync(path.join(destination,'commands.json'),JSON.stringify(commands,null,2)+'\n');
  if(result.error||result.status!==expected)throw new Error(`Native coverage command failed: ${name}`);
  assertCleanCommittedSource(root,undefined,`after ${name}`);
  return text;
 }
 const common=['--include-build-script','--no-default-ignore-filename-regex','--ignore-filename-regex','(/vendor/|/tests/|/rustc/|/registry/|/target/)'];
 const reportFiles=[];let completedReceipt=null;
 function report(name,release=false){const file=path.join(destination,name+'.lcov');run(name,'cargo',['llvm-cov','report',...(release?['--release']:[]),'--lcov',...common,'--output-path',file]);reportFiles.push(file);}
 function cargoExample(name,pkg,example,args=[],release=false,expected=0){run(name,'cargo',['run','--locked',...(release?['--release']:[]),'-p',pkg,'--example',example,'--',...args],{env:instrumented,expected});}
 try{
  const version=run('cargo-llvm-cov-version','cargo',['llvm-cov','--version']);if(!/^cargo-llvm-cov 0\.9\.1(?:\s|$)/.test(version.trim()))throw new Error('cargo-llvm-cov must be 0.9.1');
  const rust=run('rust-version','rustc',['-vV']);
  const pinned=/^channel\s*=\s*"([^"]+)"$/m.exec(fs.readFileSync(path.join(root,'rust-toolchain.toml'),'utf8'))?.[1];
  const {llvm:llvmVersion,host}=compilerIdentity(rust,pinned);
  const sysroot=run('rust-sysroot','rustc',['--print','sysroot']).trim();
  const llvmTools=path.join(sysroot,'lib','rustlib',host,'bin','llvm-profdata'+(process.platform==='win32'?'.exe':''));
  const toolsVersion=run('llvm-tools-version',llvmTools,['--version']);
  requireMatchingLlvm(toolsVersion,llvmVersion);
  // No --no-clean here: collection starts from a new build/profile directory.
  run('role-audit-inputs',process.platform==='win32'?'python':'python3',[path.join(root,'scripts/hydrate-role-audit-inputs.py'),'--root',root],{timeout:240000});
  run('workspace-tests','cargo',['llvm-cov','test','--locked','--workspace','--all-targets','--no-report']);
  instrumented={...environment,...coverageEnvironment(run('instrumentation-environment','cargo',['llvm-cov','show-env','--sh'],{stdoutOnly:true}))};
  if(path.resolve(instrumented.CARGO_LLVM_COV_TARGET_DIR)!==target||path.resolve(instrumented.CARGO_LLVM_COV_BUILD_DIR)!==target)throw new Error('Instrumentation escaped its owned build directory');
  cargoExample('schema-export','azeroth-core','export_content_schema');
  cargoExample('world-data-schema-export','azeroth-core','export_world_data_schema');
  // One valid and one invalid input each, plus the bookkeeping the shard validator skips.
  cargoExample('world-data-validation','azeroth-core','validate_world_data',['fixtures/world-data/shards','fixtures/world-data/shards/world/npcs/vanilla_1_12/1.12.1/part-0000.json'],false,1);
  cargoExample('knowledge-validation','azeroth-core','validate_knowledge',['fixtures/world-data/facts'],false,1);
  cargoExample('workflow-verification','azeroth-app','workflow_performance',['--verify']);
  cargoExample('workflow-bad-option','azeroth-app','workflow_performance',['--unsupported'],false,2);
  cargoExample('workflow-debug-refusal','azeroth-app','workflow_performance',[],false,2);
  cargoExample('search-debug-refusal','azeroth-core','search_performance',[],false,2);
  cargoExample('lua-debug-refusal','azeroth-lua-harness','callback_performance',[],false,2);
  if(process.platform==='win32'){
   const vite=path.join(path.dirname(require.resolve('vite/package.json')),'bin','vite.js');
   run('frontend-build',process.execPath,[vite,'build'],{cwd:path.join(root,'apps/desktop')});
   const bundleRun=instrumentedBundleRun(run,instrumented);
   const bundle=prepareNativeBundle(root,destination,bundleRun);
   const nativeBuildEnv={...instrumented,...(bundle?{NAVZEROTH_BUNDLE_IDENTITY_JSON:bundle.raw}:{})};
   const output=run('native-build','cargo',['build','--locked','-p','azeroth-desktop','--features','tauri/custom-protocol','--message-format=json'],{env:nativeBuildEnv});
   const artifacts=output.split(/\r?\n/).filter(line=>line.startsWith('{')).map(line=>JSON.parse(line));
   const binaries=artifacts.filter(item=>item.reason==='compiler-artifact'&&item.target.name==='azeroth-desktop'&&item.executable).map(item=>item.executable);
   if(binaries.length!==1)throw new Error('Missing unique native executable');
   stageNativeBundle(bundle,binaries[0],bundleRun);
   const state=path.join(destination,'synthetic-desktop-state'),nativeEnv={...instrumented,APPDATA:path.join(state,'roaming'),LOCALAPPDATA:path.join(state,'local'),...(bundle?{NAVZEROTH_NATIVE_STARTUP_MS:'1200000'}:{})};
   for(const dir of [nativeEnv.APPDATA,nativeEnv.LOCALAPPDATA])fs.mkdirSync(dir,{recursive:true});
   run('native-extraction-worker-refusal',binaries[0],['--navzeroth-local-data-worker-v1'],{env:nativeEnv,expected:2});
   run('native-media-worker-refusal',binaries[0],['--navzeroth-media-worker-v1'],{env:nativeEnv,expected:1});
   run('native-smoke',process.execPath,[path.join(root,'scripts/windows/Native-Smoke.mjs'),'--mode','smoke','--exe',binaries[0],'--shutdown','graceful','--port',String(19000+process.pid%10000),'--out',path.join(destination,'native-smoke.json')],{env:nativeEnv,timeout:bundle?1800000:300000});
  }
  report('debug');
  cargoExample('search-release','azeroth-core','search_performance',[],true);
  cargoExample('workflow-release','azeroth-app','workflow_performance',[],true);
  cargoExample('lua-release','azeroth-lua-harness','callback_performance',[],true);
  report('release',true);
  if(snapshot()!==before)throw new Error('Source changed during native coverage');
  const combined=Buffer.concat(reportFiles.map(file=>Buffer.concat([fs.readFileSync(file),Buffer.from('\n')])));fs.writeFileSync(path.join(destination,'native.lcov'),combined,{flag:'wx'});
  const receipt={format_version:1,kind:'rust-llvm',source_commit:indexed.source_commit,inventory_sha256:hash(before),source_root:root,
   source_hashes:Object.fromEntries(indexed.files.filter(file=>file.disposition!=='excluded').map(file=>[file.path,file.source_sha256])),platform:process.platform,tool_version:'0.9.1',rust_llvm_version:llvmVersion,llvm_tools_version:llvmVersion,
   clean_source:true,tests_passed:true,incomplete_profiles:[],raw_reports:[{path:'native.lcov',format:'lcov',sha256:hash(combined)}],
   profile_mode:process.platform==='linux'?'continuous-counter-relocation':'ordinary',commands};
  completedReceipt=receipt;
  return receipt;
 }finally{
  // Retain every raw profile, including empty/corrupt files on a failed run.
  const raw=path.join(destination,'raw-profiles');fs.mkdirSync(raw);
  const profiles=fs.existsSync(target)?fs.readdirSync(target).filter(name=>name.endsWith('.profraw')).sort():[];
  const manifest=[];
  for(const name of profiles){const source=path.join(target,name),stat=fs.lstatSync(source);if(!stat.isFile()||stat.isSymbolicLink())throw new Error('Invalid raw profile path');fs.copyFileSync(source,path.join(raw,name),fs.constants.COPYFILE_EXCL);manifest.push({name,bytes:stat.size,sha256:hash(fs.readFileSync(source))});}
  fs.writeFileSync(path.join(destination,'raw-profile-manifest.json'),JSON.stringify({format_version:1,files:manifest},null,2)+'\n',{flag:'wx'});
  // Publish success only after every raw profile has been retained successfully.
  if(completedReceipt)fs.writeFileSync(path.join(destination,'rust-receipt.json'),JSON.stringify(completedReceipt,null,2)+'\n',{flag:'wx'});
 }
}
if(process.argv[1]&&path.resolve(process.argv[1])===fileURLToPath(import.meta.url)){
 try{if(process.argv.length!==3)throw new Error('Usage: collect-rust.mjs <new-directory>');collectRust(path.resolve(process.argv[2]));console.log('Collected native Rust coverage.');}
 catch(error){console.error(error.stack);process.exitCode=1;}
}
