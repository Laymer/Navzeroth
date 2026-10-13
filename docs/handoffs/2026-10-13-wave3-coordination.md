# Wave 3 coordination and frozen boundaries

Owner: Codex /root. Claimed baseline main 449a726460fb44701cda8012f7f9255be441f22e.
Branch docs/wave3-coordination; worktree /tmp/navzeroth-wave3-coordination.
The user requests ten separate PRs, parallel implementation, reviewed merges in
order, and 0.3.0-rc.1 CI candidate artifacts. Main merges are authorized by this
request. No release publication or tag creation is authorized or included.

## Inventory and ownership

Refreshed origin/main equals baseline; root working tree clean; no open PRs.
All existing branches and 24 worktrees are preserved. Worker A (/root/addon)
owns tasks 2,6,9; Worker B (/root/gear) owns 3,7; Worker C (/root/sessions)
owns 4,8 and exclusively migration v5. Coordinator owns tasks 1,5,10 and
shared manifests, lockfiles, exports, command registration, navigation,
aggregated command contracts, CI and STATUS. Each task records a separate
baseline, branch, worktree, handoff and actual verification. Workers can send
shared wiring patches; only coordinator applies those shared-file changes.

## Frozen application boundaries

Exchange schemas remain v1. Feature DTO modules live in crates/app/src/gear.rs,
sessions.rs and routes.rs, mirrored by commands/gear.ts, sessions.ts and routes.ts.
All new native service calls use the existing blocking boundary and snake_case
arguments. Profile IDs resolve complete character/context/fixture identity on
the server; clients never select an arbitrary namespace.

- get_gear_catalog {profile_id}; compare_gear {text}. JSON comparison request
  contains profile_id, snapshot_id, revision and explicit bounded model inputs.
  Maximum 1 MiB, 128 candidates; configuration enumeration retains its cap.
- inspect_plan_route {text}: same validated PlanningRequest as plan_session.
  Recompute against expected current snapshot/revision, then concatenate selected
  travel legs and stopping route. Projection uses that exact dataset; ordered
  hops survive missing transforms, imagery and incompatible maps/floors.
- import_session_text {profile_id,text}; import_session_file {profile_id,path,format};
  list_sessions {profile_id}; get_session_detail {profile_id,session_id};
  delete_session {profile_id,session_id}. Dedicated ANAVSESSION1 framing and
  version-1 anav-session-events document; no exchange kind is added. Selected
  file format supports text or bounded SavedVariables with session_export.
  Document cap 32 KiB/depth 32; immutable context/fixture scoped storage, 128
  sessions/profile, duplicate idempotence, atomic identity-conflict refusal.

Quest capture only enriches explicit manual snapshot capture. It never infers
turn-in/history or maps objective indices to authored IDs. Addon plan state is
runtime-only; failed import retains the prior plan. Recorder is explicitly
started/sampled/paused/resumed/stopped/exported; it never samples in background.
Worker A owns Diagnostics.lua bindings; Worker C sends recorder wiring to A
and coordinator to avoid simultaneous writes. Coordinator registers new TOC files.

## Evidence reconciliation

The supplied delivery plan reports successful trusted-age display and uncertain
age after reload on the previously recorded 1.60.1.70338 / interface 16001 client.
These are user-reported results, without raw exports, independently inspected
installed revision, exact age values, or combat/zone conditions in this context.
Record only that limited clock/freshness flow. Other adapters/UI/restricted reads,
actual native interaction and full live acceptance remain unverified.

## Checks and next task

Actual checks: git inventory/status, origin refresh, open-PR inventory, documentation
and relevant source reads. No production behavior changed. git diff --check is
required before commit. CI/Windows results will be linked from the current-head PR.
Next: tasks 2–4 concurrently and coordinator route inspection (task 5); review
read-only across workers, resolve reproduced findings, merge in specified order
only after all eight CI jobs and Windows packaging pass for each current head.
