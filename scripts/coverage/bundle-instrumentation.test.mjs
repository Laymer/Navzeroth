import {test} from 'node:test';
import vm from 'node:vm';
import assert from 'node:assert/strict';
import fs from 'node:fs';import os from 'node:os';import path from 'node:path';
import {execFileSync} from 'node:child_process';
import {instrumentedBundleRun} from './native-bundle.mjs';
import {coverageEnvironment} from './rust-environment.mjs';
import {ownedProfileOutput} from './profile-output.mjs';
import {assertCleanCommittedSource} from './clean-source.mjs';
const cargo=process.env.CARGO||'cargo';
test('bundle instrumentation preserves command controls and both collector call sites',()=>{
 const environment={marker:'instrumented'};let received;
 const run=instrumentedBundleRun((...args)=>{received=args;return 'result';},environment);
 assert.equal(run('command','cargo',['run'],{timeout:120,expected:2,stdoutOnly:true,env:{wrong:true}}),'result');
 assert.deepEqual(received,['command','cargo',['run'],{timeout:120,expected:2,stdoutOnly:true,env:environment}]);
 const source=fs.readFileSync(new URL('./collect-rust.mjs',import.meta.url),'utf8');
 assert.match(source,/const bundleRun=instrumentedBundleRun\(run,instrumented\)/);
 assert.match(source,/prepareNativeBundle\(root,destination,bundleRun\)/);
 assert.match(source,/stageNativeBundle\(bundle,binaries\[0\],bundleRun\)/);
});
test('actual pinned LLVM binary report loses ordinary replaced main and retains instrumented bundle main',()=>{
 const temporary=fs.mkdtempSync(path.join(os.tmpdir(),'bundle instrumentation-'));
 try {
  const root=path.join(temporary,'source'),target=path.join(temporary,'owned target');fs.mkdirSync(root);fs.mkdirSync(target);fs.mkdirSync(path.join(root,'src'));fs.mkdirSync(path.join(root,'tests'));
  fs.copyFileSync(new URL('../../rust-toolchain.toml',import.meta.url),path.join(root,'rust-toolchain.toml'));
  fs.writeFileSync(path.join(root,'Cargo.toml'),'[package]\nname="bundle-probe"\nversion="0.1.0"\nedition="2021"\n');
  fs.writeFileSync(path.join(root,'src/main.rs'),'fn main() {\n let args:Vec<_>=std::env::args().collect();\n if args.len()>1 { println!("bundle command {}",args[1]); }\n}\n');
  fs.writeFileSync(path.join(root,'tests/entry.rs'),'#[test] fn actual_binary(){assert!(std::process::Command::new(env!("CARGO_BIN_EXE_bundle-probe")).arg("test").status().unwrap().success());}\n');
  const git=(...args)=>execFileSync('git',args,{cwd:root,stdio:'pipe'});git('init','-q');git('config','user.name','Synthetic test');git('config','user.email','synthetic@example.invalid');
  const base={...process.env,CARGO_TARGET_DIR:target,CARGO_LLVM_COV_TARGET_DIR:target,CARGO_INCREMENTAL:'0',RUSTFLAGS:process.platform==='linux'?'-C llvm-args=-runtime-counter-relocation':'',...ownedProfileOutput(target,process.platform)};
  const run=(name,exe,args,options={})=>execFileSync(exe,args,{cwd:root,env:options.env??base,encoding:'utf8',timeout:120000,stdio:['ignore','pipe','pipe']});
  assert.match(run('version',cargo,['llvm-cov','--version']),/^cargo-llvm-cov 0\.9\.1\s/);
  run('test',cargo,['llvm-cov','test','--all-targets','--no-report']);
  git('add','.');git('commit','-qm','synthetic fixture');
  const instrumented={...base,...coverageEnvironment(run('environment',cargo,['llvm-cov','show-env','--sh']))};
  const sysroot=run('sysroot',process.env.RUSTC||'rustc',['--print','sysroot']).trim();
  const host=/^host: (.+)$/m.exec(run('host',process.env.RUSTC||'rustc',['-vV']))[1];
  const llvmCov=path.join(sysroot,'lib/rustlib',host,'bin','llvm-cov'+(process.platform==='win32'?'.exe':''));
  const binary=path.join(target,'debug','bundle-probe'+(process.platform==='win32'?'.exe':''));
  const covered=label=>{
   const output=path.join(temporary,label+'.lcov');if(label!=='ordinary-replacement')run(label,cargo,['llvm-cov','report','--lcov','--output-path',output]);
   const profdata=fs.readdirSync(target).filter(name=>name.endsWith('.profdata'));assert.equal(profdata.length,1);
   const binaryReport=run(label+' actual binary',llvmCov,['export',binary,'--instr-profile='+path.join(target,profdata[0]),'--format=lcov']);
   const records=binaryReport.split('end_of_record').filter(record=>record.includes('/src/main.rs\n')||record.includes('\\src\\main.rs\n'));
   return records.flatMap(record=>record.split('\n').filter(line=>line.startsWith('DA:'))).filter(line=>Number(line.split(',')[1])>0).length;
  };
  assert(covered('before')>0);
  // Windows actual artifact logs show this CLI was rebuilt. Force that same
  // rebuild rather than relying on platform-specific Cargo cache decisions.
  run('force actual rebuild',cargo,['clean','-p','bundle-probe']);
  run('ordinary assembly',cargo,['run','--','assemble']);
  assert.throws(()=>covered('ordinary-replacement'),/no coverage data found/);
  run('force instrumented rebuild',cargo,['clean','-p','bundle-probe']);
  const bundleRun=instrumentedBundleRun(run,instrumented);
  bundleRun('assembly',cargo,['run','--','assemble']);bundleRun('verification',cargo,['run','--','verify']);
  assert(covered('instrumented-bundle')>0);
  assertCleanCommittedSource(root);
  const profiles=fs.readdirSync(target).filter(name=>name.endsWith('.profraw'));assert(profiles.length>0);assert(profiles.some(name=>fs.statSync(path.join(target,name)).size>0));
 } finally {fs.rmSync(temporary,{recursive:true,force:true});}
});

test('native collector exercises both release schema branches and real panic refusals before reporting',()=>{
 const source=fs.readFileSync(new URL('./collect-rust.mjs',import.meta.url),'utf8');
 const calls=source.split('\n').filter(line=>line.includes("'export_release_schema'"));
 const actual=[];
 vm.runInNewContext(calls.join('\n'),{cargoExample:(...args)=>actual.push(JSON.parse(JSON.stringify(args)))});
 assert.deepEqual(actual,[
  ['release-inventory-schema-export','azeroth-core','export_release_schema',['inventory']],
  ['release-evidence-schema-export','azeroth-core','export_release_schema',['evidence']],
  ['release-schema-missing-selector','azeroth-core','export_release_schema',[],false,101],
  ['release-schema-invalid-selector','azeroth-core','export_release_schema',['unsupported'],false,101],
 ]);
 for(const line of calls){
  assert(source.indexOf(line)>source.indexOf("run('instrumentation-environment'"));
  assert(source.indexOf(line)<source.indexOf("report('debug')"));
 }
 assert.match(source,/function cargoExample\([^\n]+env:instrumented,expected/);
});
