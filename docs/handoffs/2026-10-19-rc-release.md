# Development RC preparation

Owner/sole writer: Codex `/root`. Initial artifact-tooling baseline `c8577da`;
branch `release/1.0.0-rc.1`, isolated worktree `/tmp/navzeroth-rc-release`.
[Claim](../claims/2026-10-19-rc-release.md). Version preparation is pending repair
integration. Latest public release remains 0.9.2. No tag or publication yet.

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
- Lifecycle PR #204 and SemVer PR #206 are pending final fresh checks/integration.
  Their earlier strict coverage failures at worker.rs 176/201 are not passes;
  both now include merged PR #207. Native coverage collectors passing alone
  never substitutes for the combined per-file gate.

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
