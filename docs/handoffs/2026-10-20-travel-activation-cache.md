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
- Instrumented storage suite:167passed,0failed; additional app caller suite:
  297passed,0failed,3ignored across44 binaries. Ignored full-pack/environment
  scenarios remain unverified here.
- Combined storage/app LLVM coverage meets the unchanged per-file90% threshold
  in every affected file: content390/432=90.2778%; content_preferences95/100=95%;
  catalog_generation663/665=99.6992%. Storage-only content88.65% was an incomplete
  caller scope, not a gate waiver; existing app callers supplied actual execution.


Logs: `/tmp/navzeroth-travel-activation-{all-storage,clippy,coverage}.log`;
coverage JSON `/tmp/navzeroth-travel-activation-coverage.json`; app execution log
`/tmp/navzeroth-travel-activation-app-coverage.log`; final combined report
`/tmp/navzeroth-travel-activation-combined-coverage.json` (cargo llvm-cov report
-p azeroth-storage -p azeroth-app after both actual instrumented suites).

Startup pack verification/parsing/staging/composition, inspection report/hash,
and recovery initialization remain. No elapsed speedup, Windows/ordinary-PC
performance, complete suite budget sufficiency or installed acceptance claimed.
Previous106cc6functional proof remains actualfcf/0a history; this runtime change
requires a fresh coordinator optimized full-pack run and measurements after review.
Next: independent code review, then reviewed
integration and fresh functional/native/hosted gates. No keys/sign/push performed.

Independent review after frozen runtime121a2a82: coordinator production/test diff
CLEAR and independently read combined coverage390/432,95/100,663/665. Tooling and
desktop each independently cleared the runtime guards and actually executed all17
catalog_generation tests using the matched author binary (not independent rebuilds).
Logs `/tmp/navzeroth-tooling-travel-cache-independent.log` and
`/tmp/navzeroth-desktop-travel-cache-tests.log`. These reviews do not establish
Windows behavior, a new timing result or hosted acceptance. Coordinator authorized
routine branch push/draft PR targeting `release/stable-1.0.0-version`; fresh own
optimized full-pack build/run follows reviewed integration, before renewed closure.
