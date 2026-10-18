import fs from 'node:fs';
import path from 'node:path';
import crypto from 'node:crypto';
import { execFileSync } from 'node:child_process';
import { createRequire } from 'node:module';
import { fileURLToPath } from 'node:url';
const repo=path.resolve(path.dirname(fileURLToPath(import.meta.url)),'../..');
const require=createRequire(path.join(repo,'apps/desktop/package.json'));
const ts=require('typescript');
const sha=value=>crypto.createHash('sha256').update(value).digest('hex');

// Tokens only: strings/comments remain inert, including nested block comments,
// raw strings containing braces/attributes and character literals vs lifetimes.
export function rustTokens(text) {
 const tokens=[];let i=0;
 while(i<text.length){
  const start=i,c=text[i];
  if(/\s/.test(c)){i++;continue;}
  if(text.startsWith('//',i)){const end=text.indexOf('\n',i);i=end<0?text.length:end;continue;}
  if(text.startsWith('/*',i)){let depth=1;i+=2;while(i<text.length&&depth){if(text.startsWith('/*',i)){depth++;i+=2;}else if(text.startsWith('*/',i)){depth--;i+=2;}else i++;}if(depth)throw new Error('Unclosed Rust comment');continue;}
  const raw=/^(?:br|cr|r)(#*)"/.exec(text.slice(i));
  if(raw){const end=text.indexOf('"'+raw[1],i+raw[0].length);if(end<0)throw new Error('Unclosed Rust raw string');i=end+1+raw[1].length;tokens.push({value:'literal',start,end:i});continue;}
  if(c==='"'){i++;let closed=false;while(i<text.length){if(text[i]==='\\'){i+=2;}else if(text[i++]==='"'){closed=true;break;}}if(!closed)throw new Error('Unclosed Rust string');tokens.push({value:'literal',start,end:i});continue;}
  const char=c==="'" ? /^'(?:\\(?:u\{[0-9a-fA-F_]+\}|x[0-9a-fA-F]{2}|.)|[^'\\\r\n])'/u.exec(text.slice(i)) : null;
  if(char){i+=char[0].length;tokens.push({value:'literal',start,end:i});continue;}
  const ident=/^[A-Za-z_][A-Za-z_0-9]*/.exec(text.slice(i));
  if(ident){i+=ident[0].length;tokens.push({value:ident[0],start,end:i});continue;}
  i++;tokens.push({value:c,start,end:i});
 }
 return tokens;
}
export function rustTestRanges(text) {
 const tokens=rustTokens(text),ranges=[];
 for(let i=0;i<tokens.length;i++){
  // Require exactly cfg(test), not cfg(any(test, production_condition)).
  if(tokens.slice(i,i+7).map(t=>t.value).join(' ')!=='# [ cfg ( test ) ]') continue;
  let end=i+7,depth=0,body=false;
  while(end<tokens.length){const v=tokens[end].value;if(v==='{'){depth++;body=true;}else if(v==='}'){if(--depth===0&&body){end++;break;}}else if(v===';'&&!body){end++;break;}end++;}
  if(end>tokens.length || (body&&depth))throw new Error('Unclosed cfg(test) item');
  let startLine=text.slice(0,tokens[i].start).split('\n').length;
  let endLine=text.slice(0,tokens[end-1].end).split('\n').length;
  // A shared source line still belongs to production and cannot be excluded.
  const before=tokens[i-1],after=tokens[end];
  if(before && text.slice(0,before.end).split('\n').length===startLine)startLine++;
  if(after && text.slice(0,after.start).split('\n').length===endLine)endLine--;
  if(startLine<=endLine)ranges.push([startLine,endLine]);i=end-1;
 }
 return ranges;
}
export function declarationOnlyRust(text) {
 const ranges=rustTestRanges(text);const lines=text.split('\n');
 const stripped=lines.map((line,i)=>ranges.some(([a,b])=>i+1>=a&&i+1<=b)?'':line).join('\n');
 const tokens=rustTokens(stripped);
 // Conservative: function/implementation/macro bodies always require a report.
 // Constant/type/module/re-export declarations alone have no executable lines.
 return !tokens.some((t,i)=>t.value==='fn'||t.value==='impl'||t.value==='macro_rules'||(tokens[i+1]?.value==='!'&&/^[A-Za-z_]/.test(t.value)));
}
export function declarationOnlyTs(text,file='file.ts') {
 const source=ts.createSourceFile(file,text,ts.ScriptTarget.Latest,true,file.endsWith('.tsx')?ts.ScriptKind.TSX:ts.ScriptKind.TS);
 if(source.parseDiagnostics.length)throw new Error('Cannot inventory malformed TypeScript');
 return source.statements.every(node=>{
  if(ts.isInterfaceDeclaration(node)||ts.isTypeAliasDeclaration(node)||ts.isEmptyStatement(node))return true;
  if(ts.canHaveModifiers(node)&&ts.getModifiers(node)?.some(m=>m.kind===ts.SyntaxKind.DeclareKeyword))return true;
  if(ts.isImportDeclaration(node))return node.importClause?.isTypeOnly===true || (node.importClause&&!node.importClause.name&&node.importClause.namedBindings&&ts.isNamedImports(node.importClause.namedBindings)&&node.importClause.namedBindings.elements.every(e=>e.isTypeOnly));
  if(ts.isExportDeclaration(node))return node.isTypeOnly || (node.exportClause&&ts.isNamedExports(node.exportClause)&&node.exportClause.elements.every(e=>e.isTypeOnly));
  return false;
 });
}
export function disposition(file,text) {
 const parts=file.split('/');
 if(parts.includes('vendor'))return ['excluded','vendored dependency'];
 if(parts.some(p=>['tests','e2e','test-fixtures'].includes(p))||/\.(test|spec)\.[cm]?[jt]sx?$/.test(file)||/\/(?:[\w-]+_)?tests\.rs$/.test(file)||file==='apps/desktop/src/test-setup.ts'||file==='apps/desktop/playwright.config.ts')return ['excluded','test source or test configuration/fixture'];
 if(file==='schemas/v1/exchange.ts'){
  if(!text.startsWith('/* Generated from exchange.schema.json by scripts/generate-contract-types.mjs.'))throw new Error('Generated-source provenance changed');
  return ['excluded','generated by scripts/generate-contract-types.mjs; validated separately'];
 }
 if(/(?:istanbul|c8|v8)\s+ignore|coverage\s*:\s*ignore|coverage\s*\(\s*off\s*\)/i.test(text))throw new Error('Coverage-ignore annotations are prohibited');
 const declaration=file.endsWith('.rs')?declarationOnlyRust(text):declarationOnlyTs(text,file);
 return declaration?['declaration_only','No executable first-party declarations; generated derive bodies are not authored code']:['eligible',null];
}
export function inventory(root=repo) {
 const files=execFileSync('git',['ls-files','--cached','--others','--exclude-standard','-z','--','*.rs','*.ts','*.tsx'],{cwd:root}).toString().split('\0').filter(Boolean).sort();
 return {format_version:1,source_commit:execFileSync('git',['rev-parse','HEAD'],{cwd:root,encoding:'utf8'}).trim(),source_hash_normalization:'UTF-8, CRLF to LF only',files:files.map(file=>{
  const text=fs.readFileSync(path.join(root,file),'utf8').replaceAll('\r\n','\n');const [kind,reason]=disposition(file,text);
  return {path:file,language:file.endsWith('.rs')?'rust':'typescript',disposition:kind,reason,source_sha256:sha(text),line_count:text.split('\n').length,test_line_ranges:file.endsWith('.rs')&&kind!=='excluded'?rustTestRanges(text):[]};
 })};
}
if(process.argv[1]&&path.resolve(process.argv[1])===fileURLToPath(import.meta.url)){
 try {if(process.argv.length!==3)throw new Error('Usage: inventory.mjs <new-output.json>');fs.writeFileSync(process.argv[2],JSON.stringify(inventory(),null,2)+'\n',{flag:'wx'});}
 catch(error){console.error(error.message);process.exitCode=1;}
}
