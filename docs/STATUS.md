# Project status

Plan version: 1.0 plus accepted ADRs 0001, 0002 and 0003
Current milestone: M1
Current task: wave-1 parallel delegation per [ADR 0003](decisions/0003-wave1-parallel-delegation.md); twelve packages claimed below (M1.4 merged as PR 8)
Task owner: Claude Code (Fable 5.1) orchestrating one Sonnet 5.5 writer per package
Base revision: 8cca94d (origin/main after PR 8)
Last updated: 2026-10-12
Overall state: M1.1 reviewed/corrected; M1.6 CI, expanded M1.2 and M1.3 contracts merged and verified for their scopes; M1.4, M1.5 and M1.7 remain

## Milestones

| Milestone | Status | Evidence / blockers |
| --- | --- | --- |
| M0 | IN_PROGRESS | M0.1 repository/instructions/toolchain/license/CI inspection complete. M0.2 status/handoff records exist; initial decisions and sources are embedded in the implementation plan. M0.3-M0.6 need a user-operated Forever client, unavailable in this development environment (CLAUDE.md, section Environments). |
| M1 | IN_PROGRESS | M1.1 review/corrections, M1.6 workspace CI and expanded M1.2 VERIFIED and merged in PR 3. M1.3 contracts VERIFIED and merged in [PR 5](https://github.com/Laymer/Navzeroth/pull/5). M1.4, M1.5 and M1.7 remain. |
| M2 | TODO | |
| M3 | TODO | |
| M4 | TODO | |
| M5 | TODO | |
| M6 | IN_PROGRESS | Early imagery slice per [ADR 0002](decisions/0002-early-atlas-imagery-slice.md): core imagery records, projection, `prepare-atlas` CLI and Eastern Kingdoms content. M6.1/M6.2 UI parts wait for M1.4; M6.3 untouched. |
| M7 | TODO | |
| M8 | TODO | |
| M9 | TODO | |
| M10 | TODO | Optional desktop reference overlay; scoped as an independent UI experiment by ADR 0002, after the M6 atlas screen |
| M11 | DEFERRED_WITH_REASON | No verified Forever API integration yet |
| M12 | TODO | Expansion after focused release |

## Current task acceptance
M6.5 early slice (ADR 0002), claimed by Claude Code on 2026-10-12 from
0c90c25. Expected files: docs/decisions/0002-*, docs/data/, docs/compatibility/,
content/forever/atlas/ (LFS-tracked PNGs, manifest, NOTICE),
content/forever/datasets/, content/demo/atlas/, crates/core/src/entities/imagery.rs,
crates/core/src/atlas.rs, contract/dataset.rs additive members, types.rs
primitives, crates/cli/src/atlas.rs (`prepare-atlas`), fixtures/contracts/v1/,
tests/contract/check.mts, schemas/v1/ regeneration, CI pointer/consistency
checks, README and DOMAIN updates, a handoff.
Acceptance: old fixtures still valid; new fixtures pass Rust, JSON Schema and
TypeScript checks as declared; projection returns explicit gaps and never a
guessed marker; `prepare-atlas` is deterministic and refuses unmaterialized LFS
pointers outside check mode; CI never downloads LFS objects and fails on a raw
PNG blob under the LFS pattern; fmt, Clippy and workspace tests pass. The slice
is IMPLEMENTED_UNVERIFIED for anything needing the Windows desktop or the live
client. M1.4 remains the next task for UI work (PR-B in the plan).

## M6.5 early slice delivery (feature/M6.5-atlas-imagery, PR-A)
Additive `map_images`/`map_embeddings` dataset members, image-space pins,
explicit cross-build applicability, pure projection with explicit gaps and a
calibration solver, the `prepare-atlas` CLI with Git LFS pointer checks, the
synthetic demo atlas, the Eastern Kingdoms poster and layers as LFS pointers,
the reference dataset `forever-atlas-r1` (provisional `inferred` calibration,
31 measured flight-master pins) and CI pointer checks. See the
[M6.5 handoff](handoffs/2026-10-12-M6.5-atlas-imagery.md),
[ADR 0002](decisions/0002-early-atlas-imagery-slice.md),
[docs/data](data/atlas-eastern-kingdoms.md) and
[docs/compatibility](compatibility/BUILD_CAPABILITIES.md) and the
[M6.5 review](reviews/2026-10-12-M6.5.md). The Atlas screen and overlay wait
for M1.4.

## M1.3 delivery
Six versioned exchange envelope kinds with bounded semantic validation,
generated schemas/v1 JSON Schema and TypeScript, 52 synthetic fixtures with a
manifest, Rust and Node contract checks, and CI drift detection. See the
[M1.3 handoff](handoffs/2026-10-12-M1.3-contracts.md), the
[contract reference](../schemas/v1/README.md) and the
[M1.3 review](reviews/2026-10-12-M1.3.md).

## Completed behavior
All 19 section 6.1 entities, validated JSON and numeric/identifier types,
observation/freshness semantics, structured errors and all section 6.6 predicates
are implemented in the pure core. The CLI rejects unsupported arguments.
Linux and Windows CI runs five required checks. Main protection enforces them
for administrators too, with an up-to-date branch requirement.

[ADR 0001](decisions/0001-foundation-sequence.md) records sequencing and expanded
scope. Shared policy authorizes routine feature-branch pushes and draft PRs;
main pushes, merges and releases retain separate authorization requirements.

## Verification
M6.5 slice local: fmt, locked Clippy and tests passed (66 tests: 50 core, 16
CLI); schema/TypeScript regeneration deterministic; `pnpm contracts:check`
passed for 73 fixtures; `prepare-atlas` deterministic on the real poster;
`--check-pointer` passed on a pointer-only worktree. Hosted
[run 38259693876](https://github.com/Laymer/Navzeroth/actions/runs/38259693876)
on [draft PR 7](https://github.com/Laymer/Navzeroth/pull/7) passed all five
checks at 5442c74, including the Windows pointer checks. Desktop, overlay and live-client behaviour
remain unimplemented or unverified; calibration is provisional.
[PR 5](https://github.com/Laymer/Navzeroth/pull/5) was merged into main on 2026-10-12 at 12:22:58 UTC as 2c8628b; [main run 38248682966](https://github.com/Laymer/Navzeroth/actions/runs/38248682966)
passed all five checks on the merge revision.
M1.3 local: fmt, locked Clippy and tests passed (41 tests: 38 core, 3 CLI);
schema/TypeScript regeneration deterministic with no drift; pnpm contracts:check
passed for 52 fixtures. Hosted [run 38247973096](https://github.com/Laymer/Navzeroth/actions/runs/38247973096) passed all five checks at 827be4b+docs
(9c0b6a2), including Windows schema drift and contract checks.
M1.2 local formatting, locked Clippy/tests: passed (27 tests: 25 core, 2 CLI).
Rust 1.99.0, Node 24.15.0, Corepack 0.36.0, pnpm 12.11.2.
[Hosted main run 38241935794](https://github.com/Laymer/Navzeroth/actions/runs/38241935794)
passed all five checks at merge revision 514d4b0: Rust format, Rust Linux,
Rust Windows, pnpm Linux and pnpm Windows. See the
[M1.6 handoff](handoffs/2026-10-12-M1.6-ci.md) and
[M1.2 handoff](handoffs/2026-10-12-M1.2-domain.md) for exact checks and limitations.
Native desktop packaging, UI, Lua parity and live WoW behavior remain unverified
or unimplemented; pure model tests use synthetic data only.

## Open findings
No unresolved blocker for the completed scopes. See the
[M1.1 review](reviews/2026-10-12-M1.1.md),
[M1.2 self-review](reviews/2026-10-12-M1.2.md) and
[M1.3 review](reviews/2026-10-12-M1.3.md). Lua parity (M2), cross-dataset
reference resolution (M3) remain open.

## M1.4 delivery
React/Vite shell with DEMO label, typed command wrappers, azeroth-app handlers, Tauri 2 shell with empty capabilities and a Windows CI job. Hosted [run 38261091221](https://github.com/Laymer/Navzeroth/actions/runs/38261091221) passed six jobs; no window launched. See the [M1.4 handoff](handoffs/2026-10-12-M1.4-desktop-shell.md).

## Wave-1 parallel claims (2026-10-12, base 8cca94d)
Owner for every row: Claude Code (Sonnet 5.5), one writer per package, one
clone per writer; see [ADR 0003](decisions/0003-wave1-parallel-delegation.md)
for the frozen files, single owners and interface decisions. Acceptance for
every row: fmt, locked Clippy and tests (`--exclude azeroth-desktop`),
`prepare-atlas --check-pointer`, `pnpm contracts:types` with no diff,
`pnpm contracts:check`, hosted CI on the draft PR; each row also lists its
own checks. Each package records its claim and handoff in
`docs/handoffs/2026-10-12-<task>.md`. PR numbers are added by the
integration docs PR.

| Package | Tasks | Branch | Expected files | Package-specific acceptance |
| --- | --- | --- | --- | --- |
| P1 | M1.7 (+M2.4 rules) | feature/M1.7-storage | crates/storage/**, Cargo.toml member, Cargo.lock | migration test against the previous schema; CASE-007/008/009/015 |
| P2 | M2.2, M2.3 parser, M1.5 stub | feature/M2.2-ingest | crates/ingest/**, fixtures/transport/**, crates/cli/**, schemas/v1/README.md (ANAV1), Cargo.toml member, Cargo.lock | malicious SavedVariables sample never executes; `validate-fixtures` runs the two manifests |
| P3 | M4.1 | feature/M4.1-travel-graph | crates/core/src/travel.rs, lib.rs line, tests/travel.rs | CASE-004; disconnected graph reports an incomplete route |
| P4 | M4.2 | feature/M4.2-eligibility | crates/core/src/eligibility.rs, lib.rs line, tests/eligibility.rs | CASE-001/002 |
| P5 | M5.1-M5.3 (structure, synthetic weights) | feature/M5.1-gear-core | crates/core/src/gear/**, lib.rs line, gear tests, fixtures/gear/**, docs/data/ROLE_PROFILES.md | CASE-005/006/015 |
| P6 | M7.1 | feature/M7.1-reminders | crates/core/src/reminders.rs, lib.rs line, tests/reminders.rs | unchanged state repeats no advice; unknown suppression is conservative |
| P7 | M8.3 (pure slice) | feature/M8.3-session-intervals | crates/core/src/sessions.rs, lib.rs line, tests/sessions.rs, fixtures/sessions/v1/** | CASE-007; reload/clock-change gaps; offline never Observed |
| P8 | M3.2 | feature/M3.2-provider-registry | crates/core/src/sources.rs, lib.rs line, tests/sources.rs | CASE-005/015; precedence never discards evidence |
| P9 | M3.5 (library) | feature/M3.5-coverage-report | crates/core/src/coverage.rs, lib.rs line, tests/coverage.rs, fixtures/coverage/v1/** | prerequisite cycles and dangling references fail validation |
| P10 | M6.1 (ADR 0002 PR-B) | feature/M6.1-atlas-screen | crates/app/**, apps/desktop/**, apps/desktop/src-tauri/**, fixtures/desktop/atlas/** | desktop typecheck/lint/test:unit/build; Desktop shell Windows job; IMPLEMENTED_UNVERIFIED |
| P11 | M0.3 (skeleton, unverifiable) | feature/M0.3-diagnostic-addon | addons/**, .luacheckrc, crates/lua-harness/**, fixtures/addon/**, docs/compatibility/ADDON_*.md and LUA_RUNTIME_DIFFERENCES.md, ci.yml (one job), Cargo.toml member, Cargo.lock | harness tests; Lua static checks job; IMPLEMENTED_UNVERIFIED |
| P12a | M2.5 (Rust-side corpus) | feature/M2.5-predicate-parity-corpus | fixtures/predicates/v1/**, crates/core/tests/predicate_parity.rs | every manifest case consumed; expected values produced by Rust |

## Next dependency-ready work
M1.7 and the atlas screen (M6.1) are dependency-ready; M1.5 extends the M1.3 fixture
manifest. M1.4/M1.5 add their real CI targets when implemented. Lua parity for
the contracts follows the addon in M2. M0.3-M0.6 live checks remain blocked on
a user-operated client. [PR 5](https://github.com/Laymer/Navzeroth/pull/5) merged M1.3 as 2c8628b; earlier handoffs describe
their pre-merge delivery state. This documentation update claims no task and
establishes no release.
