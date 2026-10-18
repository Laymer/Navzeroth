import {test} from 'node:test';
import assert from 'node:assert/strict';
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
