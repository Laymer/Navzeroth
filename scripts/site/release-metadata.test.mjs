import test from 'node:test';
import assert from 'node:assert/strict';
import fs from 'node:fs';
import { validateRelease } from './release-metadata.mjs';
const policy = JSON.parse(fs.readFileSync(new URL('../../release/v1/artifact-policy.json', import.meta.url)));
const rc = JSON.parse(fs.readFileSync(new URL('../../docs/site/capture-release.json', import.meta.url)));
const clone = value => structuredClone(value);
test('preserves the verified historical RC identity', () => {
  assert.equal(validateRelease(rc, policy), rc);
  assert.equal(rc.source_commit, 'fcf48ea5b3c7ba79cc3603bd5005fe961bca60e3');
});
test('stable requires its complete material/source artifact set and unsigned policy', () => {
  // Synthetic metadata exercises the contract; no public stable evidence is claimed.
  const stable = clone(rc);
  stable.version = '1.0.0'; stable.tag = 'v1.0.0'; stable.prerelease = false;
  stable.assets = rc.assets.map(a => ({ ...a, name:a.name.replaceAll('1.0.0-rc.1', '1.0.0'), url:a.url.replaceAll('1.0.0-rc.1', '1.0.0') }));
  for (const name of ['AzerothNavigator-materials-1.0.0.zip', 'RELEASE_MATERIALS.json', 'RELEASE_ASSETS.json']) stable.assets.push({name,bytes:1,sha256:'a'.repeat(64),url:`https://github.com/Laymer/Navzeroth/releases/download/v1.0.0/${name}`});
  assert.equal(validateRelease(stable, policy), stable);
  for (const mutate of [x => x.assets.pop(), x => x.assets.push(x.assets[0]), x => x.prerelease = true, x => x.signature = 'signed']) {
    const altered = clone(stable); mutate(altered); assert.throws(() => validateRelease(altered, policy));
  }
});
test('refuses unverified identities, wrong versions, altered asset URLs and invalid pins', () => {
  for (const mutate of [x => x.schema_version = 2, x => x.version = '1.0.1', x => x.tag = 'v1.0.0', x => x.public_verified = false, x => x.source_commit = 'bad', x => x.application_sha256 = 'bad', x => x.public_verification_run_id = '', x => x.assets[0].bytes = 0, x => x.assets[0].sha256 = 'bad', x => x.assets[0].url += '?other', x => x.assets[0].name = '../setup.exe']) {
    const altered = clone(rc); mutate(altered); assert.throws(() => validateRelease(altered, policy));
  }
});

test('a historical RC capture cannot be relabelled as current stable', async () => {
  const { validateCaptureCaption } = await import('./release-metadata.mjs');
  const stable = { version:'1.0.0', tag:'v1.0.0' };
  validateCaptureCaption('Actual 1.0.0-rc.1 Windows app, synthetic fixture.', rc, rc);
  validateCaptureCaption('Historical 1.0.0-rc.1 Windows app, synthetic fixture.', stable, rc);
  assert.throws(() => validateCaptureCaption('Actual 1.0.0-rc.1 Windows app, synthetic fixture.', stable, rc));
  assert.throws(() => validateCaptureCaption('Historical 1.0.0 Windows app, synthetic fixture.', stable, rc));
  assert.throws(() => validateCaptureCaption('Actual 1.0.0-rc.1 Windows app, synthetic fixture.', stable, stable));
  assert.throws(() => validateCaptureCaption('Historical 1.0.0-rc.1 Windows app.', stable, rc));
});
