# Instrument bundle CLI commands during native coverage

Owner `/root/tooling_review`; baseline
`e5ecdf653ae7e1239b4af8f7e58c5cd2b2f3ee4a`; frozen repair
`a6a48ea0092c1411044359b5ded7f19c5d7d7e5d`.

Actual coverage39424036847 used PR merge checkout
`5d9c89522241eff5285494503289c940a9849d1c` (tree equal to candidate).
TypeScript and both native collectors succeeded. Windows actually passed its6
profile/clean-source regressions, exact signed84-resource assembly and compiled
verification, native smoke32/0/0, and clean-source checks, retaining605 profiles.
The combined gate nevertheless FAILED: only error was `Native entrypoint has no
execution evidence: crates/cli/src/main.rs`. Missing0 and below-threshold0 do not
make that a passing gate. Combined main coverage was371/409; Linux had371 positive
main lines, Windows had0. No installer acceptance is established by coverage smoke.

Both stable bundle assembly and installed-resource verification used ordinary
cargo run under the base environment after instrumented tests. Actual Windows
assembly log explicitly recompiles azeroth-cli, replacing the instrumented main
before LLVM object reporting. Both callbacks now use the actual show-env
instrumentation environment, preserving timeout, expected-exit and output controls.
Compiler discovery/report commands, source guards, raw-profile retention, exact
resource/signature checks and all per-file/native-entrypoint gates are unchanged.

Actual local checks:12 Node tests PASS including a real pinned cargo-llvm-cov0.9.1
and matching LLVM binary experiment; Actionlint and diff checks PASS. The synthetic
Git/Cargo fixture first executes instrumented main, then forces the ordinary
rebuild seen in the Windows logs: LLVM export of that actual executable refuses
`no coverage data found`. A forced instrumented rebuild through the shared bundle
adapter executes assembly and verification arguments and retains positive main
coverage, nonempty owned profiles and clean Git. It makes no Windows Rust execution
claim. The existing workflow fast suite now includes these2 regression cases.

Development failures were retained as development evidence: temporary fixture
initially lacked a default Rust toolchain (fixed by copying the repository pin);
Linux's ordinary Cargo cache initially reused the instrumented binary, so the
control now explicitly forces the observed rebuild rather than claiming Linux
reproduces Windows cache/discovery behavior. A prototype demanded every raw file
be nonempty; final test requires actual positive execution/nonempty profiles while
leaving all raw files present, including empties. No production profile is deleted.

Actual logs/artifacts: `/tmp/navzeroth-v8-coverage-windows-39424036847.log`,
`/tmp/navzeroth-v8-coverage-gate-39424036847.log`,
`/tmp/navzeroth-v8-coverage-verdict-39424036847/per-file-verdict.json`,
`/tmp/navzeroth-v8-coverage-39424036847/{linux,windows}` and
`/tmp/navzeroth-bundle-instrumentation-tests.log`.

Next: independent review and focused normal repair integration, renewed production
and signed receipt bindings, then fresh required coverage on the valid candidate.
Keep failed V8 combined gate failed. No keys, signing, runtime/content changes,
workflow retry, package cancellation or acceptance promotion occurred here.
