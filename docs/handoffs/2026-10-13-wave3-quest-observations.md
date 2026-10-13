# Task 2 — guarded quest observations

Owner: Codex Worker A (/root/addon), sole writer.
Baseline: 32cec0f7e004ad8cc0be7b446f5b81b8a1ec3263, prerequisite PR47.
Branch feature/wave3-quest-observations; worktree /tmp/navzeroth-wave3-quest.
Claim recorded before implementation in docs/claims/2026-10-13-wave3-quest-observations.md.

## Delivered

Pure Snapshot.readQuests injects count/info APIs. Enumeration is bounded to 1024
and rechecks the count before accepting results. Unreadable entries, exceptions,
duplicates and oversized/changed counts reject the whole capture. Confirmed
headers and invalid IDs are excluded with fixed diagnostic counts. Quest IDs are
canonical decimal strings, scoped by full captured context. Accepted=true and
readable completion booleans carry the same metadata as capture; false and zero
timestamps are preserved. Missing completion is unknown; turn-in and historical
completion remain unknown. No authored stage/objective mappings are invented.

Diagnostics binds only explicit manual snapshot capture on 1.60.1.70338/interface
16001. The catalogue remains untested, with no global capability promotion.
Successful captures persist diagnostic counts separately and expose the most recent
normalized snapshot in runtime state for the next HUD task. Failure retains prior
snapshot text, successful diagnostic counts and sequence. No events/background reads.

## Actual checks

- Initial cargo invocation failed because Cargo was absent from shell PATH; sourced
  the README-documented /home/laymer/.cargo/env and reran successfully.
- cargo fmt --all --check passed after formatting the new Rust test.
- cargo test --locked -p azeroth-lua-harness: 85 passed, zero failures.
  Includes bounded/partial quest reads, false completion, zero UTC, unknown history,
  absent stage mapping, wrong build, count changes, malformed IDs and preservation
  of an earlier export. Actual Lua-generated ANAV1 bytes imported through the Rust
  SnapshotApplication loader.
- cargo clippy --locked -p azeroth-lua-harness --all-targets -- -D warnings passed.
- Existing scratch Lua 5.1 luacheck runner: 14 files, zero warnings/errors.
- git diff --check passed.

## Limits and next task

Actual Forever quest reads, restriction behavior, log entry shape and persistence
remain UNVERIFIED. Missing whole APIs remain an explicit unknown field; a present
but incomplete enumeration refuses replacement. A successful empty log never
proves historical turn-in. The 32 KiB producer cap may reject large logs below the
entry cap rather than truncating. Count consistency cannot prove an atomic client
view when content changes without a count change. New bindings are experimental.

Next task: separate task6 branch/worktree for explicit plan paste/import UI and
conservative HUD bindings after this prerequisite is available. Coordinator owns
shared wiring, reviews, hosted checks and merges; current-head hosted CI evidence
is attached to the PR, not inferred from these synthetic local checks.
