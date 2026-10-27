# ADR 0009: Automated acceptance of the exact draft 1.0.0 installer

Accepted 2026-10-23 by the owner's approved release continuation plan.
Baseline: `9cc774e4f2656f26d544071d6bd174367ccca16a` (V19 preparation).
Owner: release coordinator; documentation writer: world_review.

## Decision and scope

For unsigned stable 1.0.0, replace ADR 0008's mandatory human draft-installer smoke
with an automated Windows smoke of the independently downloaded **exact draft
installer**. The owner explicitly approved publication after the applicable
revised gates pass without another human confirmation. A human smoke is no longer
a mandatory publication gate. This decision records authorization, not a pass.

Version the release acceptance contract additively as **schema 5**. Keep the eight
existing `hosted_windows` observations: `HOSTED-INSTALL`, `HOSTED-FIRST-START`,
`HOSTED-UPGRADE-RC1`, `HOSTED-DOWNGRADE-REFUSAL`, `HOSTED-PERSISTENCE-RECOVERY`,
`HOSTED-RESOURCES`, `HOSTED-OFFLINE` and `HOSTED-PERFORMANCE`. Add
`DRAFT-INSTALLER-SMOKE` with method **`automated_draft_windows`** as the ninth
required observation. Do not rename or promote an old `OWNER-INSTALLER-SMOKE`
record. Historical schemas 1–4 retain their reading, validation, required methods
and statuses. Their failed, skipped and unverified results remain historical.

The new observation requires an actual Windows run of the downloaded draft asset:

- Independently verify release/tag/source and production identity, asset identity,
  installer SHA-256, checksums, explicitly unsigned installer status, signed content
  receipts and exact installed resources. Run those downloaded bytes, not a local
  rebuild or a different workflow's installer.
- Exercise installation or upgrade, first start, bundled-content activation,
  representative source-backed planning, and restart/persistence. Use actual
  installed native application state and rendered Windows UI; mocked browser
  fixtures, IPC-only assertions or screenshots alone do not satisfy this smoke.
- Retain bounded actual UI/native evidence, workload, environment, automation
  identity, source/build/asset provenance and results. Independently review the
  evidence and its exact artifact binding before accepting the observation.
  No personal exports or live-game observations are needed for the engineering
  workload. Do not claim human, live-client or complete accessibility acceptance.

Draft creation and unsigned preparation must be possible while this post-draft
observation is unverified; publication remains blocked until it actually passes.
Do not make producing the draft depend on its own future smoke evidence or embed
an artifact's future checksum into itself. Relevant executable, bundled-content
or product changes invalidate the smoke and require another verified installer.
Merely recording excluded release evidence does not relabel the executed source.

## Preserved release requirements

Keep all ten protected-main checks, independent review, exact-source platform
parity and release stability. Tag only the verified resulting main commit, verify
the draft assets independently, then run the new exact-draft smoke. Publish only
when every applicable revised gate passes; re-download and verify public assets
before updating/deploying the website. Failures hold publication. Production
20-minute workload and 60-minute suite bounds and the per-file 90% coverage gate
are unchanged. This decision does not authorize time-bound inflation, quota
expansion, purchases or signing a Windows installer.

The installer remains explicitly **unsigned**. Signed content receipts and
independently recorded review evidence remain required. Distinct operator-owned
keys do not establish independent key custody. The target remains Forever
**1.60.1.70450 / interface 16001**. Exactly **1.60.1.70245** remains an admitted
historical Forever input with its true build and warning; 1.12/Classic origins,
unknowns, authored heuristics and marked travel estimates retain ADR 0007/0008's
semantics. No live-client acceptance or launch-build freeze is reintroduced.
Trainer-spell gameplay, talents, professions and enhancements remain excluded.

## Supplemental owner environment

The owner's later Windows 11 upgrade from 0.8.1 is supplemental, not a mandatory
human publication gate. Back up existing data before that upgrade, verify actual
persistence and report the installer's ordinary elevation behavior. Do not bypass
Windows elevation/security controls or automate approval of an unapproved prompt.
Disclose this Windows 11/0.8.1 scenario as **unperformed** until actual evidence
exists; hosted RC1 upgrade evidence does not establish it. An observed relevant
product defect from any supplemental run still holds publication until resolved.

## Backlog and reporting

The owner approved closing generated PR #243 as superseded and acknowledging/
closing build-watch issue #270. Its reported `wowf` 1.60.1.70532 build is a future
compatibility investigation, not an admitted input or target change. Record it in
[BUILD_WATCH.md](../compatibility/BUILD_WATCH.md); preserve the generated run's
historical evidence. This ADR does not authorize merging stale generated data.

Implement schema 5, its strict draft-evidence verification and the automated smoke
in separately claimed/reviewed work. Renew affected source/review/receipt bindings
and run fresh exact-candidate checks. Existing V19 readiness or signatures do not
prove the new schema or draft observation. Disclose automated smoke scope and the
unperformed supplemental owner scenario in release/download documentation.
