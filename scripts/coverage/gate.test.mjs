import {test} from 'node:test';
import assert from 'node:assert/strict';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import crypto from 'node:crypto';
import {spawnSync} from 'node:child_process';
import {lcovLines,istanbulLines,relativeSource,normalizeRaw,verdict,runGate} from './gate.mjs';
const commit='a'.repeat(40),sha='b'.repeat(64);
const inventory={format_version:1,source_commit:commit,files:[{path:'crates/core/src/logic.rs',language:'rust',disposition:'eligible',source_sha256:sha,line_count:100,test_line_ranges:[[50,60]]},{path:'apps/desktop/src/main.tsx',language:'typescript',disposition:'eligible',source_sha256:sha,line_count:10,test_line_ranges:[]},{path:'apps/desktop/src/types.ts',language:'typescript',disposition:'declaration_only',source_sha256:sha,line_count:10,test_line_ranges:[]}]};
const rust=platform=>({format_version:1,source_commit:commit,inventory_sha256:sha,clean_source:true,tests_passed:true,kind:'rust-llvm',platform,source_root:platform==='win32'?'C:/repo':'/repo',tool_version:'0.9.1',rust_llvm_version:'23.1.1',llvm_tools_version:'23.1.1',incomplete_profiles:[],files:[{path:inventory.files[0].path,source_sha256:sha,lines:Array.from({length:10},(_,i)=>[i+1,platform==='linux'?(i<8?1:0):(i===8?1:0)])}]});
const typescript={...rust('linux'),kind:'typescript-v8',tool_version:'5.0.3',files:[{path:inventory.files[1].path,source_sha256:sha,lines:[[1,1]]}]};
test('unions executable source lines across native platforms without rounding the threshold',()=>{
 const result=verdict(inventory,[rust('linux'),rust('win32'),typescript],sha);assert.equal(result.passes,true);assert.equal(result.files[0].covered,9);assert.equal(result.files[0].executable,10);
 const failed=verdict(inventory,[{...rust('linux'),files:[{...rust('linux').files[0],lines:Array.from({length:100},(_,i)=>[i+1,i<89?1:0])}]},rust('win32'),typescript],sha);
 // Inline test lines 50..60 are excluded from both numerator and denominator.
 assert.equal(failed.files[0].covered,78);assert.equal(failed.files[0].executable,89);assert.equal(failed.passes,false);
});
test('missing files/platforms, mismatched source/commit/tools and partial profiles cannot pass',()=>{
 assert.equal(verdict(inventory,[rust('linux'),rust('win32')],sha).passes,false);
 assert.equal(verdict(inventory,[rust('linux'),typescript],sha).passes,false);
 assert.throws(()=>verdict(inventory,[rust('linux'),{...rust('win32'),files:[]},typescript],sha),/no executable first-party/);
 for(const patch of [{source_commit:'c'.repeat(40)},{inventory_sha256:'c'.repeat(64)},{tool_version:'0.9.2'},{llvm_tools_version:'22.0.0'},{incomplete_profiles:['truncated']},{clean_source:false},{incomplete_profiles:null},{incomplete_profiles:undefined}])assert.throws(()=>verdict(inventory,[{...rust('linux'),...patch},rust('win32'),typescript],sha));
 assert.throws(()=>verdict(inventory,[{...rust('linux'),files:[{...rust('linux').files[0],source_sha256:'c'.repeat(64)}]}],sha));
});
test('missing native entrypoint execution retains a failing full per-file verdict',()=>{
 const entry={...inventory.files[0],path:'apps/desktop/src-tauri/src/main.rs'};
 const report=verdict({...inventory,files:[...inventory.files,entry]},[rust('linux'),{...rust('win32'),files:[...rust('win32').files,{path:entry.path,source_sha256:sha,lines:[[1,0]]}]},typescript],sha);
 assert.equal(report.passes,false);
 assert.deepEqual(report.errors,['Native entrypoint has no execution evidence: '+entry.path]);
 assert.equal(report.files.find(file=>file.path===entry.path).passes,false);
 assert.equal(report.files.length,inventory.files.length+1);
});
test('LCOV truncation, overflow, negative/zero line numbers and unexpected ordering fail',()=>{
 assert.deepEqual(lcovLines('SF:/repo/x.rs\nDA:1,0\nDA:2,1\nend_of_record\n'),[{path:'/repo/x.rs',lines:[[1,0],[2,1]]}]);
 for(const text of ['DA:1,1','SF:x\nDA:0,1\nend_of_record','SF:x\nDA:1,-1\nend_of_record','SF:x\nDA:1,9007199254740993\nend_of_record','SF:x\nDA:1,1','SF:x\nSF:y','end_of_record'])assert.throws(()=>lcovLines(text));
});
test('Istanbul line counts use statement starts and retain zero without threshold rounding',()=>{
 const report={x:{path:'/repo/x.ts',statementMap:{0:{start:{line:2}},1:{start:{line:2}},2:{start:{line:3}}},s:{0:0,1:1,2:0}}};
 assert.deepEqual(istanbulLines(report),[{path:'/repo/x.ts',lines:[[2,1],[3,0]]}]);assert.throws(()=>istanbulLines({x:{...report.x,s:{0:-1}}}));
});
test('normalizes checkout roots and source case only for Windows and rejects traversal',()=>{
 assert.equal(relativeSource('C:\\work\\Repo\\crates\\core.rs','c:\\WORK\\repo','win32'),'crates/core.rs');
 assert.equal(relativeSource('file:///C:/work/Repo/crates/core.rs','C:/work/Repo','win32'),'crates/core.rs');
 assert.throws(()=>relativeSource('/other/x.rs','/repo','linux'));assert.throws(()=>relativeSource('/repo/../x.rs','/repo','linux'));
 const receipt={source_root:'C:/repo',platform:'win32',source_hashes:Object.fromEntries(inventory.files.map(f=>[f.path,f.source_sha256]))};
 const records=[{path:'C:/repo/CRATES/core/src/logic.rs',lines:[[1,0]]},{path:'C:/registry/dep.rs',lines:[[1,1]]}];
 assert.equal(normalizeRaw(records,receipt,inventory)[0].path,'crates/core/src/logic.rs');
 assert.throws(()=>normalizeRaw(records,{...receipt,source_hashes:{}},inventory),/collector source hash/);
 const wrong=normalizeRaw([{...records[0],lines:[[1,1]]}],{...receipt,source_hashes:{[inventory.files[0].path]:'c'.repeat(64)}},inventory);
 assert.throws(()=>verdict(inventory,[{...rust('win32'),files:wrong}],sha),/altered coverage source/);
 assert.throws(()=>normalizeRaw([{path:'C:/repo/new.rs',lines:[]}],receipt,inventory));
});
test('declaration-only reports cannot conceal newly executable code',()=>{
 const bad={...typescript,files:[{path:inventory.files[2].path,source_sha256:sha,lines:[[1,1]]}]};assert.throws(()=>verdict(inventory,[rust('linux'),rust('win32'),bad],sha));
});

