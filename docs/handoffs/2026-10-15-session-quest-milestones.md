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
Actual separate completion-getter capture now validates:14quest booleans readable,
5true/9false; operator confirmed exact visible counts, switched level13character
and no errors. Level13matches, clockuncertain; raw files private. This is narrow
source-identified development evidence, not candidate-bound release acceptance.
Neither this development update nor synthetic event tests promote release acceptance.

Next: hosted nine checks, reviewed protected integration and resulting-main checks;
then install checked event-binding code with WoW closed and guide one actual
operator-driven quest turn-in if available. Complete licensed nine-domain inventory,
full corpus and frozen-candidate native/live acceptance remain release blockers.


PR151 initial hosted Lua lint found two over100character lines in Diagnostics.lua;
wrapped only those expressions. Added inspector completed/incomplete counts to
compare boolean semantics without printing quest identities; true/false regressions
and all18focused capture/recorder tests passed. Actual private capture yielded
5completed/9incomplete, which the operator independently confirmed. Final hosted
checks remain pending at this record.

PR150 mergedc7e0b7b after all9checks and3sourcejobs passed. Resulting-main3sourcejobs
also passed; downloaded both platform artifacts and verified all1,079Gitblobs,
all9materials, generator/input/catalog/receipt hashes. Both source archives SHA256
4cba99db2f1cdf6f347024bb4db79c8bf1681333c05db3a4a4eae53a25626716.
All1,685facts remain sourceknowledge,0targetusable; fullmain9checks still underway.


Integration follow-up: see the [development evidence closure](2026-10-15-development-evidence-closure.md)
for protected merge commits, resulting-main checks, downloaded artifacts and later
operator observations. Earlier pending statements above describe their record time.
