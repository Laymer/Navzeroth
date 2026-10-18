import {test} from 'node:test';
import assert from 'node:assert/strict';
import {lcovLines,istanbulLines,relativeSource,normalizeRaw,verdict} from './gate.mjs';
const commit='a'.repeat(40),sha='b'.repeat(64);
const inventory={format_version:1,source_commit:commit,files:[{path:'crates/core/src/logic.rs',language:'rust',disposition:'eligible',source_sha256:sha,line_count:100,test_line_ranges:[[50,60]]},{path:'apps/desktop/src/main.tsx',language:'typescript',disposition:'eligible',source_sha256:sha,line_count:10,test_line_ranges:[]},{path:'apps/desktop/src/types.ts',language:'typescript',disposition:'declaration_only',source_sha256:sha,line_count:10,test_line_ranges:[]}]};
const rust=platform=>({format_version:1,source_commit:commit,inventory_sha256:sha,clean_source:true,kind:'rust-llvm',platform,tool_version:'0.9.1',rust_llvm_version:'23.1.1',llvm_tools_version:'23.1.1',incomplete_profiles:[],files:[{path:inventory.files[0].path,source_sha256:sha,lines:Array.from({length:10},(_,i)=>[i+1,platform==='linux'?(i<8?1:0):(i===8?1:0)])}]});
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
 for(const patch of [{source_commit:'c'.repeat(40)},{inventory_sha256:'c'.repeat(64)},{tool_version:'0.9.2'},{llvm_tools_version:'22.0.0'},{incomplete_profiles:['truncated']},{clean_source:false}])assert.throws(()=>verdict(inventory,[{...rust('linux'),...patch},rust('win32'),typescript],sha));
 assert.throws(()=>verdict(inventory,[{...rust('linux'),files:[{...rust('linux').files[0],source_sha256:'c'.repeat(64)}]}],sha));
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
 const receipt={source_root:'C:/repo',platform:'win32'};
 const records=[{path:'C:/repo/CRATES/core/src/logic.rs',lines:[[1,0]]},{path:'C:/registry/dep.rs',lines:[[1,1]]}];
 assert.equal(normalizeRaw(records,receipt,inventory)[0].path,'crates/core/src/logic.rs');
 assert.throws(()=>normalizeRaw([{path:'C:/repo/new.rs',lines:[]}],receipt,inventory));
});
test('declaration-only reports cannot conceal newly executable code',()=>{
 const bad={...typescript,files:[{path:inventory.files[2].path,source_sha256:sha,lines:[[1,1]]}]};assert.throws(()=>verdict(inventory,[rust('linux'),rust('win32'),bad],sha));
});
