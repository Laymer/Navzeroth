# STABLE-ROLES — class/level heuristics, reminders and score-only output

Owner and sole writer: Codex `/root/desktop_review`.
Baseline `5d5fef9b9e2c2fcb7b124966b1c1afed4844127f`; branch
`feat/stable-role-coverage`; worktree `/tmp/navzeroth-stable-roles`.
Claim: [STABLE-ROLES](../claims/2026-10-19-stable-roles.md).

## Inventory, content and decisions

`docs/data/role-coverage-v1.json` enumerates the registered historical CMaNGOS quest
shard: 4,245 records, nine positive single-class restriction masks, overall
minimum-level values 0–60, and per-class quest minimum-level ranges 1–60. It pins
exact bytes/SHA-256, origin `vanilla_1_12 / 1.12.1`, source identity and counts.
This establishes an input inventory, not a verified Forever roster, level cap or
gameplay availability. Zero stays recorded as zero.

New authored revision `navzeroth-authored-slice-r2` contains 54 original heuristic
models (18 class/role combinations across inclusive bands 1–20, 21–40, 41–60)
and 18 reminders (six general, twelve role/level preparation prompts). All nine
input classes are covered. Weights use transparent round judgement ratios; reuse
across bands is explicitly uncalibrated extrapolation. Class abilities, talents,
forms, pets, weapon mechanics, professions and enhancements are unmodelled.
No simulator coefficients or game formulas were imported.

The deterministic `scripts/update-authored-role-coverage.py` checks bounded,
versioned inert JSON against the run manifest, rejects changed source origin or
class/level inventory, and writes inventory, catalog and party pins. New role
evidence is separate from original slice evidence; its unrecorded exact observation
time is unknown. Party procedures are unchanged. Supporting-document hashes and
catalog/dependency pins are revised explicitly.

Reminders inspect declared role and fresh level with both band bounds. The rule
language cannot inspect class, so no reminder pretends to enforce a class filter.
Model selection requires an explicit matching class assumption, fresh known
level, applicable role and resolved exclusions. Unknown/stale inputs remain
unavailable. Prompts do not implement spell training or gameplay rules.

## Individual scores without invented eligibility

Previously deliberately unknown slot layouts/proficiencies made every authored
comparison fail. The direct desktop command now has a separate
`authored_score_only_v1` result: individual pure heuristic arithmetic, original
source observations, unknown equipment eligibility and unknown acquisition
availability. There is no baseline, configuration, ranking or practical advice.
Items stay in identity order. Incomplete attributes/effects retain incomplete
certainty; zero subtotal is never promoted to known zero value.

Shared class/level/role validation applies to both paths. Manual equipment fields
cannot supply missing source facts. Full gear/route/practical calculation remains
strict and ordinary full-report serialization is unchanged. The desktop labels
scores as heuristics, shows source states and clears results on edits. Tauri
wiring and TypeScript DTOs were updated together. See the
[versioned DTO contract](../data/AUTHORED_SCORE_ONLY_V1.md).

## Actual verification

- Authored updater: repeated byte-for-byte reproduction of all three products.
- Core authored-content suite: 4 passed, covering all 54 model/band scenarios and
  unavailable/stale reminder roles.
- CLI authored pack/inventory suite: 4 passed, including source bindings,
  canonical pack bytes and tamper refusal.
- Final `build-content-v2` / `verify-pack`: passed, 18 files, 162,593 bytes,
  distribution eligible, 124 explicit game-fact gaps. This is not release readiness.
- Full app LLVM coverage (`cargo llvm-cov --locked -p azeroth-app --lib --tests`):
  256 passed, 0 failed, 1 existing ignored whole-world test. Changed source lines:
  authored_scores 81/85 (95.29%), authored_content 438/481 (91.06%), gear 399/437
  (91.30%). Thresholds were unchanged.
- Final targeted score-only suite: 3 passed, exercising every model at both
  inclusive endpoints, adjacent outside levels, unavailable/wrong/stale inputs,
  source-state preservation, incomplete subtotals and bounded duplicate refusal.
- Full desktop coverage: 66 files, 537 tests passed; per-file 90% line gate passed.
  Overall lines 95.91%; GearScreen 105/116 (90.51%).
- Frozen offline install, desktop typecheck/lint/build, Rust app/all-target Clippy
  with warnings denied, format and whitespace checks passed. Existing bundle-size
  warning remains.
- Native `cargo check --locked -p azeroth-desktop --lib` failed before application
  checking: system `dbus-1.pc` required by `libdbus-sys` is absent. Native wiring and
  Windows/WebView behavior remain unverified locally; hosted checks are required.

Intermediate checks caught a demo location surviving a test context change, Rust
ownership and test-only TS assertion errors, a large enum layout (both variants
are now boxed), and outdated supporting-document pins. The pack builder correctly
refused stale pins; the generator refreshes only the two owned supporting docs.
No validation rule, source-fact requirement or coverage threshold was relaxed.

## Limitations and next task

No live/installer run, hosted Windows result, receipt, release inventory,
activation, performance measurement, source-platform parity, push or publication
was performed. The weights are authoring, not calibrated advice. Class remains
an explicit user assumption; level/role/applicability must be known and fresh or
selection is refused. Unknown imported roles are never supplied automatically.

Joint/shared-credit gameplay, profession/trainer-spell/talent/enhancement gameplay,
source proficiencies/layouts and practical advice from this authored pack remain
unresolved or excluded. Score-only output does not fill those gaps.

Next: independent review and integration after repaired #212, exact combined-head
source/platform and Windows checks, then bundle/material/receipt and owner-smoke
verification. Rebuild packs and inventory from final source; do not reuse r1 hashes.
