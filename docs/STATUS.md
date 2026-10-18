# Project status

Updated: 2026-10-18.

Independent UI correction active: Codex `/root`, `fix/single-active-help`, baseline
`8da8301`; [claim](claims/2026-10-18-single-active-help.md),
[handoff](handoffs/2026-10-18-single-active-help.md). This fixes the competing
hover/keyboard help exposed by pending desktop continuation PR #198. Exact-head
checks precede integration; no release or acceptance state is changed.

## Current task

**Continuation 9 — performance and release materials:** Codex `/root`, sole writer,
branch `feat/continuation-performance-materials`, baseline `d34b417` (continuation 8
pending). [Claim](claims/2026-10-18-performance-materials.md),
[handoff](handoffs/2026-10-18-performance-materials.md),
[reproducible workloads and WSL measurements](data/PERFORMANCE_SLICES.md).
Optimized synthetic measurements are recorded with raw samples and exact source;
ordinary Windows performance remains unverified. Complete dependency notices,
MPL source archives and SDK terms are retained and checked before packaging and
after hosted installation. Review and exact-head checks precede integration.
Coverage, stability, development RC publication and Pages remain separate stages.

**Continuation 8 — desktop workflow and installer completion:** Codex `/root`,
sole writer, branch `feat/continuation-desktop-completion`, baseline `896539e`.
[Claim](claims/2026-10-18-desktop-completion.md),
[handoff](handoffs/2026-10-18-desktop-completion.md),
[workflow](data/LOCAL_DESKTOP_WORKFLOW.md), [PR #198](https://github.com/Laymer/Navzeroth/pull/198).
Desktop engineering and focused local review/checks are complete; exact-head
hosted/native checks remain pending. Continuation 7 merged as
[PR #197](https://github.com/Laymer/Navzeroth/pull/197) at `8da8301` after
independent review, all nine exact-head checks and source parity.
Continuation 6 merged as [PR #196](https://github.com/Laymer/Navzeroth/pull/196)
at `49472d0` after independent review, all nine exact-head checks and source parity.
[Delivery handoff](handoffs/2026-10-18-bound-delivery.md).
Continuation 5 merged as [PR #195](https://github.com/Laymer/Navzeroth/pull/195)
at `0151eb1` after independent review, all nine exact-head checks and source
workflow. Media remains local-only, uncalibrated and research-only.
[Media handoff](handoffs/2026-10-18-media-tooltips.md).
Continuation 4 merged as [PR #194](https://github.com/Laymer/Navzeroth/pull/194)
at `b590eae` after independent review, all nine exact-head checks and source
workflow. Expanded gameplay mappings and bound guide facts remain research-only;
observed turn-ins cannot establish universal prerequisites or eligibility.
Continuation 3 merged as [PR #193](https://github.com/Laymer/Navzeroth/pull/193)
at `b74549f` after independent review, all nine exact-head checks, both source
platforms and parity. [Cache development receipt](handoffs/2026-10-18-cache-development-check.md)
records native read-only capture and pure parsing; native Rust cache-stage
end-to-end and live locale remain unverified. Private copies were removed.

Continuation 1 merged as PR #191 at `167a454`. Continuation 2 merged as
[PR #192](https://github.com/Laymer/Navzeroth/pull/192) at `1cb6bb9` after independent
review, all nine exact-head checks and source workflow. Build 70450/enUS static
archive interruption, cancellation, resume/fresh digest equality and failed
replacement preservation passed in Linux/WSL; private stores were removed.
[Lifecycle handoff](handoffs/2026-10-18-extraction-lifecycle.md) links the exact
receipt. Research activation never grants playable availability. Ordinary Windows,
interactive desktop and live acceptance remain separate and unverified.

Original checkout and `.claude/` are preserved. Last measured readiness (PR #191)
reports 69 blockers, candidate/stable false, zero verified packs/evidence.
[Inventory](releases/2026-10-18-blocker-inventory.md). The pending post-launch target
supersedes historical 70374 paragraphs below. No continuation release has been
published; 0.9.2 remains latest.

**1.0.0 program (ADR [0006](decisions/0006-v1-program.md), 2026-10-17).** The owner
re-targeted 1.0.0 to the first stable live build after the 4 Nov 2026 launch and a
declared slice (Human, levels 1-20, Northshire/Elwynn/Westfall/Redridge, the
Deadmines, every Human class, solo and duo), with hybrid delivery and local-only sources (ADR 0006 §11; the earlier
owner-reported grant path was withdrawn). Tasks V0.1 to V6.2 are tracked in the root [TODO.md](../TODO.md).
V0 (owner and sole writer Claude Code, Opus 5.5; baseline `3b19812`; branch
`docs/v1-program`; claim [2026-10-17-v0](claims/2026-10-17-v0.md)) published 0.9.2
and recorded the ADR; see the [V0 handoff](handoffs/2026-10-17-v0.md). The 0.9.2
program (ADR [0005](decisions/0005-v092-program.md), G1 to G8) is complete.

**Latest published version: 0.9.2**, a developer prerelease published
2026-10-17T11:34:32Z from tag `v0.9.2` (tag object `fe63072`, commit `3b19812`);
receipt: [0.9.2 publication record](handoffs/2026-10-17-v0.9.2-publication.md). The
previous prerelease, 0.9.1 (2026-10-16, commit `92f3f51`), has its own
[publication record](handoffs/2026-10-16-v0.9.1-publication.md). The interrupted 0.9.0
publication was superseded by 0.9.1 and is not claimed. At 0.9.2 publication, readiness reported
publication allowed, stable and candidate readiness false, 66 blockers (the same
count at 0.9.1 and 0.9.2; historical, now 69),
zero verified evidence files and zero verified content packs. Full 1.0.0 content and candidate-bound acceptance gates remain unmet.
Private research catalogs remain outside public assets.

GitHub branch protection on `main` (verified read-only with `gh api` on 2026-10-17)
requires nine status checks with strict up-to-date branches: Rust format, Rust Linux,
Rust Windows, pnpm Linux, pnpm Windows, Lua static checks, Desktop shell Windows,
Mocked desktop Chromium and Windows installer and addon package. The source knowledge
workflow is not among them and passes by process convention only. Earlier records that
say "only five checks" were written before the setting changed. Some earlier commits use the `release:`
subject prefix, which predates the plan 21.4 task-ID convention.

## Preceding work

The following records describe preceding implementation and release preparation. They
are historical; where they say 0.8.1 or 0.9.0 is the latest published version, the
current section above governs.

Plan: 1.0 plus ADRs 0001–0004 and [local-data contract](../localdata.md).
Preceding task (completed by PR #154): independent private local-client-data subsystem.
Owner and sole writer: Codex `/root`; continuation baseline
`789f39272dc00c3f72ec6881a4a69fc1383a1948`, continuation branch
`feat/local-data-extraction`, claim `96a29d0`. The continuation was fast-forwarded
onto `feat/local-data-foundation` for draft PR #154; main was not changed. No
subagents. See the
[extraction handoff](handoffs/2026-10-16-local-data-extraction.md) and
[implemented workflow](data/LOCAL_EXTRACTION.md). Retained pinned native/parser
sources and metadata, static offline archive access, isolated resource-limited
self-worker, bounded WDC research readers, private content-addressed catalogs,
CLI extraction/verification/coverage and desktop inventory flow are implemented.
Canonical signed-receipt primitives and delivery-policy contracts exist; full
private-inventory release integration remains blocked. Cache/media/tooltip and
gameplay projection work remains, together with durable settings/checkpoints,
installer choices and opt-in update detection. No complete extractor/release claim.
Actual selected-installation metadata reports70425/enUS; this is not live
acceptance or70374 content. The target remains unchanged. Private client data
must stay outside Git/public artifacts. Existing release gates remain active.
Frozen implementation `af31f5d` processed 1,346 table candidates and decoded 505
from 70425/enUS. All four presets produced the same private semantic catalog;
the handoff records its hash, measured workload, exact hosted checks and gaps.

Historical interrupted task: 0.9.0 prerelease publication (superseded by 0.9.1).
Read-only GitHub reconciliation at the baseline found source workflow38803371318
and Windows package38803371406 successful, CI38803371336 cancelled, latest
published release still0.8.1. Cancellation is not a passed resulting-main gate.
See the [0.9.0 release handoff](handoffs/2026-10-15-v0.9.0-release.md) for actual
checks and publication state. Preparation alone is not publication; the previous
published preview was 0.8.1 until 0.9.1 was published.
The [integration handoff](handoffs/2026-10-15-v1-foundation-integration.md) and
[development evidence closure](handoffs/2026-10-15-development-evidence-closure.md)
record the preceding work. No final 1.0.0 acceptance is inferred from this release.

The target remains Forever `1.60.1 / 70374 / interface 16001`, across all nine
content domains. The user selected ForeverDB instead of Wowhead on 2026-10-15;
the new local-data direction supersedes it as an extraction dependency. ForeverDB
is a capability/provenance reference, not an upstream dependency. See the
[historical provider review](sources/foreverdb.md). Addon 0.34.0 was retrieved from the publisher-linked Wago release and inspected
as inert files. It advertises GPLv3; corresponding-source/file scope and 70374
validation are still pending. Publisher metadata currently names origin build
70338, with separate Classic-derived and player-observed facts. The unsent
Wowhead request is superseded. No ForeverDB material has been admitted; all nine target
inventories remain explicitly unknown. The read-only `stage-foreverdb` command
now validates pinned local quest files and separates 557 Forever-labelled from
3,491 Classic-derived source quest records (373 overlapping IDs). This is raw
staging, with no playable activation or complete-target coverage claim.

Source-independence continuation: the user requests a path requiring no publisher
reply. The [independent source strategy](sources/independent-source-strategy.md)
compares published-licence alternatives and defines retained inputs/offline builds
plus original Forever observations. Provider correspondence is optional; specific
rights, target coverage and operator acceptance remain required. See the
[research handoff](handoffs/2026-10-15-independent-source-strategy.md).

Independent build continuation: four narrow published-licence scopes are now
admitted (AtlasLootClassic declarations, Mapzeroth Retail node literals, a
WoWSims source pointer and ATT Forever Riverglades point declarations), with nine
original/licence objects retained in Git.
The [offline pipeline](data/SOURCE_PIPELINE.md) builds a deterministic uniform
source-knowledge catalog, preserving source origin, all candidates, conflicts,
unknowns and reliable/trusted-fresh precedence. Optional bounded GitHub metadata
checks discover updates without changing admission. Linux/Windows automation
compares offline products. The current 1,685 reference facts contain zero
verified target-usable facts; all nine required inventories remain unknown. This
is not an activated gameplay pack or a complete 70374 corpus. See the
[pipeline handoff](handoffs/2026-10-15-offline-source-pipeline.md) for actual checks
and integration state. Publisher replies are not required for these admitted
works. Complete target content and actual observations remain required.

Development acceptance continuation: native Windows PowerShell 5.1 is reachable
from the Linux development session. All 118 synthetic helper checks passed there;
these do not establish interactive Windows acceptance. With the game closed, the
operator-authorized addon update installed all 21 files from checked main
`e45eed1`; prior addon/data were backed up and SavedVariables hashes stayed
unchanged. A permitted persisted 70374 snapshot now parses (level 30, 30 quest
records, 19 equipment slots, uncertain clock). The operator confirmed displayed build/level and no
errors after probe, snapshot and reload. A seven-event manual session validates;
both trusted capture and uncertainty after reload are persisted. The separate
quest-completion getter is integrated in PR149. Checked main840939f is installed
with all 21 files verified, private backup and unchanged SavedVariables. The operator
confirmed a level-13 capture with exactly five completed/nine incomplete quests,
all 14 completion flags readable and no errors. PR151's explicit quest-turn-in mode
passed all nine checks and merged as `83f38c8`; all resulting-main checks passed. Its byte-verified package is installed
with unchanged SavedVariables. Actual one-quest turn-in recording validates
(five events/one milestone), and isolated desktop service import preserves
41,604 ms as unclassified, zero observed activity. The operator confirmed idle
after reload. This remains development evidence;
final candidate acceptance and actual restricted/paused cases stay unverified. See the
[milestone handoff](handoffs/2026-10-15-session-quest-milestones.md).
The [integration closure](handoffs/2026-10-15-development-evidence-closure.md) records
actual checks and artifacts. The [development capture guide](compatibility/DEVELOPMENT_CAPTURE.md) explains
the new bounded inspection command. No release acceptance row is promoted.

Version 0.8.1 was the published developer preview when this paragraph was written;
0.9.1 has since been published. Complete licensed content,
remaining live advancement/deviation bindings, complete session milestone
observations, final operator acceptance
and full-corpus Windows measurements remain unavailable. The Linux development
session and hosted Windows smoke checks do not establish interactive acceptance.
The narrow user-reported build/diagnostic result is not candidate-bound acceptance.
Freeze 1.0.0 versions only after complete licensed content, static readiness and
production work are complete. Then run candidate-bound acceptance; publish only
after the full release gate passes. See the [release contract](../release/v1/README.md). The identity image is deferred until
a published and verified 1.0.0 release, as authorized in the release plan.

## Previous program closure (historical scope)

Owner: Codex /root. Latest published preview at that time was v0.8.1 atd11853e,
published2026-10-14T18:02:38Z after reviewed isolated task/release PRs,
allnine exact-source PR/main gates, both tag jobs and verified five-file assets.
Actual822 portable Rust/195desktop/40 mocked browser tests and strict/schema/
content/pointer/PNG checks pass on code identical to the published source.
Both local pipeline repairs work against the actual Forever directory; local pack
build/verify remains distribution-ineligible and that pack was not uploaded.
All79 M0–M12 task IDs have explicit dispositions. No dependency-ready engineering
remains; native/live/model/source/API and focused-expansion prerequisites stay
blocked/deferred or unverified. A developer preview is not focused-release acceptance.
See [final publication/evidence/unblock audit](handoffs/2026-10-14-final-program-audit.md).
The final docs PR adds records only and preserves the published product source.

Today/Settings, durable restrictions/fallback, opted-in local review/tombstones,
readable explanations and typed historical recovery are implemented in published0.7.
Wave8 UI engineering is implemented in published0.8; wave9 pipeline repairs are published0.8.1; native/live acceptance
remains unverified. The completed wave9 batch implements two
finite repairs: typed explicit-empty map_embeddings pack comparison and an existing
Forever sources.json registry sidecar. Preserve schema1/source policies and unknown
redistribution rights. See [current task ledger](IMPLEMENTATION_TASKS.md).
Historical scheduling paragraphs below describe their original releases; Today and
M7.6 preparation infrastructure are not current ready tasks. Missing verified
Deadmines records, playable sources/models and native/live evidence remain external
prerequisites. [Wave8 ownership](handoffs/2026-10-14-wave8-coordination.md).
Native interactive/live/source acceptance remains explicitly unverified or blocked.
User authorizes isolated parallel task PRs, protected-main integration, tags and
publication under the approved repeat-loop plan. See
[wave-4 coordination](handoffs/2026-10-13-wave4-coordination.md).
See [task ledger](IMPLEMENTATION_TASKS.md) for every M0–M12 task.
Historical preparation/authorization statements below describe their original scope.
See [wave-3 coordination](handoffs/2026-10-13-wave3-coordination.md) for ownership,
frozen boundaries and evidence scope. v0.2.0 is published; no new tag/release
is part of this wave.
See [reconciliation handoff](handoffs/2026-10-13-reconciliation.md).
Starting main baseline: 08f6daa. Combined implementation: a08f5d7;
6f7fbb6 adds only the coordination documentation prerequisite.
Task PRs through #36 are merged into protected main f2f64a3 after fresh CI.
PR #37 supplies the final documentation. The original worktree and .claude/ remain preserved.

## Milestones

| Milestone | State | Delivered scope / remaining work |
| --- | --- | --- |
| M0 | IN_PROGRESS | Diagnostic addon/harness and limited 70338 diagnostic, level and clock-age reports exist; consolidated live checks remain unverified. |
| M1 | IMPLEMENTED_UNVERIFIED | Domain/contracts, fixtures, shell, CI and SQLite foundations implemented and checked; native window interaction not verified. |
| M2 | IN_PROGRESS | Bounded character imports/storage/watcher, guarded manual 70338 quest/equipment capture, runtime plan paste/HUD and local session transport implemented; candidate bounded HUD appearance and conservative UI gate preserve unknown/session-only movement. Party transport and consolidated live acceptance remain. |
| M3 | IN_PROGRESS | Source/conflict library, immutable revisions, desktop content workflows, bounded recovery/backup, coverage CLI and typed local content packs implemented. Real playable quest/acquisition data remains missing. |
| M4 | IN_PROGRESS | Travel/eligibility/greedy Planner, manual joint planning, bounded single-character export and conservative Lua advancement implemented. Conservative automatic clustering and guarded joint export are implemented; real stage mappings and live acceptance remain. |
| M5 | IN_PROGRESS | Fixed-point gear/acquisition comparisons, editable Gear screen and synthetic Rust/Lua parity implemented. Finite route-derived alternatives and typed explanations are implemented; real weights/catalogs and live validation remain. |
| M6 | IN_PROGRESS | Atlas search/keyboard navigation and bounded200-pin pages, dense synthetic measurements and narrow/text/forced-color checks implemented; candidate route hop selection/pages and exact-frame endpoint fit preserve geometry gaps. Native interaction and verified real road geometry remain. |
| M7 | IN_PROGRESS | Pure reminders, local parties and per-participant declared-quest eligibility/catch-up with missing/stale gates; manual joint planning, local proposal/acceptance and reminder controls are implemented; bounded preparation infrastructure is implemented; verified records, real rules and live workflows remain. |
| M8 | IN_PROGRESS | Session interval/analytics model, bounded local imports, immutable migration-v5 history, opt-in manual recorder and Sessions screen implemented. Conditional comparison/calibration engineering is implemented; complete real samples and consolidated live checks remain. |
| M9 | IN_PROGRESS | Recovery/diagnostics, cancelable guarded planning and scoped synthetic performance checks implemented; v0.9.1 is the latest published preview (0.8.1 when this row was written). Native/live acceptance, ordinary Windows performance and final coverage consolidation remain. |
| M10 | IMPLEMENTED_UNVERIFIED | Ordinary read-only native reference window and saved-plan projection implemented; candidate opt-in focused-main shortcut/interaction control is implemented, native display acceptance remains unverified. |
| M11 | DEFERRED_WITH_REASON | No verified official Forever API capability. |
| M12 | BLOCKED | Focused-release/source/coverage prerequisites remain unmet; local submission foundations exist, hosting is deferred. |

## Wave-1 review (2026-10-13; resolved)

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

Verified findings and their state. The "State on c357bf1" column is the original
record; the "Resolved in code" column (added 2026-10-16, re-checked against main
`92f3f51`) gives the current location of each fix. The blocker and all 12
important findings are fixed in code:

| Severity | Finding | State on c357bf1 | Resolved in code |
| --- | --- | --- | --- |
| Blocker | Gear: an item of unknown handedness in the off-hand slot makes a configuration Valid, Complete and definitive (`gear/configuration.rs`) | Open | Fixed: `gear/configuration.rs` (`UnknownTwoHanded` check on main-hand and off-hand slots, about lines 228-241); tests in `crates/core/tests/gear_configuration.rs` |
| Blocker | Desktop: the UI sent `profile_id` while the native command expected `profileId`, so the native app never reached the Atlas screen | Fixed in wave 2 | Fixed in wave 2 |
| Important | Gear: `recommendation_data` publishes scores as Known while unknown attributes count as 0 | Open | Fixed: `gear/configuration.rs` `recommendation_data` (baseline score Known only when the resolution is complete and certainty is Complete) |
| Important | Eligibility: unknown access, choice or sibling prerequisites are reported as Behind with a catch-up path | Open | Fixed: `eligibility.rs` acceptance outcome (about 806-827) yields Unknown while any component or reason is unresolved |
| Important | Eligibility: progress NotStarted and turn-in NotReady while turned-in/completed are Unknown | Open | Fixed: `eligibility.rs` progress derivation (about 830-858) |
| Important | Eligibility: a reset-bound quest known turned in is Eligible with no reason | Open | Fixed: `eligibility.rs` records `ResetStateUnknown` (about line 720) |
| Important | Eligibility: `ParticipantFacts` has no DataContext, so another namespace or build can supply class and race | Open | Fixed: `ParticipantFacts.context` (`eligibility.rs:50-56`) |
| Important | Reminders: the default fingerprint repeats unchanged advice on every new export | Open | Fixed: default `FingerprintBasis::OutcomeOnly` (`reminders.rs` about line 89) |
| Important | Reminders: a reminder shown without a trusted clock stays suppressed forever | Open | Fixed: `reminders.rs` `cooldown_gate` (session-relative cooldown, re-arm, new-session release) |
| Important | Sessions: the summary merge key omits DataContext, so a demo summary blocks a real one | Open | Fixed: `SessionKey = (DataContext, CharacterId, SessionId)` (`sessions.rs` about line 610) |
| Important | Sources: evidence is keyed by EvidenceId alone, so one record cannot support two records | Open | Fixed: records keyed `(RecordKey, EvidenceId)` (`sources.rs` about line 458) |
| Important | Coverage: cycle detection ignores eligibility and objective-state predicates | Open | Fixed: `coverage.rs` builds cycle edges from `Prerequisites` and `Eligibility` sites, including objective owners (about 840-878) |
| Important | Storage: a snapshot from a new session with an uncertain clock can never become current, and no explicit promotion exists | Open | Fixed: `SqliteStore::promote_snapshot` (compare-and-swap, with audit) (`crates/storage/src/sqlite.rs` about line 86) |

One finding was refuted (travel: final return hop readiness). 32 optional
findings are listed in the record unverified. The review's stale-record
findings for STATUS and README were resolved by PR 37.

Status corrections from the review: M4.2, M5.2, M7.1, M3.2, M3.5 and the M2.4
ordering rules stay IMPLEMENTED_UNVERIFIED for live gameplay (their findings above are
now fixed in code; only live verification remains).
M5.1 (structure only), M5.3, M8.3 pure intervals and the M2.5 Rust corpus met
their declared wave-1 scope; the M8.3 session-key finding is fixed in code. No
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

M4.6 pure advancement is implemented with synthetic replay tests; see its
[handoff](handoffs/2026-10-13-M4.6-plan-advancement.md). M0.4 client capability
evidence is needed before M2.1 real-character observation adapters and snapshot
export can be verified. Manual bounded export is delivered in PR #41; automatic advancement
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

## v0.1.0 preview publication

User requested preparation and GitHub publication on 2026-10-13.
See [release notes](releases/0.1.0.md) and [release handoff](handoffs/2026-10-13-v0.1.0-release.md).
PR #39 merged as 6231d79; all eight post-merge CI jobs passed (38375740810).
Full live-game release gates remain unverified; the 0.x developer preview is marked
as a prerelease under the existing workflow convention.

## M4.6 implementation

Participant-scoped objective completion, sticky preloaded alternatives and explicit
revalidation are implemented on feature/M4.6-plan-advancement. Unknown/stale
completion never advances; alternative IDs without readiness declarations stay
unavailable. Schema v1 remains unchanged. No live bindings or user-facing addon
import/export UI are implied. See the task handoff for actual checks and review.
v0.1.0 was published as a prerelease at a0ee6bc, with eight main CI jobs and tag
packaging run 38378570735 passing. The immutable release is unchanged by this work.


## M2.1 snapshot capture progress

User merged PR #43 at 09e5e69. M2.1 is claimed on feature/M2.1-snapshot-export.
The user supplied a diagnostic SavedVariables report and confirmed English client
language and retrieval after the instructed reload/logout. Sanitized evidence records
1.60.1.70338 / interface 16001; API presence remains distinct from readable values.
No dataset was relabeled from 70205 to 70338.

Pure bounded ANAV1 encoding and context-gated observation providers are implemented,
with an explicit experimental /anavdiag snapshot <anonymous-profile-id> command
for 70338. Only a guarded player-level read is attempted; other fields remain
unknown/absent. UTC/session elapsed and locale code remain unobserved. Desktop
SavedVariables defaults are AzerothNavigatorDB / snapshot_export.
See [handoff](handoffs/2026-10-13-M2.1-snapshot-export.md) for checks and limitations.
Next environment check: actual manual level capture, SavedVariables persistence and
desktop import on the recorded client. Next implementation: verified clocks and
additional field adapters after readability/restriction evidence. M0/M2 live acceptance
remains unverified; no new release or main merge is included.


## M2.1 reported live result

PR #44 merged at d5326c2. The user reports the imported preview snapshot displays
the correct level (12), one positive manual-flow sample. See the
[capability record](compatibility/BUILD_CAPABILITIES.md#manual-snapshot-follow-up--level-12)
and [handoff](handoffs/2026-10-13-M2.1-snapshot-export.md).
This verifies the level-only flow by user report in those conditions; the previously
recorded pre-test limitations remain historical. Clock/freshness, locale code,
equipment, quests, position, restrictions and plan execution remain unverified.


## v0.2.0 capture timing / publication

User authorized implementation, protected-main integration, tag and GitHub release.
See [handoff](handoffs/2026-10-13-v0.2.0-capture-timing.md) and
[release notes](releases/0.2.0.md). Clock readings stay uncertain until explicit
operator UTC comparison/confirmation; drift/reversals/failures and reload revoke
trust. No elapsed gameplay time is exported. Actual clock API/confirmation behavior
on 70338 remains unverified; level-only flow retains its one user-reported result.
Six version declarations and workspace lock entries are 0.2.0. Schema/layout v1
remain unchanged; release is a developer prerelease, not full M0/M2 acceptance.

## Wave-3 clock follow-up

The supplied user delivery plan reports trusted capture age and uncertain age after
reload on the recorded 70338 client. This is limited user-reported evidence; installed
revision, raw export, exact age and restricted-context checks were not supplied.
Quest/equipment/position bindings, addon plan UI and session workflows remain
experimental until the consolidated live checklist is evidenced.

## 0.3.0-rc.1 delivery wave

See [integration audit](handoffs/2026-10-13-wave3-integration.md),
[candidate notes](releases/0.3.0-rc.1.md) and the
[consolidated live checklist](compatibility/RC1_LIVE_CHECKLIST.md).
Quest capture, runtime plan paste/HUD, conservative equipment identities and position
readability diagnostics remain explicitly manual and experimental on 70338. Cached
captures after reload are labelled and retain original timestamps; runtime plans
clear on reload. Missing authored completion mappings never advance steps.

Gear services/screens expose current, theoretical, equip-eligible, guaranteed
individual acquisition and conditional practical comparisons. Models, categories,
proficiencies, hand relationships and effort assumptions are explicitly supplied;
unknown critical baseline/acquisition inputs block practical advice. No real tables
are invented. Same-profile refresh preserves manual model and planning edits while
invalidating results; removed catalog references are cleared safely.

Sessions use separate bounded ANAVSESSION1 local transport and migration v5 immutable
context/fixture-scoped history, duplicate idempotence, atomic conflict/capacity
refusal and explicit deletion. Manual recorder never samples in background or
resumes across reload. Unclassified intervals, gaps, progression and UTC uncertainty
remain distinct; calibration remains deferred.

Planner route inspection recomputes current request, selected travel and stopping
route; exact dataset projection permits labelled schematic endpoint links only.
Geometry/image gaps and incompatible spaces keep ordered travel inspectable.

Versions agree at 0.3.0-rc.1; CI produces installer/addon/checksums/notes/source commit
artifacts without tag creation or GitHub publication. Published releases remain
unchanged. Actual current-head reviews/checks, ordered merges and downloaded artifact
verification are recorded in the integration audit; no live results are inferred
from mocked browser flows or hosted packaging.

## v0.3.0 release preparation

User requested review and preparation. Main baseline eba537c and its RC artifacts
passed all nine hosted checks and exact source/hash/license verification. The new
release branch aligns six declarations and seven workspace lock entries to 0.3.0,
updates producer-version fixtures/generator stamps and prepares version-specific
notes and a live checklist. No exchange or dataset version is relabeled.

The documented Sessions refresh limitation was reproduced with deferred import
success/failure tests and fixed: same-profile refresh retains outcomes and updates
history; genuine client/profile replacements still reject old responses.
Actual new-source checks and review/download evidence are recorded in the release
handoff and draft PR. Version 0.3.0 retains the 0.x developer-preview prerelease
classification. Full live/native acceptance stays incomplete. Preparation does not
create a tag, merge main or publish a GitHub release; v0.2.0 stays latest published.

## Historical wave4 guidance integration

Separate task PRs59 (controls),61 (calibration),62 (joint planning),63 (evidence
delivery) and64 (local proposals), plus coordination58 and storage60, preserve
granular pushed commits and isolated ownership. See the
[review record](reviews/2026-10-14-wave4-guidance.md) for actual findings/checks.
The inherited uncommitted proposal read-failure fix was preserved, reproduced,
committed and independently reviewed. Migrations6–8 add feature state without
changing exchange or dataset identities. Full live/native acceptance stays unverified.
v0.4.0 version/notes preparation is separate; publication awaits complete protected
main integration, exact-source hosted checks and verified downloaded tag assets.
At that historical wave4 audit, dependency-ready gaps included clustering,
route-aware acquisition, cancellation, recovery, CLI workflows, preparation
infrastructure, diagnostics and Today/Settings/reference-window work. Those finite
engineering scopes were subsequently implemented; the current ledger above governs
new scheduling. Verified Deadmines records and external acceptance remain blocked.
