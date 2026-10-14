# Implementation task ledger — recurring release program

Published baseline: main `035f28e`, `v0.7.0`. Owner: Codex /root.
Fresh released-source audit: [wave8 coordination](handoffs/2026-10-14-wave8-coordination.md).
Exact current candidate checks: [0.8 preparation](handoffs/2026-10-14-v0.8.0-release.md).
This ledger separates existing engineering implementations from whole-task acceptance.
No task is promoted to live VERIFIED by source inspection or synthetic checks.
Rows are audited against STATUS, task handoffs, current module/test inventory, and
implementation-plan section 17. Exact future checks belong in task handoffs.
Section16.2 explicit redacted diagnostics is implemented in PR70/76; native file
behavior and installed capability/version evidence remain unverified.

## Current candidate and next scheduling decisions

Latest actual publication stays0.7 at035f28e. Combined0.8 preparation5b7f7b9
implements these finite UI scopes; all four feature PRs are merged through
protected main12b6ff4. Release PR/main/tag and asset gates remain pending. Portable/synthetic checks do not promote native/live acceptance.

| Plan scope | Current disposition | Evidence / remaining acceptance |
| --- | --- | --- |
|12.2 route selection/fit|IMPLEMENTED_UNVERIFIED|PR104 keyboard hop/tail pages and exact-frame schematic fit; native/geometry/calibration acceptance remains unverified.|
|12.4 HUD appearance|IMPLEMENTED_UNVERIFIED|PR103 bounded size/font/contrast/movement, protected combat gate and session-only/reset repair; actual client taint/focus/display acceptance remains unverified.|
|12.5 reference controls|IMPLEMENTED_UNVERIFIED|PR105 opt-in focused-main shortcut and requested interaction receipts; actual focus/hit-test/display acceptance remains unverified.|
|12.6 heading reflow|IMPLEMENTED_UNVERIFIED|PR106 320px/200% heading wrapping; actual screen-reader/native DPI acceptance remains unverified.|
|19.3 typed empty pack arrays|READY after actual0.8 publication|Canonical pack comparison rejects valid explicit map_embeddings=[] because typed serialization omits the empty default. Preserve schema1 and unknown-field refusal.|
|8.4/19.3 existing Forever source registry|READY after actual0.8 publication|Add sources.json for existing documented sources with exact context/evidence links and recorded policies; unknown rights remain blocked, no new permissions or playable records invented.|

Historical scheduling sections below are snapshots, not the current ready queue.
Today/Settings (PR96) and M7.6 evidence-gap infrastructure (PR90) are implemented;
verified Deadmines content stays blocked. The two READY rows above require another
separate parallel PR batch and publication audit; no exhaustion claim is made.

