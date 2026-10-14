# Recovery startup check synchronization

Owner: Codex /root/backlog_audit, sole test writer. Baseline actual origin/main
f4e964f971f36e68856e9c332b7c40493c008ac9. Branch
feature/wave5-recovery-checks; source commit70eb548.

Windows CI job115763008620 for CLI replay PR73 failed the existing recovery
asynchronous-startup test at its five-second assertion. The application library
suite took36.96seconds with16passed/1failed. This is recorded hosted evidence;
the failure did not reproduce in the local baseline run (10 recovery tests passed).

The test now sleeps10milliseconds between status reads and permits a bounded
60second completion wait. This reduces repeated reads of the publication lock
and filesystem status while allowing a busy CI host to finish opening SQLite.
The actual reason for the Windows delay is not established. The terminal status
must still be ready, and the hosted application must retain its imported profile.
This is completion synchronization, not a startup latency benchmark. Production
startup, recovery, application/native APIs and replay are unchanged.

## Actual checks

On Linux at70eb548: cargo test --locked -p azeroth-app passed91tests, no failures
or ignored tests. This includes19library tests and10recovery host tests.
The separate library-only run also passed19tests. Strict locked all-target app
Clippy, workspace cargo fmt --check and git diff --check passed. Raw local logs
are /tmp/navzeroth-recovery-checks-{baseline,tests,app,clippy}.log (not shipped).
Origin/main remained the recorded baseline after fetching before submission.

A fresh hosted Windows run is required; no Windows passing result is claimed.
Coordinator should review the test-only diff, await exact-head CI/package checks,
and integrate normally before rechecking the other batch branches. Next feature
work remains with the separate application/native owners; this task does not
change the implementation plan's native or performance acceptance gates.
