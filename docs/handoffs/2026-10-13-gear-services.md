# Acquisition-aware gear services

Owner: Codex /root/gear (Worker B), sole writer. Claimed before implementation.
Baseline: 32cec0f7e004ad8cc0be7b446f5b81b8a1ec3263 (wave coordination prerequisite).
Branch: feature/rc1-gear-services.
Worktree: /home/laymer/Devel/repos/Navzeroth-gear-services.
Read STATUS, M5/core and R6/acquisition handoffs, plan section 10 and M5,
and wave3 frozen boundaries. Shared wiring belongs to the coordinator.

Task: bounded profile-scoped catalog and comparisons, with separate equipment
eligibility and acquisition assessment and conservative practical estimates.
Checks, decisions, limitations and next task will be completed after implementation.

## Implemented behavior and decisions

Catalog returns exact active dataset and captured equipment for the complete
profile context/fixture identity. Missing content and baseline fields stay
visible. Comparison rejects replaced snapshot/revision, foreign fixture/context,
unknown item variants, duplicate candidates and requests over 1 MiB/128 items.
The existing 4,096 configuration enumeration bound remains unchanged.

Current baseline and theoretical rankings remain independent of acquisition.
Equipment eligibility is separately labelled. Obtainable conservatively includes
only individually guaranteed acquisitions, with an explicit note that combined
acquisition time/cost is not a jointly feasible plan. Probabilistic sources remain
inspectable in candidate assessments and conditional practical estimates.
Practical evaluates one acquisition against a complete legal model baseline,
requires known baseline/candidate eligibility, and refuses unknown or unmodelled
captured modifiers. Handed upgrades require an explicit hand relationship.
Class/categories/proficiency/weights/timing/budget/trial semantics are manual
assumptions. No defaults or real game tables are supplied.

Version gear-practical-v1 floors probability to millionths and expected positive
score gain to thousandths. Reserved acquisition effort covers maximum feasible
trials. Exact i128 cross-products compare gain/effort; stable ItemKey/slot ties and
zero-effort positive estimates sort deterministically. No joint probabilities or
useful lifespan are invented. A candidate may displace an overlapping off-hand
when the known two-handed layout explicitly authorizes it.

## Actual local verification

- cargo check --locked -p azeroth-app: passed.
- cargo fmt --all --check and git diff --check: passed.
- cargo clippy --locked -p azeroth-app --all-targets -- -D warnings: passed.
- cargo test --locked -p azeroth-app: 43 passed, zero failures.
- Five gear workflow regressions cover guaranteed vs conditional vs absent
  acquisition, zero-effort ranking, deterministic input reversal, false manual
  trial assumptions, restricted effort, unknown UTC age, current/replaced IDs,
  duplicate/128/1 MiB bounds, mismatched variants, complete captured baseline,
  unknown modifiers/variants/baseline eligibility, unknown scores, zero probability,
  level progression and missing hand relationships.
- pnpm --dir apps/desktop typecheck and lint: passed.

The initial eligibility fixture used NotApplicable requirements; core deliberately
keeps that unknown. Tests now explicitly declare a readable empty predicate;
no production requirement gap was converted to true. Initial updated snapshot
fixture used the wrong producer-sequence key; corrected to schema-v1 `sequence`.

Coordinator review identified baseline eligibility as a critical practical input.
Reproduced with a complete equipped item but unknown category/proficiency, then
fixed and covered by the populated-baseline regression. Read-only cross-review
and current-head hosted CI/Windows evidence are recorded separately by coordinator.
Native interaction and live client behavior remain UNVERIFIED.

Next task: separate task7 Gear screen with editable manual inputs, labelled
synthetic examples, stale-response rejection and explanations. Real catalogs,
role weights, captured modifier models and jointly feasible acquisition scheduling
remain absent and must not be inferred from these engineering foundations.
