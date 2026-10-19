# V2.3 — exact objective coverage in guide replay

Owner/sole writer: Codex `/root/guide_validation`; 2026-10-19.
Baseline `826aa21`; branch `task/guide-validation-hardening`.
[Claim](../claims/2026-10-19-guide-objectives.md).

Found and reproduced a verifier defect: with two declared objectives, marking
indexes 1 and 3 counted as complete coverage and suppressed the missing-objective
warning, despite the separate invalid-index error. Turn-in now checks every
required index rather than the marked-set size. Missing valid objectives remain
warnings; invalid indexes remain errors. Zero and unknown counts remain distinct.

Added five tests: an 11-case explicit matrix (including 255-objective boundaries),
property-based exact-set comparison, unknown-count cases, character-filtered
completion marks, and abandon/reaccept invalidation. The original matrix failed
against baseline behavior before the repair. An initial maximum-index fixture
exceeded the bounded per-step line limit; it was corrected to use separate steps,
without relaxing parser limits.

Actual checks (Linux, with `/home/laymer/.cargo/bin/cargo` because cargo is not on
this session's PATH):

- `cargo test --locked -p azeroth-ingest`: 132 passed, no failures or ignored tests;
  the RXP integration suite increased from 35 to 40 tests.
- `cargo clippy --locked -p azeroth-ingest --all-targets -- -D warnings`: passed.
- `cargo fmt --all --check` and `git diff --check`: passed.
- `cargo llvm-cov --locked -p azeroth-ingest --test rxp --json --output-path
  /tmp/navzeroth-guide-rxp-coverage.json`: 40 tests passed; focused line coverage
  `rxp_verify.rs` 809/851 (95.06%), `rxp.rs` 541/569 (95.08%). This focused run
  is not the full project/native coverage gate; no before/after percentage claim.

No real guides, client facts or personal exports were added. No evidence or
acceptance status was promoted. Live WoW and native Windows checks remain
unverified; this pure replay repair requires neither environment. V2.3 authored
slice routes and V2.5 reviewed completeness still require actual build-bound facts.
Next: obtain those inputs and run bound review/inventory workflows; independent
engineering may continue while they are unavailable. Hosted PR checks are pending.
