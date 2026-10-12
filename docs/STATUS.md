# Project status

Plan version: 1.0 plus accepted ADR 0001
Current milestone: M1
Current task: none claimed; M1.4 and M1.7 are dependency-ready
Task owner: unassigned
Base revision: 2c8628b1abb202d32c788cba0d49f8c1c21789fe (PR 5 merged into main)
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
| M6 | TODO | |
| M7 | TODO | |
| M8 | TODO | |
| M9 | TODO | |
| M10 | TODO | Optional desktop reference overlay |
| M11 | DEFERRED_WITH_REASON | No verified Forever API integration yet |
| M12 | TODO | Expansion after focused release |

## Current task acceptance
No task is claimed. The next writer claims M1.4 (minimal Tauri/React shell with
explicit demo mode and typed command wrappers) or M1.7 (minimal
profiles/snapshots/imports SQLite migration and storage adapter) from current
main, recording owner, baseline, expected files and acceptance checks first.
Both can consume `contract::decode_envelope` and `schemas/v1/exchange.ts`.
M1.5 extends `fixtures/contracts/v1/manifest.json`.

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

## Next dependency-ready work
M1.4 and M1.7 are dependency-ready and unclaimed; M1.5 extends the M1.3 fixture
manifest. M1.4/M1.5 add their real CI targets when implemented. Lua parity for
the contracts follows the addon in M2. M0.3-M0.6 live checks remain blocked on
a user-operated client. [PR 5](https://github.com/Laymer/Navzeroth/pull/5) merged M1.3 as 2c8628b; earlier handoffs describe
their pre-merge delivery state. This documentation update claims no task and
establishes no release.
