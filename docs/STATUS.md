# Project status

Plan version: 1.0 plus accepted ADR 0001
Current milestone: M1
Current task: M1.3 (VERIFIED locally on feature/M1.3-contracts; draft PR, hosted CI pending)
Task owner: Claude Code (Opus 5.5), single writer
Base revision: bc386c23394ba8ff115e8c8595068f98d25ea5ea (origin/main after PR 4; PR 3 merged as 514d4b0)
Last updated: 2026-10-12
Overall state: M1.1 reviewed/corrected; M1.6 CI and expanded M1.2 merged and verified for their scopes; M1.3-M1.5 and M1.7 remain

## Milestones

| Milestone | Status | Evidence / blockers |
| --- | --- | --- |
| M0 | IN_PROGRESS | M0.1 repository/instructions/toolchain/license/CI inspection complete. M0.2 status/handoff records exist; initial decisions and sources are embedded in the implementation plan. M0.3-M0.6 need a user-operated Forever client, unavailable in this development environment (CLAUDE.md, section Environments). |
| M1 | IN_PROGRESS | M1.1 review/corrections, M1.6 workspace CI and expanded M1.2 VERIFIED and merged in PR 3. M1.3 contracts VERIFIED locally (draft PR). M1.4, M1.5 and M1.7 remain. |
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
finalize versioned schemas/types and drift checks. Claimed by Claude Code on
2026-10-12 from bc386c2. Expected files: crates/core/src/contract/, RoutePlan
completion in crates/core/src/entities/planning.rs, JSON Schema derivations,
schemas/v1/, fixtures/contracts/v1/, tests/contract/, root package.json/lockfile,
an azeroth-cli export-schemas command, CI contract steps and documentation.
Acceptance: unsupported versions, invalid observation states, unknown operators
and incompatible build/namespace contexts are rejected; fixtures pass Rust,
JSON Schema and TypeScript checks as declared; committed schemas/types cannot
drift silently in Linux/Windows CI; fmt, Clippy and workspace tests pass. Use existing validated domain models and the [domain reference](DOMAIN.md).
The [continuation handoff](handoffs/2026-10-12-M1.3-continuation.md) records the
merged baseline; the [Claude Code prompt](prompts/claude-M1.3-contracts.md) defines
the next session's scope and acceptance checks. Contract fixtures needed to verify
M1.3 belong with that change; the broader M1.5 fixture/demo work remains separate.

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
M1.3 local: fmt, locked Clippy and tests passed (41 tests: 38 core, 3 CLI);
schema/TypeScript regeneration deterministic with no drift; pnpm contracts:check
passed for 52 fixtures. Hosted CI for M1.3 is pending on its draft PR.
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
reference resolution (M3) and hosted Windows evidence for the new contract
steps remain open.

## Next dependency-ready work
After M1.3 review/merge: M1.4 (desktop shell with demo mode) and M1.7 (snapshot
store) can consume the contracts; M1.5 extends the fixture manifest. M1.4/M1.5 add their real CI targets when implemented.
M0.3-M0.6 live checks remain blocked on a user-operated client.
[PR 3](https://github.com/Laymer/Navzeroth/pull/3) was merged into main on
2026-10-12 at 11:19:41 UTC as 514d4b0. Earlier handoffs describe their pre-merge
delivery state; the continuation handoff supersedes that state. This documentation
update does not claim M1.3 or establish a release.
