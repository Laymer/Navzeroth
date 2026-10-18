import { test } from 'node:test';
import assert from 'node:assert/strict';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { gzipSync } from 'node:zlib';
import { safeRelative, closure, verifyTree, crateFiles, digest } from './dependency-notices.mjs';

test('paths reject absolute/traversal/platform separators and permit scoped package paths', () => {
  assert.equal(safeRelative('js/@tauri-apps_api-2.12.3/LICENSE'), 'js/@tauri-apps_api-2.12.3/LICENSE');
  for (const p of ['../secret','a/../b','/tmp/x','a//b','a/./b','C:/x','a\\b','a\0b','']) assert.throws(()=>safeRelative(p));
});
test('closure follows normal/build dependencies, excludes dev-only and workspace members, handles cycles', () => {
  const deps = (pkg,kind) => ({pkg,dep_kinds:[{kind}]});
  const metadata = {workspace_members:['root','local'],packages:['root','local','build','normal','dev'].map(id=>({id})),resolve:{nodes:[
    {id:'root',deps:[deps('local',null),deps('build','build'),deps('dev','dev')]},
    {id:'local',deps:[deps('normal',null)]},{id:'build',deps:[deps('normal',null)]},{id:'normal',deps:[deps('root',null)]}
  ]}};
  assert.deepEqual(closure(metadata,'root').map(p=>p.id),['build','normal']);
  assert.throws(()=>closure(metadata,'missing'),/Missing resolved/);
});
test('exact notice whitelist rejects missing, altered, extra and symlink files', () => {
  const dir=fs.mkdtempSync(path.join(os.tmpdir(),'nav-notice-test-'));
  try {
    fs.mkdirSync(path.join(dir,'rust')); const file=path.join(dir,'rust','LICENSE');fs.writeFileSync(file,'original');
    const expected=new Map([['rust/LICENSE',Buffer.from('original')]]);
    verifyTree(dir,expected);
    fs.writeFileSync(file,'altered'); assert.throws(()=>verifyTree(dir,expected),/Altered/);
    fs.writeFileSync(file,'original');fs.writeFileSync(path.join(dir,'personal-export.lua'),'private');assert.throws(()=>verifyTree(dir,expected),/extra/);
    fs.unlinkSync(path.join(dir,'personal-export.lua'));fs.unlinkSync(file);assert.throws(()=>verifyTree(dir,expected),/Missing/);
    // File symlinks are unavailable on ordinary Windows without developer mode.
    if (process.platform!=='win32') { fs.symlinkSync('/dev/null',file);assert.throws(()=>verifyTree(dir,expected),/Symbolic/); }
  } finally { fs.rmSync(dir,{recursive:true,force:true}); }
});
function archive(name,body,kind='0') {
  const header=Buffer.alloc(512);header.write(name);header.write(body.length.toString(8).padStart(11,'0')+'\0',124);header.fill(32,148,156);header.write(kind,156);
  const sum=header.reduce((a,b)=>a+b,0);header.write(sum.toString(8).padStart(6,'0')+'\0 ',148);
  return gzipSync(Buffer.concat([header,body,Buffer.alloc((512-body.length%512)%512),Buffer.alloc(1024)]));
}
test('crate notices are bound to lock checksums and bounded validated inert tar members', () => {
  const bytes=archive('example-1/LICENSE',Buffer.from('license'));
  assert.equal(crateFiles(bytes,digest(bytes)).get('example-1/LICENSE').toString(),'license');
  assert.throws(()=>crateFiles(bytes,'0'.repeat(64)),/checksum/);
  const traversal=archive('../secret',Buffer.from('x'));assert.throws(()=>crateFiles(traversal,digest(traversal)),/Unsafe/);
  const symlink=archive('x',Buffer.alloc(0),'2');assert.throws(()=>crateFiles(symlink,digest(symlink)),/Unsupported/);
  const truncated=gzipSync(Buffer.alloc(512,1));assert.throws(()=>crateFiles(truncated,digest(truncated)),/Malformed/);
});