test('real Windows Rust module aliases normalize before inventory and hash validation',()=>{
 const receipt={...rust('win32'),source_hashes:{[inventory.files[0].path]:sha}};
 const alias='C:\\repo\\crates\\core\\tests\\..\\src\\logic.rs';
 assert.equal(relativeSource(alias,'C:/repo','win32'),inventory.files[0].path);
 assert.equal(normalizeRaw([{path:alias,lines:[[1,1]]}],receipt,inventory)[0].path,inventory.files[0].path);
 assert.throws(()=>normalizeRaw([{path:alias,lines:[[1,1]]}],{...receipt,source_hashes:{}},inventory),/collector source hash/);
 for(const bad of ['C:/repo/../outside.rs','C:/repo/x/../../outside.rs','C:/repo/crates/core/src/logic.rs:stream'])assert.throws(()=>relativeSource(bad,'C:/repo','win32'));
 assert.throws(()=>normalizeRaw([{path:'C:/repo/../outside.rs',lines:[[1,1]]}],receipt,inventory),/outside recorded checkout/);
 assert.deepEqual(normalizeRaw([{path:'C:/repo/rustc/'+ 'b'.repeat(40)+'/library/std/src/sys/thread_local/native/mod.rs',lines:[[1,1]]}],receipt,inventory),[]);
 for(const bad of ['C:/repo/rustc/not-a-commit/library/std.rs','C:/repo/rustc/'+ 'b'.repeat(40)+'/application.rs'])assert.throws(()=>normalizeRaw([{path:bad,lines:[[1,1]]}],receipt,inventory),/missing from inventory/);
 const known='rustc/'+ 'b'.repeat(40)+'/library/owned.rs';
 assert.throws(()=>normalizeRaw([{path:'C:/repo/'+known,lines:[[1,1]]}],receipt,{...inventory,files:[...inventory.files,{...inventory.files[0],path:known}]}),/collector source hash/);
});

