# Project status

Plan: 1.0 plus ADRs 0001–0004. Updated: 2026-10-14.
Current task: final0.5.0 candidate/main/tag asset verification and publication.
Owner: Codex /root coordinator; released baseline2d5de64 (v0.4.0), reconciled task main d09a2e0.
v0.4.0 is published as a developer prerelease at main2d5de64 (PR #65).
PR79 prepares0.5.0; it is not yet tagged or published. See
[candidate preparation](handoffs/2026-10-14-v0.5.0-preparation.md).
All wave5 task PRs are merged, including frontend planning/navigation and typed
field-expiry correction80, after independent review and fresh nine-job checks.
Final candidate677 Rust/131 desktop/25 mocked browser checks are recorded in the
release handoff. Publication still requires verified exact main/tag assets.
Native/live acceptance remains unverified.
See [wave5 coordination/publication evidence](handoffs/2026-10-14-wave5-coordination.md).
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
| M2 | IN_PROGRESS | Bounded character imports/storage/watcher, guarded manual 70338 quest/equipment capture, runtime plan paste/HUD and local session transport implemented. Party transport and consolidated live acceptance remain. |
| M3 | IN_PROGRESS | Source/conflict library, immutable revisions, desktop content workflows, bounded recovery/backup, coverage CLI and typed local content packs implemented. Real playable quest/acquisition data remains missing. |
| M4 | IN_PROGRESS | Travel/eligibility/greedy Planner, manual joint planning, bounded single-character export and conservative Lua advancement implemented. Automatic clustering, joint export, real stage mappings and live acceptance remain. |
| M5 | IN_PROGRESS | Fixed-point gear/acquisition comparisons, editable Gear screen and synthetic Rust/Lua parity implemented. Route-derived acquisition integration, real weights/catalogs and live validation remain. |
| M6 | IN_PROGRESS | Atlas search/keyboard navigation and bounded200-pin pages, dense synthetic measurements and narrow/text/forced-color checks implemented; schematic route inspection preserves geometry gaps. Native interaction and verified real road geometry remain. |
| M7 | IN_PROGRESS | Pure reminders, local parties and per-participant declared-quest eligibility/catch-up with missing/stale gates; manual joint planning, local proposal/acceptance and reminder controls are implemented; preparation, real rules and live workflows remain. |
| M8 | IN_PROGRESS | Session interval/analytics model, bounded local imports, immutable migration-v5 history, opt-in manual recorder and Sessions screen implemented. Conditional comparison/calibration engineering is implemented; complete real samples and consolidated live checks remain. |
| M9 | IN_PROGRESS | Recovery/diagnostics, cancelable guarded planning and scoped synthetic performance checks implemented;0.5.0 candidate verification/publication ongoing. v0.4.0 remains latest published preview. Native/live acceptance, ordinary Windows performance and final coverage consolidation remain. |
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

## Wave4 guidance integration

Separate task PRs59 (controls),61 (calibration),62 (joint planning),63 (evidence
delivery) and64 (local proposals), plus coordination58 and storage60, preserve
granular pushed commits and isolated ownership. See the
[review record](reviews/2026-10-14-wave4-guidance.md) for actual findings/checks.
The inherited uncommitted proposal read-failure fix was preserved, reproduced,
committed and independently reviewed. Migrations6–8 add feature state without
changing exchange or dataset identities. Full live/native acceptance stays unverified.
v0.4.0 version/notes preparation is separate; publication awaits complete protected
main integration, exact-source hosted checks and verified downloaded tag assets.
Dependency-ready gaps found during audit remain in the task ledger, including
clustering, route-aware acquisition, cancellation, recovery, CLI workflows,
preparation infrastructure, diagnostics and Today/Settings/reference-window work.
