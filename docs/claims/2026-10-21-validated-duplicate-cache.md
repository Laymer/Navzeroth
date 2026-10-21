# Retain SQL-validated duplicate catalogs

Owner world_review, sole writer. Baseline fd722928889a84dce6eba59c91cfeab4ba55fa6f
(tooling frozen strict-decoded validation repair, parent baseline2466b744).
Branch fix/stable-validated-duplicate-cache, isolated worktree.
Scope content_catalog.rs/content_preferences.rs and focused cache tests/handoff;
tooling separately owns content.rs/hash.rs. Preserve all canonical input, SQL-row,
source/conflict/hash/coverage, historical and destructive validation boundaries.
Reuse only strict SQL-loaded duplicate catalog under original managed generation
stamp in owned IMMEDIATE transaction; no seeding fresh insert from input.
No production time-limit, release canonical, receipt, key or acceptance changes.
Read STATUS, recovery-runtime V14 handoff and plan8.4 before implementation.
