# v1 stable-release enforcement

Owner / sole writer: Codex `/root/onboarding`.
Task: batch 2 CLI, packaging and tag enforcement for the full-content v1 release.
Baseline: `6db096b`; branch `feat/v1-release-enforcement`;
isolated worktree `/tmp/navzeroth-v1-enforcement`.
Claim recorded before implementation on 2026-10-14.

Read AGENTS.md, STATUS, final published-program handoff, implementation-plan
sections 19–20, and the coordinator's frozen release/v1 contract. Own CLI release
I/O module, entrypoint/library wiring, focused tests, packaging workflow and
Build-Windows.ps1. Core release types/validation remain coordinator-owned; source
adapters and acceptance records remain with their assigned workers. Own only the
record-binding/CLI details section in release/v1/README.md, as delegated by root.

Independent review of coordinator commit6db096b identified unsupported schemas
being ordinary prerelease blockers instead of errors, possible oversized-vector
CPU amplification after flagging bounds, and missing binding from declared
coverage IDs to actual pack records. Root owns core corrections. The frozen CLI
mapping uses canonical JSON-array strings [revision, collection, record identity]
from typed datasets; unsupported role/party/dungeon formats remain explicit
blockers until supported contracts exist. No invented models or game records.

Implementation, actual checks, limitations and next task will be recorded below.
