# Guided Windows acceptance kit

Owner/sole writer: Codex `/root/acceptance`, claimed 2026-10-15 before implementation.
Branch `feat/release-acceptance-kit`, isolated worktree `/tmp/navzeroth-release-acceptance`,
baseline `d59aa91`. Scope: new acceptance runner, helpers, tests and operator guide.
Coordinator owns packaging/verifier changes and shared release contracts.
Read project instructions, status, latest release-material/runbook handoffs, release
contracts and implementation-plan M9/definition-of-done requirements.

Implementation and actual checks will be recorded below. Real Windows and Forever
sessions remain unverified; this worker cannot operate either environment.

## Delivered

New `Start-AcceptanceSession.ps1` delegates source-bound candidate/material/artifact
verification to PR6's fifth `Assert-ReleaseArtifacts` argument. It fails closed if
that verifier is absent. Installed addon contents are compared exactly with the
verified ZIP before and after operator work. The runner binds private draft results
to source/production/inventory/material/artifact/scenario/runbook identities, collects
non-identifying environment fields, explicit observed build tuple, operator timings
and reviewed sanitized evidence hashes. It checkpoints results in a new directory
outside the checkout and never writes the release acceptance manifest.

The helper queues all 15 live/10 Windows full procedures and externally predeclared
inventory scenarios; missing units/stale inventory digests refuse. No invented
scenarios are shipped. At least one scenario per unit is a structural minimum;
independent review must verify actual record/subcase coverage. Operator assertions
cannot automatically establish client truth or privacy sanitization. An incomplete
session retains missing rows as unverified and lacks a final identity recheck.

## Actual verification

- PowerShell 7 on Linux (`/tmp/navzeroth-pwsh/pwsh`) passed 16 synthetic checks:
  complete procedure queue, full duo procedure, stale/missing inventory scenarios,
  missing pass evidence, invalid result state, explicit unverified/zero duration,
  JSON checkpoint, exact installed-addon match and extra/changed/missing files,
  traversal/duplicate ZIP entries, and runner syntax parsing.
- `git diff --check` passed.
- An initial fixture used an underscore in a scenario ID and correctly failed its
  strict grammar; the synthetic ID fixture was fixed before the successful runs.
- Native Windows/PowerShell 5.1 execution, real candidate preflight integration,
  live Forever operation and full-content performance are unverified. Candidate
  integration depends on coordinator-owned PR6; unavailable corpus/rights remain
  external release blockers. No product acceptance status was promoted.

Next: independently review this PR, integrate PR6 candidate verifier, exercise its
actual preflight with a complete candidate, and operate the native/live sessions.

Independent review by `/root/inventory` found that a runner from checkout A could
verify checkout B while recording B's fingerprint. Fixed by requiring the executing
tooling directory to equal the candidate root's ordinary `scripts/windows` directory;
reparse paths refuse, and the candidate gate then checks those tracked bytes.
Added matching/foreign-checkout regressions. All 18 synthetic PS7 checks now pass;
`git diff --check` passes. Real native/live environments remain unverified.

Independent re-review of `bef88ae` by `/root/inventory` closed that finding and
reran all 18 portable checks successfully. No remaining blocker/important finding
within this scope; integrated candidate/native/live runs remain unverified.
