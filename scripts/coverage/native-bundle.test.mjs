import {test} from 'node:test';
import assert from 'node:assert/strict';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import {prepareNativeBundle,stageNativeBundle} from './native-bundle.mjs';
test('stable native coverage requires explicit trusted assembly and verifies staged resources',()=>{
 const root=fs.mkdtempSync(path.join(os.tmpdir(),'native-bundle-'));
 try {
  fs.writeFileSync(path.join(root,'Cargo.toml'),'version = "1.0.0-rc.1"');
  assert.equal(prepareNativeBundle(root,root,()=>assert.fail()),null);
  stageNativeBundle(null,'unused',()=>assert.fail());
  fs.writeFileSync(path.join(root,'Cargo.toml'),'version = "1.0.0"');
  assert.throws(()=>prepareNativeBundle(root,root,()=>{}));
  fs.mkdirSync(path.join(root,'release/v1'),{recursive:true});
  const identity=path.join(root,'release/v1/bundled-content-build-identity.json');
  for(const raw of ['{}','{"app_version":"1.0.0"}\n','x'.repeat(65537)]){
   fs.writeFileSync(identity,raw);assert.throws(()=>prepareNativeBundle(root,root,()=>{}));
  }
  fs.writeFileSync(identity,'{"app_version":"1.0.0"}');
  assert.throws(()=>prepareNativeBundle(root,root,()=>{throw new Error('production signature refused');}),/signature refused/);
  const commands=[];
  const bundle=prepareNativeBundle(root,root,(name,exe,args)=>{
   commands.push({name,exe,args});fs.mkdirSync(path.join(root,'bundled-content'));fs.writeFileSync(path.join(root,'bundled-content/manifest.json'),'verified bytes');
  });
  const exe=path.join(root,'build/native.exe');fs.mkdirSync(path.dirname(exe));
  stageNativeBundle(bundle,exe,(name,command,args)=>commands.push({name,command,args}));
  assert.equal(fs.readFileSync(path.join(root,'build/bundled-content/manifest.json'),'utf8'),'verified bytes');
  assert(commands[0].args.includes('assemble-bundled-content'));
  assert(commands[1].args.includes('check-bundled-content'));
  assert(commands.every(command=>command.args.includes(identity)));
  assert.throws(()=>stageNativeBundle(bundle,exe,()=>{}),/already exists/);
  fs.rmSync(path.join(root,'build/bundled-content'),{recursive:true});
  assert.throws(()=>stageNativeBundle(bundle,exe,()=>{throw new Error('wrong copied resource');}),/wrong copied/);
  fs.writeFileSync(path.join(root,'Cargo.toml'),'no version');assert.throws(()=>prepareNativeBundle(root,root,()=>{}),/version/);
 } finally {fs.rmSync(root,{recursive:true,force:true});}
});
