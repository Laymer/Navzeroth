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
