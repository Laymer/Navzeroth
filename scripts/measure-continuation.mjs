// Explicit synthetic measurements, without timing thresholds or acceptance promotion.
import {execFileSync} from "node:child_process";
import {createHash} from "node:crypto";
import {readFileSync,writeFileSync,existsSync} from "node:fs";
import {cpus,platform,release,totalmem,arch,homedir} from "node:os";
import {resolve,dirname,join} from "node:path";
import {fileURLToPath,pathToFileURL} from "node:url";
const root=resolve(dirname(fileURLToPath(import.meta.url)),"..");
const sha=bytes=>createHash("sha256").update(bytes).digest("hex");
export function jsonLines(text,count){const rows=text.split(/\r?\n/).filter(line=>line.trimStart().startsWith("{")).map(line=>JSON.parse(line));if(rows.length!==count)throw new Error("Unexpected measurement report count");return rows;}
export function verifySeries(result,count=100){
 const values=result.samples_ms;if(!Array.isArray(values)||values.length!==count||values.some(value=>typeof value!=="number"||!Number.isFinite(value)||value<0))throw new Error("Invalid raw timing samples");
 const sorted=[...values].sort((a,b)=>a-b);const median=count%2?(sorted[(count-1)/2]):(sorted[count/2-1]+sorted[count/2])/2;
 for(const [key,expected] of Object.entries({median_ms:median,p95_ms:sorted[Math.ceil(count*.95)-1],max_ms:sorted[count-1]}))if(result[key]!==expected)throw new Error(`Timing summary mismatch: ${key}`);
 if(result.min_ms!==undefined&&result.min_ms!==sorted[0])throw new Error("Timing minimum mismatch");
 return result;
}
function command(program,args,options={}){return execFileSync(program,args,{cwd:root,encoding:"utf8",maxBuffer:16*1024*1024,...options});}
export function verifyBuildConfiguration(env=process.env, directory=root, home=homedir()) {
 if(Object.keys(env).some(key => env[key] && (/^(RUSTFLAGS|CARGO_ENCODED_RUSTFLAGS|RUSTC|RUSTC_WRAPPER|RUSTC_WORKSPACE_WRAPPER|RUSTC_BOOTSTRAP)$/.test(key) || /^CARGO_(PROFILE_|BUILD_|TARGET_.+_(RUSTFLAGS|LINKER|RUNNER)$)/.test(key)))) throw new Error("Measurements require source-default release flags without environment overrides");
 const dirs=[];let current=resolve(directory);
 while(true){dirs.push(join(current,'.cargo'));const parent=dirname(current);if(parent===current)break;current=parent;}
 dirs.push(env.CARGO_HOME ? resolve(env.CARGO_HOME) : join(home,'.cargo'));
 if(dirs.some(dir=>existsSync(join(dir,'config')) || existsSync(join(dir,'config.toml')))) throw new Error('Measurements refuse inherited Cargo configuration; use an environment without Cargo config overrides');
}
export function measure(output){
 verifyBuildConfiguration();
 const destination=resolve(output);if(existsSync(destination))throw new Error("Measurement output already exists");
 const commit=command("git",["rev-parse","HEAD"]).trim();if(!/^[a-f0-9]{40}$/.test(commit)||command("git",["status","--porcelain","--untracked-files=normal"]).trim())throw new Error("Measurements require a clean committed source worktree");
 const metadata=JSON.parse(command("cargo",["metadata","--locked","--format-version","1","--no-deps"]));
 command("cargo",["build","--locked","--release","--examples","-p","azeroth-app","-p","azeroth-core","-p","azeroth-lua-harness"],{stdio:["ignore","ignore","inherit"],timeout:900_000});
 const executable=name=>join(metadata.target_directory,"release","examples",name+(platform()==="win32"?".exe":""));
 const names=["workflow_performance","search_performance","callback_performance"];const binaries=Object.fromEntries(names.map(name=>[name,sha(readFileSync(executable(name)))]));
 const run=name=>command(executable(name),[],{timeout:900_000});
 const workflow=jsonLines(run(names[0]),1)[0];
 if(workflow.fixture!=="synthetic-continuation-workflow-v1"||workflow.build_mode!=="release"||workflow.samples!==100||workflow.warmup!==10||workflow.retained_snapshots!==110||workflow.synthetic!==true||workflow.gameplay_verified!==false||workflow.results?.length!==6)throw new Error("Workflow measurement scope mismatch");
 workflow.results.forEach(result=>verifySeries(result));
 const search=jsonLines(run(names[1]),4);const expected=["focused-solo","larger-solo","focused-joint-2","focused-joint-40"];
 search.forEach((result,index)=>{if(result.workload!==expected[index]||result.cancel_after_checkpoints!==1000||!Number.isFinite(result.cancel_ms)||result.cancel_ms<0)throw new Error("Planning measurement scope mismatch");verifySeries(result);});
 const lua=jsonLines(run(names[2]),1)[0];if(lua.fixture!=="synthetic-hud-refresh-burst-v1"||lua.samples!==100||lua.warmup!==10||lua.callbacks_per_sample!==100||lua.live_client_verified!==false)throw new Error("Lua measurement scope mismatch");verifySeries(lua);
 const inputs=["fixtures/performance/continuation-v1.json","fixtures/performance/search-v1.json","fixtures/contracts/v1/character-snapshot.json","fixtures/contracts/v1/route-plan.json","fixtures/desktop/content/demo-revision.json","crates/app/examples/workflow_performance.rs","crates/core/examples/search_performance.rs","crates/core/examples/support/mod.rs","crates/lua-harness/examples/callback_performance.rs","crates/lua-harness/tests/common/mod.rs","Cargo.lock"];
 if(command("git",["rev-parse","HEAD"]).trim()!==commit||command("git",["status","--porcelain","--untracked-files=normal"]).trim())throw new Error("Measurement source changed during execution");
 const report={format_version:1,source_commit:commit,recorded_at_utc:new Date().toISOString(),synthetic:true,acceptance_evidence:false,ordinary_windows_acceptance:"unverified",environment:{platform:platform(),kernel:release(),architecture:arch(),cpu_model:cpus()[0]?.model??null,logical_cpus:cpus().length,visible_memory_bytes:totalmem(),background_activity:"uncontrolled",node:process.version,rust:command("rustc",["--version"]).trim(),cargo:command("cargo",["--version"]).trim(),build:"cargo --locked --release; source defaults, environment flag/profile overrides refused",rustflags_present:!!process.env.RUSTFLAGS,encoded_rustflags_present:!!process.env.CARGO_ENCODED_RUSTFLAGS},inputs_sha256:Object.fromEntries(inputs.map(path=>[path,sha(readFileSync(join(root,path)))])),binaries_sha256:binaries,workflow,search,lua,limits:["Synthetic development workloads; no real content coverage, rights, calibration or gameplay claim.","Linux current-process RSS when observable, otherwise null; no ordinary-Windows memory claim.","Fresh snapshot history growth is intentional. Short RSS traces do not prove absence of long-term leaks.","Core checkpoint cancellation excludes desktop IPC, scheduling and worker termination latency.","Host CPU load, thermal policy and storage cache were not controlled."]};
 writeFileSync(destination,JSON.stringify(report,null,2)+"\n",{flag:"wx",mode:0o600});return {source_commit:commit,output_sha256:sha(readFileSync(destination)),workflows:workflow.results.length+search.length+1};
}
if(process.argv[1]&&import.meta.url===pathToFileURL(resolve(process.argv[1])).href){try{if(process.argv.length!==4||process.argv[2]!=="--output")throw new Error("Usage: node scripts/measure-continuation.mjs --output <new-report.json>");process.stdout.write(JSON.stringify(measure(process.argv[3]))+"\n");}catch(error){process.stderr.write(`Measurement refused: ${error.message}\n`);process.exitCode=1;}}
