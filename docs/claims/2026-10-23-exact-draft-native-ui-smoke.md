# Exact downloaded draft native UI smoke driver

Owner world_review, sole writer. Baseline9cc774e4f2656f26d544071d6bd174367ccca16a.
Branch test/exact-draft-native-ui-smoke, isolated worktree. Scope NEW
Draft-Installer-Workload.mjs, draft-installer-ui.mjs and dedicated tests/handoff.
Root owns PS/producers/workflow and existing Stable helpers; desktop owns existing
StableWorkload/progress. Use actual installed verified pack directories, native
rendered Content/Planner controls, persisted state and bounded screenshots.
No synthetic content accepted as shipped content; manual engineering profile/
scenario stays explicitly disclosed. Reuse existing native/stable helper exports.
No real local app/network/game execution; mocks/dev-browser checks only until
independent review and actual hosted exact downloaded installer run.
Read STATUS, V19 handoff, ADR0008 and plan20.2; revised ADR0009 in separate PR271.
