import {test} from "node:test";
import assert from "node:assert/strict";
import {jsonLines,verifySeries} from "./measure-continuation.mjs";
test("measurement parsing refuses missing/extra reports and invalid JSON",()=>{assert.deepEqual(jsonLines('header\n{"n":0}\n',1),[{n:0}]);assert.throws(()=>jsonLines('{}\n{}',1));assert.throws(()=>jsonLines('header',1));assert.throws(()=>jsonLines('{truncated',1));});
test("raw samples preserve zero and use exact nearest-rank summaries without rounding",()=>{const report={samples_ms:[0,3,1,2],min_ms:0,median_ms:1.5,p95_ms:3,max_ms:3};assert.equal(verifySeries(report,4),report);assert.throws(()=>verifySeries({...report,p95_ms:2.999},4));assert.throws(()=>verifySeries({...report,samples_ms:[0,NaN,1,2]},4));assert.throws(()=>verifySeries(report));assert.throws(()=>verifySeries({...report,min_ms:1},4));});
