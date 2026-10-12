# Project status

Plan version: 1.0
Current milestone: M1
Current task: M1.2 domain foundation and rules (IN_PROGRESS)
Task owner: ChatGPT/Codex
Base revision: 8d3af68 (CI verified on Linux and Windows)
Last updated: 2026-10-12
Overall state: M1.1 reviewed and VERIFIED for local workspace acceptance; M1.6 VERIFIED for the existing workspace; expanded M1.2 claimed

## Milestones

| Milestone | Status | Evidence / blockers |
| --- | --- | --- |
| M0 | IN_PROGRESS | M0.1 repository/instructions/toolchain/license/CI inspection complete. M0.2 status/handoff records exist; initial decisions and sources are embedded in the implementation plan. M0.3-M0.6 need a user-operated Forever client, unavailable in this development environment (CLAUDE.md, section Environments). |
| M1 | IN_PROGRESS | M1.1 reviewed; M1.6 CI verified. M1.2 active; UI, schemas and storage remain. |
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
M1.2 owner: ChatGPT/Codex. Files: crates/core modules and tests, core dependencies,
CLI banner, domain reference and evidence records. Implement validated primitives,
all 19 section 6.1 entities, observations/freshness and all section 6.6 predicates.
Acceptance: positive/negative JSON tests, uncertainty and freshness scenarios,
full predicate truth/limit tests, locked checks on Linux/Windows. See ADR 0001.

## Completed behavior
Cargo/pnpm workspace setup and M1.1 corrections are complete. CLI rejects
unsupported arguments. Shared instructions permit routine feature-branch pushes
and draft PRs. See [review](reviews/2026-10-12-M1.1.md),
[handoff](handoffs/2026-10-12-M1.1-review.md), and
[ADR 0001](decisions/0001-foundation-sequence.md).

## Verification
WSL/Linux: Rust 1.99.0 formatting, locked Clippy and tests pass (two CLI tests,
one placeholder smoke test); frozen offline pnpm 12.11.2 install passes with
Node 24.15.0 and Corepack 0.36.0. All five Linux/Windows checks passed at 8d3af68; see the [CI handoff](handoffs/2026-10-12-M1.6-ci.md). Main protection was applied and read back.
M1.1 local acceptance does not require future CI. Native UI and live WoW checks
are unavailable; no such behavior is claimed.

## Open findings
No unresolved M1.1 blocker. The placeholder smoke test is replaced by M1.2.
See the review for resolved findings and historical record corrections.

## Next dependency-ready work
Expanded M1.2 is active. M1.3 exchange contracts follows. M0.3-M0.6 live checks remain
blocked on a user-operated client; independent foundation work continues.
