# V110.4a — progression observation foundation

Owner/sole writer Codex `/root`; baseline `6b4dfe51`; isolated branch
`feat/v1.1-guided-progression`. Claim: `docs/claims/2026-11-10-v110-progression-state.md`.

## Implemented

Pure core collection lookup preserves complete/partial/unavailable, missing companion,
stale/uncertain time, false and zero. Absence is established only by a fresh complete
collection. Existing Observation metadata preserves manual/addon provenance;
RecordMap provides duplicate and 1024-entry refusal. SnapshotBinding checks exact
snapshot/character/context/source revision/capture metadata and supplied persisted
snapshot SHA-256. Old exchange schemas and snapshot types are unchanged.

## Actual checks

- `cargo test --locked -p azeroth-core --test progression_state`: 4 passed.
- `cargo test --locked -p azeroth-core`: passed; existing ignored fixture generator
  and whole-world LFS workload remain ignored, not verified by this run.
- `cargo clippy --locked -p azeroth-core --all-targets -- -D warnings`: passed.
- `cargo fmt --all --check` and `git diff --check`: passed.
- `cargo llvm-cov --locked -p azeroth-core --test progression_state --json`:
  progression/mod.rs 44/45 covered lines (97.78%), 3/3 functions. This is focused
  module coverage, not the repository-wide release coverage gate.
- Read-only reviewer `/root/review_v110_contract`: no blockers; independently ran
  generated focused executable (4 passed) and whitespace check, did not rebuild.

Cargo was absent from shell PATH; successful commands used
`/home/laymer/.cargo/bin/cargo`. Initial PATH failures are not test failures.

## Limitations and next task

This foundation is not yet consumed by application services. Callers must obtain
SHA-256 from the selected persisted snapshot and enforce binding before lookup;
never trust a companion's self-declared digest as the selected snapshot identity.
No progression exchange, migrations, capture, saved plans, generated frontend
contracts or usable workflows are claimed. No live verification or release gate
promotion. Exact 70862 interface evidence remains pending.

Next: V110.4b versioned companion payload with bounded decoding and old-import
compatibility, then storage/manual inputs; V110.3 gameplay facts/content can proceed
independently. Complete all tracker items before candidate preparation/publication.
