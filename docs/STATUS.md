# Project status

Plan version: 1.0 plus accepted ADR 0001
Current milestone: M1
Current task: M1.3 (dependency-ready)
Task owner: unassigned
Base revision: e408848 (verified domain implementation; later documentation commit records handoff)
Last updated: 2026-10-12
Overall state: M1.1 reviewed/corrected; M1.6 CI and expanded M1.2 verified for their scopes; M1.3-M1.5 and M1.7 remain

## Milestones

| Milestone | Status | Evidence / blockers |
| --- | --- | --- |
| M0 | IN_PROGRESS | M0.1 repository/instructions/toolchain/license/CI inspection complete. M0.2 status/handoff records exist; initial decisions and sources are embedded in the implementation plan. M0.3-M0.6 need a user-operated Forever client, unavailable in this development environment (CLAUDE.md, section Environments). |
| M1 | IN_PROGRESS | M1.1 reviewed; M1.1 review, M1.6 workspace CI and expanded M1.2 VERIFIED. M1.3 next; UI, shared schemas and storage remain. |
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
M1.3: define snapshot, plan, evidence, dataset and gear exchange contracts;
finalize versioned schemas/types and drift checks. Claim a fresh baseline before
implementation. Use existing validated domain models and the [domain reference](DOMAIN.md).

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
Local formatting, locked Clippy/tests: passed (27 tests: 25 core, 2 CLI).
Rust 1.99.0, Node 24.15.0, Corepack 0.36.0, pnpm 12.11.2.
[Hosted run 38241081691](https://github.com/Laymer/Navzeroth/actions/runs/38241081691)
passed all five Linux/Windows checks for e408848. See the
[M1.6 handoff](handoffs/2026-10-12-M1.6-ci.md) and
[M1.2 handoff](handoffs/2026-10-12-M1.2-domain.md) for exact checks and limitations.
Native desktop packaging, UI, Lua parity and live WoW behavior remain unverified
or unimplemented; pure model tests use synthetic data only.

## Open findings
No unresolved blocker for the completed scopes. See the
[M1.1 review](reviews/2026-10-12-M1.1.md) and
[M1.2 self-review](reviews/2026-10-12-M1.2.md). Complete exchange envelopes,
dataset-wide reference resolution and cross-runtime compatibility are M1.3+ work.

## Next dependency-ready work
M1.3 is unclaimed. M1.4/M1.5 add their real CI targets when implemented.
M0.3-M0.6 live checks remain blocked on a user-operated client.
[Draft PR 3](https://github.com/Laymer/Navzeroth/pull/3) contains the current work;
no merge or release has been performed.
