import {test,expect} from '@playwright/test';
// @ts-expect-error shared Node/browser helper has no application declaration.
import {exerciseDraftUi} from '../../../scripts/windows/draft-installer-ui.mjs';
type Inputs={context:unknown;identity:unknown;packDirectories:unknown[]};
test('real Content Planner Settings and Today controls produce and retain native-bound source plan',async({page})=>{
 await page.goto('/e2e/support/draft-installer-ui.html');
 const inputs=await page.evaluate(()=>{const h=(window as unknown as {__draftUiRegression:Inputs}).__draftUiRegression;return {context:h.context,identity:h.identity,packDirectories:h.packDirectories};});
 const call=(command:string,args?:unknown)=>page.evaluate(async({command,args})=>(window as unknown as {__draftUiRegression:{call:(command:string,args:unknown)=>Promise<unknown>}}).__draftUiRegression.call(command,args),{command,args});
 const screenshots:string[]=[];
 const fresh=await exerciseDraftUi(page,call,{mode:'fresh',...inputs,capture:async(name:string)=>{screenshots.push(name);},timeout:()=>15000});
 expect(fresh.evidence.checks).toEqual({first_start:true,bundled_activation:true,source_backed_planning:true,restart_persistence:false});
 expect(fresh.state.accepts_inferred_travel).toBe(false);expect(screenshots).toContain('fresh-planning-result');
 const request=await page.evaluate(()=>(window as unknown as {__draftUiRegression:{request:()=>{candidates:{objective:string}[]}}}).__draftUiRegression.request());
 expect(request.candidates[0]?.objective).toBe('quest-stage-ui-source');
 const restarted=await exerciseDraftUi(page,call,{mode:'restart',...inputs,saved:fresh.state,capture:async(name:string)=>{screenshots.push(name);},timeout:()=>15000});
 expect(restarted.evidence.checks.restart_persistence).toBe(true);expect(screenshots).toContain('restart-today');
 const calls=await page.evaluate(()=>(window as unknown as {__draftUiRegression:{calls:string[]}}).__draftUiRegression.calls);
 expect(calls.filter(c=>c==='stage_verified_content_bundle')).toHaveLength(2);expect(calls).toContain('start_planning_job');expect(calls).toContain('pin_current_plan');expect(calls).not.toContain('plan_session');expect(calls).not.toContain('stage_content');
});

for(const failure of ['wrong-native-plan','empty-ui-plan'])test(`real screens cannot promote ${failure}`,async({page})=>{
 await page.goto('/e2e/support/draft-installer-ui.html');
 const inputs=await page.evaluate(value=>{const h=(window as unknown as {__draftUiRegression:Inputs&{setFailure:(value:string)=>void}}).__draftUiRegression;h.setFailure(value);return {context:h.context,identity:h.identity,packDirectories:h.packDirectories};},failure);
 const call=(command:string,args?:unknown)=>page.evaluate(async({command,args})=>(window as unknown as {__draftUiRegression:{call:(command:string,args:unknown)=>Promise<unknown>}}).__draftUiRegression.call(command,args),{command,args});
 await expect(exerciseDraftUi(page,call,{mode:'fresh',...inputs,timeout:()=>1000})).rejects.toThrow();
 const calls=await page.evaluate(()=>(window as unknown as {__draftUiRegression:{calls:string[]}}).__draftUiRegression.calls);
 expect(calls).toContain('start_planning_job');if(failure==='empty-ui-plan')expect(calls).not.toContain('pin_current_plan');
});
