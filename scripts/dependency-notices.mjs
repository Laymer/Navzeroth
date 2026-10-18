/** Deterministic notices for the Windows desktop resolved normal/build closure.
 * This deliberately includes build dependencies; it is not a runtime SBOM.
 * All network acquisition happens through locked Cargo/pnpm before this tool.
 */
import fs from 'node:fs';
import path from 'node:path';
import crypto from 'node:crypto';
import { gunzipSync } from 'node:zlib';
import { execFileSync } from 'node:child_process';
import { createRequire } from 'node:module';
import { fileURLToPath } from 'node:url';

export const digest = bytes => crypto.createHash('sha256').update(bytes).digest('hex');
export function safeRelative(value) {
  if (typeof value !== 'string' || !/^[A-Za-z0-9_@.+/-]+$/.test(value) || value.split('/').some(p => !p || p === '.' || p === '..') || value.startsWith('/')) throw new Error('Invalid notice path');
  return value;
}
export function closure(metadata, root) {
  const nodes = new Map(metadata.resolve.nodes.map(n => [n.id, n]));
  const seen = new Set();
  function visit(id) {
    if (seen.has(id)) return;
    seen.add(id);
    const node = nodes.get(id);
    if (!node) throw new Error('Missing resolved node');
    for (const dep of node.deps) if (dep.dep_kinds.some(k => k.kind !== 'dev')) visit(dep.pkg);
  }
  visit(root);
  return metadata.packages.filter(p => seen.has(p.id) && !metadata.workspace_members.includes(p.id));
}
function regular(file, max = 2 * 1024 * 1024) {
  const s = fs.lstatSync(file);
  if (!s.isFile() || s.isSymbolicLink() || s.size > max) throw new Error('Notice is not a bounded regular file');
  return fs.readFileSync(file);
}
function filesBelow(dir, prefix = '') {
  return fs.readdirSync(dir).sort().flatMap(name => {
    const rel = prefix + name;
    safeRelative(rel);
    const s = fs.lstatSync(path.join(dir, name));
    if (s.isSymbolicLink()) throw new Error('Symbolic link in notices');
    return s.isDirectory() ? filesBelow(path.join(dir, name), rel + '/') : [rel];
  });
}
export function verifyTree(dir, expected) {
  const actual = filesBelow(dir).filter(p => p !== 'DEPENDENCIES.json');
  const names = [...expected.keys()].sort();
  if (JSON.stringify(actual.sort()) !== JSON.stringify(names)) throw new Error('Missing or extra packaged notice');
  for (const name of names) if (!regular(path.join(dir, name), 16 * 1024 * 1024).equals(expected.get(name))) throw new Error('Altered notice: ' + name);
}
export function crateFiles(bytes, expected) {
  if (digest(bytes) !== expected) throw new Error('Crate archive checksum differs from lock');
  const tar = gunzipSync(bytes, { maxOutputLength:128*1024*1024 });
  const files = new Map();
  let longName=null;
  for (let offset=0, count=0; offset+512<=tar.length; count++) {
    if (count>20000) throw new Error('Crate entry quota');
    const header=tar.subarray(offset,offset+512);
    if (header.every(b=>b===0)) break;
    const str=(start,end)=>header.subarray(start,end).toString('utf8').replace(/\0.*$/s,'');
    const size=parseInt(str(124,136).trim(),8);
    const stored=parseInt(str(148,156).trim(),8);
    const sum=header.reduce((sum,b,i)=>sum+(i>=148&&i<156?32:b),0);
    if (stored!==sum || !Number.isSafeInteger(size) || size<0 || offset+512+size>tar.length) throw new Error('Malformed crate archive');
    const name=longName ?? ((str(345,500)?str(345,500)+'/':'')+str(0,100));
    longName=null;
    const kind=str(156,157);
    if (kind==='' || kind==='0') {
      if (name.startsWith('/') || name.includes('\\') || name.split('/').some(p=>p==='..'||p==='.'||!p)) throw new Error('Unsafe crate member');
      if (files.has(name)) throw new Error('Duplicate crate member');
      files.set(name,tar.subarray(offset+512,offset+512+size));
    } else if (kind==='L') {
      longName=tar.subarray(offset+512,offset+512+size).toString('utf8').replace(/\0.*$/s,'');
    } else if (kind!=='5') throw new Error('Unsupported crate archive member: '+kind);
    offset+=512+Math.ceil(size/512)*512;
  }
  return files;
}
function generate(repo) {
  const output = new Map();
  const put = (name, bytes) => {
    safeRelative(name);
    if (output.has(name)) throw new Error('Duplicate notice destination');
    output.set(name, bytes);
  };
  const read = file => regular(path.join(repo, file), 16 * 1024 * 1024);
  const configBytes = read('config/dependency-notice-overrides.json');
  const config = JSON.parse(configBytes);
  if (config.format_version !== 1) throw new Error('Unknown override format');
  const retained = file => {
    const data = read('config/dependency-notices/' + safeRelative(file.file));
    if (digest(data) !== file.sha256) throw new Error('Altered retained notice');
    return data;
  };
  const metadata = JSON.parse(execFileSync('cargo', ['metadata', '--locked', '--format-version', '1', '--filter-platform', 'x86_64-pc-windows-msvc'], { cwd: repo, maxBuffer: 32 * 1024 * 1024 }));
  const root = metadata.packages.find(p => p.name === 'azeroth-desktop');
  if (!root) throw new Error('Missing desktop root');
  const rust = closure(metadata, root.id).sort((a,b) => (a.name+'@'+a.version < b.name+'@'+b.version ? -1 : a.name+'@'+a.version > b.name+'@'+b.version ? 1 : 0));
  const cargoLock = read('Cargo.lock').toString();
  const lockChecksums = new Map(cargoLock.split('[[package]]').slice(1).map(s => {
    const name = /^name = "([^"]+)"$/m.exec(s)?.[1];
    const version = /^version = "([^"]+)"$/m.exec(s)?.[1];
    return [name+'@'+version, /^checksum = "([a-f0-9]{64})"$/m.exec(s)?.[1]];
  }));
  const packages = [];
  for (const p of rust) {
    const id = p.name+'@'+p.version;
    if (!p.license) throw new Error('Missing declared license: ' + id);
    const dir = path.dirname(p.manifest_path);
    const registry = p.source?.startsWith('registry+');
    if (!registry && path.resolve(dir) !== path.join(repo, 'vendor/wowdbdefs-rs')) throw new Error('Unreviewed non-registry dependency');
    const checksum = registry ? lockChecksums.get(id) : null;
    if (registry && !checksum) throw new Error('Missing locked registry checksum');
    const cache = registry ? path.join(path.dirname(path.dirname(path.dirname(dir))), 'cache', path.basename(path.dirname(dir)), path.basename(dir)+'.crate') : null;
    const archive = registry ? regular(cache,16*1024*1024) : null;
    const members = registry ? crateFiles(archive,checksum) : null;
    if (registry && !regular(p.manifest_path).equals(members.get(path.basename(dir)+'/Cargo.toml') ?? Buffer.alloc(0))) throw new Error('Modified registry manifest');
    const names = fs.readdirSync(dir).filter(n => /^(licen[cs]e|copying|notice|copyright|authors)([._-]|$)/i.test(n) && fs.lstatSync(path.join(dir,n)).isFile()).sort();
    const notices = [];
    for (const name of names) {
      const bytes = regular(path.join(dir, name));
      if (registry && !bytes.equals(members.get(path.basename(dir)+'/'+name) ?? Buffer.alloc(0))) throw new Error('Modified registry notice');
      const dest = 'rust/'+p.name+'-'+p.version+'/'+name;
      put(dest, bytes); notices.push(dest);
    }
    for (const entry of config.overrides.filter(e => e.packages.includes(id))) {
      const dest = 'rust/'+p.name+'-'+p.version+'/'+entry.file;
      put(dest, retained(entry)); notices.push(dest);
    }
    if (!notices.length) throw new Error('No retained license: ' + id);
    let source = null;
    if (config.source_archive_licenses.some(license => p.license.split(/\s+(?:OR|AND)\s+/).includes(license))) {
      if (!registry) throw new Error('Source archive requires reviewed registry');
      source = 'sources/'+p.name+'-'+p.version+'.crate'; put(source, archive);
    }
    packages.push({ ecosystem: 'cargo', name:p.name, version:p.version, license:p.license, registry_checksum:checksum, repository:p.repository, notices, retained_source:source });
  }
  const jsSeen = new Set();
  function jsPackage(name, from) {
    const require = createRequire(path.join(from, 'package.json'));
    let main;
    try { main = require.resolve(name+'/package.json'); }
    catch { main = require.resolve(name); }
    let dir = path.dirname(main);
    while (!fs.existsSync(path.join(dir, 'package.json'))) {
      const up = path.dirname(dir); if (up === dir) throw new Error('Missing JS package'); dir = up;
    }
    const p = JSON.parse(regular(path.join(dir, 'package.json')));
    if (p.name !== name || !p.version || !p.license) throw new Error('Unexpected JS package identity/license');
    const id = name+'@'+p.version;
    if (jsSeen.has(id)) return;
    jsSeen.add(id);
    const entry=config.npm_archives.find(e=>e.name===name && e.version===p.version);
    if (!entry) throw new Error('Unreviewed npm archive: '+id);
    const bytes=retained(entry);
    const integrity='sha512-'+crypto.createHash('sha512').update(bytes).digest('base64');
    const lock=read('pnpm-lock.yaml').toString().split('\n');
    const index=lock.findIndex(line=>line==='  '+id+':' || line==="  '"+id+"':");
    if (index<0 || integrity!==entry.integrity || !lock[index+1]?.includes('integrity: '+integrity+'}')) throw new Error('npm archive differs from lock');
    const members=crateFiles(bytes,entry.sha256);
    if (!regular(path.join(dir,'package.json')).equals(members.get('package/package.json') ?? Buffer.alloc(0))) throw new Error('Modified npm manifest');
    const notices = [];
    for (const file of fs.readdirSync(dir).filter(n => /^(license|copying|notice)([._-]|$)/i.test(n)).sort()) {
      const dest = 'js/'+name.replaceAll('/', '_')+'-'+p.version+'/'+file;
      const bytes=regular(path.join(dir,file));
      if (!bytes.equals(members.get('package/'+file) ?? Buffer.alloc(0))) throw new Error('Modified npm notice');
      put(dest, bytes); notices.push(dest);
    }
    if (!notices.length) throw new Error('No JS license: '+id);
    packages.push({ ecosystem:'npm', name, version:p.version, license:p.license, notices });
    for (const dep of Object.keys(p.dependencies ?? {}).sort()) jsPackage(dep, dir);
    for (const dep of Object.keys(p.optionalDependencies ?? {}).sort()) jsPackage(dep, dir);
  }
  const desktop = path.join(repo,'apps/desktop');
  for (const dep of Object.keys(JSON.parse(regular(path.join(desktop, 'package.json'))).dependencies).sort()) jsPackage(dep, desktop);
  for (const file of config.webview2_sdk.files) put('webview2-sdk/'+file.file, retained(file));
  const sdkPackage = rust.find(p => p.name === 'webview2-com-sys' && p.version === '0.39.1');
  if (!sdkPackage) throw new Error('Unreviewed WebView2 crate version');
  for (const loader of config.webview2_sdk.loaders) {
    const bytes = regular(path.join(path.dirname(sdkPackage.manifest_path), safeRelative(loader.crate_path)),16*1024*1024);
    if (bytes.length !== loader.bytes || digest(bytes) !== loader.sha256) throw new Error('WebView2 SDK loader changed');
  }
  const compiler = execFileSync('rustc',['-vV'],{encoding:'utf8'});
  if (!compiler.includes('release: '+config.rust_stdlib.release+'\n')) throw new Error('Unreviewed Rust standard library version');
  if (!compiler.includes('commit-hash: '+config.rust_stdlib.compiler_commit+'\n')) throw new Error('Unreviewed Rust compiler commit');
  for (const file of config.rust_stdlib.files) put('rust-stdlib/'+file.file.replace(/^rust-stdlib\//,''), retained(file));
  for (const file of ['ED25519-LICENSE.txt','RFD-LICENSE.txt']) put(file, read('crates/local-data/notices/'+file));
  if (output.size > 2048 || [...output.values()].reduce((sum,b) => sum+b.length,0) > 32*1024*1024) throw new Error('Notice closure exceeds quota');
  const manifest = { format_version:1, target:'x86_64-pc-windows-msvc', scope:'Resolved desktop normal/build dependencies, including build tools and workspace feature unification; not a runtime-only SBOM. npm production dependencies. Rust standard library notices. Separate Microsoft SDK terms.', cargo_lock_sha256:digest(read('Cargo.lock')), pnpm_lock_sha256:digest(read('pnpm-lock.yaml')), overrides_sha256:digest(configBytes), packages, webview2_sdk:config.webview2_sdk, rust_stdlib:{release:config.rust_stdlib.release,compiler_commit:config.rust_stdlib.compiler_commit,notice_source:config.rust_stdlib.notice_source}, files:[...output.entries()].sort(([a],[b]) => a < b ? -1 : a > b ? 1 : 0).map(([name,bytes]) => ({path:name,bytes:bytes.length,sha256:digest(bytes)})) };
  return { output, manifest:Buffer.from(JSON.stringify(manifest,null,2)+'\n') };
}
export function verifyInstalled(repo, dir) {
  const {output,manifest}=generate(repo);
  const configDir=path.join(repo,'apps/desktop/src-tauri');
  const resources=JSON.parse(regular(path.join(configDir,'tauri.windows.conf.json'))).bundle.resources;
  for (const [source,dest] of Object.entries(resources)) {
    if (!dest.startsWith('local-data-notices/') || dest==='local-data-notices/') continue;
    const name=safeRelative(dest.slice('local-data-notices/'.length));
    if(output.has(name)) throw new Error('Colliding static notice');
    output.set(name,regular(path.resolve(configDir,source)));
  }
  verifyTree(dir,output);
  if (!regular(path.join(dir,'DEPENDENCIES.json')).equals(manifest)) throw new Error('Installed dependency manifest differs');
  console.log('Installed dependency notices and retained sources match exact source bytes.');
}
export function main(repo, mode) {
  if (!['--check','--write'].includes(mode)) throw new Error('Expected --check or --write');
  const dir = path.join(repo,'crates/local-data/notices');
  const {output, manifest} = generate(repo);
  if (mode === '--check') {
    verifyTree(dir, output);
    if (!regular(path.join(dir,'DEPENDENCIES.json')).equals(manifest)) throw new Error('Stale dependency manifest');
    console.log('Dependency notices and retained source closure verified ('+output.size+' files).');
    return;
  }
  // Refuse to remove any unrecognized existing file during regeneration.
  const oldManifest = path.join(dir,'DEPENDENCIES.json');
  const known = fs.existsSync(oldManifest) ? JSON.parse(regular(oldManifest)).files.map(f => safeRelative(f.path)) : ['ED25519-LICENSE.txt','RFD-LICENSE.txt'];
  const allowed = new Set([...known,'DEPENDENCIES.json']);
  for (const file of filesBelow(dir)) if (!allowed.has(file)) throw new Error('Unrecognized existing notice; preserve and review: '+file);
  const backup = dir+'.previous';
  if (fs.existsSync(backup)) throw new Error('Previous notice backup exists; inspect recovery');
  const stage = fs.mkdtempSync(dir+'.staging-');
  try {
    for (const [name,bytes] of output) { const dest=path.join(stage,name); fs.mkdirSync(path.dirname(dest),{recursive:true}); fs.writeFileSync(dest,bytes,{flag:'wx'}); }
    fs.writeFileSync(path.join(stage,'DEPENDENCIES.json'),manifest,{flag:'wx'});
    verifyTree(stage,output);
    fs.renameSync(dir,backup);
    try { fs.renameSync(stage,dir); } catch (error) { fs.renameSync(backup,dir); throw error; }
    fs.rmSync(backup,{recursive:true});
  } finally { if (fs.existsSync(stage)) fs.rmSync(stage,{recursive:true}); }
  console.log('Wrote deterministic dependency notices ('+output.size+' files).');
}
if (process.argv[1] && path.resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  try {
    const repo=path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');
    if(process.argv[2]==='--installed' && process.argv.length===4) verifyInstalled(repo,path.resolve(process.argv[3]));
    else if(process.argv.length===3) main(repo,process.argv[2]);
    else throw new Error('Usage: dependency-notices.mjs --check | --write | --installed <notice-directory>');
  }
  catch (error) { console.error(error.message); process.exitCode=1; }
}