test('end-to-end gate verifies independent collector hashes and derives verdict from raw counters',()=>{
 const dir=fs.mkdtempSync(path.join(os.tmpdir(),'nav-gate-adversary-'));
 const digest=data=>crypto.createHash('sha256').update(data).digest('hex');
 const write=(name,value)=>fs.writeFileSync(path.join(dir,name),typeof value==='string'?value:JSON.stringify(value));
 try {
  write('inventory.json',inventory);const invSha=digest(fs.readFileSync(path.join(dir,'inventory.json')));
  const receipts=[];
  for(const platform of ['linux','win32']){
   const body=`SF:${platform==='linux'?'/repo/':'C:/repo/'}crates/core/src/logic.rs\nDA:1,1\nend_of_record\n`;
   write(platform+'.lcov',body);
   const receipt={...rust(platform),inventory_sha256:invSha,source_hashes:{[inventory.files[0].path]:sha},raw_reports:[{path:platform+'.lcov',format:'lcov',sha256:digest(body)}]};
   write(platform+'.json',receipt);receipts.push(path.join(dir,platform+'.json'));
  }
  const raw={source:{path:'/repo/apps/desktop/src/main.tsx',statementMap:{0:{start:{line:1}}},s:{0:0}}};
  write('ui-raw.json',raw);
  write('tests.json',{numFailedTests:0,numFailedTestSuites:0,numTotalTests:1});
  const ui={...typescript,inventory_sha256:invSha,source_hashes:{[inventory.files[1].path]:sha},raw_reports:[{path:'ui-raw.json',format:'istanbul',sha256:digest(fs.readFileSync(path.join(dir,'ui-raw.json')))},{path:'tests.json',format:'test-results',sha256:digest(fs.readFileSync(path.join(dir,'tests.json')))}]};
  // The claimed summary says covered, but the raw source statement was never run.
  write('ui.json',ui);receipts.push(path.join(dir,'ui.json'));
  const outcome=runGate(path.join(dir,'inventory.json'),receipts,path.join(dir,'verdict.json'));
  assert.equal(outcome.passes,false);assert.equal(outcome.files[1].covered,0);
  for(const [index,patch] of [
   {raw_reports:ui.raw_reports.slice(0,1)},
   {raw_reports:[ui.raw_reports[0],ui.raw_reports[0]]},
   {kind:'typescript-config-v8',raw_reports:ui.raw_reports.slice(0,1)},
   {kind:'typescript-config-v8',raw_reports:ui.raw_reports},
  ].entries()){
   write('ui.json',{...ui,...patch});
   assert.throws(()=>runGate(path.join(dir,'inventory.json'),receipts,path.join(dir,`missing-raw-${index}.json`)),/Required raw coverage evidence/);
  }

  write('ui.json',{...ui,source_hashes:{[inventory.files[1].path]:'c'.repeat(64)}});
  assert.throws(()=>runGate(path.join(dir,'inventory.json'),receipts,path.join(dir,'bad-source.json')),/altered coverage source/);
  write('ui.json',ui);write('ui-raw.json',{...raw,extra:'altered'});
  assert.throws(()=>runGate(path.join(dir,'inventory.json'),receipts,path.join(dir,'bad-raw.json')),/altered raw/);
 }finally{fs.rmSync(dir,{recursive:true,force:true});}
});

function evidenceFixture(t){
 const dir=fs.mkdtempSync(path.join(os.tmpdir(),'nav-gate-inputs-'));
 t.after(()=>fs.rmSync(dir,{recursive:true,force:true}));
 const write=(name,bytes)=>fs.writeFileSync(path.join(dir,name),bytes);
 write('inventory.json',JSON.stringify(inventory));
 const body='SF:/repo/crates/core/src/logic.rs\nDA:1,1\nend_of_record\n';
 write('raw.lcov',body);
 write('receipt.json',JSON.stringify({...rust('linux'),
  inventory_sha256:crypto.createHash('sha256').update(fs.readFileSync(path.join(dir,'inventory.json'))).digest('hex'),
  source_hashes:{[inventory.files[0].path]:sha},
  raw_reports:[{path:'raw.lcov',format:'lcov',sha256:crypto.createHash('sha256').update(body).digest('hex')}],
 }));
 return {dir,write,run:()=>runGate(path.join(dir,'inventory.json'),[path.join(dir,'receipt.json')],path.join(dir,'verdict.json'))};
}

test('inventory size and regular-file bounds apply before reading any bytes',t=>{
 const fixture=evidenceFixture(t),file=path.join(fixture.dir,'inventory.json');
 fs.truncateSync(file,64*1024*1024+1);
 const original=fs.readSync;let reads=0;
 t.mock.method(fs,'readSync',(...args)=>{reads++;return original(...args);});
 assert.throws(fixture.run,/bounded regular file/);
 assert.equal(reads,0);assert.equal(fs.existsSync(path.join(fixture.dir,'verdict.json')),false);
});

test('inventory and receipt symbolic links are refused even with matching bytes',t=>{
 const fixture=evidenceFixture(t);
 for(const name of ['inventory.json','receipt.json']){
  const file=path.join(fixture.dir,name),saved=file+'.saved';
  fs.renameSync(file,saved);fs.symlinkSync(saved,file);
  assert.throws(fixture.run,/bounded regular file/);
  fs.unlinkSync(file);fs.renameSync(saved,file);
 }
});

