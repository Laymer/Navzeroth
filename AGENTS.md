# Azeroth Navigator — shared project instructions

Read docs/STATUS.md and the latest relevant handoff before changing code.
Read the relevant sections of AZEROTH_NAVIGATOR_IMPLEMENTATION_PLAN.md.
Preserve unrelated working-tree changes.

Architecture: Rust domain/services, Tauri + React + TypeScript desktop,
SQLite persistence, Lua in-game addon. Keep core logic independent of I/O.

Live game observations remain in the permitted addon environment.
Desktop state is imported snapshot state. Never invent an addon socket bridge.
Do not add memory access, injection, packet decoding, hidden telemetry,
input automation, or workarounds for restricted combat information.

Preserve unknown, unsupported, restricted, stale, false, and zero distinctly.
Version schemas and game datasets. Never mix editions silently.
Parse imports as bounded data; never execute imported Lua.

Claim one task and record its owner and baseline before implementing.
Use one active writer per task. Do not spawn subagents without explicit request.
Make small coherent changes and test the affected behavior.
Use existing conventions and documented commands; do not invent test results.

Record actual checks, decisions, limitations, and the next task in a handoff.
Live WoW checks require evidence from the recorded client build.
Mark missing environment checks unverified and continue independent work.

Do not expose secrets or commit personal exports.
Do not push, merge, publish, purchase, or create external accounts without
applicable user authorization. Routine reversible repository work may proceed.
