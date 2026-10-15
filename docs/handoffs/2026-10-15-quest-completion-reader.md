# Separate quest-log completion getter

Owner/sole writer: Codex `/root`; baseline main e45eed1, claim3dd43c9;
isolated worktree `/tmp/navzeroth-quest-completion-reader`, branch
`fix/quest-completion-reader`. Existing worker `/root/acceptance` independently
reviewed read-only with no findings and passed all four quest-capture tests.
No new agents. Shared evidence/status remain coordinator owned; PR148 is the
preceding tooling change and will integrate separately.

Actual source-identified70374 development captures showed30quest records but
zero readable completion flags. The existing adapter expected GetInfo.isComplete.
The operator confirmed both IsComplete and IsQuestFlaggedCompleted are functions;
this establishes presence only, not boolean return semantics or final acceptance.

The manual snapshot binding now passes optional C_QuestLog.IsComplete into the
pure reader. Only valid numeric IDs already in the current log are queried;
headers/invalid IDs never reach it. Calls are bounded by the existing1024entry
limit and wrapped individually. Boolean false/true are preserved with original
capture metadata. Exceptions/zero/nil/other shapes stay Unknown with fixed reason
counts and no raw errors or fallback to a stale info field. If the separate getter
is absent, existing guarded legacy boolean-field behavior remains compatible.
No history lookup, inferred turn-in, objective mapping or automatic advancement
is introduced. Diagnostic capability statuses remain unchanged/untested.

Actual checks: all121Lua harness tests passed across23groups, including both new
getter regressions; strict all-target Lua-harness Clippy, formatting and whitespace
passed. Integration regression exercises stubbed70374 slash capture, JSON export
and existing Rust import. Other regression covers false/true, numericzero,
exception redaction, missing getter, invalid/header filtering and1024/1025bounds.
These are synthetic tests; actual patched-client getter returns remain unverified.
An exploratory addition of the optional getter to the diagnostic catalogue made
its all-present golden fixture fail; that unrelated catalogue change was removed
before the final passing run. Existing diagnostics formats/fixtures are unchanged.

Meanwhile the operator completed current-main manual session recording and clock
checks through PR148 tooling: persisted7event/46102ms-relative recording, matching
build/level30, trusted capture after confirmation and new uncertain capture after
reload. These are development observations, not frozen1.0.0acceptance. Full9domain
inventory/content/rights and advancement/deviation bindings remain outstanding.

Next: all nine hosted PR checks, protected merge after independent review, verify
resulting main, then update the operator's addon with the game closed and compare
actual completion booleans with the visible quest log. Keep missing/restricted
observations unknown; never treat a complete quest as already turned in.
