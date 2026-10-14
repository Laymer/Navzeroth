# Freshness fixture clock margin

Owner: Codex /root. Claimed baseline5123a41, published739a822/v0.6.0.
Separate test-only source df6f5e0; product freshness deadlines/guard code unchanged.
PR94 Rust Linux at5f7aff1 failed its initial successful-calculation assertion:
SnapshotOutdated. The fixture left only50ms until level expiry, and the service
correctly accounts for actual calculation time. Runner load consumed that margin.
Job115897716305/run38577256513 evidence is retained in
/tmp/navzeroth-submissions-main-ci-failure.log. This is a failed check, not a pass.

Fixtures now leave50seconds before independent field expiry and explicitly advance
injected UTC100seconds to cross it. Future observations likewise become usable
only after the explicit transition. Capture remains fresh; original assertions
still require selected advice before expiry, publication refusal after expiry,
filtered results on recalculation, and rejection when uncertain data becomes usable.
False/zero, unknown/unsupported/restricted/not-applicable, stale, uncertain and
extension metadata classes retain their separate behavior.

Actual seven focused cases and strict targeted Clippy passed at df6f5e0. Logs
/tmp/navzeroth-planning-clock-{focused,clippy}.log. No failed hosted task is merged;
updated dependency heads must pass all nine fresh checks after main reconciliation.
Next: exact-source peer review, protected test PR integration, then dependent task
reconciliation and preview publication. No native/live acceptance follows.
