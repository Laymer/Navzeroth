# Stable inventory generator schema 5 followup

Owner tooling_review. Baseline a1a33fc52da61e0bd8c9cff242db42d938f0f22b.
Authorized followup to schema5 PR272: own scripts/generate-stable-inventory.py,
its data-generators/tests/test_stable_inventory_generation.py regressions and
handoff only. Emit schema5 and pristine automated final row reproducibly without
rewriting canonical release/v1 outputs, historical artifacts or source grants.
