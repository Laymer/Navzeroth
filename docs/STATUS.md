# Project status

Plan: 1.0 plus ADRs 0001–0004. Updated: 2026-10-13 (wave-1 review recorded).
Current task: none claimed on main. M9.4 Windows packaging automation is claimed by Claude Code on
feature/M9.4-windows-packaging (base 6bb2649, owner request); see its [handoff](handoffs/2026-10-13-M9.4-windows-packaging.md).
The first review of the wave-1 PRs is recorded below; its open findings come before new feature work.
Owner: Codex root (merge coordination); implementation owners are in ADR 0004.
Starting main baseline: 08f6daa. Combined implementation: a08f5d7;
6f7fbb6 adds only the coordination documentation prerequisite.
Task PRs through #36 are merged into protected main f2f64a3 after fresh CI.
PR #37 supplies the final documentation. The original worktree and .claude/ remain preserved.

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
| M9 | IN_PROGRESS | Today/Imports/Atlas workflow and browser tests; Windows installer/addon packaging automation (M9.4, IMPLEMENTED_UNVERIFIED on its branch). Remaining planning/gear/session screens, signing and release notes remain. |
| M10 | TODO | Optional native reference overlay. |
| M11 | DEFERRED_WITH_REASON | No verified official Forever API capability. |
| M12 | TODO | Expansion after focused release. |

## Wave-1 review (2026-10-13)

The wave-1 PRs (9-22, main 8cca94d..08f6daa) had no review before merging: the
planned review workflow failed on a spend limit. A read-only review then ran
at 08f6daa with eight area reviewers, an integration reviewer and adversarial
verification of every blocker and important finding; see the
[wave-1 review record](reviews/2026-10-13-wave1-main.md). Wave 2 (PRs 23-37)
was not part of it and has only the focused reviews listed in its integration
handoff.

At 08f6daa all checks passed (397 Rust tests, 56 desktop unit tests, contract,
fixture and pointer checks; main run 38298517870 green on all seven jobs). The
ADR 0003 frozen files were untouched and the dependency rules held.

Verified findings and their state on c357bf1:

| Severity | Finding | State on c357bf1 |
| --- | --- | --- |
| Blocker | Gear: an item of unknown handedness in the off-hand slot makes a configuration Valid, Complete and definitive (`gear/configuration.rs`) | Open |
| Blocker | Desktop: the UI sent `profile_id` while the native command expected `profileId`, so the native app never reached the Atlas screen | Fixed in wave 2 |
| Important | Gear: `recommendation_data` publishes scores as Known while unknown attributes count as 0 | Open |
| Important | Eligibility: unknown access, choice or sibling prerequisites are reported as Behind with a catch-up path | Open |
| Important | Eligibility: progress NotStarted and turn-in NotReady while turned-in/completed are Unknown | Open |
| Important | Eligibility: a reset-bound quest known turned in is Eligible with no reason | Open |
| Important | Eligibility: `ParticipantFacts` has no DataContext, so another namespace or build can supply class and race | Open |
| Important | Reminders: the default fingerprint repeats unchanged advice on every new export | Open |
| Important | Reminders: a reminder shown without a trusted clock stays suppressed forever | Open |
| Important | Sessions: the summary merge key omits DataContext, so a demo summary blocks a real one | Open |
| Important | Sources: evidence is keyed by EvidenceId alone, so one record cannot support two records | Open |
| Important | Coverage: cycle detection ignores eligibility and objective-state predicates | Open |
| Important | Storage: a snapshot from a new session with an uncertain clock can never become current, and no explicit promotion exists | Open |

One finding was refuted (travel: final return hop readiness). 32 optional
findings are listed in the record unverified. The review's stale-record
findings for STATUS and README were resolved by PR 37.

Status corrections from the review: M4.2, M5.2, M7.1, M3.2, M3.5 and the M2.4
ordering rules stay IMPLEMENTED_UNVERIFIED until their findings above are fixed.
M5.1 (structure only), M5.3, M8.3 pure intervals and the M2.5 Rust corpus met
their declared wave-1 scope, but M8.3 now has the open session-key finding. No
wave-1 task is VERIFIED for live gameplay; M0.3 and M6.1 need the client and
the native window.

Other open items: Dependabot alert GHSA-wrw7-89jp-8q8g (medium) on `glib`
0.18.5, a Linux-only transitive dependency of the Tauri GTK stack since M1.4,
whose automatic update runs fail upstream; the SavedVariables key differs
between the ingest fixtures (`export`, ANAV1) and the addon layout
(`diagnostic_export`), both unverified until M0.5; "Lua static checks" and
"Desktop shell Windows" are still not required checks (owner decision).

## Wave-2 delivery and verification

See [integration handoff](handoffs/2026-10-12-wave2-integration.md) for all twelve
PR links, checked commits, CI runs, actual reviews, limitations and next tasks.
[ADR 0004](decisions/0004-wave2.md) records ownership and frozen interfaces.
Each task also has a claim/handoff. The user authorized ready-and-merge after CI;
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

First fix the open blocker and important findings of the wave-1 review above
(gear, eligibility, reminders, sessions, sources, coverage, storage promotion).
Then continue with the items below.

The PR stacks are merged after protected CI; see the [merge audit](handoffs/2026-10-12-wave2-merge.md). Integrate party persistence,
planner/export UI and analytical display using existing explicit DTO boundaries;
author permitted real content and explicit goal/gear models. Live client checks
need sanitized evidence from the recorded build. Releases and PR/main merges need
separate applicable authorization.

Merge follow-up: schema-valid UTC values outside JavaScript Date range now display numeric epoch milliseconds, with positive/negative safe-limit and zero regressions. Corrected desktop checks: 65 unit tests and four Chromium workflows passed; synchronized integration hosted CI passed all eight jobs.
