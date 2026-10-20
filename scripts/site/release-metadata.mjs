import assert from 'node:assert/strict';

// Published website identities only. Adding another release requires a reviewed
// tooling change; publication of the already-supported stable needs only site data.
export function validateRelease(release, policy) {
  assert.equal(release.schema_version, 1);
  assert.ok(['1.0.0-rc.1', '1.0.0'].includes(release.version));
  const allowed = policy.releases.find(row => row.version === release.version);
  assert.equal(policy.schema_version, 1);
  assert.ok(allowed, 'Release needs an explicit source-bound artifact policy');
  assert.equal(release.tag, `v${release.version}`);
  assert.equal(release.prerelease, allowed.prerelease);
  assert.equal(release.signature, 'unsigned');
  assert.equal(release.signature, allowed.signature);
  assert.equal(release.public_verified, true);
  assert.match(release.source_commit, /^[a-f0-9]{40}$/);
  assert.match(release.application_sha256, /^[a-f0-9]{64}$/);
  assert.match(release.public_verification_run_id, /^\d+$/);
  const names = [`AzerothNavigator-${release.version}-x64-setup.exe`, `AzerothNavigator-addon-${release.version}.zip`, 'CANDIDATE_NOTES.md', 'SHA256SUMS.txt', 'SOURCE_COMMIT.txt'];
  if (release.version === '1.0.0') names.push(`AzerothNavigator-materials-${release.version}.zip`, 'RELEASE_MATERIALS.json', 'RELEASE_ASSETS.json');
  assert.deepEqual(release.assets.map(asset => asset.name).sort(), names.sort());
  for (const asset of release.assets) {
    assert.ok(Number.isSafeInteger(asset.bytes) && asset.bytes > 0);
    assert.match(asset.sha256, /^[a-f0-9]{64}$/);
    assert.equal(asset.url, `https://github.com/Laymer/Navzeroth/releases/download/${release.tag}/${asset.name}`);
  }
  return release;
}

export function validateCaptureCaption(caption, current, captured) {
  assert.match(caption, /synthetic/i);
  assert.ok((caption.match(/\b\d+\.\d+\.\d+(?:-rc\.\d+)?\b/g) ?? []).includes(captured.version), 'Capture caption must identify its actual release');
  if (captured.tag !== current.tag) assert.match(caption, /historical/i, 'Earlier release captures must be labelled historical');
}
