# Claim: remove runner-speed dependence from freshness regression fixtures

Owner: Codex /root, sole writer. Protected main5123a41, published739a822.
Branch test/planning-clock-slack; isolated worktree /tmp/navzeroth-planning-clock-slack.
Read STATUS, field-freshness/wave6 guard handoffs and plan6.2/19 checks.
Hosted PR94 Rust Linux failed before its asserted expiry transition because the
fixture left only50ms of real elapsed slack. Production correctly rejected a now-
stale field. Scope: widen fixture margins and advance injected later UTC across the
same transitions; preserve all freshness deadlines/product code and meaningful
stale/newly-usable/false/zero/unknown/extension regressions. Separate test-only PR,
focused seven cases and fresh hosted gates. No failed run is counted as passing.
