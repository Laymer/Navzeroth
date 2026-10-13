# Gear screen

Owner: Codex /root/gear (Worker B), sole writer. Claimed before implementation.
Baseline: af16a7d5f583c6e49cba3b144033bebf414d3ea8 (task3 PR50 head).
Branch: feature/rc1-gear-screen.
Worktree: /home/laymer/Devel/repos/Navzeroth-gear-screen.
Read STATUS, wave3 boundary and gear-services handoff, plan section10/M5.5.
Task7 depends on task3. Coordinator owns navigation and shared wiring.
Checks/decisions/limitations recorded after implementation.

## Implementation and decisions

Gear navigation opens profile-scoped catalog inspection and manual model controls:
role, weight JSON in thousandths, class/proficiency JSON, explicit hand pairing,
candidate categories/source choice, effort, spending ceiling and trial assumptions.
Blank effort/status controls remain unknown. Source probabilities are active
content observations, never a hardcoded drop table. Empty role weights or session
budget are refused before a command is sent. Backend errors are displayed.

The synthetic example is available only for fixture-labelled catalogs. It clearly
labels equal fictional attribute weights, inapplicable-category assumptions and
fictional effort; it does not select an acquisition source or assert player trial
independence. Captured equipment/variant/modifier gaps, current score contributions,
configuration legality and exclusions, future progression, detailed acquisition
uncertainty and practical rejection reasons remain inspectable. The UI shows
conditional estimates and reserved effort explicitly and supplies no lifespan.

Any changed input, profile/context refresh or newer calculation invalidates results
and ignores late responses. Coordinator review reproduced that the application
refreshes views every minute and the initial implementation erased manual edits.
The corrected same-profile refresh clears results and refetches content while
preserving weights/proficiency/effort/candidate controls; it prunes removed item
keys and missing/mismatched source IDs. A profile/client change resets assumptions
so a synthetic model cannot silently carry into a real profile.

## Actual checks

- Desktop typecheck/lint/build: passed.
- Desktop unit tests: 82 passed, zero failures (three Gear UI flows plus a Rust
  DTO golden reader added). Manual validation, synthetic labels, real-data gating,
  backend replaced-input errors, superseded responses and preservation of editing
  across routine refresh are covered.
- cargo test --locked -p azeroth-app --test gear_workflow: six passed.
- cargo clippy --locked -p azeroth-app --all-targets -- -D warnings: passed.
- Actual Rust GearCatalog/GearReport bytes committed as fixtures/desktop/gear JSON.
  A normal Rust regression checks byte-derived document equality; explicit fixture
  regeneration uses UPDATE_GEAR_GOLDENS=1. Desktop and Chromium read these outputs.
- Chromium command mocks: eight workflows passed, including the new isolated
  e2e/gear.spec.ts Gear navigation/model/explanation/input-invalidation workflow.
- cargo fmt --all --check and git diff --check: passed.

Mock browser evidence is separate from native interaction and live client behavior,
which remain UNVERIFIED. Hosted Windows/current-head checks are recorded by the
coordinator. Initial test compile/lint errors (readonly arrays/non-null assertions)
were corrected and checks rerun. No manifest, exchange schema or version changed.

Next: coordinator integration/review and task10 candidate packaging. Real item
catalogs, source/proficiency tables, validated role models, captured modifier models
and joint acquisition scheduling remain absent; this screen does not claim them.
