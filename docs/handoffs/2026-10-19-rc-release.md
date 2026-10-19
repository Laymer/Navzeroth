# Development RC preparation

Owner/sole writer: Codex `/root`. Initial artifact-tooling baseline `c8577da`;
branch `release/1.0.0-rc.1`, isolated worktree `/tmp/navzeroth-rc-release`.
[Claim](../claims/2026-10-19-rc-release.md). All repair PRs are merged; final repair baseline
`cb5d68b579934b324ac0e999384c64524ccbc260` is integrated cleanly. Latest public release remains 0.9.2. No tag or publication yet.

## Repair receipts

- PR #205 merged `e9bb77042ebaf306c8d35c82fcb8bde46eaf092c` at
  2026-10-19T08:42:30Z. Independent read-only approval of final head
  `8e4a59c6ef6de95e7e4b5730372daacfbe0ab106`; all ten required checks passed.
  CI 39221168492, Coverage 39221168487, package 39221168540,
  source/parity 39221168489. Coverage measured 251 passing files.
- PR #207 merged `c8577da03ace3c101c682dce75d9d3238dd2e9bf` at
  2026-10-19T09:15:22Z. Independent read-only approval of final head
  `47ef518c26c35a9b8ba99f24f84e913e001eab0a`; all ten required checks passed.
  CI 39224729666, Coverage 39224729653, package 39224729698,
  source/parity 39224729749. The new deterministic worker protocol checks
  address missing branch evidence without weakening coverage requirements.
- PR #204 merged `ac6037a44fb41a24dba7728a54cbf603eb364b87` at
  2026-10-19T09:40:48Z. Independent read-only approval of final head
  `bbdd5e4bb499c84f7329884c14f54e16ac7dd254`; all ten required checks passed.
  CI 39228695506, Coverage 39228695446, package 39228695449,
  source/parity 39228695445. Combined receipt: 251 measured files passed,
  no errors, worker.rs 189/204; evaluated PR merge tree `eede1c457a748eb9a0f829757013fdf9f3841948`.
- SemVer PR #206 is refreshed onto that main at
  `c725ed3440bffd158dfbd4fa9eb84a2c5240bbbd`; independent approval without
  findings and helper tests 16/16 passed. All ten required checks passed:
  CI 39231927999, Coverage 39231927982 and package 39231927967.
  Merged at 2026-10-19T10:05:12Z as `cb5d68b579934b324ac0e999384c64524ccbc260`.
  Earlier strict coverage failures at worker.rs 176/201 are not passes.
  Native collectors passing alone never substitutes for the combined gate.

## Download verification tooling

The new manually dispatched hosted-Windows workflow checks draft/public state,
exact tag/checkout/source identity, the five-file asset set and checksums. It binds
the installer to the independently recorded tag-packaging hash before installing
in the disposable runner profile. It verifies exact addon ZIP entries/bytes,
all installed resources against source, dependency notices/retained source closure,
version metadata and unsigned Authenticode status. It records asset and installed
file hashes, run identity, numeric/string PE metadata and evidence scope.

Ordinary packaging also runs the exact installed inventory verifier before release.
Only application/uninstaller binaries and mapped source resources are admitted;
unexpected files fail. Stable-release signing requirements are unchanged. The
new download verifier is intentionally limited to unsigned 1.0.0-rc.1.

Actual local checks: installed-tree mutation test passes (changed/missing/extra/
linked resources refused); current resource mappings pass on a synthetic copied
tree of 542 source files and two placeholder binaries; PowerShell 5.1 parser
check passes. These are not installation evidence. Preliminary independent review
found no blockers and requested expanded PE metadata, now included. Final-head
review and hosted execution are pending.

Next: merge the reviewed repair PRs after passing checks, integrate resulting main,
then synchronize versions, regenerate producer fixtures/notices, finalize notes,
and measure readiness/material selection on the committed tree. Exact resulting
release commit and tag checks, draft/public asset verification, publication and
separate Pages delivery remain unfinished. Candidate/stable readiness remain false.

## Version and evidence preparation

Six canonical declarations and eight internal Cargo.lock entries are 1.0.0-rc.1.
Historical exchange/dataset identities remain unchanged. Addon producer fixtures,
reference/Today goldens, synthetic atlas producer metadata and dependency manifest
are regenerated through their existing commands. Notes explicitly identify an
unsigned development prerelease; candidate/stable readiness remain false.
Installation guides reconcile the pending post-launch target with ADR 0006 while
keeping 0.9.2 download links. Private extraction/cache receipts are retained.

Actual regeneration/check results and committed-tree readiness output follow.
No release PR or publication is claimed by this preparation alone.

Actual local regeneration: addon fixture suite 7/7, reference 3/3 and Today 11/11
passed; synthetic atlas preparation exited 0. Dependency notices regenerated and
verified 506 retained notice/source files. New installed-tree mutation test and
synthetic 542-resource mapping check passed; PowerShell 5.1 parser check passed.
Locked pnpm dependencies installed. Hosted final-head checks remain pending.

Readiness/material selection on committed `b020b45d2b57a1cba2f12f6e364a1a58c3cec77e`
both exited 0: publication allowed for this development version, candidate/stable
false, 69 exact blockers, zero verified evidence/packs and empty material selection.
[Version-specific inventory](../releases/1.0.0-rc.1-readiness.md) records every
blocker, owner and clearing condition alongside raw committed-tree outputs.
