# Project status

Plan version: 1.0
Current milestone: M1
Current task: M1.1
Task owner: Claude Code
Base revision: 7182a931c14d6e47209c62c7e837535ae03ae047 (M1.1 baseline)
Last updated: 2026-10-12
Overall state: M0.1 repository intake complete; M1.1 workspace initialization claimed and in progress

## Milestones

| Milestone | Status | Evidence / blockers |
| --- | --- | --- |
| M0 | IN_PROGRESS | M0.1 repository/instructions/toolchain/license/CI inspection complete (see [this handoff](handoffs/2026-10-12-M1.1-workspace-init.md)). M0.2 status/decision/handoff records already exist from the prior session. M0.3-M0.6 need a user-operated Forever client, unavailable in this development environment (CLAUDE.md, section Environments). |
| M1 | IN_PROGRESS | M1.1 claimed; workspace/toolchain initialization underway. M1.2-M1.7 remain. |
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
Complete M1.1 in [plan section 17](../AZEROTH_NAVIGATOR_IMPLEMENTATION_PLAN.md#17-implementation-milestones-and-task-backlog) / [section 5](../AZEROTH_NAVIGATOR_IMPLEMENTATION_PLAN.md#5-repository-structure-and-toolchain): initialize the Cargo and pnpm workspaces, pin a tested Rust toolchain and package manager, and document the real (not aspirational) local development commands. Scope is limited to workspace/toolchain initialization; domain types (M1.2), contracts (M1.3), the Tauri/React shell (M1.4), fixtures (M1.5), CI (M1.6), and SQLite migrations (M1.7) are separate follow-up tasks.

## Completed behavior
Prepared the implementation plan, shared AGENTS.md instructions, CLAUDE.md entry point, project status, and initial handoff (prior session). M0.1 repository/toolchain/license/CI inspection is complete this session: working tree was clean, license is BSD-3-Clause, no toolchain or CI files existed, no live Forever client is reachable from this environment. No application or addon behavior has been implemented yet.

## Verification
Documentation checks: instruction templates match plan section 22; the plan matches the previously prepared source; Markdown links resolve to files and headings.

Core tests, mocked UI, native Windows, and live WoW checks: not run; no implementation exists yet. No live capability has been verified.

## Open findings
No implementation review findings or ADRs yet. Forever build capabilities and official API availability remain unverified as described in the plan.

## Next dependency-ready work
M1.1: initialize the Rust/pnpm workspaces and pin tested toolchain/dependency versions (in progress this session). Continue independent M1 foundation work since live-client checks (M0.3-M0.6) remain unavailable. Follow the starter prompt in [plan section 23.2](../AZEROTH_NAVIGATOR_IMPLEMENTATION_PLAN.md#232-prompt-to-start-claude-code).
