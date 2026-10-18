// Vitest loads its own configuration before starting V8 coverage. Collect that
// real module separately with the same AST-aware V8 converter, without exclusions.
import fs from 'node:fs';
import path from 'node:path';
import {Session} from 'node:inspector';
import {registerHooks,createRequire} from 'node:module';
import {fileURLToPath,pathToFileURL} from 'node:url';
import {execFileSync} from 'node:child_process';
import crypto from 'node:crypto';
const root=path.resolve(path.dirname(fileURLToPath(import.meta.url)),'../..');
const desktop=path.join(root,'apps/desktop');
const require=createRequire(path.join(desktop,'package.json'));
const ts=require('typescript');
const coverageRequire=createRequire(require.resolve('@vitest/coverage-v8'));
const {convert}=await import(pathToFileURL(coverageRequire.resolve('ast-v8-to-istanbul')).href);
const {parseAstAsync}=await import(pathToFileURL(require.resolve('vite')).href);
export async function collectConfiguration(destination){
 if(fs.existsSync(destination))throw new Error('Configuration coverage output already exists');
 const files=new Set(['vite.config.ts','csp.ts'].map(name=>pathToFileURL(path.join(desktop,name)).href));
 const compiled=new Map();
 const hooks=registerHooks({load(url,context,next){
  if(!files.has(url))return next(url,context);
  const text=fs.readFileSync(fileURLToPath(url),'utf8');
  const result=ts.transpileModule(text,{fileName:fileURLToPath(url),compilerOptions:{target:ts.ScriptTarget.ES2024,module:ts.ModuleKind.ESNext,sourceMap:true,inlineSources:true}});
  const sourceMap=JSON.parse(result.sourceMapText);sourceMap.sources=[url];
  compiled.set(url,{code:result.outputText,sourceMap,source_sha256:crypto.createHash('sha256').update(text.replaceAll('\r\n','\n')).digest('hex')});
  return {format:'module',shortCircuit:true,source:result.outputText};
 }});
 const session=new Session();session.connect();
 const post=(method,args={})=>new Promise((resolve,reject)=>session.post(method,args,(error,result)=>error?reject(error):resolve(result)));
 let result;
 try{
  await post('Profiler.enable');await post('Profiler.startPreciseCoverage',{callCount:true,detailed:true});
  const {default:config}=await import(pathToFileURL(path.join(desktop,'vite.config.ts')).href);
  const native=JSON.parse(fs.readFileSync(path.join(desktop,'src-tauri/tauri.conf.json'),'utf8'));
  if(config.build.assetsInlineLimit!==0 || config.preview.headers['Content-Security-Policy']!==native.app.security.csp || config.preview.host!=='127.0.0.1')throw new Error('Production configuration safety contract failed');
  result=await post('Profiler.takePreciseCoverage');
 }finally{await post('Profiler.stopPreciseCoverage').catch(()=>{});session.disconnect();hooks.deregister();}
 const scripts=result.result.filter(script=>files.has(script.url));
 if(scripts.length!==files.size || compiled.size!==files.size)throw new Error('Missing configuration V8 entry');
 let istanbul={};
 for(const script of scripts){const input=compiled.get(script.url);Object.assign(istanbul,await convert({code:input.code,sourceMap:input.sourceMap,coverage:script,ast:parseAstAsync(input.code)}));}
 const receipt={format_version:1,source_commit:execFileSync('git',['rev-parse','HEAD'],{cwd:root,encoding:'utf8'}).trim(),node:process.version,collector:'node:inspector precise V8 + pinned Vitest dependency ast-v8-to-istanbul',files:[...compiled].map(([url,input])=>({path:path.relative(root,fileURLToPath(url)).split(path.sep).join('/'),source_sha256:input.source_sha256}))};
 fs.mkdirSync(destination);
 for(const [name,data] of Object.entries({'configuration-v8.json':{result:scripts,compiled:Object.fromEntries(compiled)},'configuration-istanbul.json':istanbul,'configuration-receipt.json':receipt}))fs.writeFileSync(path.join(destination,name),JSON.stringify(data,null,2)+'\n',{flag:'wx'});
 return receipt;
}
if(process.argv[1]&&path.resolve(process.argv[1])===fileURLToPath(import.meta.url)){
 try{if(process.argv.length!==3)throw new Error('Usage: configuration.mjs <new-directory>');await collectConfiguration(path.resolve(process.argv[2]));console.log('Collected configuration V8 coverage.');}
 catch(error){console.error(error.stack);process.exitCode=1;}
}
