# V110.4b — versioned progression companion exchange

Owner/sole writer Codex `/root`; baseline `bf332bea`; PR #297 isolated worktree.
Claim: `docs/claims/2026-11-10-v110-progression-exchange.md`.

## Implemented

New progression_snapshot envelope with payload_version 1, exact predecessor
binding/dependencies and separate companion capture identity. Typed observations
cover spells/ranks, money, talents/points, professions, recipes, scoped bags/bank
and equipment enhancements. Quantities preserve zero; recipes preserve false;
partial/unavailable collections never imply absence. Exact item context and
per-scope aggregate uniqueness, observed skill maxima and permanent expiry rules
are validated. Import uses existing byte/depth/map bounds. Default snapshot import
still refuses companions; explicit opt-in does not populate character summaries.

Generated Rust schema/TypeScript and a shared synthetic fixture; all old schema
definitions remain unchanged. Old typed importers refuse the new kind. No changes
to character/route v1. Documentation specifies selected persisted hash checks,
nested freshness, equipment matching and separate manual state integration.

## Actual verification

- Core tests passed in the combined run (existing ignored fixture/LFS checks remain
  ignored). Ingest rerun passed after fixing its historical six-kind assertion.
- Focused progression exchange 4 tests and state 4 tests pass; ingest tests include
  explicit opt-in/refusal and actual fixture predecessor SHA-256/binding checks.
- `pnpm contracts:types` and `pnpm contracts:check`: 74 shared fixtures pass.
- Core/ingest all-target Clippy with warnings denied, fmt and whitespace pass.
- Focused llvm-cov: new contract/progression.rs 22/22 lines, progression/snapshot.rs
  37/37 lines, existing progression/mod.rs 44/45 lines. Not a full release gate.
- Read-only review found no blockers, verified unchanged old schema definitions,
  fixture binding/hash and focused test executable. Added both recommended tests
  (actual predecessor hash and valid permanent/not-applicable expiry).

Initial new ingest tests failed to compile from passing String instead of &str;
corrected before successful checks. The obsolete six-kind assertion failed after
core tests completed; it now uses ContractKind::ALL. No bounds were enlarged.

## Remaining

No migration, application import/UI, addon capture or Lua progression parity yet.
Fixtures establish engineering behavior only, not actual player state. Exact
70862 interface evidence remains pending. Next: V110.4c bounded SQLite companion
storage separated from manual inputs, selected-snapshot invalidation and restart/
backup/restore tests. Then application commands and saved workflow plans.