| Task | Current disposition | Remaining work / scheduling decision |
| --- | --- | --- |
| M0.1 | IMPLEMENTED_UNVERIFIED | Existing intake/diagnostics records; verify current scope as part of audit. |
| M0.2 | IMPLEMENTED_UNVERIFIED | Existing intake/diagnostics records; verify current scope as part of audit. |
| M0.3 | IMPLEMENTED_UNVERIFIED | Existing intake/diagnostics records; verify current scope as part of audit. |
| M0.4 | BLOCKED | Recorded-build live field/event/restriction checks unavailable. |
| M0.5 | BLOCKED | Consolidated live export/persistence/plan import checks unavailable. |
| M0.6 | BLOCKED | Only level/clock user reports exist; consolidated capability decisions pending. |
| M1.1 | IMPLEMENTED_UNVERIFIED | Existing engineering implementation and recorded checks; full milestone acceptance remains incomplete. |
| M1.2 | IMPLEMENTED_UNVERIFIED | Existing engineering implementation and recorded checks; full milestone acceptance remains incomplete. |
| M1.3 | IMPLEMENTED_UNVERIFIED | Existing engineering implementation and recorded checks; full milestone acceptance remains incomplete. |
| M1.4 | IMPLEMENTED_UNVERIFIED | Existing engineering implementation and recorded checks; full milestone acceptance remains incomplete. |
| M1.5 | IMPLEMENTED_UNVERIFIED | Existing engineering implementation and recorded checks; full milestone acceptance remains incomplete. |
| M1.6 | IMPLEMENTED_UNVERIFIED | Existing engineering implementation and recorded checks; full milestone acceptance remains incomplete. |
| M1.7 | IMPLEMENTED_UNVERIFIED | Existing engineering implementation and recorded checks; full milestone acceptance remains incomplete. |
| M2.1 | IMPLEMENTED_UNVERIFIED | Existing engineering implementation and recorded checks; full milestone acceptance remains incomplete. |
| M2.2 | IMPLEMENTED_UNVERIFIED | Existing engineering implementation and recorded checks; full milestone acceptance remains incomplete. |
| M2.3 | IMPLEMENTED_UNVERIFIED | Existing engineering implementation and recorded checks; full milestone acceptance remains incomplete. |
| M2.4 | IMPLEMENTED_UNVERIFIED | Existing engineering implementation and recorded checks; full milestone acceptance remains incomplete. |
| M2.5 | IMPLEMENTED_UNVERIFIED | Existing engineering implementation and recorded checks; full milestone acceptance remains incomplete. |
| M2.6 | BLOCKED | Real quest transition needs recorded client plus authored stage mapping. |
| M3.1 | IMPLEMENTED_UNVERIFIED | Existing engineering implementation and recorded checks; full milestone acceptance remains incomplete. |
| M3.2 | IMPLEMENTED_UNVERIFIED | Existing engineering implementation and recorded checks; full milestone acceptance remains incomplete. |
| M3.3 | BLOCKED | Permitted build-specific playable quest source records missing. |
| M3.4 | BLOCKED | Verified quest/service/travel/acquisition records missing. |
| M3.5 | IMPLEMENTED_UNVERIFIED | Existing engineering implementation and recorded checks; full milestone acceptance remains incomplete. |
| M3.6 | IMPLEMENTED_UNVERIFIED | Existing engineering implementation and recorded checks; full milestone acceptance remains incomplete. |
| M4.1 | IMPLEMENTED_UNVERIFIED | Existing engineering implementation and recorded checks; full milestone acceptance remains incomplete. |
| M4.2 | IMPLEMENTED_UNVERIFIED | Existing engineering implementation and recorded checks; full milestone acceptance remains incomplete. |
| M4.3 | IMPLEMENTED_UNVERIFIED | PR83/84 conservative same-declared-node clustering merged; no inferred shared-kill savings; real session acceptance remains missing. |
| M4.4 | IMPLEMENTED_UNVERIFIED | Existing engineering implementation and recorded checks; full milestone acceptance remains incomplete. |
| M4.5 | IMPLEMENTED_UNVERIFIED | Existing engineering implementation and recorded checks; full milestone acceptance remains incomplete. |
| M4.6 | IMPLEMENTED_UNVERIFIED | Existing engineering implementation and recorded checks; full milestone acceptance remains incomplete. |
| M4.7 | BLOCKED | Documented actual play session and comparable planned route missing. |
| M5.1 | IMPLEMENTED_UNVERIFIED | Structure/arithmetic exists; real four-scenario models/weights remain missing. |
| M5.2 | IMPLEMENTED_UNVERIFIED | Existing engineering implementation and recorded checks; full milestone acceptance remains incomplete. |
| M5.3 | IMPLEMENTED_UNVERIFIED | Wave7 typed readable contributions, unmodeled effects and exact input/participant reasons are implemented in published0.7 PR97; real-model/live acceptance remains unverified. |
| M5.4 | IMPLEMENTED_UNVERIFIED | PR86 finite route-derived acquisition alternatives merged; explicitly bound sources/legs and independent opportunities. Combined acquisition scheduling and real catalogs are not claimed. |
| M5.5 | IMPLEMENTED_UNVERIFIED | Existing engineering implementation and recorded checks; full milestone acceptance remains incomplete. |
| M5.6 | IMPLEMENTED_UNVERIFIED | Existing engineering implementation and recorded checks; full milestone acceptance remains incomplete. |
| M5.7 | IMPLEMENTED_UNVERIFIED | Synthetic Rust/Lua parity exists; representative real changes need input/evidence. |
| M6.1 | IMPLEMENTED_UNVERIFIED | Demo/reference geography implemented; real route/geography/license/calibration acceptance remains scoped. |
| M6.2 | IMPLEMENTED_UNVERIFIED | Demo/reference geography implemented; real route/geography/license/calibration acceptance remains scoped. |
| M6.3 | IMPLEMENTED_UNVERIFIED | Demo/reference geography implemented; real route/geography/license/calibration acceptance remains scoped. |
| M6.4 | IMPLEMENTED_UNVERIFIED | Demo/reference geography implemented; real route/geography/license/calibration acceptance remains scoped. |
| M6.5 | IMPLEMENTED_UNVERIFIED | Demo/reference geography implemented; real route/geography/license/calibration acceptance remains scoped. |
| M6.6 | IMPLEMENTED_UNVERIFIED | PR68 dense rendering and PR78 reflow merged; candidate PR104 route selection/fit and PR106 heading reflow add finite accessibility engineering. Native focus/DPI/target performance remain unverified. |
| M7.1 | IMPLEMENTED_UNVERIFIED | Persisted guarded desktop controls and deduplication delivered in PR59; live delivery remains unverified. |
| M7.2 | IMPLEMENTED_UNVERIFIED | Evidence delivery gates/manual checklist delivered in PR63; real category rules require missing records/observations. |
| M7.3 | IMPLEMENTED_UNVERIFIED | Manual joint planning with all-member readiness, exclusions and fairness delivered in PR62; real workflow remains unverified. |
| M7.4 | DEFERRED_WITH_REASON | Requires verified party addon-message capability and limits. |
| M7.5 | IMPLEMENTED_UNVERIFIED | Local CAS proposals/acceptance64 and agreed recipient export85 merged; no verified remote consent or game messaging. |
| M7.6 | BLOCKED | PR90 bounded evidence-gap preparation infrastructure merged; verified Forever Deadmines records remain missing. |
| M7.7 | BLOCKED | Complete paladin/mage live workflow evidence missing. |
| M8.1 | IMPLEMENTED_UNVERIFIED | Existing engineering implementation and recorded checks; full milestone acceptance remains incomplete. |
| M8.2 | IMPLEMENTED_UNVERIFIED | Existing engineering implementation and recorded checks; full milestone acceptance remains incomplete. |
| M8.3 | IMPLEMENTED_UNVERIFIED | Existing engineering implementation and recorded checks; full milestone acceptance remains incomplete. |
| M8.4 | IMPLEMENTED_UNVERIFIED | Conditional complete-session comparison/calibration delivered in PR61; real complete samples and native interaction missing. |
| M8.5 | DEFERRED_WITH_REASON | Requires real permitted combat-log format and intended-use verification. |
| M9.1 | IMPLEMENTED_UNVERIFIED | PR88 exact-context compatibility/coverage consolidation and historical hash-bound inventory merged; real playable catalogs and consolidated live acceptance remain missing. |
| M9.2 | IMPLEMENTED_UNVERIFIED | PR72 recovery/backup/integrity/startup guards and PR77 completion checks merged; interactive Windows replacement remains unverified. |
| M9.3 | IMPLEMENTED_UNVERIFIED | PR69/75/74 measurements/control, guarded jobs/frontend and PR80 typed-field expiry correction merged. Scoped synthetic checks pass; ordinary Windows/live performance remains unverified. |
| M9.4 | IMPLEMENTED_UNVERIFIED | Hosted package/install/start/uninstall smoke exists; native interaction and game smoke missing. |
| M9.5 | IMPLEMENTED_UNVERIFIED | Existing engineering implementation and recorded checks; full milestone acceptance remains incomplete. |
| M9.6 | IMPLEMENTED_UNVERIFIED | v0.7.0 published developer prerelease with verified assets; focused-release acceptance remains incomplete. |
| M10.1 | IMPLEMENTED_UNVERIFIED | PR89 ordinary imported reference window merged; candidate PR105 focused-main opt-in shortcut and requested interaction controls implemented; native interaction remains unverified. |
| M10.2 | BLOCKED | Interactive native focus/DPI/monitor/borderless checks unavailable. |
| M10.3 | IMPLEMENTED_UNVERIFIED | PR89 separate capture/field age and unsupported exclusive-fullscreen explanations merged; native display-mode evidence missing. |
| M10.4 | IMPLEMENTED_UNVERIFIED | PR89 ordinary desktop fallback implemented and mocked; actual native fallback behavior remains unverified. |
| M11.1 | DEFERRED_WITH_REASON | No verified official Forever endpoint/auth/terms evidence. |
| M11.2 | DEFERRED_WITH_REASON | Depends on M11.1 enablement prerequisites. |
| M11.3 | DEFERRED_WITH_REASON | Requires authorized real endpoint/access. |
| M11.4 | DEFERRED_WITH_REASON | Depends on verified provider/auth/retention capability. |
| M11.5 | DEFERRED_WITH_REASON | Only if verified flow requires a confidential secret/backend. |
| M12.1 | BLOCKED | Focused-release prerequisites not met; expand finite permitted coverage units afterward. |
| M12.2 | BLOCKED | Focused-release prerequisites not met; expand finite permitted coverage units afterward. |
| M12.3 | BLOCKED | Focused-release prerequisites not met; expand finite permitted coverage units afterward. |
| M12.4 | BLOCKED | Focused-release prerequisites not met; expand finite permitted coverage units afterward. |
| M12.5 | DEFERRED_WITH_REASON | Local first; authenticated hosting not currently justified. |
| M12.6 | BLOCKED | Focused-release prerequisites not met; expand finite permitted coverage units afterward. |
| M12.7 | BLOCKED | Focused-release prerequisites not met; expand finite permitted coverage units afterward. |

