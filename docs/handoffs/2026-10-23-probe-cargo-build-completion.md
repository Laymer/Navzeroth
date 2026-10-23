# Cargo-only build completion repair

Owner: desktop_review. Baseline23c8047832c03e07dd96dba359b1dcefe9b2fae7;
claim07a1aebd and retention scope extensionfad39f6e preceded edits. Frozen code
38ca7bb80fb4c2fbf2cdd49417202f03c98d2b6a in isolated branch
fix/probe-cargo-build-completion, `/tmp/navzeroth-cargo-build-completion`.

Actual probe39921053021 build.json records Cargo exit0, timed_outtrue1200.032s,
no capture errors; stderr says optimized Finished5m40s. There is no retained
root-exit timestamp, pipe state, process identity or build.stdout for that run.
It demonstrates the strict tree/EOF completion boundary, not a particular daemon
or exact Cargo root-exit time. Installation/recovery never ran. No prior failure
is promoted to a pass, and no runtime performance claim follows from this fix.

Only the known Cargo --no-run call opts into root-completion cleanup. When root
exits, all remaining processes in its freshly owned tree are terminated; no child
is exempted/detached by name. Capture still drains within existing byte and cleanup
bounds. Windows JobObject accounting must become inactive within the shared2s
cleanup bound; query/kill/close/capture failures, nonzero root and timeout still
fail with retained bounded JSON. Generic run's strict root+pipe+tree predicate
and shared OwnedTree/importer code remain unchanged. Build1200s/workload1200s and
workflow60min limits are unchanged. POSIX killpg(0) can include zombies; its
postkill job_inactive diagnostic is null rather than a false inactivity claim.

After drain, strict Cargo JSON requires a unique final build-finished success=true
and one expected test compiler artifact, exact manifest/harness source paths,
owned release/deps binary path/name, regular nonempty bytes and binary/source/
stdout hashes. A contradictory compiler error or malformed/duplicate authority
field refuses before installation. The workflow adds exactly one artifact path
for the already <=16MiB build.stdout, preserving future independent protocol review.

Actual checks on final source:

-12 inert Python tests PASS on Linux3.14 (6.163s) and official portable native
 Windows Python3.14.0 (6.966s). Native invocation:
 `/mnt/c/Users/ikope/AppData/Local/Temp/navzeroth-python3140/python.exe -m unittest discover -s "$(wslpath -w /tmp/navzeroth-cargo-build-completion/scripts/tests)" -p test_hosted_recovery_probe.py -v`.
-Real own-child fixtures with inherited and noninherited pipes complete Cargo-only
 cleanup (~0.13s native) with job inactive and dead recorded child; the default
 generic root-exit/orphan fixture still times out/refuses at5s.
-Protocol negatives cover missing/false/nonbool/duplicate/nonfinal terminal,
 malformed/duplicate JSON, ambiguous/missing artifact, wrong source/manifest/kind,
 outside/missing/empty binary and compiler errors. Root7, hang, output overflow,
 injected kill/close/capture failures retain diagnostics and refuse.
-First added Linux child-liveness check raced asynchronous SIGKILL. A bounded own
 PID poll fixed the test; Windows production cleanup now independently waits for
 job inactivity. No unbounded zombie wait or unrelated process control was added.
-An actual tiny dependency-free offline Linux Cargo test build (not application
 or Windows compiler) completed0.552s and its real JSON/artifact passed the new
 validator. Input/output pins and fixture path are retained at
 /tmp/navzeroth-cargo-protocol-actual.json. No full application build was repeated.
-Python byte compilation, actionlint and git diff --check pass. Shared process
 helper and importer compare byte-identical to baseline.

Logs: /tmp/navzeroth-cargo-build-linux-tests.log,
/tmp/navzeroth-cargo-build-native-tests.log. All native fixtures are inert; no
installer/app/profile/registry/network control, keys, canonical evidence or probe
dispatch occurred. Next: independent frozen-source root/tooling/world review,
then coordinator-owned bounded hosted probe. Its actual runtime remains unverified.
