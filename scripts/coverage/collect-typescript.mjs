import fs from 'node:fs';
import path from 'node:path';
import crypto from 'node:crypto';
import {spawnSync,execFileSync} from 'node:child_process';
import {createRequire} from 'node:module';
import {fileURLToPath} from 'node:url';
import {inventory} from './inventory.mjs';
const root=path.resolve(path.dirname(fileURLToPath(import.meta.url)),'../..');
const desktop=path.join(root,'apps/desktop');
const require=createRequire(path.join(desktop,'package.json'));
const hash=value=>crypto.createHash('sha256').update(value).digest('hex');
function snapshot(){
 if(execFileSync('git',['status','--porcelain','--untracked-files=normal'],{cwd:root,encoding:'utf8'}).trim())throw new Error('Coverage requires clean committed source');
 return JSON.stringify(inventory(root),null,2)+'\n';
}
export function collectTypeScript(destination){
 if(!['linux','win32'].includes(process.platform))throw new Error('Unsupported coverage platform');
 if(fs.existsSync(destination)||path.resolve(destination).startsWith(root+path.sep))throw new Error('Use a new coverage directory outside the checkout');
 const before=snapshot(),indexed=JSON.parse(before);
 for(const name of ['vitest','@vitest/coverage-v8'])if(JSON.parse(fs.readFileSync(require.resolve(name+'/package.json'),'utf8')).version!=='5.0.3')throw new Error('Coverage/Vitest version must be 5.0.3');
 fs.mkdirSync(destination,{recursive:true});fs.writeFileSync(path.join(destination,'inventory.json'),before,{flag:'wx'});
 const v8=path.join(destination,'v8'),testReport=path.join(destination,'tests.json');
 const vitest=path.join(path.dirname(require.resolve('vitest/package.json')),'vitest.mjs');
 const result=spawnSync(process.execPath,[vitest,'run','--coverage','--coverage.reportsDirectory='+v8,'--reporter=default','--reporter=json','--outputFile='+testReport],{cwd:desktop,stdio:'inherit',timeout:300000});
 if(result.error)throw result.error;
 const coverage=path.join(v8,'coverage-final.json');
 if(!fs.existsSync(coverage)||!fs.existsSync(testReport))throw new Error('Vitest did not produce complete raw reports');
 const tests=JSON.parse(fs.readFileSync(testReport,'utf8'));
 const testsPassed=tests.numFailedTests===0&&tests.numFailedTestSuites===0&&tests.numTotalTests>0;
 if(snapshot()!==before)throw new Error('Source changed during coverage');
 const sourceHashes=Object.fromEntries(indexed.files.filter(f=>f.disposition!=='excluded').map(f=>[f.path,f.source_sha256]));
 const common={format_version:1,source_commit:indexed.source_commit,inventory_sha256:hash(before),source_root:root,source_hashes:sourceHashes,platform:process.platform,tool_version:'5.0.3',clean_source:true,incomplete_profiles:[],tests_passed:testsPassed};
 const raw=(relative,format)=>({path:relative,format,sha256:hash(fs.readFileSync(path.join(destination,relative)))});
 fs.writeFileSync(path.join(destination,'typescript-receipt.json'),JSON.stringify({...common,kind:'typescript-v8',command_exit_code:result.status,raw_reports:[raw('v8/coverage-final.json','istanbul'),raw('tests.json','test-results')]},null,2)+'\n',{flag:'wx'});
 const configDir=path.join(destination,'configuration');
 const config=spawnSync(process.execPath,[path.join(root,'scripts/coverage/configuration.mjs'),configDir],{cwd:root,stdio:'inherit',timeout:60000});
 if(config.error||config.status!==0)throw config.error??new Error('Configuration coverage failed');
 const configReceipt=JSON.parse(fs.readFileSync(path.join(configDir,'configuration-receipt.json'),'utf8'));
 if(configReceipt.source_commit!==indexed.source_commit)throw new Error('Configuration commit changed');
 for(const file of configReceipt.files)if(sourceHashes[file.path]!==file.source_sha256)throw new Error('Configuration compiled source differs');
 if(snapshot()!==before)throw new Error('Source changed during configuration coverage');
 fs.writeFileSync(path.join(destination,'configuration-receipt.json'),JSON.stringify({...common,kind:'typescript-config-v8',tests_passed:true,raw_reports:[raw('configuration/configuration-istanbul.json','istanbul'),raw('configuration/configuration-v8.json','v8')]},null,2)+'\n',{flag:'wx'});
 return {tests_passed:testsPassed,command_exit_code:result.status,tests:tests.numTotalTests};
}
if(process.argv[1]&&path.resolve(process.argv[1])===fileURLToPath(import.meta.url)){
 try{if(process.argv.length!==3)throw new Error('Usage: collect-typescript.mjs <new-directory>');const result=collectTypeScript(path.resolve(process.argv[2]));console.log(JSON.stringify(result));if(!result.tests_passed||result.command_exit_code!==0)process.exitCode=1;}
 catch(error){console.error(error.stack);process.exitCode=1;}
}
