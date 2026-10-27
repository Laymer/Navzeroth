# Independent public content review V19

Owner world_review. Baseline 56f5fa53bb2c546a7b3efb4d02bb5e5727c62531 (detached); tested source 619e498c4aab55d653a1b0c5767bc81a9c99b321 (PR 269 head, merge d9658560), production 2aefc67b79569a01a461315648b64b860599cabb2f67de1b52911025e3ef47e6 over 3132 exact Git-tree entries excluding only docs/ and release/. This handoff is not pinned by the review it describes.

## What was checked by this reviewer

- The production fingerprint method reproduces 2aefc67b for 619e498c, d9658560 and 56f5fa53, 8fd5fa29 for d7b7474d and 77e88f41 for 7ac0b1a5.
- The only non-docs/non-release path differing between d7b7474d and 619e498c is content/bundled/signers-v1.json (one line). Nothing else differs.
- The policy holds exactly two signers (operator, then review_evidence) with distinct -r2 key IDs and distinct 32-byte lowercase-hex public keys, revoked false, operator_controls_both_keys true, in the same single-line canonical shape with no trailing newline. docs/releases/1.0.0-content-public-identities.json carries the same two public keys (its ids are stable-1.0.0-*-r2, as the previous pair was stable-1.0.0-*).
- No Rust, TypeScript, script, workflow or test under crates, apps, scripts or .github contains the previous public keys or key IDs. The policy is compiled in through include_bytes in crates/cli/src/bundled_content.rs, crates/cli/src/release/bundled.rs and apps/desktop/src-tauri/src/lib.rs; crates/app/tests/bundled_content.rs also reads it.
- The V17 archived receipt carries the previous key IDs; archived receipts stay historical evidence.
- The nine canonical observation rows are pristine unverified; bundled_content is absent from release/v1/acceptance-evidence.json; its tested_source_commit and production_sha256 are 619e498c and 2aefc67b; release/v1/bundled-content-{manifest,build-identity,receipt}.json are absent.
- All FileRefs in release/v1/acceptance-evidence.json (93 occurrences, 68 unique paths) and in the V19 review match bytes/SHA-256 and Git blob or LFS pointer oid/size. LFS-backed paths in this worktree are unhydrated pointers; the referenced LFS objects were also found in the shared local object store and recomputed there without network. Two V18 pins were stale and are repinned (signers-v1.json, offline-candidate-v18 handoff).
- The unsigned V18 archive (release/history/2026-10-21-unsigned-content-v18) matches its history.json byte for byte.
- python3 scripts/release/test_prepare_bundle.py: 8 tests, OK.

## What this reviewer did not do

No PowerShell, Rust build, Rust test or workflow test was run. The PR 268 executions (Test-NativeSmokeHelpers 17 checks on native Windows PowerShell 5.1 and PowerShell 7.6.7, Test-StableAcceptanceHelpers 36 and Test-StableWorkflowGate 12 on PowerShell 5.1, 6 workflow Node tests) and the PR 269 executions (azeroth-cli bundled_content and bundled_receipts 2+2 passing, azeroth-cli lib release::bundled 8 passing, azeroth-app bundled_content 7 passing) were run by the root session. The hosted effect of the uninstall repair is unverified; the diagnosis of the V17 RC1 failure is an inference. Private key and seed material was neither read, listed nor hashed.

## Facts carried forward

V17 hosted run 39603870404: six passes, HOSTED-UPGRADE-RC1 failed (installer exit code 2), HOSTED-DOWNGRADE-REFUSAL not run, owner smoke unverified. PR 268 merged at 2692631f. The signer rotation was authorized by the owner because the original seeds could not be located; both new keys remain operator-controlled and were generated outside Git, and no signature with either new key exists yet. Distinct keys do not establish independent custody.

## Next

Fresh unsigned V19 preparation and reproduction before any key use, signing with the rotated external keys, independent closure checks, then one candidate push. No main merge, tag or publication.
