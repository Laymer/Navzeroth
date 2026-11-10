# ADR 0011: 1.1.0 launch compatibility and guided progression

Accepted 2026-11-10 by the owner's supplied implementation plan.
Baseline: `aaa73b94794f5a32162b430b2314f53bca4e1a7b`.
Task V110.1; owner and sole writer Codex `/root`.

## Decision

Release stable **1.1.0 directly**, superseding unpublished 1.0.0 preparation.
Extend ADR 0008's explicitly unsigned Windows installer exception and ADR 0009's
automated downloaded-draft Windows acceptance to 1.1.0. Signed content receipts
remain mandatory. Automated acceptance does not establish live gameplay,
ordinary-PC performance or human accessibility verification. Preserve historical
records and failed attempts; neither 1.0.0 receipts nor its acceptance satisfy 1.1.0.
ADR 0010's bounded hosted firewall isolation remains applicable.

Replace the four feature deferrals in ADRs 0008/0009 with required usable workflows:

| Workflow | Required behavior |
| --- | --- |
| Trainer spells | Prioritize eligible unlearned spells, trainers, prerequisites, known costs and budget shortfalls; unknown knowledge or affordability gives conditional advice. |
| Talents | Editable authored class/role builds and ordered next points; validate ranks, prerequisites, tiers, level budgets and respec implications. |
| Professions/crafting | Rank progression and selected output quantities; materials, known inventory subtraction and acquisition steps; preserve uncertain skill-ups, yields, cooldowns and missing sources. |
| Enhancements | Equipment compatibility, effects, acquisition/materials and permanent/temporary distinctions; numeric ranking only with structured effects and an explicit scoring model. |

Training, Talents and Professions belong on Progression; enhancement advice belongs
on Gear. Link applicable steps to Planner, Atlas, Today and reminders. Show source origins, confidence,
conflicts and unresolved requirements beside advice. Label talent recommendations
as authored guidance. Manual completion records only
planning progress. No purchasing, talent allocation, crafting or application is
automated. Live auction pricing and combat simulation remain outside scope.

## Exact client and content

Recheck installed metadata (plan reports `wowf 1.60.1.70862`) and establish interface
from permitted client evidence before admitting the tuple. Do not infer interface
from a nearby build. Preserve older exact identities; no nearest-build fallback.
Refresh admitted sources and local extraction against the selected tuple, retaining
historical source builds, encryption/unavailability, conflicts and explicit unknowns.
Review later build differences; never silently retarget a candidate.

Keep world-data v1 reference semantics. Introduce separately versioned typed facts
for spell requirements/costs, talent prerequisite/point rules, profession ranks,
recipe materials/yields and enhancement applicability/effects, each with provenance
and completeness. Empty legacy recipe lists, false cooldown flags, absent talent
prerequisites and prose effects cannot establish gameplay facts.

Enumerate supported classes, roles, level bands, professions and records from the
refreshed catalog within levels 1–60. Require usable authored guidance across the
declared coverage of all four workflows. Do not imply every game record is known.
Include facts/guidance in inventory validation, source closure, signed bundles and
release review. Missing information never becomes zero, false or no prerequisites.

## State and application boundaries

Add `progression_snapshot`, payload version 1, bound to the exact character
snapshot, character, client tuple, dataset context and capture metadata. Preserve
character-snapshot and route-plan v1. Old exports remain accepted; old importers
must explicitly refuse the new kind. Collections distinguish complete, partial and
unavailable; missing companions and absent partial entries mean unknown.

Where permitted, explicitly capture known spells/ranks and money, talent allocation
and points, profession skills/ranks and recipes, inventory quantities with bag/bank
scope, and equipped enhancements with observable temporary expiry. Capability-gate
addon capture and preserve restricted/unsupported status. Provide separate manual
inputs with manual provenance. Never execute imported Lua or create a live bridge.

Add additive SQLite migrations for companions and saved plans. Implement pure,
deterministic Rust services, typed application/Tauri commands and generated
TypeScript contracts. Persist snapshot/content/ruleset dependencies. Changing the
selected character snapshot invalidates predecessor advice.

## Verification and publication

Version-aware tooling explicitly permits supported release versions and refuses
malformed/unlisted versions. Retain acceptance schema 5 and its nine observations;
extend workloads/evidence checks to all four rendered workflows, including the
independently downloaded draft Windows UI test and restart persistence.

Required regression families: old snapshot compatibility; exact identity/build
mismatch; partial versus complete-empty; stale/false/zero; learned spells and unknown
versus zero costs; unavailable trainers and insufficient money; talent cycles/ranks/
prerequisites/overspending/build edits; recipe cycles/overflow/rounding/unknown stock/
uncertain skill-ups; enhancement restrictions/stacking/missing effects/expiry;
migration/backup/restore/restart/activation rollback/offline operation; Rust/Lua parity.

Require fresh protected-main checks, per-file 90% coverage, two-platform source
parity, independent read-only review and release stability. Preserve workload limits;
diagnose failures instead of expanding bounds. Finish features/content/docs before
version freeze. Align application/addon declarations and locks to 1.1.0 using existing
generators. Archive superseded evidence, inventory actual coverage, check full packs,
record tested source/production fingerprints, commit public review and prepared
bundle identities, independently reproduce, then sign and verify fresh receipts.

`release-build-readiness --root . --version 1.1.0` must allow preparation with only
nine pristine pending observations. Run all eight installed-app cases using published
1.0.0-rc.1 as mandatory upgrade baseline, including persistence/downgrade refusal.
Normally merge the reviewed release PR only after checks pass. Run fresh checks and
release stability on resulting main; tag that verified commit `v1.1.0`.

Require eight passing observations bound to exact packaged installer bytes before
draft readiness. Verify complete assets, checksums, source, unsigned status, signed
content, notices and required sources. Independently download and exercise the
actual draft Windows UI/native workflows, independently review evidence and completed
GitHub provenance, then construct the external overlay without rewriting signed
resources. Relevant product/content changes invalidate affected evidence.

Publish without another permission request only when
`release-readiness --root . --version 1.1.0 --evidence <verified-overlay>` reports
zero blockers, stable readiness and publication allowed. Re-download and verify
public assets before updating the website/download instructions. Record URL, tag,
source, workflow attempts, asset identities, actual checks, measured resources,
limitations and next task in a publication receipt. Missing evidence holds
publication while independent work continues. This ADR itself establishes no pass.
