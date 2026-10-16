# G1 reconcile 0.9.1 publication records and record ADR 0005

Owner and sole writer: Claude Code workflow agent (0.9.2 program, task G1).
Baseline: `origin/main` `92f3f512a68481163e3d44600e3938d07592af4e`.
Branch `docs/g1-reconcile`; worktree `/home/laymer/Devel/worktrees/g1`.
No subagents. The shared checkout is not touched.

Scope: documentation and production doc-comments only; no behaviour change.

Files: `initial.md`, `localdata.md`, `docs/STATUS.md`,
`docs/IMPLEMENTATION_TASKS.md`, `docs/handoffs/2026-10-16-v0.9.1-release.md`,
new `docs/handoffs/2026-10-16-v0.9.1-publication.md`, new
`docs/handoffs/2026-10-16-g1-reconcile.md`,
`docs/sources/source-build-result.json`, `V1_RELEASE_PROCESS.md`,
`release/v1/README.md`, `RELEASE_MATERIALS.md`, `WINDOWS_INSTALL.md`,
`docs/releases/0.9.0.md`, the unsent ForeverDB request note, doc comments in
`crates/core/src/{coverage,reminders}.rs` and `gear/configuration.rs`,
`schemas/v1/README.md`, and new `docs/decisions/0005-v092-program.md`.
`docs/releases/0.9.1.md` stays byte-identical (it is the published
`CANDIDATE_NOTES.md`).

Intended checks: `cargo test --locked -p azeroth-cli`; markdown link check over
changed files; `release-readiness --version 0.9.1` still reports 66 blockers;
`cargo fmt --all --check`; read-only `gh` verification of release facts.
