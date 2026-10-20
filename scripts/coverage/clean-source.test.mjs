import {test} from 'node:test';
import assert from 'node:assert/strict';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import {execFileSync} from 'node:child_process';
import {assertCleanCommittedSource,MAX_STATUS_BYTES} from './clean-source.mjs';
test('real Git clean passes and tracked/untracked dirt refuses with exact bounded diagnostics',()=>{
 const root=fs.mkdtempSync(path.join(os.tmpdir(),'native-source-status-'));
 const git=(...args)=>execFileSync('git',args,{cwd:root,stdio:'pipe'});
 try{
  git('init','-q');git('config','user.name','Synthetic test');git('config','user.email','synthetic@example.invalid');
  const tracked=path.join(root,'tracked.txt');fs.writeFileSync(tracked,'canonical\n');git('add','.');git('commit','-qm','synthetic source');
  assert.doesNotThrow(()=>assertCleanCommittedSource(root));
  fs.writeFileSync(tracked,'changed\n');
  assert.throws(()=>assertCleanCommittedSource(root),error=>error.message.includes('git status: " M tracked.txt\\n"'));
  git('checkout','--','tracked.txt');fs.writeFileSync(path.join(root,'untracked output.txt'),'generated');
  assert.throws(()=>assertCleanCommittedSource(root,undefined,'after synthetic writer'),error=>error.message.includes('untracked output.txt')&&error.message.includes('after synthetic writer'));
  assert.equal(fs.readFileSync(path.join(root,'untracked output.txt'),'utf8'),'generated');
 }finally{fs.rmSync(root,{recursive:true,force:true});}
});
test('source status reads are bounded and failure never becomes clean',()=>{
 assert.throws(()=>assertCleanCommittedSource('unused',(_exe,_args,options)=>{
  assert.equal(options.maxBuffer,MAX_STATUS_BYTES);assert.equal(options.timeout,120000);
  return Buffer.alloc(MAX_STATUS_BYTES+1,120);
 }),/exceeds.*bound/);
 assert.throws(()=>assertCleanCommittedSource('unused',()=>{throw new Error('git failed');}),/git failed/);
 assert.throws(()=>assertCleanCommittedSource('unused',()=>Buffer.from('?? control\u001b[31m\n')),error=>{
  assert(!error.message.includes('\u001b'));assert(error.message.includes('\\u001b'));return true;
 });
});
test('native command checks status after recording its result and retains failure profiles',()=>{
 const source=fs.readFileSync(new URL('./collect-rust.mjs',import.meta.url),'utf8');
 const record=source.indexOf("fs.writeFileSync(path.join(destination,'commands.json')");
 const check=source.indexOf('assertCleanCommittedSource(root,undefined,`after ${name}`)');
 assert(record>=0&&check>record);
 assert(check<source.indexOf('  return text;',record));
 assert.match(source,/finally\{[\s\S]*raw-profile-manifest/);
});
