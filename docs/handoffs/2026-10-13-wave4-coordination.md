# Wave 4: joint guidance and recurring releases

Owner: Codex /root. Baseline main 1924a844c6878f818c59be6e252419e5d8775825.
The user approved the parallel implementation/release plan and requested execution.
v0.3.0 is published at the baseline; no open PRs; original working tree clean.
Existing worktrees and published history are preserved.

## Ownership and frozen boundaries

Three workers each claim one task before coding in a separate branch/worktree:
A: M7.3 joint planning, then separate M7.5 proposal/acceptance PR.
B: M7.1 reminder persistence/controls, then separate M7.2 rule delivery PR.
C: M8.4 conservative session comparison/calibration.
Root owns shared module exports, snapshots service helpers, native registration,
command aggregation/client, navigation, manifests/locks, migration registry and STATUS.
Workers own new feature-scoped modules/tests and their task claim/handoff.
UI ownership: A PartyScreen and new party components; B new ReminderScreen;
C SessionsScreen and new calibration components. No worker edits shared files.
Workers send wiring/interface requirements to root; root alone applies shared changes.
Storage feature files are disjoint; migration numbers allocated by root before use.

Core remains pure; desktop consumes imported snapshots with full context/fixture
identity. No exchange-schema changes, automatic game input or addon bridge.
Missing real records/models remain gaps. New DTOs are feature-scoped and bounded;
mutation guards identify expected snapshots/revision, refresh invalidates derived
results. All native I/O stays behind existing blocking application boundary.

## PR/release protocol

Each task has granular commits pushed to its draft PR, actual targeted checks,
read-only review and current-head hosted CI. Merge in dependency order through
protected main with merge commits; never force-push. Coordinator checks combined
main and releases next minor preview (first v0.4.0); fixes-only batches use patches.
Version/notes changes have their own release PR. Tag exact checked main; existing
workflow creates draft and assets. Verify downloaded installer/addon/checksums/
source commit/notes before publication. Provide asset links and exact audit.
Re-evaluate all M0–M12 tasks after publication and launch next ready batch.

## Completion and external dependencies

Exhaust dependency-ready work, then report blocked/deferred/unverified items without
claiming full completion. Live WoW and interactive Windows require actual evidence.
Party messaging, combat logs and official API remain capability-gated. M12 expansion
respects focused-release prerequisites. Local file submissions/moderation first;
hosting deferred without demonstrated need; no spending/account creation authorized.

## Actual checks

Read AGENTS, STATUS, latest release and relevant plan sections; clean git inventory;
origin/main refresh; GitHub reports published v0.3.0 and no open PRs. No runtime
checks claimed by this documentation task. Next: launch three claimed worker tasks,
finish detailed task ledger and integrate shared boundaries.
