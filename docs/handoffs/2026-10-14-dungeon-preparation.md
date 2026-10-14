# Evidence-gap dungeon preparation infrastructure

Owner: Codex /root/backlog_audit, sole feature writer. Claimed at 8d6de73 before
code; baseline protected main cb7e03db0acbdb36e5d9079394d87d4218d1253f (published
v0.5.0 f525911 plus guard82). Branch feat/dungeon-preparation. Read STATUS,
coordination, M7 party handoffs and plan11.2/M7.6 before implementation.
Final backend source 7d24a28; UI source db32c00. Recipient export85's consistent
read helper dependency 6cc6adb merged normally after preserving own source; latest
main3202c6d merged normally at34f08ef. Root owns protected integration/publication.

## Behavior and decisions

New app dungeon_preparation module owns bounded schema1 local packages, current
character/party capture, pure domain evaluation, source/evidence inspection and
bounded report publication. Strict duplicate-key parsing rejects malformed/future
feature requests without input echo. Full context/fixture/revision and all declared
known content/evidence/source/item links are checked. Typed unavailable section
observations, zero/false values, source confidence and permissions stay distinct.
The local package always remains explicitly unverified; linking evidence never
promotes an arbitrary author assertion. No production Deadmines facts are supplied.

Owned snapshots/content are captured in a consistent read transaction. Existing
quest eligibility and predicate services evaluate outside the store lock. Missing,
stale or uncertain captures retain explanatory member rows without definite
assessments. Shared manual-origin entrance routes require every member fresh,
all-member usable requirements, and stateless supported travel. Pruned member
false/unknown/restricted gates retain typed evidence in route_exclusions. Formation
and run estimates, role labels, references and manual checklists remain declarations;
no stat weights, probability estimation, consumable possession or encounter facts
are inferred. Explicit source probability/cost observations are retained.

The complete report must serialize within4MiB before final validation. One final
transaction checks all cohort identities/content/member order, fresh-subset typed
observation classes and a late capture-class sample; no nested publish transaction.
This requires the already implemented consistent-read primitive from recipient85,
not duplicated storage logic. No schema/migration, network or file persistence.

Dungeons is an ordinary primary screen with imported character/local-party choice
and useful package-missing view. Manual JSON and the explicitly synthetic example
are in collapsed Advanced local reference editor. Known requirements without a
current assessment are labelled unavailable, not merely Known. Imported member
freshness, quest catch-up/gaps, condition evidence, declared reward role/requirements,
source uncertainty and manual checklists are visible. Native command, SDK types,
browser-demo refusal and additive nav/lib wiring belong to this feature. Epoch
identity guards hide invalidated reports and discard late replies after newer work.

## Actual checks and peer correction

On merged source34f08ef: cargo test --locked -p azeroth-app passed147tests, no
failures/ignored tests. New8integration cases plus4private cases cover strict/bounded
input and actual oversized evaluation refusal, exact references/context/revision,
knownfalse/declaredzero/unavailable states, fixed serialized wire fixture, shared
route second-member gates, stale/missing cohorts, unlocked work plus snapshot/
content/membership mutation, partial-cohort own field expiry and remote field use.
Strict app all-target Clippy, workspace fmt and diff checks pass. Local logs:
/tmp/navzeroth-preparation-{app,tests,units,clippy}.log. Latest UI-only change leaves
this backend source unchanged.

Independent Atlas review found a real blocker at b2567a9: a fresh member's
ParticipantState predicate could read a stale-capture member's independently fresh
level field, which the fresh-subset guard omitted. With that field50ms from expiry,
a100ms before-publication delay returnedTrue while recalculation returnedUnknown.
The peer baseline failure log is /tmp/navzeroth-peer-preparation-remote-field-repro.log.
Correction7d24a28 filters evaluator party context to fresh captures while preserving
the full explanatory cohort. A fixed-clock regression verifiesUnknown both before
and after the remote field deadline without sleeping. Atlas independently reran
8integration/4private tests and strict Clippy at exact7d24a28 and found no remaining
confirmed backend blocker; peer worktree stayed unchanged/clean.

Desktop139unit tests passed after current-main integration; final4panel tests pass
again after the explicit unevaluated-requirement label. Typecheck/lint/build pass.
Five mocked Chromium cases (new Dungeons flow and all12 nav buttons at320/640px,
16/32px root font) passed after integration; final label-only change is unit checked.
The Dungeons case covers collapsed advanced editing, local party, actual generated
wire fixture, stale capture, false condition, declaredzero estimate, source evidence,
manual checklist and320px bounds. Port1420 is released. Logs:
/tmp/navzeroth-preparation-{desktop,ui,typecheck,lint,build,browser}.log.
These are portable/mocked checks, not actual native/live interaction.

Actual preliminary checks caught synthetic tests importing same-position snapshots
instead of advancing producer/capture sequences, a Known predicate short-circuit
that prevented the intended output expansion, strict Clippy issues, fixture typing,
and mocked browser import/command wiring. Fixed before final passing checks. No
failed, interrupted or merely compiled command is counted as a passing test.

## Remaining scope and next task

This implements preparation infrastructure and its evidence-gap screen, not the
first verified Deadmines package. Real access/quest/item/role/consumable/entrance/run
and encounter records require permitted build-scoped evidence. There is no generic
party-ready result, production template, upload, game action or overlay. Ordinary
Windows performance, native window interaction and recorded-client acceptance
remain unverified. The UI is local-only and browser demo never claims stored state.

Root reviews the final UI/source, merges dependencies/current main normally,
requires fresh exact-head hosted jobs and integrates this separate PR. Next content
work is a verified source-covered preparation package; continue other independent
engineering while its real-data prerequisites remain blocked.

## Coordinator frontend reconciliation

Root read the final ordinary/advanced preparation flow and found no confirmed UI
blocker. After normal main6e3f717 reconciliation, both Dungeons and Compatibility
remain registered; thirteen navigation controls retain the original geometry,
Tab/focus and Enter checks. Four preparation panel tests, strict typecheck/lint and
five mocked Chromium cases (preparation plus320/640px at16/32px text) passed.
Logs /tmp/navzeroth-prep-reconciled-{ui,typecheck,lint,browser}.log. This does not
promote the synthetic package or native/live behavior. Fresh exact-head checks
on the reconciled source still gate protected-main merge.
