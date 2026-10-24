# Independent public content review V18

Owner world_review. Baseline 854da8b1d35cc101160fcef890e6876bdbef2c5e (detached); tested source d7b7474d22323a84ca37ad895d26ba8400910b25, production 8fd5fa2982164e040bcdf999190fd307ef76ef7adddb94cdf9b8681389b21493 over 3132 exact Git-tree entries excluding only docs/ and release/.

Additive release/v1/public-content-review-v18.json preserves V1-V17 immutable and pins V17 in prior_reviews.

Verified by this reviewer
- The production fingerprint recomputes to 8fd5fa29 for d7b7474d, the PR 268 merge 2692631f and 854da8b1, and to 77e88f41 for the V17 reviewed 7ac0b1a5.
- Against 7ac0b1a5 only scripts/windows/Native-Smoke-Helpers.ps1, Stable-Acceptance.ps1 and Test-NativeSmokeHelpers.ps1 differ outside docs/ and release/; no other version bump exists there. Against ac45a4f4 only scripts/windows and the offline probe workflow differ, so runtime Rust, UI, addon, content and Cargo are unchanged.
- All FileRef occurrences in release/v1/acceptance-evidence.json (93 occurrences, 68 unique paths) and in the V17 review (366, 356) match bytes and SHA-256 and the Git blob or LFS pointer oid/size. The V18 review has 384 occurrences, 377 unique paths. Four content paths are Git-LFS pointers in this worktree (no hydration); only pointer oid and size were checked for them.
- The four V17 archives (signed-content-v17, v17-upgrade-failure, v17-ci-source-pages, v17-coverage) match every history.json byte size and SHA-256 and the committed Git blobs, with no unlisted files. The three old canonical signed files equal the archived copies and the 8b12aec1 blobs and are absent from release/v1.
- Hosted run 39603870404 case records: six pass, HOSTED-UPGRADE-RC1 fail (exit code 2), no HOSTED-DOWNGRADE-REFUSAL record; the recorded overlay shows the same with owner smoke and downgrade unverified. Recovery took 774.576 s. The archived coverage verdict has 276 measured files (0 below the 90 percent integer threshold) and 29 declaration-only files.
- acceptance-evidence.json carries nine pristine unverified checks, tested_source_commit d7b7474d, the new production hash, and no bundled_content.

Not done or not verified
- The diagnosis (the uninstall wait ignored the registry entry) is an inference from code and timing. The repair is unverified on a hosted runner.
- The PR 268 test executions (Test-NativeSmokeHelpers 17 checks on PS 5.1 and 7.6.7, Test-StableAcceptanceHelpers 36, Test-StableWorkflowGate 12, 6 workflow Node tests) were run by the root session, not by this reviewer, who ran no PowerShell, build or test.
- Unsigned installer under ADR 0008, operator control of both signing keys, earlier historic origins and unknown validation, the narrow V16 recovery margin history, and the absence of live-client or ordinary-PC claims are retained from V17. No keys, signing, push or publication by this writer. This handoff is not self-pinned.

Next: root review and merge of this record, reproducible unsigned preparation committed before keys, signing with the owner's external keys, independent closure checks, then one candidate push. No main merge, tag or publication.
