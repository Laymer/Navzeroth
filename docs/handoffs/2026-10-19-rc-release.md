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

Next: finish final-head checks and independent review for release PR #209,
then require fresh exact-resulting-commit CI, Coverage, source/parity, ordinary
Windows packaging/native upgrade/refusal and stability. Tag only that verified
commit; verify draft downloads before publication and public downloads afterward.
Separate Pages delivery follows public verification. Candidate/stable readiness
remain false; stable publication and acceptance-candidate promotion are excluded.

## Version and evidence preparation

Six canonical declarations and eight internal Cargo.lock entries are 1.0.0-rc.1.
Historical exchange/dataset identities remain unchanged. Addon producer fixtures,
reference/Today goldens, synthetic atlas producer metadata and dependency manifest
are regenerated through their existing commands. Notes explicitly identify an
unsigned development prerelease; candidate/stable readiness remain false.
Installation guides reconcile the pending post-launch target with ADR 0006 while
keeping 0.9.2 download links. Private extraction/cache receipts are retained.

Release PR [#209](https://github.com/Laymer/Navzeroth/pull/209) is open.
Actual regeneration/check results and committed-tree readiness output follow.
No tag or publication has occurred.

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

Both independent read-only reviews approved preparation head `b8ee77d` without
blocking findings. The version reviewer identified a stale next-task paragraph,
corrected here. Final-head refresh review and hosted checks remain required.
Readiness/material selection was also rerun successfully at `b8ee77d`: the same
69 blockers, candidate/stable false, empty selection and production fingerprint.

Ordinary unsigned RC packaging now rehearses the complete verifier against the
prepared asset set before its existing install/smoke/upgrade checks. This tests
installation, ZIP/resource/source closure, signature and PE metadata checks before
tagging. Its receipt is labelled prepared-rc-verification, not draft/public
download evidence. Draft/public workflows still verify independently downloaded
bytes against the recorded tag-packaging hash. Hosted execution remains pending.

## Release preparation integration

PR #209 merged at 2026-10-19T10:43:41Z as
`fcf48ea5b3c7ba79cc3603bd5005fe961bca60e3`. Both independent read-only
reviewers approved final head `da390c44c865108f543723c0f7cda2a41ff85623`.
All ten required checks passed: CI 39236035802, Coverage 39236035691,
Windows package 39236035787; source/parity 39236035740 also passed.
The combined coverage receipt evaluated PR merge tree
`de82179d574e40b04d0020d7da3581dd39c0db90`, with no errors.
Prepared Windows installation verified 544 installed files, matching addon ZIP
and dependency closure; installer and application were NotSigned, with version
strings 1.0.0-rc.1. Native smoke passed 31 checks, including all three picker
cancellations and unchanged workflow selections. Upgrade population passed 5
checks; verification passed 7; downgrade refusal exited 2 and retained the RC.
These are prepared PR artifact checks, not public download verification.

Readiness/material selection both exited 0 on the exact resulting commit above:
[readiness](../releases/1.0.0-rc.1-release-commit-readiness.json),
[materials](../releases/1.0.0-rc.1-release-commit-materials.json).
Production fingerprint `721f07a667fc9aa3f1e99db8adf86bbd480f57526a8250a5fafc99150b580a21`;
69 blockers, publication allowed, candidate/stable false, zero verified evidence
and content packs, empty material selection. This supersedes the earlier
preparation fingerprint for exact release-commit verification.

Fresh resulting-commit checks are running: CI 39239511840, Coverage 39239511851,
Windows package 39239511894, source/parity 39239511923, and dispatched CI with
stability 39239530676. No tag or publication is inferred from pending runs.

All fresh resulting-commit runs above completed successfully, including the
dispatched stability job (both browser projects three times, full Rust suite
twice, Lua harness three times). The combined coverage verdict binds
`fcf48ea5b3c7ba79cc3603bd5005fe961bca60e3`, passes with no errors and retains
the exact 90/100 unrounded per-file threshold. Windows native smoke passed
31/31, upgrade population 5/5 and upgrade verification 7/7; downgrade refusal
exited 2, did not time out and retained 1.0.0-rc.1.

Annotated tag `v1.0.0-rc.1`, object
`b55d72da23b2edab79971edd8d813ac68a4e0f46`, points to that exact commit.
Ordinary tag packaging run 39242904060 is in progress; no acceptance-candidate
flag or test-signing mode is used. Publication remains pending draft download
verification and successful tag checks.

## Public development prerelease

Tag package 39242904060 passed: all five artifact hashes verified locally; native
smoke 31/31, upgrade 7/7 and downgrade refusal exit 2 retaining RC. Initial draft
verification 39245077836 failed before installation because its contents-read
workflow token could not discover the draft. Independent review approved manual
verifier permission correction `683c2ae` (PR #210), and draft run 39245500923
passed against unchanged release source/bytes. Public run 39245910009 also passed
after publication. Local public downloads match all five draft/tag artifacts,
GitHub asset digests and addon source. Both hosted receipts verify all 544 installed
files, dependency notice/source closure, SemVer metadata and NotSigned installer/app.

[Publication receipt](../releases/1.0.0-rc.1-publication.json) retains commit, tag
object, workflow IDs, every asset hash, signing status and scope. Raw receipts
are in `docs/releases/1.0.0-rc.1-receipts/`. No stable/acceptance promotion occurred.

External PR #208 merged after the immutable release source; it is not part of
the RC. PR #210 was refreshed onto that main as `32f4828`; independent review
confirmed unchanged three-file diff and approved it. Fresh checks remain pending.
Next delivery is Pages using actual verified release captures and RC-only downloads.
