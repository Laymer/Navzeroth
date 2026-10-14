# Solo and joint clustering integration

Owner: Codex /root. Released baseline f525911, claim e7f2900.
Dependency: separate pure clustering PR83; root independently ran its final14
regressions together with core/app tests. Worker source469599c was merged normally,
with its handoff6ff5186 and protected-main guard helper cb7e03d; no history rewrite.

Solo planning evaluates each original actual requirement against the imported
snapshot and passes typed evidence/truth into the clustering service. Existing
participant, service-location, quest-stage and unavailable requirement filters
remain. Joint planning reuses existing distinct/fresh/exact-context member checks,
per-member original readiness and utilities, and all-members-usable directed graph.
A separate clustered adapter retains the old JointResult wire shape and the legacy
unclustered pure function. App reports add schema1 clustering explanations outside
flattened PlannerResult. The metadata omits duplicate plan data. Jobs, route
inspection and exports use the same updated app calculation path.

Cluster priority is minimum participant SUM utility; each original choice still
records its minimum member contribution. Screens explain this distinction, shared
inbound travel and additive completion/service costs. No proximity, shared kills,
service savings, speculative unlocks or verified game content is inferred. Old
synthetic/demo reports may omit the additive metadata; ordinary steps remain valid.

Actual checks on backend39398fe plus final core469599c and guard cb7e03d:
407 core/app tests passed (285 core,122 app),0 failures. Strict all-target core/app
Clippy and workspace fmt passed. New3 workflow regressions prove declared same-node
service+quest travel charged once, singleton budget fallback, bounded v1 export of
original IDs without internal variants, joint[9,1]+[1,9] priority10 with original
choice utility1, and restricted quest stage remaining filtered in solo/joint.
Initial synthetic import fixtures failed because QuestProgress identity/context
were missing; corrected before passing checks. No failed result is claimed as pass.

Desktop131 unit tests, typecheck/lint/build and focused workflow/joint/planning-job
mocked Chromium checks passed. Logs /tmp/navzeroth-wave6-integration-*.log.
Mocked browser checks exercise UI lifecycle, not native backend or live gameplay.
Native Windows interactive and recorded-client acceptance remain unverified.
Source-locked peer review of backend39398fe is pending with the core worker;
hosted nine-job checks and dependency reconciliation are required before merge.
Next: review findings, merge separate core task after exact-head checks, update
this separate integration PR with main and rerun all nine checks before merging.
