# Validated catalog reuse during travel activation

Owner `/root/world_review`; claimed baseline `fd2bd791`, isolated
`fix/stable-travel-activation-cache`. Actual V8 first-start took994553ms,
startup340613ms; later modes each toggle inferred travel twice. Each toggle
previously decoded the huge catalog twice despite a valid immutable cache.
This source-backed diagnosis does not establish a new timing measurement.

`catalog_revision` now uses the existing managed immediate-read/cache wrapper in
autocommit, retaining uncached validation inside arbitrary transactions/savepoints.
Travel activation owns an IMMEDIATE transaction before beginning an existing
Generation read scope. It verifies cached-or-fully-loaded catalog identity, then
drops the read guard before disposition/pointer/preference writes. The transaction
still commits these changes together or rolls back on any failure. No new cache,
new trust, SQL hook, schema, budget, producer, policy, canonical or receipt changes.
Existing local generation, external data_version, main/TEMP schema, shadow,
terminal-overflow and managed-scope guards retain their behavior. Connection/hook
ownership remains private; no new raw SQL or incremental-BLOB API is exposed.

Actual checks before code freeze:
- All storage targets:167passed,0failed,0ignored in20 binaries.
- All storage targets Clippy with warnings denied:pass; format/diff check:pass.
- Actual thread-local load counter proves three warm false/true/false toggles
  and ordinary key lookups cause ZERO full loads, retaining catalog/index Arcs.
- Tests cover uncached arbitrary/savepoint reads, nested activation refusal;
  local/external catalog/source/coverage/hash corruption; DDL and TEMP shadows;
  rolled-back content corruption; preference-write fault atomic pointer/choice
  rollback; disposition changes refused even with a cached catalog.
- Instrumented storage suite passes. Storage-only line coverage is95% for
  content_preferences,99.69% catalog_generation,88.65% content (broader caller
  tests needed). Combined app caller coverage is running; no per-file90pass yet.

Logs: `/tmp/navzeroth-travel-activation-{all-storage,clippy,coverage}.log`;
coverage JSON `/tmp/navzeroth-travel-activation-coverage.json`.

Startup pack verification/parsing/staging/composition, inspection report/hash,
and recovery initialization remain. No elapsed speedup, Windows/ordinary-PC
performance, complete suite budget sufficiency or installed acceptance claimed.
Previous106cc6functional proof remains actualfcf/0a history; this runtime change
requires a fresh coordinator optimized full-pack run and measurements after review.
Next: independent code review, complete affected-file coverage, then reviewed
integration and fresh functional/native/hosted gates. No keys/sign/push performed.
