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
are still pending, so the complete `LIVE-BUILD` row is not passed. Later user
reports identify Windows 11, out-of-combat conditions and no errors from probe/
status commands; actual output and installed source identity remain pending.

## Delivered procedures and evidence boundary

The runbook defines the coordinator's exact 25 mandatory `LIVE-*`/`WIN-*` IDs,
expected methods and pass/fail/unverified vocabulary. It binds operator evidence
to source/artifacts/environment/content/inventory and expands aggregate cases to
every mandatory inventory unit. Procedures cover the complete real duo scenario,
field states, transfer/watching/reload, restrictions/HUD, progression/deviation,
gear/preparation/reminders, sessions/next plan and interactive Windows lifecycle,
recovery, offline use, focus/reference/display/accessibility/full-size performance.
The evidence log retains the actual user-reported GetBuildInfo tuple and narrow
environment/diagnostic reports; it does not promote API or full `LIVE-BUILD`
acceptance.

Actual baseline source inspection found these independently actionable gaps:

- `Diagnostics.lua` and `HudAppearance.lua` still gate observations and UI on
  70338. Updating the version condition alone would not establish capability truth.
- `PlanControls.lua` explicitly has no completion mappings or current dependency
  context; there is no user-facing live advancement path to verify yet.
- Snapshot/session producers store strings in SavedVariables; no supported
  snapshot copy-out UI or CLI extraction command exists for the real text path.
- Manual session sampling observes level/unclassified spans, not complete real
  route/quest milestones. Such sessions cannot prove measured route calibration.
- `PartyScreen.tsx` still renders stale "Session planning is not available yet."
  after its implemented planning component. Root owns scheduling its separate fix.

No existing historical document, source code or schema was changed. Optional API,
party messaging, combat logs and hosting stay explicitly deferrable; the mandatory
live/content/native rows do not become optional when blocked.

## Actual verification

- Inspected installed helper and NSIS hooks, addon commands/gates/plan controls,
  native screens and reference control behavior against each procedure. Read the
  actual wave8 reference-interaction handoff after an initial guessed file name
  was absent; the failed read was not validation evidence.
- A local Python read-only audit found exactly the agreed 25 unique IDs and valid
  existing Markdown file links. Three links intentionally await coordinator
  integration: `release/v1/README.md`, `coverage-inventory.json`, and
  `acceptance-evidence.json`. Recheck them on the combined branch.
- `git diff --check` and `git diff --cached --check` passed. Independent peer
  review follows before final integration. No code tests are required for
  these documentation-only additions; no live or interactive Windows check ran
  in this worker environment.

Draft [PR #114](https://github.com/Laymer/Navzeroth/pull/114) carries the granular
claim, reported-client evidence and runbook commits. Runbook implementation is
`e2b29d1e3824a7419e3da586648e43e11f4be70d`; later changes append actual user
reports and review records only unless identified otherwise.

Next: independent peer review and coordinator-owned evidence contract integration;
then gather the user's installed-addon/OS/combat diagnostics, address the recorded
engineering blockers in isolated tasks, and execute applicable runbook cases on
the exact packaged candidate. Mandatory missing evidence holds stable v1.0.0.
