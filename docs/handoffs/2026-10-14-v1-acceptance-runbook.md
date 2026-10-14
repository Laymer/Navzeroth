# v1 acceptance runbook — claim and handoff

Owner/sole writer: Codex `/root/acceptance`. Claimed 2026-10-14 before
implementation in `/tmp/navzeroth-v1-acceptance`, branch
`docs/v1-acceptance-runbook`, baseline
`bb3782cbac13b5fb3e6ca8196278ba7fcc427474`.

Scope: consolidate actionable live Forever and interactive Windows acceptance
procedures in `docs/compatibility/V1_ACCEPTANCE_RUNBOOK.md`, with stable evidence
row IDs coordinated with `/root`. Root owns the versioned evidence manifest and
coverage inventory. This task owns only the new runbook and this claim/handoff;
it does not change source, existing historical evidence, schemas, or release state.

Read `AGENTS.md`, `docs/STATUS.md`, the final program audit and v0.8.1 release
handoffs, existing live/install/recovery/transport/HUD documents, and the relevant
implementation-plan requirements and acceptance sections before implementation.

The requested frozen target is Forever `1.60.1.70374`; the expected interface
`16001` must be observed in-game before collecting target acceptance. Historical
`70338` observations and `70205` reference content do not transfer. Live client
and interactive Windows access are unavailable to this worker, so all such checks
remain unverified. Synthetic/browser/hosted checks cannot establish those results.

Scope extension authorized by coordinator on 2026-10-14: also own the new
`docs/compatibility/FOREVER_70374_EVIDENCE.md` to retain the user's fresh build
identification report. The reported `/dump GetBuildInfo()` tuple includes
`1.60.1`, `70374`, `Oct 12 2026`, `16001`, `""`, `" "`. This is narrow
user-reported client identification; installed addon revision and test conditions
are still pending, so the complete `LIVE-BUILD` row is not passed.

Planned verification: inspect the actual slash commands, native UI labels,
installer and helper behavior; check local Markdown links and diff whitespace;
obtain an independent peer review. Record actual results below after completion.
