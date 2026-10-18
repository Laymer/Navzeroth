import {test} from "node:test";
import assert from "node:assert/strict";
import {jsonLines,verifySeries} from "./measure-continuation.mjs";
test("measurement parsing refuses missing/extra reports and invalid JSON",()=>{assert.deepEqual(jsonLines('header\n{"n":0}\n',1),[{n:0}]);assert.throws(()=>jsonLines('{}\n{}',1));assert.throws(()=>jsonLines('header',1));assert.throws(()=>jsonLines('{truncated',1));});
test("raw samples preserve zero and use exact nearest-rank summaries without rounding",()=>{const report={samples_ms:[0,3,1,2],min_ms:0,median_ms:1.5,p95_ms:3,max_ms:3};assert.equal(verifySeries(report,4),report);assert.throws(()=>verifySeries({...report,p95_ms:2.999},4));assert.throws(()=>verifySeries({...report,samples_ms:[0,NaN,1,2]},4));assert.throws(()=>verifySeries(report));assert.throws(()=>verifySeries({...report,min_ms:1},4));});

test('build guard rejects target flags, profile overrides and inherited Cargo config before compiling', async () => {
 const {verifyBuildConfiguration}=await import('./measure-continuation.mjs');
 const {mkdtempSync,mkdirSync,writeFileSync,rmSync}=await import('node:fs');
 const {tmpdir}=await import('node:os');const {join}=await import('node:path');
 const dir=mkdtempSync(join(tmpdir(),'nav-build-guard-'));
 try {
  const source=join(dir,'source');mkdirSync(source);const home=join(dir,'home');mkdirSync(home);
  verifyBuildConfiguration({CARGO_TARGET_DIR:join(dir,'target')},source,home);
  for(const key of ['CARGO_TARGET_X86_64_UNKNOWN_LINUX_GNU_RUSTFLAGS','CARGO_PROFILE_RELEASE_LTO','CARGO_BUILD_RUSTFLAGS','RUSTC_WRAPPER']) assert.throws(()=>verifyBuildConfiguration({[key]:'override'},source,home),/environment overrides/);
  mkdirSync(join(dir,'.cargo'));writeFileSync(join(dir,'.cargo','config.toml'),'[profile.release]\nopt-level=0\n');
  assert.throws(()=>verifyBuildConfiguration({},source,home),/inherited Cargo/);
 } finally {rmSync(dir,{recursive:true,force:true});}
});
