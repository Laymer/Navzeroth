# Windows first-start smoke claim

Owner and sole writer: Codex `/root`.
Baseline: `32c5e2d9c3cb3f374001ef7a45b16d5b4da2508d`.
Branch: `fix/windows-startup-smoke`; isolated worktree.

Scope: replace the fixed 20-second termination with bounded process/store
observation, fresh-store protection, failure diagnostics before cleanup and
focused smoke-harness tests. Preserve installer/uninstaller assertions and the
existing minimum liveness observation. No production startup changes.

Evidence: PR143 packaging runs 38746649696 and 38751564696 observed a live
process but no store at 20 seconds. An unchanged first revision passed a rerun.
The root cause remains unknown; file creation does not establish UI readiness.
Read STATUS, ForeverDB staging handoff and implementation-plan M9/CI sections.
