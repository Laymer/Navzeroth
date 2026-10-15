# Opt-in quest milestone recording

Owner and sole writer: Codex `/root`. Baseline main
`840939f7118d8022c1a12513f05f4571889a95a2`; isolated worktree
`/tmp/navzeroth-session-quest-milestones`, branch `feat/session-quest-milestones`.
PR150 source implementation is frozen at5508eaf pending hosted checks. Root
retains/transfers ownership of Session.lua, Diagnostics.lua, capture inspector,
tests and shared acceptance documentation to this task. No new agents/writers.

Read STATUS, current native/capture/getter and previous session-recorder handoffs,
and plan7.4/14/M8. Actual operator70374 probes report QUEST_TURNED_IN and
QUEST_LOG_UPDATE valid via C_EventUtils.IsEventValid; function presence/event
validity is not actual payload/dispatch verification. Generated Blizzard interface
source (live branch mirrored atGethe/wow-ui-source, QuestLogDocumentation.lua)
is a reference for questID as first turn-in payload, not Forever target proof.

Add a separate explicit start-quests mode to the existing manual recorder; keep
manual start behavior. Gate supported event registration, require exact recorded
build context, validate only bounded numeric quest identity and preserve gaps/
unclassified timing. Export remains manual and immutable after stop. Do not
record chat/identity/rewards/combat values or automate game actions. No objective
mapping, inferred history or auto-advancement. Test paused/stopped/unsupported/
malformed/overflow/context-change and Rust import behavior. Actual callback and
visual comparison stay unverified until the operator's checked client session.
