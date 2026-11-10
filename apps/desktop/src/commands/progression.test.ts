import { createClient } from "./client";
import { demoAdapter } from "./demoAdapter";
import type { CommandAdapter } from "./contract";
import type { ManualProgressionRequest } from "../../../../schemas/v1/progression-api";
import fixture from "../../../../fixtures/contracts/v1/progression-snapshot.json";

const request: ManualProgressionRequest = {
  profile_id: "selected-profile",
  expected: fixture.payload.binding as ManualProgressionRequest["expected"],
  id: "manual-one",
  spells: {status:"unknown"}, money: {status:"known", value:"0"},
  talents: {status:"unknown"}, available_talent_points:{status:"known",value:0},
  professions: {status:"unknown"}, recipes:{status:"known",value:{completeness:"partial",entries:{recipe:false}}},
  bags:{status:"unknown"}, bank:{status:"unknown"}, equipment_enhancements:{},
};
it("transports expected binding and manual false/zero without inventing provenance", async () => {
  const calls: {command:string;input:unknown}[] = [];
  const adapter:CommandAdapter = {invoke(command,input) {calls.push({command,input}); return Promise.reject({code:"SNAPSHOT_OUTDATED",message:"Reload inputs",retryable:false});}};
  const client=createClient(adapter);
  expect(await client.getProgressionView("selected-profile")).toEqual({ok:false,error:{code:"SNAPSHOT_OUTDATED",message:"Reload inputs",retryable:false}});
  await client.saveManualProgression(request);
  const imported={profile_id:request.profile_id,expected:request.expected,text:"ANAV1:explicit-copied-input"};
  await client.importProgression(imported);
  expect(calls).toEqual([
    {command:"get_progression_view",input:{profile_id:"selected-profile"}},
    {command:"save_manual_progression",input:{text:JSON.stringify(request)}},
    {command:"import_progression_text",input:{text:JSON.stringify(imported)}},
  ]);
});
it("browser demo explicitly refuses persistent progression actions", async () => {
  const client=createClient(demoAdapter);
  for (const result of [await client.getProgressionView("demo"),await client.saveManualProgression(request),await client.importProgression({profile_id:request.profile_id,expected:request.expected,text:"ANAV1:input"})]) {
    expect(result.ok).toBe(false);
  }
});