## Iteration rule

After each publication, inspect every row against the released revision; record
new checks, findings and source/capability evidence; promote only supported scope.
Launch all dependency-ready tasks in three-worker batches with separate PRs.
When only external prerequisites remain, stop with explicit unblock requirements.
No release is created for an unchanged blocker ledger. Historical handoffs remain
immutable evidence; this current ledger does not erase their actual results.

## Historical dependency-ready follow-ups found during wave4 review

- M4.5/M7.5: participant-scoped export of agreed joint plans through bounded v1 transport.
- M3 pipeline / section19.3: doctor67, bounded deterministic content packs71 and synthetic replay73 are merged; real permitted source/coverage pipeline stays missing.
- M9.3: measurements/control69, backend75, frontend74 and typed-field correction80 are merged; target-hardware/live checks remain unverified.
- M9.2: recovery72 and completion-check hardening77 are merged; native replacement remains unverified.
- Section12.1: Today selected-party/goal/current-plan summary and Settings are incomplete.
- Section16.2: bounded explicit diagnostics70/76 is merged; native execution remains unverified.
- M7.6: evidence-gap preparation infrastructure can proceed; verified Deadmines package stays blocked.

These gaps were actionable at that historical review. Today/Settings and M7.6
infrastructure were subsequently implemented; this list is not the current queue.
Existing foundations and synthetic tests do not prove full plan acceptance. Wave6 starts at published v0.5.0 source f525911; every task retains a separate
claim, branch, PR and release audit. See the wave6 coordination handoff.

