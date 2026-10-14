# Bounded QuestieDB staging claim

- Owner / sole writer: Codex `/root/public_sources`.
- Baseline: `bb3782cbac13b5fb3e6ca8196278ba7fcc427474`.
- Worktree / branch: `/tmp/navzeroth-v1-ingestion` / `feat/questiedb-staging`.
- Scope: `crates/ingest` parser, reports and synthetic tests; claim and handoff.
  CLI is owned by another worker. Coordinator owns source/evidence contracts.
- Prerequisites: shared instructions, STATUS, final-program handoff and source/
  parser/test sections of the implementation plan read; frozen source audit
  `7ec7429` / PR119 provides the four raw wrappers and field maps.
- Acceptance: parse bounded literal data without Lua execution; preserve nil,
  false, zero, field absence and wide integer masks; reject unexpected wrappers,
  unknown schema fields, duplicate keys, unsafe expressions and excessive input;
  report original source identity and unresolved correction/build/permission
  coverage. No playable activation or upstream content redistribution.
- Verification: meaningful synthetic malformed/boundary/preservation tests,
  affected crate tests and lint/format; optional local pinned source files only
  for parser measurements, with no copied gameplay fixtures committed.
