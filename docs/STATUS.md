# Project status

Plan version: 1.0
Current milestone: M1
Current task: M1.2 (dependency-ready; M1.1 complete)
Task owner: unassigned
Base revision: fc8ebc3535e63e04341e815c152cc030ff757abb (M1.1 complete)
Last updated: 2026-10-12
Overall state: M0.1 repository intake complete; M1.1 workspace/toolchain initialization complete (IMPLEMENTED_UNVERIFIED pending CI); M1.2-M1.7 remain

## Milestones

| Milestone | Status | Evidence / blockers |
| --- | --- | --- |
| M0 | IN_PROGRESS | M0.1 repository/instructions/toolchain/license/CI inspection complete. M0.2 status/decision/handoff records already exist from the prior session. M0.3-M0.6 need a user-operated Forever client, unavailable in this development environment (CLAUDE.md, section Environments). |
| M1 | IN_PROGRESS | M1.1 complete: Cargo workspace (azeroth-core, azeroth-cli) and pnpm workspace initialized, Rust 1.99.0 and pnpm 12.11.2 pinned, local commands verified and documented. See [the M1.1 handoff](handoffs/2026-10-12-M1.1-workspace-init.md). Status is IMPLEMENTED_UNVERIFIED, not VERIFIED, because no CI exists yet (that is M1.6) - only local manual runs back this. M1.2-M1.7 remain. |
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
M1.1 is complete; see the checklist in [the M1.1 handoff](handoffs/2026-10-12-M1.1-workspace-init.md). M1.2 ("create the core domain types, observation semantics, schema versioning, and application errors," [plan section 17](../AZEROTH_NAVIGATOR_IMPLEMENTATION_PLAN.md#17-implementation-milestones-and-task-backlog)) is the next dependency-ready task; no owner has claimed it yet.

## Completed behavior
Prepared the implementation plan, shared AGENTS.md instructions, CLAUDE.md entry point, project status, and initial handoff (prior session). M0.1 repository/toolchain/license/CI inspection is complete: working tree was clean, license is BSD-3-Clause, no live Forever client is reachable from this environment.

M1.1 added: `rust-toolchain.toml` pinning Rust 1.99.0 (rustfmt, clippy); a virtual Cargo workspace with two dependency-free placeholder crates, `azeroth-core` (lib) and `azeroth-cli` (bin, links against core); a pnpm workspace (`pnpm-workspace.yaml` matching `apps/*`, currently empty - `apps/desktop` is M1.4's job) with the package manager pinned to pnpm 12.11.2 via `corepack use`; a "Development setup" section in README listing only commands that were actually run; `.gitignore` extended for Node/pnpm tooling. No domain types, contracts, UI, or storage exist yet - those are M1.2-M1.7.

## Verification
Local manual runs only, this session, against rustc/cargo 1.99.0 and pnpm 12.11.2 (both installed fresh on this host - see handoff "Validation" for exact commands and the corepack version-skew issue hit and resolved along the way): `cargo fmt --all --check`, `cargo clippy --locked --workspace --all-targets -- -D warnings`, and `cargo test --locked --workspace` all exit 0 (1 real unit test passes); `pnpm install` exits 0 against the zero-package workspace and produces a committed lockfile.

No CI exists yet (M1.6), so this is IMPLEMENTED_UNVERIFIED by an automated gate, not VERIFIED. Mocked UI, native Windows, and live WoW checks: not applicable - no UI or addon code exists yet.

## Open findings
No implementation review findings or ADRs yet. Forever build capabilities and official API availability remain unverified as described in the plan. This dev host needed rustup and a corepack upgrade (0.24.0 to 0.36.0, to resolve pnpm 12.11.2's binary layout) installed from scratch; neither is a repository change, both are recorded in the M1.1 handoff.

## Next dependency-ready work
M1.2: create the core domain types, observation semantics, schema versioning, and application errors in `crates/core` ([plan section 6](../AZEROTH_NAVIGATOR_IMPLEMENTATION_PLAN.md#6-contracts-and-domain-model)). M1.6 (CI) is a reasonable alternative if the next owner prefers automated enforcement before more logic lands, but M1.2 is a harder prerequisite for M1.3/M1.4/M1.7. M0.3-M0.6 remain blocked pending a user-operated Forever client. Follow the starter prompt in [plan section 23.2](../AZEROTH_NAVIGATOR_IMPLEMENTATION_PLAN.md#232-prompt-to-start-claude-code).
