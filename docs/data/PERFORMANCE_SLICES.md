# Reproducible development performance slices

These fixtures measure engineering behavior, not game content or acceptance.
The declarations precede measurement: `fixtures/performance/continuation-v1.json`
and the existing `search-v1.json`. No personal installation/export/store is read.

The application harness creates one owned temporary directory and on-disk SQLite
store. It measures six operations with 10 warmups and 100 samples: fresh synthetic
snapshot imports, exact duplicates, current snapshot lookup, active synthetic
content lookup, repeat staging and malformed staging with activation preservation.
Fresh imports intentionally retain 110 snapshots; duplicates must not add history.
Fresh-import timing includes synthetic input construction, parsing, normalization
and transactions. Refusal timing also includes checking that activation survived.
Current RSS is sampled every ten operations on Linux; other platforms report null.
Main SQLite/WAL/shared-memory sizes are reported separately. No private path is
reported, and the temporary store is removed after closing the application service.

Planning reuses the original deterministic star fixtures unchanged: focused solo,
larger solo and focused parties of two/40. Each has 128 candidates; graph sizes
and disconnected-node conditions are declared in search-v1. Timings include request
construction; fixture construction is excluded. Cancellation interrupts after
1,000 core checkpoints. This excludes IPC, scheduling and extraction-worker stop
latency. Lua measures 100 synthetic HUD refresh callbacks per sample in the strict
Lua 5.1 sandbox; explicit collection brackets retained-memory measurement. It does
not measure native game events, restricted combat APIs or operator play.

Run with Cargo/rustc/Node on PATH from a clean committed worktree:

```
node --test scripts/measure-continuation.test.mjs
cargo test -p azeroth-app --example workflow_performance
node scripts/measure-continuation.mjs --output /tmp/new-performance-report.json
```

The runner refuses existing outputs, dirty source and release compiler/profile
environment overrides. It builds optimized examples before measuring, runs them
sequentially with bounded timeouts and validates raw sample counts and exact
nearest-rank p95/median/max summaries. It records source commit, fixture/code/lock
hashes, executable hashes, compiler/build mode and visible CPU/OS/memory metadata.
No hostname, user name, account path or environment-variable value is recorded.
`CARGO_TARGET_DIR` may point to an existing target directory; the CLI executable is
not rebuilt or replaced. The app example's `--verify` runs a small debug harness
check, which must never be presented as an optimized performance measurement.

Report raw samples, scope, source and environment with the measurements. Host load,
thermal policy, filesystem cache and background activity are uncontrolled. RSS
traces cover a finite repeated-use window and do not prove absence of long-term
leaks. Ordinary Windows and live/native acceptance stay unverified until supported
operator evidence exists; hosted Windows is a separate environment. These reports
have `acceptance_evidence: false`, cannot satisfy candidate-bound performance proof,
and do not relax release policy. Initial sub-100 ms lookup/sub-500 ms planning goals
are comparison budgets, not numerical CI thresholds or broad support claims.

## Recorded WSL measurement, 2026-10-18

[Raw report](../performance/2026-10-18-continuation-wsl-bd4021a.json) records
source `bd4021a2edaed3f7707e44927fd19ebb3adb5329`, optimized build, exact inputs,
compiler/CPU/OS, all 100 samples per workload and binary hashes. Report SHA256:
`8c3e8b4f7258815a9521ef4815f621d679f78ec54e5e64637c16b65fcbc24f83`.

| Workload | p95 ms |
| --- | ---: |
| Fresh snapshot import (110 snapshots intentionally retained) | 8.602727 |
| Duplicate snapshot import | 8.114945 |
| Current profile lookup | 0.065012 |
| Active content lookup | 0.109808 |
| Repeated staging | 0.852676 |
| Refused staging and preserved-activation lookup | 0.060922 |
| Focused solo planning | 175.843693 |
| Larger solo planning | 419.483811 |
| Joint planning, two members | 170.957616 |
| Joint planning, 40 members | 173.127421 |
| 100 synthetic addon HUD callbacks | 0.661797 |

Linux RSS during fresh imports grew from 10,240 to 10,752 KiB while intentionally
retaining 110 snapshots. The duplicate-import trace remained at 10,752 KiB;
lookup, activation and staging traces remained flat within each measured window
at their respective later baselines. Lua explicit-GC retained memory increased
by 2,944 bytes (505,131 to 508,075). These finite windows do not prove absence of
leaks. Core cancellation at 1,000 checkpoints ranged 0.088383–0.183628 ms; this is
not an extraction-worker or desktop cancellation measurement. Earlier actual
extraction interruption/resume evidence remains in the lifecycle handoff.

Ordinary Windows performance measurements remain unverified. WSL host load and
caches were uncontrolled; synthetic timings do not establish a live-client budget
or release acceptance. The frozen extraction CLI binary was SHA256-checked before
and after measurement and remained unchanged.

The original bd4021a runner rejected ordinary RUSTFLAGS/profile environment
settings but did not yet enforce target-specific flags or inherited Cargo config.
A subsequent read-only audit found no applicable config files or target/profile/
wrapper variables in this measurement environment. The raw report remains
unchanged. The current runner additionally rejects those channels, with no-build
regression checks; the older report alone does not prove that stronger guard ran.
