# Draft verifier permission correction

Owner/sole writer Codex `/root`; baseline `fcf48ea`, isolated branch
`fix/rc-draft-verification`. [Claim](../claims/2026-10-19-rc-draft-verifier.md).

Tag packaging 39242904060 passed and produced an unpublished development RC.
Draft downloads locally match all five tag artifacts, checksum entries, GitHub
asset digests, source identity and addon source bytes. Hosted draft verification
39245077836 failed before installation: `gh release view` returned release not
found under a contents-read workflow token. This is not a verification pass.

The manual verifier job now has contents-write scope for draft visibility. Its
commands still only read metadata/download/install/verify in the hosted runner;
there is no release mutation. Top-level permission remains read. Exact release
source checkout and installer hash binding are unchanged. The RC tag and assets
are unchanged; this is verification orchestration, not a replacement release.

Next: independent review, actual draft verification, all final-head PR checks,
merge, then record successful draft/public receipts before Pages publication.

## Integration receipt

Independent read-only review approved final refreshed head
`32f482850688180c28cde3125089f55296fa6315`. All ten required checks passed:
CI 39248994969, Coverage 39248994715, Windows package 39248994724.
PR #210 merged 2026-10-19T12:32:12Z as
`f20783f4ff93cb30aa15b5cd585e8571236b496e`.
The corrected verifier already passed draft 39245500923 and public 39245910009
on reviewed driver `683c2ae`, checking unchanged release source `fcf48ea`.
[Publication receipt](../releases/1.0.0-rc.1-publication.json) retains the exact
asset and application identities; the initial failure remains recorded above.
