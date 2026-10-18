# Continuation 1 — release-policy reconciliation

Owner and sole writer: Codex `/root`; date: 2026-10-18.
Baseline: `7aecba8f170bd35fe2ece0dbce2c98a4337dfa4a`.
Branch: `docs/continuation-release-policy`; original checkout and `.claude/` preserved.
Claim: [ownership](../claims/2026-10-18-continuation-policy.md).

## Changed

Reconciled merged PRs #188–#190 and V2.4's authored-content limitations in STATUS
and TODO. ADR 0006 §13 and the release process distinguish early development
rc.1 from acceptance-candidate packaging; stable authorization is excluded for
this continuation. Recorded all 69 gate blockers with owners and clearing
conditions in the [inventory](../releases/2026-10-18-blocker-inventory.md).

## Actual validation

- GitHub confirms #188 merged at `7aecba8`, #189 at `afe2221`, #190 at `2e85981`.
- Baseline exact-main CI [39110542765](https://github.com/Laymer/Navzeroth/actions/runs/39110542765),
  Windows package [39110542823](https://github.com/Laymer/Navzeroth/actions/runs/39110542823),
  source [39110542778](https://github.com/Laymer/Navzeroth/actions/runs/39110542778): successful.
- `cargo` was absent from shell PATH; used `/home/laymer/.cargo/bin/cargo`.
- `CARGO_INCREMENTAL=0 CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0
  cargo run --locked -p azeroth-cli -- release-readiness --root . --version
  1.0.0-rc.1`: compiled with Rust 1.99.0, then correctly refused mismatched
  application/addon/lock versions, exit 1. No version was changed to bypass this.
- Built CLI, clean detached baseline root, `release-readiness --version 0.9.2`:
  exit 0, development publication allowed, candidate/stable false, 69 blockers,
  zero verified evidence files/packs.
- `git diff --check`: passed before review.

## Limitations and review focus

Documentation only. No live WoW, ordinary Windows, signing, coverage threshold or
publication verification is claimed. The full requested continuation is not
complete. Review policy consistency and exact blocker ownership. Independent
review and current-head hosted checks are required before merge.

## Next task

Continuation 2: durable extraction lifecycle. Inspect existing worker/store and
app orchestration, claim a new branch from reconciled main, implement versioned
source-bound checkpoints and atomic activation/rollback with failure tests.
