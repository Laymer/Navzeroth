# V110.9c — unsigned1.1.0 artifact policy

Owner/sole writer Codex `/root`; baseline `a410133e`; isolated PR #297 branch.
Claim: `docs/claims/2026-11-10-v110-unsigned-policy.md`.

Added exactly1.1.0, stable/non-prerelease, unsigned to the machine-readable artifact
policy as already authorized by ADR0011. Historical exceptions remain unchanged.
This is an artifact signing exception, not signed-content or acceptance evidence.

Actual checks:22 PowerShell policy checks passed using retained LinuxPS7
`/tmp/navzeroth-pwsh/pwsh`; the website release-metadata Node suite passed;
whitespace passed. New tests refuse using1.0.0 policy/tag identities for1.1.0.
Independent read-only app_progression_review found no blocker and verified exact
selection plus retained downstream readiness/publication gates; reviewer ran
whitespace but did not rerun suites. Native Windows policy execution is not claimed.

Remaining package-windows1.0.0 branches, inventory, native coverage bundle
preparation, version-specific four-feature workloads and evidence validation remain
required. No application/addon version freeze, fresh signatures, acceptance,
main merge, tag or publication occurred. Original checkout remains on
fix/site-dragon-background with its untracked data-generators directory preserved.

Next: typed gameplay facts, full guided workflows and remaining version-aware
release preparation. The observed Windows packaging binding refusal is recorded in
the V110.9b handoff; it must not be bypassed or treated as installer acceptance.
