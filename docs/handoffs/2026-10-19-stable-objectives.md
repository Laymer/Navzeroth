# STABLE-OBJECTIVES — source-backed solo planner objectives

Owner and sole writer: Codex `/root/desktop_review`.
Baseline `5d5fef9b9e2c2fcb7b124966b1c1afed4844127f`; branch
`feat/stable-quest-objectives`; isolated `/tmp/navzeroth-stable-objectives`.
Claim: [STABLE-OBJECTIVES](../claims/2026-10-19-stable-objectives.md).

## Change and decisions

The generated pack declares quest stages but has no profile-specific objectives.
The solo planner now derives deterministic, character-bound objectives from declared,
source-backed stages. Existing authored objectives for a stage take precedence.
Empty evidence never establishes a new source-backed objective. The projection is
bounded at 131,072 total planner objectives, retains quest evidence/context, and
leaves expected time unknown. A stage identifier establishes neither objective
counts nor descriptive details. Original content, source fields and snapshots are
unchanged; the generator and exchange schemas are unchanged.

Catalog first/subsequent pages, calculation, async jobs through the same calculation,
and plan export use the same projection. Pages retain the original catalog hash,
revision binding and 200-row limit. Origin lookups retain quest source marks.
Known source eligibility and prerequisites are conjunctive; unavailable states remain
unavailable. Existing observed accepted/completed/turned-in/stage readiness and
freshness gates still apply. The new export ruleset is
`planner-quest-stage-projection-v2`; authored-only exports retain their prior ruleset.

The planner screen now names quest/stage, acquisition and service targets, shows
requirements observation status without calling it eligibility, and distinguishes
unknown source duration from manual estimates. Partial exports retain explanations
for filtered unresolved stages.

## Actual checks

- `pnpm install --frozen-lockfile --offline`: passed.
- Desktop typecheck, lint and production build: passed; existing chunk-size warning.
- Full desktop Vitest coverage: 66 files, 540 tests passed; per-file 90% line gate
  passed. Overall lines 95.90%; PlannerScreen 121/129 (93.79%). Four added UI cases
  distinguish known, unknown, restricted and unsupported requirements.
- Rust Clippy, locked app/all-targets with warnings denied: passed.
- Rust format and diff whitespace: passed.
- `cargo llvm-cov --locked -p azeroth-app --lib --tests --json --output-path
  /tmp/navzeroth-stable-objectives-coverage.json`: 258 passed, 0 failed,
  1 existing ignored whole-world test. Changed source files all exceed 90% lines:
  catalog_pages.rs 254/275 (92.36%).
  plan_export.rs 244/269 (90.71%).
  planning.rs 195/211 (92.42%).
  planning_objectives.rs 111/122 (90.98%).
  snapshots.rs 903/992 (91.03%).
  world.rs 783/812 (96.43%).

Initial positive synthetic readiness lacked trusted per-field metadata and correctly
failed; the fixture was corrected without relaxing production freshness checks.
Initial broad tests exposed an unconditional disclosure changing authored-only
export goldens and a page-hash incompatibility. Both were corrected; no golden
files were regenerated. A partial-plan export regression exercises preservation
of unresolved stage readiness and the export-assumption boundary.

## Limitations and next task

These are synthetic source-shaped regressions, not an installed full-pack or live
client acceptance result. Hosted Windows, actual installer, real full-pack loading,
source receipts, resource measurements and owner smoke remain unverified here.
The representative fixture matches the generated shape (declared stages/evidence,
unknown completion and no persisted objectives), not real WoW facts.

Planner snapshots have no class/race/faction facts. Quests with such source
restrictions therefore remain unresolved rather than implicitly eligible; no new
profile facts are invented. Stages lacking fresh known observations remain
unavailable. Joint planning continues to require authored multi-participant
objectives; shared credit/fairness is not inferred. These limitations need an
explicit supported-scope decision before promising broader gameplay coverage.

Next: independent review, integration after repaired PR #212, exact merged-source
checks, and real bundled-pack planning/Windows verification as part of the release
program. No push, PR, merge, tag or publication was performed by this task.
