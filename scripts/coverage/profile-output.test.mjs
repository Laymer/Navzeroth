import {test} from 'node:test';
import assert from 'node:assert/strict';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import {execFileSync} from 'node:child_process';
import {ownedProfileOutput} from './profile-output.mjs';
import {coverageEnvironment} from './rust-environment.mjs';
import {assertCleanCommittedSource} from './clean-source.mjs';
const rustc=process.env.RUSTC||'rustc';
const cargo=process.env.CARGO||'cargo';
function temporary(action){const root=fs.mkdtempSync(path.join(os.tmpdir(),'owned native profile-'));try{return action(root);}finally{fs.rmSync(root,{recursive:true,force:true});}}
function repository(root){
 fs.mkdirSync(root);const git=(...args)=>execFileSync('git',args,{cwd:root,stdio:'pipe'});
 git('init','-q');git('config','user.name','Synthetic test');git('config','user.email','synthetic@example.invalid');fs.writeFileSync(path.join(root,'tracked.txt'),'synthetic\n');git('add','.');git('commit','-qm','synthetic fixture');
}
test('actual instrumented Rust default dirties Git; owned output preserves nonempty profile and clean Git',()=>temporary(root=>{
 const ordinary=path.join(root,'ordinary checkout'),controlled=path.join(root,'controlled checkout'),target=path.join(root,'owned target');
 repository(ordinary);repository(controlled);fs.mkdirSync(target);
 const source=path.join(root,'synthetic.rs'),binary=path.join(root,'synthetic'+(process.platform==='win32'?'.exe':''));
 fs.writeFileSync(source,'#[inline(never)] fn measured(n:u32)->u32 { if n>0 { n+1 } else { 0 } }\nfn main(){ assert_eq!(measured(1),2); }\n');
 const flags=['--edition=2021','-C','instrument-coverage',...(process.platform==='linux'?['-C','llvm-args=-runtime-counter-relocation']:[])];
 execFileSync(rustc,[...flags,source,'-o',binary],{timeout:120000,stdio:'pipe'});
 const base={...process.env};delete base.LLVM_PROFILE_FILE;delete base.LLVM_PROFILE_FILE_NAME;
 execFileSync(binary,[],{cwd:ordinary,env:base,timeout:30000,stdio:'pipe'});
 const defaults=fs.readdirSync(ordinary).filter(name=>name.endsWith('.profraw'));
 assert(defaults.length>0);assert(defaults.every(name=>fs.statSync(path.join(ordinary,name)).size>0));
 assert.throws(()=>assertCleanCommittedSource(ordinary),/profraw/);
 execFileSync(binary,[],{cwd:controlled,env:{...base,...ownedProfileOutput(target,process.platform)},timeout:30000,stdio:'pipe'});
 const profiles=fs.readdirSync(target).filter(name=>name.endsWith('.profraw'));
 assert(profiles.length>0);assert(profiles.every(name=>name.startsWith('nav-')&&fs.statSync(path.join(target,name)).size>0));
 assert.doesNotThrow(()=>assertCleanCommittedSource(controlled));
 // Both the baseline and corrected raw files still exist; no ignore/delete is
 // used to make either source check pass. The collector's finally can retain
 // all corrected files using its existing target-directory profile selection.
 assert(defaults.every(name=>fs.existsSync(path.join(ordinary,name))));
}));
test('pinned cargo llvm-cov show-env accepts controlled base profile and preserves its owned target',()=>temporary(root=>{
 const target=path.join(root,'owned target');fs.mkdirSync(target);
 const env={...process.env,CARGO_TARGET_DIR:target,CARGO_LLVM_COV_TARGET_DIR:target,CARGO_INCREMENTAL:'0',RUSTFLAGS:process.platform==='linux'?'-C llvm-args=-runtime-counter-relocation':'',...ownedProfileOutput(target,process.platform)};
 const options={cwd:path.resolve(import.meta.dirname,'../..'),env,encoding:'utf8',timeout:120000,stdio:['ignore','pipe','pipe']};
 assert.match(execFileSync(cargo,['llvm-cov','--version'],options),/^cargo-llvm-cov 0\.9\.1\s/);
 const actual=coverageEnvironment(execFileSync(cargo,['llvm-cov','show-env','--sh'],options));
 assert.equal(path.resolve(actual.CARGO_LLVM_COV_TARGET_DIR),target);
 assert.equal(path.resolve(actual.CARGO_LLVM_COV_BUILD_DIR),target);
 assert.equal(actual.LLVM_PROFILE_FILE,env.LLVM_PROFILE_FILE);
}));
test('profile modes retain platform patterns and reject relative or unsupported destinations',()=>{
 const target=path.resolve(os.tmpdir(),'profile target');
 assert.equal(ownedProfileOutput(target,'linux').LLVM_PROFILE_FILE_NAME,'nav-%p-%c.profraw');
 assert.equal(ownedProfileOutput(target,'win32').LLVM_PROFILE_FILE_NAME,'nav-%p-%m.profraw');
 assert.throws(()=>ownedProfileOutput('relative','linux'),/absolute/);
 assert.throws(()=>ownedProfileOutput(target,'unsupported'),/platform/);
});
