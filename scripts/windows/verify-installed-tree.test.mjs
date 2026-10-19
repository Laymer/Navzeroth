import { test } from 'node:test';
import assert from 'node:assert/strict';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
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
