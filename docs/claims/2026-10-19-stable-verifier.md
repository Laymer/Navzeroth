# STABLE-VERIFIER claim

Owner/sole writer: Codex `/root`. Baseline `826aa21862f7ef15d290c453d5bab3caa7806474`.
Branch `fix/stable-download-verifier`; worktree `/tmp/navzeroth-stable-verifier`.
Read STATUS, RC publication/Windows handoffs, implementation plan sections 19–21.
Scope: versioned explicit unsigned release policy and download-verifier inputs,
retaining exact source/artifact binding and Authenticode status checks. No version
bump, gate waiver, tag or publication. Policy is owner-authorized ADR 0008 (#218).
Checks: helper contract tests, existing artifact tests and PowerShell parsing where
available; hosted Windows execution remains unverified until CI runs.
