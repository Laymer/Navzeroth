# Opt-in quest turn-in milestones

Owner/sole writer Codex `/root`; baseline840939f, claimc401687. Isolated branch
`feat/session-quest-milestones`, worktree `/tmp/navzeroth-session-quest-milestones`.
Merged source PR150 mainc7e0b7b before combined verification. Existing acceptance
worker reviewed read-only with no important findings; no new agents or writers.

`/anavsession start-quests <anonymous-profile-id>` explicitly enables normal
QUEST_TURNED_IN observations in the existing manually managed recorder. Default
start remains manual-only. IsEventValid must return booleantrue and protected
RegisterEvent must succeed with nil/true. Missing/false/throwing/malformed support
refuses opt-in without changing recording identity or saved exports. The operator
reports both QUEST_TURNED_IN/QUEST_LOG_UPDATE valid on70374; actual callback and
payload are not yet verified. Only turn-in is registered; no polling or log inference.

Only active recording receives callbacks; paused/stopped/manual recordings ignore
them. Fresh supported build identity must match the recorded context. First payload
must be a numeric positive integer fitting u32; rewards/other arguments are ignored.
Unreadable identity/context/clock/capacity stops safely with fixed diagnostics.
Each milestone has an explicit unclassified boundary; no active/productive time,
loot entitlement, history, objectives or automatic advancement is inferred. Export
remains manual, bounded and prior valid batches survive refusals. Reload stays idle.
Capture inspector adds only a validated turn-in count, never identities.

Actual checks:262 CLI/Lua-harness tests passed across43 summaries,0failed,
3existingignored, including11recorder and7capture tests. Integration checks use
stubbed70374normal event dispatch, validate Rust session import, preserve manual/
paused/stopped behavior and cross-build refusal. Pure timing/import check yields
35,000ms unclassified/5,000ms gaps/0observed with two milestones. Tests exercise
missing/false/string/throwing support, registrationfailure, malformed payload,
ignored opaque rewards, capacity refusal and saved export retention.

The first overflow test incorrectly assumed128events would precede the existing
JSON/node limit and failed; changed its injected test-only event cap to5 to verify
atomic multi-event refusal without weakening production limits. Final suite passed.
Strict all-target CLI/Lua-harness Clippy, formatting and whitespace passed.
The actual1.0.0readiness command still refuses the intentionally0.8.1production
versions before content/evidence evaluation; no release freeze or gate bypass.

Current checked main840939f addon was installed with the operator-confirmed game
closed; all21files matched Git, private backup saved and SavedVariables unchanged.
Actual separate completion-getter capture/visual comparison remains pending.
Neither this development update nor synthetic event tests promote release acceptance.

Next: hosted nine checks, reviewed protected integration and resulting-main checks;
then install checked event-binding code with WoW closed and guide one actual
operator-driven quest turn-in if available. Complete licensed nine-domain inventory,
full corpus and frozen-candidate native/live acceptance remain release blockers.
