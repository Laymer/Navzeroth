# Continuation 9 — performance and release materials

Owner/sole writer Codex `/root`; baseline `d34b417`, branch
`feat/continuation-performance-materials`. [Claim](../claims/2026-10-18-performance-materials.md).
Continuation 8/PR #198 merged at `9d046b7`; resulting main is integrated. Original checkout, `.claude/` and frozen extraction CLI
are preserved. No personal files or client data were added.

## Measurements

Declared continuation-v1 fixtures before measuring optimized source
`bd4021a2edaed3f7707e44927fd19ebb3adb5329`. The runner built release examples,
then measured sequentially: six actual disk/SQLite application operations,
four existing synthetic planning/cancellation workloads and one Lua callback
burst workload. Every workload retains all 100 samples, warmup/scope and exact
nearest-rank summaries. Source/fixture/lock/executable hashes, Rust/Cargo/Node,
visible CPU/kernel/memory and uncontrolled host-load limits are recorded.
[Raw report and p95 table](../data/PERFORMANCE_SLICES.md). Report SHA256
`8c3e8b4f7258815a9521ef4815f621d679f78ec54e5e64637c16b65fcbc24f83`.
Fresh-import p95 8.602727 ms; current-profile lookup 0.065012 ms; planning
170.957616–419.483811 ms. Fresh imports intentionally retain 110 snapshots;
duplicates preserve history. Failure checks verify activation survived refusal.
Linux RSS is sampled every ten operations. Lua explicit-GC memory grew 2,944
bytes; no flat-memory/no-leak claim. Core cancellation excludes worker/IPC stop.
Ordinary Windows performance, live gameplay and acceptance remain unverified.
The frozen extraction CLI SHA256 was unchanged before/after the runner.

Review identified unguarded Cargo target/config override channels in the original
runner. The original receipt remains unchanged; a read-only audit found no
applicable config or flag/profile/wrapper environment variables. The current
runner refuses target/build/profile/wrapper settings and inherited Cargo config,
with regression tests that do not compile. This subsequent audit is distinct from
proof that the stronger guard executed for the original measurement.

## Distribution materials

Retain 506 files / 4,130,780 bytes of notices and source material (excluding the
manifest): 271 Rust resolved normal/build dependencies, four npm production
packages, standard-library notices, separate Microsoft WebView2 SDK terms and
five original MPL source archives. This includes build tools/workspace feature
unification; it is not a runtime-only SBOM. Full license texts remain unchanged.
Registry manifests/notices match checksum-verified original crate archives;
source archives match Cargo.lock. npm manifests/notices match retained original
tarballs whose SHA512 integrity matches pnpm-lock. Reviewed missing-license
inputs have commit-pinned origins and hashes. Compiler/SDK changes require review.
Microsoft SDK 1.0.3800.47 NuGet (8,973,762 bytes) SHA256
`56c9f26bdd07916a2d1949fb58a5c7e434dfa1173577dca879206050c4e718db`
contains all nine byte-identical wrapper loader variants; its LICENSE/NOTICE are
separate from the Rust wrapper's MIT license. No SDK source availability is claimed.

`dependency-notices.mjs --check` refuses missing, altered, extra, symlinked or
unbounded notice files and stale closure/lock records. Regeneration stages a new
sibling tree, preserves unknown files by refusal and restores the previous tree
if activation fails. Retained hashed inputs have explicit Git checkout attributes.
Windows packaging verifies after dependency acquisition and verifies the installed
notice directory, including static parser notices, after actual hosted install.

Tauri's existing resource map selects addon/helper, public parser notices/fonts
and this bounded notice tree. Private stores/indexes, caches, exports, generated
media and signing keys have no resource mapping. An extra synthetic private export
in an installed-directory simulation was refused. Existing release-material
transport uses explicit selected files/asset manifests; its negative tests stay
required. Ordinary Windows/operator installation is not established by hosted CI.

## Actual verification and next task

- Two Rust application-example invariant/statistics tests passed.
- Workspace all-target Clippy excluding the native shell passed, as did formatting.
- Seven Node notice/measurement tests passed, including traversal, malformed and
  checksum-altered archives, altered/missing/extra/symlink files and config guards.
- Full regeneration followed by exact closure verification passed. Synthetic
  installed-directory source-byte verification and extra-file refusal passed.
- Both independent read-only reviewers' provenance/checkout/build-guard findings
  were resolved; both reported no remaining actionable findings. Reviewers ran
  the seven focused tests. Native Windows packaging was not repeated locally.
- Native PowerShell 5.1 release-material harness was attempted but could not run:
  this Windows host lacks Git on PATH. Do not count it as passed; hosted packaging
  retains the native harness. Exact-head CI/source/package checks remain pending.

Next: integrate reviewed continuation 8/main and merge this PR only after every
applicable exact-head check passes. Then begin the single substantial review/test/
coverage PR: inventory every first-party executable Rust/TS file, enforce ≥90%
per-file coverage using pinned Linux+Windows instrumentation, preserve nine checks
and add Coverage gate. Independent defects get separate PRs. Version/notes,
stability, verified development `1.0.0-rc.1` publication and Pages follow later.
No release/tag or acceptance promotion was performed.
