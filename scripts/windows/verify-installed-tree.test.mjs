import { test } from 'node:test';
import assert from 'node:assert/strict';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { createHash } from 'node:crypto';
import { verifyInstalledTree } from './verify-installed-tree.mjs';

test('installed inventory binds bytes and refuses missing, extra, altered and linked resources', () => {
  const root = fs.mkdtempSync(path.join(os.tmpdir(), 'anav-installed-'));
  try {
    const repo = path.join(root, 'source'), installed = path.join(root, 'installed');
    const config = path.join(repo, 'apps/desktop/src-tauri');
    fs.mkdirSync(config, { recursive: true });
    fs.mkdirSync(path.join(config, 'resources'));
    fs.mkdirSync(path.join(installed, 'notices'), { recursive: true });
    fs.writeFileSync(path.join(config, 'tauri.windows.conf.json'), JSON.stringify({ bundle: { resources: { 'resources/': 'notices/' } } }));
    fs.writeFileSync(path.join(config, 'resources/LICENSE'), 'retained notice');
    const notice = path.join(installed, 'notices/LICENSE');
    fs.writeFileSync(notice, 'retained notice');
    for (const binary of ['azeroth-desktop.exe', 'uninstall.exe']) fs.writeFileSync(path.join(installed, binary), 'generated');
    assert.equal(verifyInstalledTree(repo, installed).filter(f => f.source_verified).length, 1);
    const installedLink = path.join(root, 'installed-link');
    fs.symlinkSync(installed, installedLink, 'junction');
    assert.throws(() => verifyInstalledTree(repo, installedLink), /Linked/);
    fs.unlinkSync(installedLink);
    const sourceDirectory = path.join(config, 'resources');
    const moved = path.join(config, 'actual-resources');
    fs.renameSync(sourceDirectory, moved);
    fs.symlinkSync(moved, sourceDirectory, 'junction');
    assert.throws(() => verifyInstalledTree(repo, installed), /Linked/);
    fs.unlinkSync(sourceDirectory);
    fs.renameSync(moved, sourceDirectory);
    fs.writeFileSync(notice, 'altered');
    assert.throws(() => verifyInstalledTree(repo, installed), /differs/);
    fs.unlinkSync(notice);
    assert.throws(() => verifyInstalledTree(repo, installed), /inventory/);
    fs.writeFileSync(notice, 'retained notice');
    const extra = path.join(installed, 'private-export.txt');
    fs.writeFileSync(extra, 'must not ship');
    assert.throws(() => verifyInstalledTree(repo, installed), /inventory/);
    fs.unlinkSync(extra);
    // Directory links are available without Windows developer-mode file symlinks.
    fs.symlinkSync(path.join(config, 'resources'), path.join(installed, 'linked'), 'junction');
    assert.throws(() => verifyInstalledTree(repo, installed), /Linked/);
  } finally { fs.rmSync(root, { recursive: true, force: true }); }
});

test('schema4 installed bundles bind every resource to explicit source pins', () => {
  const root = fs.mkdtempSync(path.join(os.tmpdir(), 'anav-installed-bundle-'));
  try {
    const repo = path.join(root, 'source'), installed = path.join(root, 'installed');
    const write = (base, relative, bytes) => { const file = path.join(base, relative); fs.mkdirSync(path.dirname(file), { recursive: true }); fs.writeFileSync(file, bytes); return file; };
    write(repo, 'apps/desktop/src-tauri/tauri.windows.conf.json', JSON.stringify({ bundle: { resources: {} } }));
    for (const binary of ['azeroth-desktop.exe', 'uninstall.exe']) write(installed, binary, 'generated');
    const pin = relative => { const raw = fs.readFileSync(path.join(repo, relative)); return { path: relative, bytes: raw.length, sha256: createHash('sha256').update(raw).digest('hex') }; };
    write(repo, 'content/raw-source.txt', 'exact original source bytes');
    write(repo, 'docs/public-review.json', 'Synthetic public review, not a real acceptance');
    write(repo, 'release/v1/bundle-receipt.json', 'Synthetic bytes: cryptography is tested by the Rust receipt verifier');
    const manifest = { format: 'navzeroth-installed-content', schema_version: 1, review: pin('docs/public-review.json'), files: [pin('content/raw-source.txt'), pin('docs/public-review.json')] };
    write(repo, 'release/v1/bundle-manifest.json', JSON.stringify(manifest));
    const evidence = { schema_version: 4, bundled_content: { manifest: pin('release/v1/bundle-manifest.json'), receipt: pin('release/v1/bundle-receipt.json'), review: manifest.review } };
    write(repo, 'release/v1/acceptance-evidence.json', JSON.stringify(evidence));
    for (const resource of manifest.files) write(installed, 'bundled-content/' + resource.path, fs.readFileSync(path.join(repo, resource.path)));
    write(installed, 'bundled-content/manifest.json', fs.readFileSync(path.join(repo, evidence.bundled_content.manifest.path)));
    write(installed, 'bundled-content/receipt.json', fs.readFileSync(path.join(repo, evidence.bundled_content.receipt.path)));
    assert.equal(verifyInstalledTree(repo, installed).filter(f => f.source_verified).length, 4);
    // A second pathname read must never substitute unverified parsed manifest
    // bytes after the descriptor's pinned digest has passed.
    write(repo, 'content/undeclared.txt', 'not declared by pinned manifest');
    write(installed, 'bundled-content/content/undeclared.txt', 'not declared by pinned manifest');
    const substituted = structuredClone(manifest);
    substituted.files.push(pin('content/undeclared.txt'));
    const originalRead = fs.readFileSync;
    let unverifiedReopens = 0;
    fs.readFileSync = function(file, ...args) {
      if (String(file) === path.join(repo, evidence.bundled_content.manifest.path)) {
        unverifiedReopens++;
        return JSON.stringify(substituted);
      }
      return originalRead.call(this, file, ...args);
    };
    try {
      assert.throws(() => verifyInstalledTree(repo, installed), /inventory/);
      assert.equal(unverifiedReopens, 0);
    } finally { fs.readFileSync = originalRead; }
    fs.rmSync(path.join(installed, 'bundled-content/content/undeclared.txt'));

    write(installed, 'bundled-content/content/raw-source.txt', 'altered');
    assert.throws(() => verifyInstalledTree(repo, installed), /differs/);
    write(installed, 'bundled-content/content/raw-source.txt', 'exact original source bytes');
    write(installed, 'bundled-content/undeclared.txt', 'not permitted');
    assert.throws(() => verifyInstalledTree(repo, installed), /inventory/);
    fs.rmSync(path.join(installed, 'bundled-content/undeclared.txt'));
    write(repo, 'content/raw-source.txt', 'altered source pin');
    assert.throws(() => verifyInstalledTree(repo, installed), /source differs/);
    write(repo, 'content/raw-source.txt', 'exact original source bytes');
    manifest.files.push(manifest.files[0]);
    write(repo, 'release/v1/bundle-manifest.json', JSON.stringify(manifest));
    evidence.bundled_content.manifest = pin('release/v1/bundle-manifest.json');
    write(repo, 'release/v1/acceptance-evidence.json', JSON.stringify(evidence));
    assert.throws(() => verifyInstalledTree(repo, installed), /Duplicate/);
  } finally { fs.rmSync(root, { recursive: true, force: true }); }
});
