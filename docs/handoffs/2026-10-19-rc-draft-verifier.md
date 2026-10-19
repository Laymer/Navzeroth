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
