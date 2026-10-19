// Exact resource inventory for disposable hosted release verification.
import fs from 'node:fs';
import path from 'node:path';
import { createHash } from 'node:crypto';
import { fileURLToPath } from 'node:url';

const hash = file => createHash('sha256').update(fs.readFileSync(file)).digest('hex');
function files(root, prefix = '') {
  return fs.readdirSync(root).sort().flatMap(name => {
    const relative = prefix + name;
    const stat = fs.lstatSync(path.join(root, name));
    if (stat.isSymbolicLink()) throw new Error('Linked resource refused');
    if (stat.isDirectory()) return files(path.join(root, name), relative + '/');
    if (!stat.isFile()) throw new Error('Non-file resource refused');
    return [relative];
  });
}
function safe(value) {
  if (!value || value.includes('\\') || value.startsWith('/') || value.split('/').some(p => !p || p === '.' || p === '..')) throw new Error('Unsafe resource destination');
  return value;
}
export function verifyInstalledTree(repo, installed) {
  const configDir = path.join(repo, 'apps/desktop/src-tauri');
  const resources = JSON.parse(fs.readFileSync(path.join(configDir, 'tauri.windows.conf.json'), 'utf8')).bundle.resources;
  const expected = new Map();
  for (const [source, destination] of Object.entries(resources)) {
    const from = path.resolve(configDir, source);
    const stat = fs.lstatSync(from);
    if (stat.isSymbolicLink()) throw new Error('Linked source refused');
    const add = (relative, file) => {
      safe(relative);
      if (expected.has(relative)) throw new Error('Duplicate resource destination');
      expected.set(relative, hash(file));
    };
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
