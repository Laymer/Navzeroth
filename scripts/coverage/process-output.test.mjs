import test from 'node:test';import assert from 'node:assert/strict';
import fs from 'node:fs';import os from 'node:os';import path from 'node:path';
import {loggedProcess} from './process-output.mjs';import {coverageEnvironment} from './rust-environment.mjs';
test('informational stderr is retained but cannot contaminate inert environment assignments',()=>{
 const root=fs.mkdtempSync(path.join(os.tmpdir(),'nav-coverage-output-'));
 try{
  const assignments="LLVM_PROFILE_FILE='/tmp/raw-%p.profraw'\nRUSTC_WRAPPER=/cargo/llvm-cov\nCARGO_LLVM_COV_TARGET_DIR='/tmp/build'\n";
  const log=path.join(root,'environment.log');
  const result=loggedProcess(process.execPath,['-e',`process.stderr.write('info: instrumenting coverage\\n');process.stdout.write(${JSON.stringify(assignments)});`],{log,stdoutOnly:true,timeout:10000});
  assert.equal(result.process.status,0);assert.equal(result.text,assignments);
  assert.equal(coverageEnvironment(result.text).CARGO_LLVM_COV_TARGET_DIR,'/tmp/build');
  const retained=fs.readFileSync(log,'utf8');assert.ok(retained.includes('info: instrumenting coverage'));assert.ok(retained.includes(assignments));
  assert.throws(()=>coverageEnvironment(retained),/Unexpected coverage environment/);
 }finally{fs.rmSync(root,{recursive:true,force:true});}
});
test('nonzero child results retain stderr and are not silently reclassified as success',()=>{
 const root=fs.mkdtempSync(path.join(os.tmpdir(),'nav-coverage-output-error-'));
 try{
  const log=path.join(root,'failure.log');
  const result=loggedProcess(process.execPath,['-e',"process.stderr.write('refused');process.exitCode=2"],{log,stdoutOnly:true,timeout:10000});
  assert.equal(result.process.status,2);assert.equal(result.text,'');assert.equal(fs.readFileSync(log,'utf8'),'refused');
 }finally{fs.rmSync(root,{recursive:true,force:true});}
});
