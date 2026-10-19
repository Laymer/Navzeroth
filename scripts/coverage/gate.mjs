import fs from 'node:fs';
import path from 'node:path';
import crypto from 'node:crypto';
import {fileURLToPath} from 'node:url';
const hash=bytes=>crypto.createHash('sha256').update(bytes).digest('hex');
function sourcePath(file,platform){
 if(!file.startsWith('file:'))return file;
 if(platform!=='win32')return fileURLToPath(file);
 const url=new URL(file),pathname=decodeURIComponent(url.pathname);
 return url.hostname&&url.hostname!=='localhost'?'//'+url.hostname+pathname:pathname.replace(/^\/([A-Za-z]:\/)/,'$1');
}
export function relativeSource(file,root,platform){
 file=sourcePath(file,platform);
 const clean=value=>value.replaceAll('\\','/').replace(/\/$/,'');
 // Rust #[path] modules retain internal ../ aliases in Windows LCOV. Resolve
 // those lexically before checking containment; never resolve paths on this host.
 const source=path.posix.normalize(clean(file)),base=path.posix.normalize(clean(root)),prefix=base+'/';
 if(!(platform==='win32'?source.toLowerCase().startsWith(prefix.toLowerCase()):source.startsWith(prefix)))throw new Error('Source lies outside recorded checkout');
 const rel=source.slice(prefix.length);
 if(rel.split('/').some(p=>!p||p==='.'||p==='..')||rel.includes(':'))throw new Error('Unsafe normalized source path');
 return rel;
}
export function lcovLines(text){
 const files=[];let file=null;
 for(const line of text.split(/\r?\n/)){
  if(line.startsWith('SF:')){if(file)throw new Error('Unclosed LCOV file');file={path:line.slice(3),lines:[]};}
  else if(line.startsWith('DA:')){
   if(!file)throw new Error('LCOV line without file');
   const fields=line.slice(3).split(',');
   if(!/^[1-9][0-9]*$/.test(fields[0])||!/^\d+$/.test(fields[1]??''))throw new Error('Invalid LCOV count');
   const entry=fields.slice(0,2).map(Number);if(entry.some(n=>!Number.isSafeInteger(n)))throw new Error('LCOV integer overflow');file.lines.push(entry);
  }else if(line==='end_of_record'){if(!file)throw new Error('LCOV terminal without file');files.push(file);file=null;}
 }
 if(file)throw new Error('Truncated LCOV report');return files;
}
export function istanbulLines(report){
 return Object.values(report).map(file=>{
  const lines=new Map();
  for(const [id,statement] of Object.entries(file.statementMap)){
   const line=statement.start.line,count=file.s[id];
   if(!Number.isSafeInteger(line)||line<=0||!Number.isSafeInteger(count)||count<0)throw new Error('Invalid Istanbul statement');
   lines.set(line,Math.max(lines.get(line)??0,count));
  }
  return {path:file.path,lines:[...lines].sort(([a],[b])=>a-b)};
 });
}
export function verdict(inventory,receipts,inventorySha){
 if(inventory.format_version!==1||!/^[a-f0-9]{40}$/.test(inventory.source_commit))throw new Error('Invalid source inventory');
 const indexed=new Map();
 for(const file of inventory.files){
  if(indexed.has(file.path)||file.path.startsWith('/')||file.path.split('/').some(p=>p==='.'||p==='..'||!p))throw new Error('Duplicate/unsafe inventory path');
  if(!['eligible','declaration_only','excluded'].includes(file.disposition)||!['rust','typescript'].includes(file.language)||!Number.isSafeInteger(file.line_count)||file.line_count<1||!Array.isArray(file.test_line_ranges)||!/^[a-f0-9]{64}$/.test(file.source_sha256))throw new Error('Invalid inventory entry');
  indexed.set(file.path,file);
 }
 const lines=new Map(),seen=new Set(),platforms=new Set(),errors=[];
 for(const receipt of receipts){
  if(receipt.format_version!==1||receipt.source_commit!==inventory.source_commit||receipt.inventory_sha256!==inventorySha)throw new Error('Coverage commit/inventory mismatch');
  if(!['linux','win32'].includes(receipt.platform)||typeof receipt.source_root!=='string'||!(/^(?:[A-Za-z]:[\\/]|\/)/.test(receipt.source_root)))throw new Error('Invalid native coverage platform/root');
  if(receipt.tests_passed!==true||receipt.clean_source!==true||!Array.isArray(receipt.incomplete_profiles)||receipt.incomplete_profiles.length)throw new Error('Incomplete or dirty coverage run');
  if(receipt.kind==='rust-llvm'){
   if(receipt.tool_version!=='0.9.1'||receipt.rust_llvm_version!==receipt.llvm_tools_version||!receipt.rust_llvm_version)throw new Error('Unpinned or mismatched Rust coverage tools');
   if(!receipt.files.some(file=>file.lines?.some(([line,count])=>Number.isSafeInteger(line)&&Number.isSafeInteger(count)&&count>0)))throw new Error('Native report has no executable first-party evidence');
   const entrypoints=receipt.platform==='win32'?['crates/cli/src/main.rs','apps/desktop/src-tauri/src/main.rs']:['crates/cli/src/main.rs'];
   for(const required of entrypoints){
    const declared=indexed.get(required);
    if(declared?.disposition==='eligible'&&!receipt.files.some(file=>file.path===required&&file.lines?.some(([line,count])=>count>0&&!declared.test_line_ranges.some(([start,end])=>line>=start&&line<=end))))errors.push('Native entrypoint has no execution evidence: '+required);
   }
   platforms.add(receipt.platform);
  }else if(['typescript-v8','typescript-config-v8'].includes(receipt.kind)){
   if(receipt.tool_version!=='5.0.3')throw new Error('Unpinned TypeScript coverage tools');
  }else throw new Error('Unknown coverage collector');
  for(const file of receipt.files){
   const declared=indexed.get(file.path);
   if(!declared||declared.disposition==='excluded'||file.source_sha256!==declared.source_sha256)throw new Error('Undeclared or altered coverage source');
   if((receipt.kind==='rust-llvm')!==(declared.language==='rust'))throw new Error('Coverage language mismatch');
   if(!Array.isArray(file.lines))throw new Error('Missing executable lines');
   seen.add(file.path);const combined=lines.get(file.path)??new Map();
   for(const [line,count] of file.lines){
    if(!Number.isSafeInteger(line)||line<=0||!Number.isSafeInteger(count)||count<0||line>declared.line_count)throw new Error('Invalid coverage line/count');
    if(declared.test_line_ranges.some(([start,end])=>line>=start&&line<=end))continue;
    combined.set(line,(combined.get(line)??false)||count>0);
   }
   lines.set(file.path,combined);
  }
 }
 if(!platforms.has('linux')||!platforms.has('win32'))errors.push('Native Rust Linux and Windows reports are required');
 const files=inventory.files.filter(f=>f.disposition!=='excluded').map(file=>{
  const executable=lines.get(file.path)??new Map(),covered=[...executable.values()].filter(Boolean).length;
  if(file.disposition==='declaration_only'){
   if(executable.size)throw new Error('Declaration-only inventory contains executable lines');
   return {path:file.path,disposition:'declaration_only',executable:0,covered:0,passes:true};
  }
  const present=seen.has(file.path)&&executable.size>0;
  return {path:file.path,disposition:present?'measured':'missing',executable:executable.size,covered,passes:present&&covered*100>=executable.size*90};
 });
 return {format_version:1,source_commit:inventory.source_commit,threshold:{covered_numerator:90,executable_denominator:100,rounding:false},passes:errors.length===0&&files.every(f=>f.passes),errors,files};
}
export function normalizeRaw(records,receipt,inventory){
 const known=new Map(inventory.files.map(file=>[receipt.platform==='win32'?file.path.toLowerCase():file.path,file]));
 const prefix=receipt.source_root.replaceAll('\\','/').replace(/\/$/,'')+'/';
 if(known.size!==inventory.files.length)throw new Error('Ambiguous Windows source case');
 const result=[];
 for(const record of records){
  const raw=sourcePath(record.path,receipt.platform);
  const lexical=raw.replaceAll('\\','/'),normalized=path.posix.normalize(lexical);
  const underRoot=value=>receipt.platform==='win32'?value.toLowerCase().startsWith(prefix.toLowerCase()):value.startsWith(prefix);
  if(!underRoot(normalized)){
   if(underRoot(lexical))throw new Error('Source lies outside recorded checkout');
   continue;
  }
  const rel=relativeSource(record.path,receipt.source_root,receipt.platform);
  if(!/\.(rs|ts|tsx)$/.test(rel))continue;
  const entry=known.get(receipt.platform==='win32'?rel.toLowerCase():rel);
  if(!entry){
   // LLVM on Windows anchors rustc's virtual /rustc/<commit>/library paths
   // under the checkout drive. These are compiler standard-library sources,
   // not repository sources. Known inventory files always take precedence.
   if(/^rustc\/[a-f0-9]{40}\/library\//.test(rel)&&receipt.kind==='rust-llvm')continue;
   if(rel.split('/').some(p=>['target','node_modules','vendor'].includes(p)))continue;
   throw new Error('Source missing from inventory');
  }
  if(entry.disposition==='excluded')continue;
  if(!receipt.source_hashes || !Object.hasOwn(receipt.source_hashes,entry.path))throw new Error('Missing independent collector source hash');
  result.push({path:entry.path,source_sha256:receipt.source_hashes[entry.path],lines:record.lines});
 }
 return result;
}
function readBounded(file,max){
 const before=fs.lstatSync(file);
 if(!before.isFile()||before.isSymbolicLink()||before.size>max)throw new Error('Coverage input is not a bounded regular file');
 const fd=fs.openSync(file,fs.constants.O_RDONLY|(fs.constants.O_NOFOLLOW??0)|(fs.constants.O_NONBLOCK??0));
 try{
  const stat=fs.fstatSync(fd);
  if(!stat.isFile()||stat.dev!==before.dev||stat.ino!==before.ino||stat.size!==before.size)throw new Error('Coverage input changed during read');
  // Read at most the checked size plus one byte, including if the file grows.
  const bytes=Buffer.alloc(stat.size+1);let used=0,count;
  while(used<bytes.length&&(count=fs.readSync(fd,bytes,used,bytes.length-used,null))>0)used+=count;
  if(used!==stat.size)throw new Error('Coverage input changed during read');
  return bytes.subarray(0,used);
 }finally{fs.closeSync(fd);}
}
function readJson(file,max=64*1024*1024){return JSON.parse(readBounded(file,max).toString('utf8'));}
function rawReportPath(receiptFile,relative){
 const root=path.dirname(path.resolve(receiptFile));let current=root;
 for(const component of ['',...relative.split('/').slice(0,-1)]){
  current=path.join(current,component);
  const stat=fs.lstatSync(current);
  if(stat.isSymbolicLink()||!stat.isDirectory())throw new Error('Raw coverage report directory is linked or not a directory');
 }
 return path.join(root,relative);
}
export function runGate(inventoryFile,receiptFiles,output){
 if(fs.existsSync(output))throw new Error('Coverage verdict output already exists');
 const inventoryBytes=readBounded(inventoryFile,64*1024*1024),inventory=JSON.parse(inventoryBytes.toString('utf8'));
 const receipts=receiptFiles.map(file=>{
  const receipt=readJson(file);
  const records=[];
  if(!Array.isArray(receipt.raw_reports)||!receipt.raw_reports.length)throw new Error('Raw coverage reports are required');
  const required={ 'rust-llvm':['lcov'], 'typescript-v8':['istanbul','test-results'], 'typescript-config-v8':['istanbul','v8'] }[receipt.kind];
  if(!required||receipt.raw_reports.length!==required.length||new Set(receipt.raw_reports.map(report=>report.path)).size!==required.length||required.some(format=>receipt.raw_reports.filter(report=>report.format===format).length!==1))throw new Error('Required raw coverage evidence is missing or duplicated');
  for(const report of receipt.raw_reports){
   if(typeof report.path!=='string'||!/^[A-Za-z0-9_@.+/-]+$/.test(report.path)||report.path.split(/[\\/]/).some(p=>!p||p==='.'||p==='..')||path.isAbsolute(report.path))throw new Error('Unsafe raw report path');
   const full=rawReportPath(file,report.path);
   const bytes=readBounded(full,256*1024*1024);
   if(hash(bytes)!==report.sha256)throw new Error('Missing/altered raw coverage report');
   if(report.format==='lcov' && receipt.kind==='rust-llvm')records.push(...lcovLines(bytes.toString()));
   else if(report.format==='istanbul' && ['typescript-v8','typescript-config-v8'].includes(receipt.kind))records.push(...istanbulLines(JSON.parse(bytes)));
   else if(report.format==='v8'&&receipt.kind==='typescript-config-v8'){if(!Array.isArray(JSON.parse(bytes).result))throw new Error('Missing raw V8 scripts');}
   else if(report.format==='test-results'&&receipt.kind==='typescript-v8'){const tests=JSON.parse(bytes);if(tests.numFailedTests!==0||tests.numFailedTestSuites!==0||!(tests.numTotalTests>0))throw new Error('TypeScript tests failed');}
   else throw new Error('Unexpected raw coverage format');
  }
  // Reconstruct counts from raw reports; never trust a claimed per-file summary.
  return {...receipt,files:normalizeRaw(records,receipt,inventory)};
 });
 const result=verdict(inventory,receipts,hash(inventoryBytes));fs.writeFileSync(output,JSON.stringify(result,null,2)+'\n',{flag:'wx'});return result;
}
if(process.argv[1]&&path.resolve(process.argv[1])===fileURLToPath(import.meta.url)){
 try{if(process.argv.length<5)throw new Error('Usage: gate.mjs <inventory.json> <new-verdict.json> <receipt.json>...');const result=runGate(process.argv[2],process.argv.slice(4),process.argv[3]);console.log(JSON.stringify({passes:result.passes,missing:result.files.filter(f=>f.disposition==='missing').length,below_threshold:result.files.filter(f=>!f.passes).length}));if(!result.passes)process.exitCode=1;}
 catch(error){console.error(error.message);process.exitCode=1;}
}
