# Task 2 — guarded quest observations

Owner: Codex Worker A (/root/addon), sole writer. Claimed 2026-10-13.
Baseline: 32cec0f7e004ad8cc0be7b446f5b81b8a1ec3263 (coordination PR47).
Branch: feature/wave3-quest-observations. Worktree: /tmp/navzeroth-wave3-quest.
Scope: bounded pure quest reader, explicit 70338 manual capture binding, Lua
harness regressions and documentation. No shared manifests, STATUS or schemas.
Prerequisite: PR47. Read STATUS, wave3 coordination, snapshot/clock/advancement
handoffs and relevant observation/HUD plan sections before implementation.
Accepted quests and readable completion flags preserve unavailable states.
Absence proves no history; objective index mappings remain absent. Partial or
oversized reads reject replacement. Live client checks remain unverified.
