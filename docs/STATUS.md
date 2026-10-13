# Project status

Plan: 1.0 plus ADRs 0001–0004. Updated: 2026-10-13.
Current task: PR #39 Windows packaging review and integration; next implementation is M4.6.
Owner: Codex root, sole writer; claimed at PR head 1273c57 / main d7ee6d5.
See [reconciliation handoff](handoffs/2026-10-13-reconciliation.md).
Starting main baseline: 08f6daa. Combined implementation: a08f5d7;
6f7fbb6 adds only the coordination documentation prerequisite.
Task PRs through #36 are merged into protected main f2f64a3 after fresh CI.
PR #37 supplies the final documentation. The original worktree and .claude/ remain preserved.

## Milestones

| Milestone | State | Delivered scope / remaining work |
| --- | --- | --- |
| M0 | IN_PROGRESS | Diagnostic addon and harness exist; recorded Forever client checks remain unverified. |
| M1 | IMPLEMENTED_UNVERIFIED | Domain/contracts, fixtures, shell, CI and SQLite foundations implemented and checked; native window interaction not verified. |
| M2 | IN_PROGRESS | Bounded character imports, snapshot storage UI, selected-file watcher, Lua decoder/predicates/plan/HUD foundation. Party transport imports and verified live bindings remain. |
| M3 | IN_PROGRESS | Source/conflict library, immutable revisions, desktop staging/activation/rollback and coverage/source inspection, migration/backup and coverage CLI. Real playable quest/acquisition data remains missing. |
| M4 | IN_PROGRESS | Travel, eligibility and pure deterministic greedy planner. Single-character Planner service/UI with explicit utilities, estimates, exclusions and stopping routes; bounded manual ANAV1 export implemented; automatic step advancement remains. |
| M5 | IN_PROGRESS | Fixed-point gear core, finite-session acquisitions, bounded synthetic Rust/Lua scoring parity. Real weights/catalogs, gear UI and live bindings remain. |
| M6 | IN_PROGRESS | Demo atlas, markers from selected stored snapshots, layers, pin search/filters and keyboard navigation. Reference geography stays distinct from unlocks. Route rendering remains. |
| M7 | IN_PROGRESS | Pure reminders, local parties and per-participant declared-quest eligibility/catch-up with missing/stale gates; joint planning, preparation and live workflows remain. |
| M8 | IN_PROGRESS | Session interval model, timing metadata correction and pure progression/timing analytics. Addon recorder and calibration remain. |
| M9 | IN_PROGRESS | Today/Imports/Atlas/Party/Content/Planner workflows and browser tests; gear/session screens remain; Windows packaging is under PR #39 review (see the PR39 handoff). |
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

## Reconciliation at 2026-10-13

The historical table above describes c357bf1 and remains unchanged as evidence.
The reconciliation branch fixes its open blocker and important domain findings,
adds audited v4 snapshot promotion and finishes M4.4. See the
[reconciliation handoff](handoffs/2026-10-13-reconciliation.md) for regression
coverage and actual checks. PR #40 merged at 301dc60 after all eight hosted jobs passed. M4.5
bounded manual export is implemented in PR #41; see the
[export handoff](handoffs/2026-10-13-M4.5-plan-export.md). Native interaction and
live client behavior remain unverified. Optional findings and the external GTK
limitation remain recorded separately in the original review.

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
Automatic malformed, duplicate, older and incomparable imports preserve current state.
History-only outcomes explain their reasons. SQLite migrations and content backups
preserve snapshot hash/ordering behavior. The watcher uses a single selected path,
500 ms polling, 750 ms stability and bounded retries/cancellation.

Desktop character snapshots are supported; PartySnapshot transport imports remain
explicitly rejected. Local party configuration references individually imported
character profiles (29c3188, 87 affected Rust tests passed, Clippy/fmt passed).
Profile selection persists for the application session.
Imported age comes from backend queries, refreshed every minute in open views.
The atlas still serves the demo reference imagery; a stored position may report a
namespace/build/floor gap instead of inventing a placement. Planner consumes exact imported state/content with explicit manual assumptions; real
gameplay coverage remains missing. Acquisition remains a library foundation.
Lua plan invalidation does not imply completion. Lua gear accepts synthetic inputs.

Live WoW observations/transitions and interactive native window behavior remain
UNVERIFIED. Hosted Windows packaging smoke evidence is revision-scoped in the M9.4
and PR39 handoffs. No socket bridge, input automation, hidden telemetry or forbidden game
access was added. Remaining watcher/platform/model limitations are in the handoffs.

## Next dependency-ready work

M4.6: explicit Lua advancement and stable alternative/invalidation policy, with
replay fixtures. Manual bounded export is delivered in PR #41; automatic advancement
and a live addon import UI remain later tasks. Continue analytical/gear views and
permitted real-content authoring when their inputs are available. Live client
checks need sanitized evidence from the recorded build. PR #39 integrates the Windows
packaging branch after the planner/export reconciliation.

Latest local checks: 495 Rust tests, zero failures, one existing ignored generator;
78 desktop unit tests and seven Chromium workflows; locked fmt/Clippy,
typecheck/lint/build, schema/type drift, contract/transport/content and Lua parity.
PRs #40 and #41 and resulting main d7ee6d5 passed all eight hosted jobs;
GitHub is the authoritative merge/check record.
Native interaction/live checks remain unverified. Releases still need separate
authorization. The branch inventory and ancestry audit are in the reconciliation
handoff; every initially outstanding tip and the preserved planner draft reached
main through PR #40 without rewriting or deleting branches/worktrees.

Merge follow-up: schema-valid UTC values outside JavaScript Date range now display numeric epoch milliseconds, with positive/negative safe-limit and zero regressions. Corrected desktop checks: 65 unit tests and four Chromium workflows passed; synchronized integration hosted CI passed all eight jobs.

## PR #39 packaging review

Owner: Codex /root (sole writer), baseline 1273c57 against main d7ee6d5.
Review and integration authorized by the user; see
[review handoff](handoffs/2026-10-13-PR39-review.md). Original hosted packaging
evidence remains in [M9.4 handoff](handoffs/2026-10-13-M9.4-windows-packaging.md).
Current main status and historical review records were preserved during conflict resolution.
