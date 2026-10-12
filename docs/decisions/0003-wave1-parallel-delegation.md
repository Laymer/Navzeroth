# ADR 0003: wave-1 parallel delegation of dependency-ready tasks

Date: 2026-10-12
Status: accepted by the owner (plan-mode approval of the parallelization plan)
Tasks: M1.7 (+M2.4 rules), M2.2, M2.3 (parser), M1.5 (importer stub), M4.1,
M4.2, M5.1-M5.3, M7.1, M8.3, M3.2, M3.5, M6.1 (ADR 0002 PR-B), M0.3 (addon
skeleton, unverifiable), M2.5 (Rust-side parity corpus)

## Context
Main at 8cca94d holds M1.1-M1.4, M1.6 and the M6.5 early slice. Every task so
far had one writer. The owner asked for the remaining plan to be divided into
as many independent slices as possible and implemented in parallel by Sonnet
5.5 agents, each on its own branch and draft PR. Plan section 21.3 allows this
when the user explicitly assigns parallel tasks: isolated branches, disjoint
ownership, interface dependencies resolved first. Section 21.8 requires an ADR
for a material sequencing deviation; several slices start before their
milestone's nominal dependencies.

## Options considered
Continue strictly sequentially; parallelize only the M1 remainder; or
parallelize every slice that needs no live client, no invented game data and
no other unmerged branch. The owner chose the last.

## Decision
- Early starts on synthetic fixtures only: M4.1/M4.2 (plan section 17 M4
  allows synthetic planner tests early), M5.1-M5.3 (profile structure and
  configuration/score logic; every weight is labelled synthetic, no real role
  weights are shipped), M7.1 (pure reminder engine; suppression inputs stay
  unknown/conservative), M8.3 (pure interval model; the addon recorder of
  M8.1 waits for M2), M3.2 and M3.5 (pure core libraries; persistence and CLI
  wiring wait for wave 2). None of these claims real-world behaviour.
- One writer per package, one local clone per agent, never the owner's
  checkout. Packages, branches and owned paths are recorded in
  docs/STATUS.md (wave-1 claim table). Owner is "Claude Code (Sonnet 5.5)"
  under Fable 5.1 orchestration.
- Frozen in wave 1 (no package edits them): crates/core/src/contract/**,
  crates/core/src/entities/** (including doc comments), evaluation.rs,
  predicate.rs, types.rs, error.rs, observation.rs, tests/common/mod.rs,
  schemas/v1/exchange.*, fixtures/contracts/v1/**, content/**, README.md,
  docs/STATUS.md (after this claim table), docs/DOMAIN.md, docs/data/SOURCES.md,
  docs/compatibility/BUILD_CAPABILITIES.md and numbered ADRs. New types,
  reason enums, error construction and test helpers stay local to each
  package module. No new ErrorCode variants.
- Single owners for shared files: crates/cli/** and schemas/v1/README.md
  (M2.2 ingest); crates/app/**, apps/desktop/** and pnpm-lock.yaml (M6.1);
  addons/**, .luacheckrc and one new ci.yml job (M0.3). Cargo.toml members
  and Cargo.lock change only for the storage, ingest and lua-harness crates
  (plus one sha2 edge for azeroth-app); `cargo update` without `-p` is not
  allowed. Each core package adds one `pub mod` line to crates/core/src/lib.rs
  at its alphabetical slot. Fixtures go under a package-owned fixtures/<area>/.
- Interface decisions resolved before spawning: the ingest crate is normative
  for the ANAV1 text grammar; the addon package is normative for the
  SavedVariables global `AzerothNavigatorDB` and its key layout (UNVERIFIED
  until M0.5), and its export is not ANAV1; the storage crate owns snapshot
  identity (sha2 over the canonical validated envelope) and ordering
  (session first, then trusted clock, else incomparable); the coverage
  library is the only owner of prerequisite-cycle and dangling-reference
  validation; the source-registry module is the only owner of evidence
  precedence; the atlas screen reads only the embedded demo snapshot,
  labelled DEMO, until M2.4 wires storage. Storage and ingest do not depend
  on each other in wave 1.
- Shared conventions: closed, area-prefixed reason enums per module; only
  `Truth::True` is a definite affirmative, Unknown is never coerced; Millis
  and UtcMillis with checked arithmetic; fixed-point integers for scores;
  DataContext-qualified keys everywhere (CASE-005/011/015); the 1024 bound;
  BTree collections; synthetic data (wow_forever, build SYNTHETIC, namespace
  demo, fixture true); callers pass now_utc_ms; nothing is labelled live.
- The Lua half of M2.5 (predicate.lua, JSON decoder, Lua runner, CI step)
  waits for the addon skeleton to merge; wave 1 ships only the Rust-anchored
  fixture corpus.
- Merge order to minimise lockfile churn: this record, M2.5 corpus, M3.5,
  M4.2, M5.x, M7.1, M8.3, M3.2, M4.1, M2.2, M6.1, M1.7, M0.3, then an
  integration docs PR. Published branches incorporate later main by merge,
  never rebase. Main merges remain owner actions.

## Consequences
Twelve draft PRs land as IMPLEMENTED_UNVERIFIED or VERIFIED-for-scope with
hosted CI evidence only; M6.1 and M0.3 cannot be verified without the Windows
desktop and the live client. STATUS/README/DOMAIN are reconciled by one
integration docs PR after merges. Wave 2 (planner, app wiring, watcher,
remaining repositories, CLI wiring, overlay window, Lua parity runner, e2e)
starts after wave-1 merges.

## Evidence
Partition stress-tested on 2026-10-12 by a read-only workflow (12 per-package
verifiers in isolated worktrees, an overlap judge, a completeness critic and
an integration-mechanics analyst); its corrections (P12 split, M8.3
narrowing, M5.1 structure-only, no numbered ADRs from packages, frozen core
files) are applied above.
