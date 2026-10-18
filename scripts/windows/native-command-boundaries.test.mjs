import {test} from 'node:test';
import assert from 'node:assert/strict';
import fs from 'node:fs';
import {COMMAND_BOUNDARIES,validateRefusal} from './native-command-boundaries.mjs';
test('native boundary matrix is bounded and rejects transport errors and false successes',()=>{
 assert.ok(COMMAND_BOUNDARIES.length>40&&COMMAND_BOUNDARIES.length<100);
 for(const entry of COMMAND_BOUNDARIES){
  assert.match(entry.command,/^[a-z_]+$/);
  assert.ok(JSON.stringify(entry.args).length<1024);
  assert.doesNotMatch(entry.command,/^select_local_(installation|material)$/);
  assert.throws(()=>validateRefusal(entry,{ok:true}));
  for(const error of ['unknown command',{},null,{code:'INVALID_INPUT',message:'bad'}])assert.throws(()=>validateRefusal(entry,{ok:false,error}));
  validateRefusal(entry,{ok:false,error:{code:'INVALID_INPUT',message:'Invalid synthetic input',retryable:false}});
 }
});
test('native refusal requests name real Rust handlers and their serialized argument keys',()=>{
 const source=fs.readFileSync(new URL('../../apps/desktop/src-tauri/src/lib.rs',import.meta.url),'utf8');
 const commands=new Map();
 for(const [,options,name,parameters]of source.matchAll(/#\[tauri::command([^\]]*)\]\s*async fn (\w+)(?:<[^>]+>)?\(([\s\S]*?)\)\s*->/g)){
  const names=[...parameters.matchAll(/(?:^|[,\n])\s*(\w+)\s*:/g)].map(match=>match[1]).filter(name=>!['state','app','window','controller'].includes(name));
  commands.set(name,names.map(name=>options.includes('snake_case')?name:name.replace(/_([a-z])/g,(_,letter)=>letter.toUpperCase())).sort());
 }
 for(const entry of COMMAND_BOUNDARIES){
  assert.ok(commands.has(entry.command),entry.command);
  assert.deepEqual(Object.keys(entry.args).sort(),commands.get(entry.command),entry.command);
 }
});