test('raw report size, type and final symlink are checked before parsing',t=>{
 const fixture=evidenceFixture(t),file=path.join(fixture.dir,'raw.lcov');
 const body=fs.readFileSync(file);
 fs.truncateSync(file,256*1024*1024+1);
 assert.throws(fixture.run,/bounded regular file/);
 fs.unlinkSync(file);fs.mkdirSync(file);
 assert.throws(fixture.run,/bounded regular file/);
 fs.rmdirSync(file);fixture.write('saved.lcov',body);
 fs.symlinkSync(path.join(fixture.dir,'saved.lcov'),file);
 assert.throws(fixture.run,/bounded regular file/);
});

test('nested raw reports work but linked parent directories cannot supply evidence',t=>{
 const fixture=evidenceFixture(t),receiptFile=path.join(fixture.dir,'receipt.json');
 const receipt=JSON.parse(fs.readFileSync(receiptFile));
 fs.mkdirSync(path.join(fixture.dir,'reports'));
 fs.renameSync(path.join(fixture.dir,'raw.lcov'),path.join(fixture.dir,'reports','raw.lcov'));
 receipt.raw_reports[0].path='reports/raw.lcov';
 fixture.write('receipt.json',JSON.stringify(receipt));
 assert.equal(fixture.run().passes,false); // Missing Windows and UI, not a claimed pass.
 fs.unlinkSync(path.join(fixture.dir,'verdict.json'));
 fs.renameSync(path.join(fixture.dir,'reports'),path.join(fixture.dir,'external'));
 fs.symlinkSync(path.join(fixture.dir,'external'),path.join(fixture.dir,'reports'),'junction');
 assert.throws(fixture.run,/directory is linked/);
 assert.equal(fs.existsSync(path.join(fixture.dir,'verdict.json')),false);
});

for(const mutation of ['growth','truncation'])test(`bounded evidence read refuses concurrent ${mutation}`,t=>{
 const fixture=evidenceFixture(t),original=fs.readSync;let mutated=false,total=0;
 const file=path.join(fixture.dir,'inventory.json'),originalSize=fs.statSync(file).size;
 t.mock.method(fs,'readSync',(fd,buffer,offset,length,position)=>{
  if(!mutated){
   mutated=true;
   if(mutation==='growth')fs.appendFileSync(file,' '.repeat(4096));
   else fs.truncateSync(file,1);
  }
  assert.ok(buffer.length<=originalSize+1);
  const count=original(fd,buffer,offset,length,position);total+=count;return count;
 });
 assert.throws(fixture.run,/changed during read/);
 assert.ok(total<=originalSize+1);
 assert.equal(fs.existsSync(path.join(fixture.dir,'verdict.json')),false);
});

test('replacement between path inspection and open is refused',t=>{
 const fixture=evidenceFixture(t),file=path.join(fixture.dir,'inventory.json');
 const original=fs.openSync;let replaced=false;
 t.mock.method(fs,'openSync',(target,...args)=>{
  if(target===file&&!replaced){
   replaced=true;fs.renameSync(file,file+'.old');fs.copyFileSync(file+'.old',file);
  }
  return original(target,...args);
 });
 assert.throws(fixture.run,/changed during read/);
 assert.equal(fs.existsSync(path.join(fixture.dir,'verdict.json')),false);
});


test('replacement with a FIFO is refused without blocking before descriptor inspection',{skip:process.platform==='win32'},t=>{
 const fixture=evidenceFixture(t),fifo=path.join(fixture.dir,'fifo');
 const create=spawnSync('mkfifo',[fifo],{encoding:'utf8'});
 assert.equal(create.status,0,create.stderr);
 const file=path.join(fixture.dir,'inventory.json');
 const child=spawnSync(process.execPath,['--input-type=module','-e',`
  import fs from 'node:fs';
  import assert from 'node:assert/strict';
  import {runGate} from ${JSON.stringify(new URL('./gate.mjs',import.meta.url).href)};
  const file=${JSON.stringify(file)},original=fs.openSync;let replaced=false;
  fs.openSync=(target,...args)=>{
   if(target===file&&!replaced){replaced=true;fs.unlinkSync(file);fs.renameSync(${JSON.stringify(fifo)},file);}
   return original(target,...args);
  };
  assert.throws(()=>runGate(file,[${JSON.stringify(path.join(fixture.dir,'receipt.json'))}],${JSON.stringify(path.join(fixture.dir,'verdict.json'))}),/changed during read/);
 `],{encoding:'utf8',timeout:3000});
 assert.equal(child.error,undefined,child.error?.message);
 assert.equal(child.status,0,child.stderr);
 assert.equal(fs.existsSync(path.join(fixture.dir,'verdict.json')),false);
});