## v0.5.0 released-source audit

Published2026-10-14T10:42:55Z at f525911. Wave6 claims M4.3 clustering, M5.4
route-derived acquisition and M4.5/M7.5 agreed recipient export in separate PRs.
Additional ready local foundations: M3 durable content quarantine/revocation with
safe active fallback and reactivation refusal; bounded opt-in local submission,
moderation and withdrawal lifecycle (sections8.4/8.6 preparatory infrastructure).
M12.4 full community acceptance remains BLOCKED by focused-release and real source
prerequisites; developer-preview tags do not satisfy that gate. Hosting is deferred.

## Historical v0.6.0 candidate audit and next ready batch

Task PRs81–90 are integrated; exact checked heads and actual findings are recorded
in the candidate handoff. No publication is implied by this candidate audit.
The four finite ready scopes after publication are:

- Section12.1: persisted explicit Today/Settings choices and a guarded current-plan
  reference, with bounded read-only plan text available to the ordinary reference view.
- M3 pipeline: durable content quarantine/revocation, safe eligible fallback or
  no-active state, and reactivation refusal. Rollback alone is insufficient.
- Sections8.4/8.6 preparatory infrastructure: bounded opt-in local submissions,
  review, withdrawal/revocation and permission-gated export. No hosting or evidence
  confidence promotion. Full M12.4 acceptance remains blocked.
