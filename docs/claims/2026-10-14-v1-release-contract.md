# V1 release contract and integration claim

- Owner/sole writer: Codex /root.
- Baseline: `bb3782cbac13b5fb3e6ca8196278ba7fcc427474` (`main`, clean).
- Branch/worktree: `feat/v1-release-contract`, `/tmp/navzeroth-v1-coordinator`.
- Task: versioned full-content inventory and acceptance evidence contract, stable
  publication enforcement, integration and final evidence reconciliation.
- Authorization: user supplied the v1.0.0 plan, explicitly requested three
  implementation workers plus coordinator and authorized reviewed protected-main
  integration and publication only after mandatory coverage/acceptance gates.
- Target: Forever `1.60.1.70374`; actual client identification is pending. No
  evidence from `70338` or reference data from `70205` is transferable.
- Intended files: new release-domain/CLI modules and tests, `release/v1/`,
  `.github/workflows/`, Windows packaging integration, current status/task records
  and new handoffs. Exchange schema v1 remains unchanged unless a demonstrated
  incompatibility requires a separately reviewed migration.
- Validation: bounded/strict manifest parsing; unknown/failed/missing inventory,
  evidence and source mismatch rejection; prerelease handling; valid synthetic
  acceptance path; affected Rust tests/Clippy/fmt; all nine existing hosted checks
  on each merge candidate. Synthetic passes do not establish live acceptance.

## Batch 1 ownership

| Task | Sole writer | Branch | Files |
| --- | --- | --- | --- |
| Public source and coverage audit | Codex /root/public_sources | docs/v1-public-source-audit | docs/sources and own claim/handoff |
| Consolidated acceptance runbook | Codex /root/acceptance | docs/v1-acceptance-runbook | V1_ACCEPTANCE_RUNBOOK and own claim/handoff |
| Current onboarding/packaging docs | Codex /root/onboarding | docs/v1-onboarding | README, install docs, release process, own claim/handoff |

Each worktree starts at the recorded baseline. Coordinator alone owns shared
inventory/evidence interfaces, STATUS and task ledger. Independent source reviews
precede integration. Historical records and unrelated worktrees remain preserved.
