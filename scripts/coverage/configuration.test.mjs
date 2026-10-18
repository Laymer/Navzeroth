import test from 'node:test';
import assert from 'node:assert/strict';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import {execFileSync} from 'node:child_process';
import {fileURLToPath} from 'node:url';
import {convertConfigurationScript} from './configuration.mjs';

test('real default-export configuration has measured statements; zero V8 hits stay zero',async()=>{
 const root=path.resolve(path.dirname(fileURLToPath(import.meta.url)),'../..');
 const parent=fs.mkdtempSync(path.join(os.tmpdir(),'nav-configuration-test-'));
 const output=path.join(parent,'report');
 try{
  execFileSync(process.execPath,[path.join(root,'scripts/coverage/configuration.mjs'),output],{timeout:60000});
  const raw=JSON.parse(fs.readFileSync(path.join(output,'configuration-v8.json'),'utf8'));
  const report=JSON.parse(fs.readFileSync(path.join(output,'configuration-istanbul.json'),'utf8'));
  for(const script of raw.result){
   const actual=Object.values(report).find(file=>script.url.endsWith('/'+path.basename(file.path)));
   assert.ok(actual);assert.ok(Object.keys(actual.s).length>0,'Executable config must never disappear from report');
   assert.ok(Object.values(actual.s).every(count=>count>0));
   const zero={...script,functions:script.functions.map(fn=>({...fn,ranges:fn.ranges.map(range=>({...range,count:0}))}))};
   const unexecuted=await convertConfigurationScript(zero,raw.compiled[script.url]);
   const statements=Object.values(unexecuted).flatMap(file=>Object.values(file.s));
   assert.ok(statements.length>0);assert.ok(statements.every(count=>count===0),'Converter must not invent hits');
  }
 }finally{fs.rmSync(parent,{recursive:true,force:true});}
});
