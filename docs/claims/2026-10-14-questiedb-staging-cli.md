# QuestieDB staging CLI claim

- Owner / sole writer: Codex `/root/public_sources`.
- Baseline: `d49b1b745027493d0aea0b3a070c735e2737813f`.
- Worktree / branch: `/tmp/navzeroth-v1-staging-cli` /
  `feat/questiedb-staging-cli`.
- Scope: new `crates/cli` staging module and synthetic command tests, command
  registration after the onboarding owner's freeze and normal history merge,
  usage documentation and handoff. No ingest/core/interface changes.
- Prerequisites read: shared instructions, STATUS, final-program audit, bounded
  ingestion handoff, and source/import/CLI/test sections of the implementation
  plan. Source audit PR119 and staging PR121 remain the dependencies.
- Acceptance: explicit local raw-entity paths, optional unverified origin
  declarations, bounded regular-file reads, strict argument handling, JSON
  metadata reports without record serialization or content activation. Reject
  duplicate input/options, missing inputs, symlinks, special and oversized files.
  No fetch, directory discovery, build inference or source execution.
- Verification: synthetic CLI argument/file/error/report checks, affected crate
  tests and format/lint, plus the four already-pinned local source files without
  committing their contents. Independent review before coordinator integration.
- Existing source permissions, build completeness and eighteen unapplied
  corrections remain release blockers independently of CLI success.
