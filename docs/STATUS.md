# Project status

Plan: 1.0 plus ADRs 0001–0004. Updated: 2026-10-12.
Current task: wave-2 integration documentation and hosted verification.
Owner: Codex root; three isolated implementation workers (see ADR 0004).
Starting main baseline: 08f6daa. Combined implementation: a08f5d7;
6f7fbb6 adds only the coordination documentation prerequisite.
All work is in feature branches/draft PRs; main and original .claude/ are preserved.

## Milestones

| Milestone | State | Delivered scope / remaining work |
| --- | --- | --- |
| M0 | IN_PROGRESS | Diagnostic addon and harness exist; recorded Forever client checks remain unverified. |
| M1 | IMPLEMENTED_UNVERIFIED | Domain/contracts, fixtures, shell, CI and SQLite foundations implemented and checked; native window interaction not verified. |
| M2 | IN_PROGRESS | Bounded text/file imports, snapshot storage UI, selected-file watcher, Lua decoder/predicates/plan/HUD foundation. Party storage and verified live bindings remain. |
| M3 | IN_PROGRESS | Source/conflict library, immutable contextual content revisions, activation/rollback, migration/backup and coverage CLI. Real playable quest/acquisition data remains missing. |
| M4 | IN_PROGRESS | Travel, eligibility and pure deterministic greedy planner. Caller supplies clusters/goal utilities; planner UI/export integration remains. |
| M5 | IN_PROGRESS | Fixed-point gear core, finite-session acquisitions, bounded synthetic Rust/Lua scoring parity. Real weights/catalogs, gear UI and live bindings remain. |
| M6 | IN_PROGRESS | Demo atlas, markers from selected stored snapshots, layers, pin search/filters and keyboard navigation. Reference geography stays distinct from unlocks. Route rendering remains. |
| M7 | IN_PROGRESS | Pure reminder foundations; integrated party/preparation workflows remain. |
| M8 | IN_PROGRESS | Session interval model, timing metadata correction and pure progression/timing analytics. Addon recorder and calibration remain. |
| M9 | IN_PROGRESS | Today/Imports/Atlas workflow and browser tests; remaining planning/gear/session screens and packaging remain. |
| M10 | TODO | Optional native reference overlay. |
| M11 | DEFERRED_WITH_REASON | No verified official Forever API capability. |
| M12 | TODO | Expansion after focused release. |

## Wave-2 delivery and verification

See [integration handoff](handoffs/2026-10-12-wave2-integration.md) for all twelve
PR links, checked commits, CI runs, actual reviews, limitations and next tasks.
[ADR 0004](decisions/0004-wave2.md) records ownership and frozen interfaces.
Each task also has a claim/handoff on its feature branch. Review remains open;
implemented synthetic foundations do not establish live gameplay verification.

Combined local checks: locked Rust fmt/Clippy and workspace tests excluding the
native desktop passed (463 tests, 0 failures, 1 existing ignored generator).
Desktop typecheck/lint/build and 64 unit tests passed. Four Chromium command-mock
workflows passed. Contract/schema drift checks passed; 73 contract and 33 transport
fixtures validated; Lua parity and content coverage checks passed; four committed
imagery pointers checked. Combined hosted CI is recorded in the integration handoff.

## Decisions and limitations

Exchange schemas are unchanged. Desktop commands have explicit application DTOs;
profile IDs resolve full character/edition/build/interface/namespace/fixture keys.
Malformed, duplicate, older and incomparable imports preserve current state.
History-only outcomes explain their reasons. SQLite migrations and content backups
preserve snapshot hash/ordering behavior. The watcher uses a single selected path,
500 ms polling, 750 ms stability and bounded retries/cancellation.

Desktop character snapshots are supported; party imports are explicitly rejected
until storage supports them. Profile selection persists for the application session.
Imported age comes from backend queries, refreshed every minute in open views.
The atlas still serves the demo reference imagery; a stored position may report a
namespace/build/floor gap instead of inventing a placement. Planner/acquisition
outputs are independent library foundations, not integrated production advice.
Lua plan invalidation does not imply completion. Lua gear accepts synthetic inputs.

Live WoW observations/transitions, native window interaction and packaging remain
UNVERIFIED. No socket bridge, input automation, hidden telemetry or forbidden game
access was added. Remaining watcher/platform/model limitations are in the handoffs.

## Next dependency-ready work

Review the draft PRs and their prerequisite stacks. Integrate party persistence,
planner/export UI and analytical display using existing explicit DTO boundaries;
author permitted real content and explicit goal/gear models. Live client checks
need sanitized evidence from the recorded build. Releases and PR/main merges need
separate applicable authorization.
