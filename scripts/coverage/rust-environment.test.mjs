import test from 'node:test';import assert from 'node:assert/strict';
import {coverageEnvironment,compilerIdentity,requireMatchingLlvm} from './rust-environment.mjs';
const base="LLVM_PROFILE_FILE='/tmp/raw-%p-%c.profraw'\nRUSTC_WRAPPER=/cargo/llvm-cov\nCARGO_LLVM_COV_TARGET_DIR='/tmp/owned target'";
test('coverage shell assignments remain data, preserving spaces, backslashes, quotes and encoded flags',()=>{
 const result=coverageEnvironment(base+"\nRUSTFLAGS='-C\x1finstrument-coverage'\nWINDOWS='C:\\work\\target'\nQUOTED='owner'\\''s directory'");
 assert.equal(result.CARGO_LLVM_COV_TARGET_DIR,'/tmp/owned target');assert.equal(result.WINDOWS,'C:\\work\\target');assert.equal(result.QUOTED,"owner's directory");assert.equal(result.RUSTFLAGS,'-C\x1finstrument-coverage');
 assert.equal(coverageEnvironment(base+"\nINERT='$(must-not-execute); `nor this`'").INERT,'$(must-not-execute); `nor this`');
});
test('malformed, duplicated, incomplete or executable shell expressions are refused',()=>{
 for(const extra of ['X=$(command)','X=`command`','X=a;b','X="value"',"X='unclosed",'X=truncated\\','RUSTC_WRAPPER=/different','not an assignment'])assert.throws(()=>coverageEnvironment(base+'\n'+extra));
 assert.throws(()=>coverageEnvironment('X=value'),/Missing instrumentation/);
});

test('compiler and LLVM identities match exact versions rather than a shared string prefix',()=>{
 const rust='release: 1.99.0\nhost: x86_64-unknown-linux-gnu\nLLVM version: 23.1.1\n';
 assert.deepEqual(compilerIdentity(rust,'1.99.0'),{host:'x86_64-unknown-linux-gnu',llvm:'23.1.1'});
 assert.doesNotThrow(()=>requireMatchingLlvm('LLVM (http://llvm.org/):\n  LLVM version 23.1.1-rust-1.99.0-stable\n','23.1.1'));
 assert.throws(()=>compilerIdentity(rust,'1.99.1'));
 assert.throws(()=>compilerIdentity('release: 1.99.0','1.99.0'));
 assert.throws(()=>compilerIdentity('host: x86_64-unknown-linux-gnu\nLLVM version: 23.1.1',undefined));
 for(const actual of ['23.1.10','23.1.0','22.1.1'])assert.throws(()=>requireMatchingLlvm('LLVM version '+actual,'23.1.1'));
});
