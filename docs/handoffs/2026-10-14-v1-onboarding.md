# v1 onboarding and packaging documentation

Owner / sole writer: Codex `/root/onboarding`.
Task: batch 1 current onboarding and packaging documentation for the authorized
full-content v1.0.0 program.
Baseline: `bb3782cbac13b5fb3e6ca8196278ba7fcc427474`.
Branch: `docs/v1-onboarding`; isolated worktree: `/tmp/navzeroth-v1-onboarding`.
Claim recorded before implementation on 2026-10-14.

Read AGENTS.md, docs/STATUS.md, the final program audit, the 0.8.1 release
handoff and implementation-plan sections 19–20. Own README.md,
docs/compatibility/WINDOWS_INSTALL.md, docs/compatibility/ADDON_INSTALL.md,
docs/releases/V1_RELEASE_PROCESS.md and this handoff. The coordinator owns
shared evidence/inventory contracts, scripts, workflows and aggregate status.

Scope: verify current commands and packaging behavior against source; replace
obsolete preview onboarding; distinguish published 0.8.1 and recorded 70338
observations from the frozen v1 target Forever 1.60.1.70374. Document installation,
upgrade, retention, source rights, candidate gates and authorized publication
without inferring live/native acceptance or playable-content completeness.

Checks, review, decisions, limitations and next task will be recorded below.

## Implementation and decisions

Claim commit `107e901`; onboarding/release-process commit `dde4680`; Windows
packaging/retention commit `cb60914e29cff32871928e34d449b1992ac3743e`.
README now links published 0.8.1 and separates its version, recorded 70338 client,
historical 70205 atlas and frozen 70374 target. ADDON_INSTALL documents the current
manual quest/equipment capture, clock confirmation, runtime plan/HUD and manual
session commands while retaining actual compatibility/evidence limits. It no
longer describes the addon as diagnostic-only or the M2.1 code as unreleased.

WINDOWS_INSTALL now matches actual workflow triggers, five published assets,
source metadata, locked commands, backup-before-upgrade and silent retention.
The source inspection found that optional helper SavedVariables deletion applies
to every account under the selected game's conventional WTF/Account directory;
both guides state that scope and the WhatIf preview. Running-process detection
is described as a best-effort observed-path check, not proof the client is closed.

V1_RELEASE_PROCESS records the user's full-content target and existing publication
authorization, required real acceptance/source rights, nine required checks and
exact-source candidate/tag/public asset verification. The coordinator specified
canonical release/v1/README.md and coverage-inventory.json/acceptance-evidence.json;
these and the release-readiness command are explicitly pending separate PRs in
this baseline. Current five-file packaging lacks approved playable content and
required corresponding third-party sources; it cannot establish full v1 readiness.
No public schema, production code, workflow, dataset, original handoff or shared
status/ledger changed in this task.

## Actual verification

- Local Markdown link/anchor check: 37 links across README, both install guides
  and V1_RELEASE_PROCESS passed. Planned root-owned paths are named as planned,
  not linked to missing files.
- `/tmp/navzeroth-pwsh/pwsh -NoProfile -File scripts/windows/Test-AddonInstaller.ps1`:
  all 30 existing synthetic helper checks passed (Linux PowerShell 7). This covers
  copy/update/removal, WhatIf retention, ownership, selected folders, source bounds
  and simulated running-client refusal, not interactive native acceptance.
- `/tmp/navzeroth-pwsh/pwsh -NoProfile -File scripts/windows/Build-Windows.ps1
  -AddonOnly -ExpectedVersion v0.8.1 -OutputDirectory
  /tmp/navzeroth-v1-onboarding-addon` passed from clean `cb60914`.
  Independent Python verification passed the exact four-file AddonOnly set,
  three strict checksum entries, raw 40-ASCII-plus-LF source commit, exact notes,
  ZIP integrity and all 20 addon/license file bytes against that checkout.
  This locally built ZIP is not a new publication or the previously published ZIP.
- `gh api repos/Laymer/Navzeroth/releases/tags/v0.8.1` returned draft=false,
  prerelease=true, published_at=2026-10-14T18:02:38Z, and the exact five asset
  names/sizes/SHA-256 digests recorded in the historical publication audit.
- `git diff --check` passed. Commands, option names, UI labels, stored keys,
  version gates, package contents and retention behavior were checked against
  their current source files.

No Rust/desktop implementation changed, so broad portable/UI suites were not
rerun for this documentation task. Windows installer generation, interactive
Windows operation and live 70374 behavior were not run and remain unverified.
Independent peer review and exact-head hosted PR results are recorded separately
when completed; previous release passes do not stand in for these PR checks.

## Next task

Coordinator: merge the reviewed batch-1 docs through protected checks, integrate
canonical inventory/evidence contracts and stable-readiness enforcement, then
replace explicitly pending command/contract language only when implemented.
Preserve the full-content blockers until permitted source records and actual
70374/Windows acceptance exist. No tag, main merge or publication was performed
by this documentation worker.

Draft PR: https://github.com/Laymer/Navzeroth/pull/113. Task branch is published;
review and hosted checks are pending at this record. Independent contract review
for the coordinator is the worker's next assigned task, without edits to its tree.

Follow-up user evidence forwarded by the coordinator: GetBuildInfo reports
1.60.1 / 70374 / Oct 12 2026 / 16001, Windows 11 out of combat; probe/status
indicate no errors and `/anavdiag` reports 0.8.1. The source archive hash was not
inspected. README/addon/release-process docs now preserve this narrow report,
without promoting unchanged 70338-gated adapters or full target acceptance.

Independent reviewer Codex `/root/acceptance` inspected `26b7ebb` (product docs
`cb60914`), checked commands/UI labels against source, diff whitespace and all 37
relative links, and found no blocking/important findings. Its optional clarification
was applied: preservation backups belong outside installation and application-data
directories; recovery originals live beside the database and can be removed with
application-data deletion. Subsequent build-report/retention-only edits do not
alter commands, application code or evidence promotion rules.
