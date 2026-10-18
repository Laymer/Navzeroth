import {test} from 'node:test';
import assert from 'node:assert/strict';
import {rustTokens,rustTestRanges,declarationOnlyRust,declarationOnlyTs,disposition} from './inventory.mjs';
test('Rust lexical inventory keeps strings/comments inert and lifetimes distinct from chars',()=>{
 const text=`/* nested /* fn fake(){} */ comment */\npub const TEXT: &str = r###"fn fake() { #[cfg(test)] }"###;\npub type Ref<'a> = &'a str;\npub const CHAR: char = '}';`;
 assert.equal(declarationOnlyRust(text),true);assert.deepEqual(rustTestRanges(text),[]);
 assert.equal(rustTokens("'\\u{1234}'")[0].value,'literal');
 for(const bad of ['/* open','r#"open','"open'])assert.throws(()=>rustTokens(bad));
});
test('only exact cfg(test) item ranges are excluded; production condition remains executable',()=>{
 const source='pub const N:u32=1;\n#[cfg(test)]\nmod tests {\n #[test] fn check(){ assert_eq!(N,1); }\n}\n';
 assert.deepEqual(rustTestRanges(source),[[2,5]]);assert.equal(declarationOnlyRust(source),true);
 assert.equal(declarationOnlyRust('#[cfg(any(test,unix))] fn production(){}'),false);
 assert.deepEqual(rustTestRanges('fn production(){} #[cfg(test)] mod tests { fn test(){} }'),[]);
 assert.equal(declarationOnlyRust('declare_runtime!();'),false);
 assert.equal(declarationOnlyRust('pub const fn compute()->u32{1}'),false);
});
test('TypeScript declarations are reported without executable lines, unloaded behavior stays eligible',()=>{
 assert.equal(declarationOnlyTs('import type {Thing} from "x"; export interface A {zero:0} export type B = false;'),true);
 assert.equal(declarationOnlyTs('declare global {interface Window {x:string}} export {};','env.d.ts'),true);
 for(const source of ['import "side-effect";','export const zero=0;','export function unused(){return false;}','enum A {Zero}'])assert.equal(declarationOnlyTs(source),false);
 assert.throws(()=>declarationOnlyTs('export const = ;'));
});
test('build scripts, native entrypoints and examples remain eligible; test/vendor/generated exclusions are explicit',()=>{
 for(const file of ['crates/local-data/build.rs','apps/desktop/src-tauri/src/main.rs','crates/core/examples/search.rs'])assert.equal(disposition(file,'fn main(){}')[0],'eligible');
 assert.equal(disposition('apps/desktop/src/main.tsx','start();')[0],'eligible');
 assert.equal(disposition('crates/core/tests/case.rs','fn main(){}')[0],'excluded');
 assert.equal(disposition('vendor/a/src/lib.rs','fn main(){}')[0],'excluded');
 assert.throws(()=>disposition('crates/core/src/logic.rs','#[coverage(off)] fn hidden(){}'));
 assert.throws(()=>disposition('apps/desktop/src/logic.ts','/* v8 ignore next */ export const hidden=1;'));
 assert.throws(()=>disposition('schemas/v1/exchange.ts','export const bad=1;'));
});
