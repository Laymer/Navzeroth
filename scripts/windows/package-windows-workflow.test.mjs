import assert from 'node:assert/strict';
import fs from 'node:fs';
import test from 'node:test';

const workflow = fs.readFileSync(new URL('../../.github/workflows/package-windows.yml', import.meta.url), 'utf8');
const blocks = workflow.split(/^      - /m).slice(1);
function step(name) {
  const matches = blocks.filter(block => block.startsWith(`name: ${name}\n`));
  assert.equal(matches.length, 1, `exactly one ${name}`);
  return matches[0];
}
const build = step('Build installer and addon package (Windows PowerShell 5.1)');
const producer = step('Actual schema4 hosted Windows acceptance over the signed full bundle');
const overlay = step('Record schema4 hosted observations without inventing missing cases');
const gate = step('Verify stable hosted evidence and exact materials before accepting source or tag artifacts');

test('source and tag stable builds select the same mandatory bundle and prebuild path', () => {
  assert.match(build, /REF_TYPE -eq 'tag'.*REF_NAME.*tauri\.conf\.json/);
  assert.match(build, /\$stable = -not \$candidate -and \$expected -ceq 'v1\.0\.0'/);
  assert.match(build, /assemble-bundled-content .*--identity release\/v1\/bundled-content-build-identity\.json/);
  assert.match(build, /\$phaseOptions\['ReadinessPhase'\] = 'prebuild'/);
  assert.match(build, /\$phaseOptions\['BundleBuildIdentityPath'\]/);
});

test('both generic smoke paths retain a bounded stable startup and historical default', () => {
  for (const name of [
    'Native WebView2 smoke over CDP (blocking regression check, not WIN-* acceptance)',
    'Native upgrade smoke from the published baseline (blocking regression check, not WIN-* acceptance)',
  ]) {
    assert.match(step(name), /NAVZEROTH_NATIVE_STARTUP_MS: \$\{\{ steps\.build_windows\.outputs\.stable == 'true' && '1200000' \|\| '30000' \}\}/);
  }
});

test('actual installer-bound producer runs before the evidence overlay and final material gate', () => {
  assert.match(producer, /if: steps\.build_windows\.outputs\.stable == 'true'/);
  assert.doesNotMatch(producer.split('run: |')[0], /ref_type|ref_name/);
  assert.match(producer, /Get-FileHash \$setup\.FullName -Algorithm SHA256/);
  assert.match(producer, /Stable-Acceptance\.ps1/);
  assert.match(producer, /-InstallerSha256 \$hash/);
  assert.match(producer, /-BaselineSha256 \$pin\[0\]\.sha256/);
  assert.match(producer, /-BuildIdentity release\/v1\/bundled-content-build-identity\.json/);
  assert.match(overlay, /id: hosted_evidence/);
  assert.match(overlay, /Write-HostedAcceptanceEvidence\.ps1 .* -ResultsDirectory .*stable-cases/);
  assert.ok(workflow.indexOf(producer) < workflow.indexOf(overlay));
  assert.ok(workflow.indexOf(overlay) < workflow.indexOf(gate));
});

test('source and tag artifacts fail closed on missing producer or overlay before draft materials verification', () => {
  assert.match(gate, /if: always\(\) && steps\.build_windows\.outputs\.stable == 'true'/);
  assert.doesNotMatch(gate.split('run: |')[0], /ref_type|ref_name/);
  assert.match(gate, /steps\.stable_acceptance\.outcome.*-ne 'success' -or.*steps\.hosted_evidence\.outcome.*-ne 'success'.*throw/);
  assert.match(gate, /Verify-ReleaseArtifacts\.ps1 -Version '1\.0\.0' -Directory dist\/windows -ReadinessPhase draft -Evidence dist\/acceptance-evidence\/acceptance-evidence\.json/);
  assert.match(gate, /\$LASTEXITCODE -ne 0.*throw/);
  assert.doesNotMatch(gate, /-AcceptanceCandidate|-ReadinessPhase prebuild/);
});

test('draft publication still waits for package success and verifies its downloaded evidence', () => {
  const draft = workflow.split('\n  draft-release:\n')[1];
  assert.ok(draft);
  assert.match(draft, /needs: package/);
  assert.match(draft, /stable-acceptance-evidence-/);
  assert.match(draft, /command=release-draft-readiness/);
  assert.match(draft, /--draft --verify-tag/);
  assert.doesNotMatch(draft, /release publish/);
});

test('both Windows shells run bounded synthetic contract checks without local actual-file arguments', () => {
  for (const [name, shell] of [
    ['Schema-bound release contract tests (Windows PowerShell 5.1)', 'powershell'],
    ['Schema-bound release contract tests (PowerShell 7)', 'pwsh'],
  ]) {
    const block = step(name);
    assert.match(block, new RegExp(`shell: ${shell}\\n`));
    assert.match(block, /run: \.\/scripts\/windows\/Test-ReleaseContractBounds\.ps1\s*$/);
    assert.doesNotMatch(block, /ActualRoot/);
  }
});