- Sections10.3/9.8/12.1: ordinary readable typed gear/acquisition explanation views.
  Existing contributions, unmodeled effects and reasons need no invented game facts.

PR86 satisfies finite route-aware opportunity review. Automatic combined acquisition
scheduling, speculative useful lifespan, beam search and shared-kill savings do not
become mandatory extensions without their missing models/evidence. Re-audit the
actual published source before launching this next batch. Native/live/source/API
unblock requirements and local-first hosting deferral remain unchanged.

## v0.6.0 released-source audit

Published2026-10-14T12:49:07Z at739a822. Exact task/PR/main/tag checks and five-file
asset verification are recorded in wave7 coordination. The four previously found
ready scopes remain absent in this released source and are now claimed by distinct
writers. Shared migration9/recovery registration precedes feature integration.
No new external prerequisite was inferred away, and no unchanged blocker-only
release is planned. After the next publication repeat this audit against its tag.

## Historical wave7 candidate and next ready batch

Candidate0.7 engineering adds Today/Settings/server-owned guarded plan pinning,
durable quarantine/revocation/fallback, bounded opted-in local moderation/export,
readable gear/acquisition reasons, schema10 and typed populated backup recovery.
Task PRs remain isolated; release handoff records their final checked/merged sources.
This section does not claim publication or native/live verification.

Independent full-plan UI audit identified three finite additional scopes not supplied
by the narrower milestone implementations:

| Plan requirement | Ready engineering | External acceptance |
| --- | --- | --- |
|12.2| Accessible route-segment selection and bounded per-frame schematic fit | Actual road geometry, calibration/rights and native map interaction remain unverified. |
|12.4| Bounded HUD resize/font/contrast preferences and conservative UI-operation gates | Recorded-client combat restrictions, taint, focus and display checks remain unverified. |
|12.5| Explicit opt-in focused-desktop shortcut and reference interaction choice | Native focus, hit testing, chord conflicts and display modes remain unverified. |

Claim these independently against the actual published0.7 source after publication,
then run another parallel PR/release batch. Imported snapshots, permitted addon
observations, exact contexts and all reported data states remain unchanged in scope.
World expansion still requires the focused-release evidence; missing real sources,
models, capability/format evidence and native/live measurements remain blockers.

## v0.7.0 published-source audit

Published2026-10-14T15:14:38Z at035f28e with five verified assets. Wave7 scopes
are implemented; the three remaining finite section12 UI gaps are now claimed by
distinct wave8 writers. Exact publication/review/check provenance and external
unblock requirements are in the wave8 handoff. No task is promoted to live VERIFIED.
