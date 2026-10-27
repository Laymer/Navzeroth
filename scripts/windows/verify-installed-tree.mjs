// Exact resource inventory for disposable hosted release verification.
import fs from 'node:fs';
import path from 'node:path';
import { createHash } from 'node:crypto';
import { fileURLToPath } from 'node:url';

const MAX_FILE = 576 * 1024 * 1024;
const identity = stat => [stat.dev, stat.ino, stat.size, stat.mtimeNs, stat.ctimeNs].join(':');
function noLinkedComponents(file) {
  const absolute = path.resolve(file), parsed = path.parse(absolute);
  let current = parsed.root;
  for (const component of absolute.slice(parsed.root.length).split(path.sep).filter(Boolean)) {
    current = path.join(current, component);
    if (fs.lstatSync(current).isSymbolicLink()) throw new Error('Linked resource component refused');
  }
}
function hash(file, maximum = MAX_FILE, capture = false) {
  noLinkedComponents(file);
  const before = fs.lstatSync(file, { bigint: true });
  if (!before.isFile() || before.isSymbolicLink() || before.size > BigInt(maximum)) throw new Error('Invalid bounded source/resource file');
  const descriptor = fs.openSync(file, fs.constants.O_RDONLY | (fs.constants.O_NOFOLLOW ?? 0) | (fs.constants.O_NONBLOCK ?? 0));
  try {
    if (identity(fs.fstatSync(descriptor, { bigint: true })) !== identity(before)) throw new Error('Resource changed before reading');
    const digest = createHash('sha256'), buffer = Buffer.alloc(65536), chunks = [];
    let count = 0, length;
    while ((length = fs.readSync(descriptor, buffer, 0, buffer.length, null)) > 0) {
      count += length;
      if (count > maximum) throw new Error('Resource exceeds bound');
      if (capture) chunks.push(Buffer.from(buffer.subarray(0, length)));
      digest.update(buffer.subarray(0, length));
    }
    if (BigInt(count) !== before.size || identity(fs.fstatSync(descriptor, { bigint: true })) !== identity(before)
      || identity(fs.lstatSync(file, { bigint: true })) !== identity(before)) throw new Error('Resource changed while reading');
    const sha256 = digest.digest('hex');
    return capture ? { sha256, bytes: Buffer.concat(chunks) } : sha256;
  } finally { fs.closeSync(descriptor); }
}
function files(root, prefix = '', depth = 0, budget = { entries: 0 }) {
  noLinkedComponents(root);
  if (!fs.lstatSync(root).isDirectory()) throw new Error('Invalid resource directory');
  if (depth > 32) throw new Error('Resource tree exceeds depth bound');
  return fs.readdirSync(root).sort().flatMap(name => {
    if (++budget.entries > 32768) throw new Error('Resource tree exceeds entry bound');
    const relative = prefix + name;
    const stat = fs.lstatSync(path.join(root, name));
    if (stat.isSymbolicLink()) throw new Error('Linked resource refused');
    if (stat.isDirectory()) return files(path.join(root, name), relative + '/', depth + 1, budget);
    if (!stat.isFile()) throw new Error('Non-file resource refused');
    return [relative];
  });
}
function safe(value) {
  if (!value || value.includes('\\') || value.startsWith('/') || value.split('/').some(p => !p || p === '.' || p === '..')) throw new Error('Unsafe resource destination');
  return value;
}
function installedBundle(repo, add) {
  const evidencePath = path.join(repo, 'release/v1/acceptance-evidence.json');
  if (!fs.existsSync(evidencePath)) return;
  noLinkedComponents(evidencePath);
  const evidence = JSON.parse(hash(evidencePath, 64 * 1024 * 1024, true).bytes.toString('utf8'));
  if (![4, 5].includes(evidence.schema_version) || !evidence.bundled_content) return;
  const binding = evidence.bundled_content;
  const source = (ref, maximum = MAX_FILE, capture = false) => {
    if (!ref || !/^[a-z0-9_./-]+$/.test(ref.path) || !/^[a-f0-9]{64}$/.test(ref.sha256)
      || !Number.isSafeInteger(ref.bytes) || ref.bytes <= 0 || ref.bytes > maximum) throw new Error('Invalid bundle resource pin');
    safe(ref.path);
    const file = path.join(repo, ref.path);
    const verified = hash(file, maximum, capture);
    if (fs.lstatSync(file).size !== ref.bytes || (capture ? verified.sha256 : verified) !== ref.sha256) throw new Error('Bundle source differs from release evidence');
    return capture ? verified.bytes : file;
  };
  if (binding.manifest.bytes > 4 * 1024 * 1024 || binding.receipt.bytes > 65536) throw new Error('Bundle document exceeds bound');
  const manifest = JSON.parse(source(binding.manifest, 4 * 1024 * 1024, true).toString('utf8'));
  source(binding.receipt);
  const same = (a, b) => a && b && a.path === b.path && a.bytes === b.bytes && a.sha256 === b.sha256;
  if (manifest.format !== 'navzeroth-installed-content' || manifest.schema_version !== 1 || !Array.isArray(manifest.files)
    || !manifest.files.length || manifest.files.length > 8190 || !same(manifest.review, binding.review)
    || !manifest.files.some(ref => same(ref, binding.review))) throw new Error('Invalid source-bound installed bundle');
  add('bundled-content/manifest.json', source(binding.manifest));
  add('bundled-content/receipt.json', source(binding.receipt));
  let bytes = 0;
  for (const ref of manifest.files) {
    if (['manifest.json', 'receipt.json'].includes(ref.path)) throw new Error('Reserved bundle resource path');
    bytes += ref.bytes;
    if (bytes > 2 * 1024 * 1024 * 1024) throw new Error('Bundle resources exceed bound');
    add('bundled-content/' + ref.path, source(ref));
  }
}
export function verifyInstalledTree(repo, installed) {
  const configDir = path.join(repo, 'apps/desktop/src-tauri');
  noLinkedComponents(path.join(configDir, 'tauri.windows.conf.json'));
  noLinkedComponents(installed);
  const resources = JSON.parse(hash(path.join(configDir, 'tauri.windows.conf.json'), 4 * 1024 * 1024, true).bytes.toString('utf8')).bundle.resources;
  const expected = new Map();
  const add = (relative, file) => {
    safe(relative);
    if (expected.has(relative)) throw new Error('Duplicate resource destination');
    expected.set(relative, hash(file));
  };
  installedBundle(repo, add);
  for (const [source, destination] of Object.entries(resources)) {
    const from = path.resolve(configDir, source);
    const stat = fs.lstatSync(from);
    if (stat.isSymbolicLink()) throw new Error('Linked source refused');
    if (stat.isDirectory()) {
      if (!destination.endsWith('/')) throw new Error('Directory destination must end with slash');
      for (const file of files(from)) add(destination + file, path.join(from, file));
    } else if (stat.isFile()) add(destination, from);
    else throw new Error('Non-file source refused');
  }
  const generated = ['azeroth-desktop.exe', 'uninstall.exe'];
  const actual = files(installed);
  const names = [...expected.keys(), ...generated].sort();
  if (JSON.stringify(actual.sort()) !== JSON.stringify(names)) throw new Error('Installed file inventory differs from source resources and generated binaries');
  const receipt = actual.map(relative => {
    const sha256 = hash(path.join(installed, relative));
    if (expected.has(relative) && sha256 !== expected.get(relative)) throw new Error('Installed resource differs: ' + relative);
    return { path: relative, sha256, source_verified: expected.has(relative) };
  });
  return receipt;
}
if (process.argv[1] && path.resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  if (process.argv.length !== 4) throw new Error('Expected repository and installed directory');
  console.log(JSON.stringify(verifyInstalledTree(path.resolve(process.argv[2]), path.resolve(process.argv[3])), null, 2));
}
