# v1 public-source and full-content inventory audit claim

- Owner / sole writer: Codex `/root/public_sources`.
- Baseline: `bb3782cbac13b5fb3e6ca8196278ba7fcc427474`.
- Branch / worktree: `docs/v1-public-source-audit` / `/tmp/navzeroth-v1-sources`.
- Scope: audit QuestieDB first, then CMaNGOS Classic DB; record immutable
  revisions, hashes, provenance, distribution policy, field mappings, coverage
  gaps and bounded-adapter requirements for the requested Forever
  `1.60.1.70374` target. Produce audit documentation and machine-readable source
  audit records; do not import or execute upstream Lua/SQL.
- Shared interfaces: coordinator owns the versioned coverage/evidence contract
  and aggregate registry. This task does not edit shared STATUS or task ledger.
- Preconditions read: AGENTS.md, docs/STATUS.md,
  docs/handoffs/2026-10-14-final-program-audit.md, implementation-plan source,
  travel, equipment, reminders, dungeon and party sections.
- Acceptance: primary-source links and pinned hashes; explicit source counts
  versus unknown target completeness; independent permission/build decisions;
  an actionable mapping and coverage-unit inventory; actual validation and
  limitations recorded in a handoff. No v1 completeness claim follows from an
  upstream support statement.
