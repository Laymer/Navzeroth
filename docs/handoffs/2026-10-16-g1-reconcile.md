# G1: reconcile 0.9.1 publication records and record ADR 0005

Owner and sole writer: Claude Code workflow agent (0.9.2 program). Baseline
`origin/main` `92f3f512a68481163e3d44600e3938d07592af4e`; branch
`docs/g1-reconcile`; worktree `/home/laymer/Devel/worktrees/g1`; claim
[2026-10-16-g1-reconcile](../claims/2026-10-16-g1-reconcile.md). No subagents. The
shared checkout was not touched. Documentation and production doc-comments only.

## Done

- `initial.md` made version-agnostic (latest tag resolved with `gh release list`,
  `--version <current>`, no "finish the 0.9.0 draft"); `localdata.md` baseline
  paragraph and the "nine required checks" wording corrected.
- `docs/STATUS.md`: one "Current task" section, local-data paragraph moved under
  "Preceding work", 0.9.1 stated as latest published with its receipt, five
  enforced checks and the `release:` prefix recorded, wave-1 table annotated with
  code evidence (blocker and all 12 important findings fixed), M9 row fixed.
  `docs/IMPLEMENTATION_TASKS.md` header and a currency note.
- New [publication record](2026-10-16-v0.9.1-publication.md) with tag object, run
  IDs, publish time and SHA-256 of all five assets from a fresh `gh release
  download`; release handoff no longer relies on `/tmp`; small `/tmp` receipts
  preserved under `evidence/2026-10-16-v0.9.1/`.
- `docs/sources/source-build-result.json`: hosted parity recorded as passed
  (runs 38892199064 and 38894373036, tested commit `92f3f51`). Field semantics
  unchanged; the recorded build hashes were verified equal to the hosted artifacts.
- Stale references fixed in `V1_RELEASE_PROCESS.md`, `release/v1/README.md` (eight
  internal lock entries), `RELEASE_MATERIALS.md` (`<version>`), `WINDOWS_INSTALL.md`,
  `docs/releases/0.9.0.md` (marked not published, tag link replaced), the ForeverDB
  request note; doc comments in `coverage.rs`, `gear/configuration.rs`,
  `reminders.rs` and `schemas/v1/README.md`.
- `docs/releases/0.9.1.md` is unchanged (published `CANDIDATE_NOTES.md`).
- ADR [0005](../decisions/0005-v092-program.md).

## Checks run

- `cargo test --locked -p azeroth-cli`: 141 passed, 0 failed, 3 ignored (summed over all
  suites), run with `CARGO_TARGET_DIR=/home/laymer/Devel/cargo-targets/g1`.
- `cargo run --locked -p azeroth-cli -- release-readiness --root . --version 0.9.1`
  on the committed tree: exit 0, publication allowed, stable and candidate not
  ready, 66 blockers, 26 pending, 0 verified evidence files and packs. A dirty tree
  reports 67 (extra "working tree contains changes" blocker).
- `cargo fmt --all --check` and `cargo clippy --locked -p azeroth-core --all-targets
  -- -D warnings` for the doc-comment edits: both clean.
- Relative-link check (node script) over all changed markdown.
- `gh release view`, `gh release download`, `sha256sum -c`, `gh run view`, `gh api`
  (read-only) for every published fact.

## Unverified

- Hosted CI for this branch (pending at the time of writing).
- Nothing about native Windows, live-client behaviour or 1.0.0 readiness changed.
- Code line numbers in the wave-1 table are approximate ("about") and were checked
  by reading, not by running the original regression tests.

## Review resolution

Review of head `d13efb5` against base `92f3f51` (record:
[2026-10-16-g1-review](../reviews/2026-10-16-g1-review.md)). No blockers; two
important findings (one reported twice) and two optional ones, all resolved.

| Finding | Action | Evidence |
| --- | --- | --- |
| important: gear doc comment said unknown `two_handed` is Uncertain only with two or more items | Rewrote the bullets in `crates/core/src/gear/configuration.rs` to match `classify()`: the hand-slot rule has no item-count condition; only the `unique_group` rule needs `placed.len() >= 2`, and it leaves the configuration Valid | Compared with `classify()` lines 228-250; `cargo test --locked -p azeroth-core` (361 passed, 0 failed), fmt, Clippy and release-readiness (exit 0) re-run |
| important: publication record said all receipts "agree" though `main-package-receipt.json` has a different installer hash | Investigated: two "Windows package" runs built `92f3f51` (main push 38894373306, tag push 38896174846); re-downloaded both and hashed: `ecf8e0c2...` (6,871,442 B) vs `874c80b5...` (6,868,991 B). Release published the tag run's installer. Claim corrected and cause stated; added to Unverified | `gh run download` + `sha256sum` |
| optional: same mismatch in the evidence README | README now states the difference and cause | as above |
| optional: `verify.py` not re-runnable | README says it hard-codes `/tmp/navzeroth-release-v091` and that receipts are recorded output. (The "four of five assets" wording was checked and is consistent: `SHA256SUMS.txt` is the fifth file; clarified.) | Read of `verify.py` |

## Limitations

- The reason the two installer builds differ was not investigated.

## Next

G2 (end-to-end harness), G2b (native smoke) and G3 (addon GUI) per the program plan;
the 0.9.2 notes must carry the errata for `docs/releases/0.9.1.md`.
