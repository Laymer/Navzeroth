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

Independent final review: root repeated the exact workflow four-file suite8/8
PASS at14252c6c; world independently reviewed CLEAR. Author's12-test command
additionally included `scripts/coverage/rust-environment.test.mjs` (4 cases):

```
CARGO=/home/laymer/.cargo/bin/cargo RUSTC=/home/laymer/.cargo/bin/rustc PATH=/home/laymer/.cargo/bin:$PATH node --test scripts/coverage/bundle-instrumentation.test.mjs scripts/coverage/native-bundle.test.mjs scripts/coverage/profile-output.test.mjs scripts/coverage/clean-source.test.mjs scripts/coverage/rust-environment.test.mjs
```

V8 retained artifact identities (run39424036847): Linux12264644445,
Windows12266554990, verdict12267302615, TypeScript12264568824 and baseline12264608783.
SHA-256 of extracted historical evidence:

| Platform/file | Bytes | SHA-256 |
|---|---:|---|
| Linux rust-receipt.json |41617|4d8de54614c9b1fe0f2f482e2ad556f530ac51a3e6638f208ab3d50f7053e344|
| Linux native.lcov |9063670|e026a42bdc5bb18794c86eed5a605eab1f7ba161999ab7e71ff21d14b32ea61b|
| Linux debug.lcov |6545153|a721be69fff4104de661b321b84f70d6274b84c2bf464b8a7334cef23758f7f2|
| Linux release.lcov |2518515|15b8a75dc2e3559af8b4d06f0c0b9578370e4beeea1e4d1666cdcd90aa8c028e|
| Windows rust-receipt.json |44792|566f5cce706ce0b43c3f36229cb71f2ad61b9e8c005639c6cd9d6aba8f80702b|
| Windows native.lcov |10723136|ec9bc7655c9b86aec90b5cda65eaca97be7381eb47cdfed1b6de5dc5e77b376f|
| Windows debug.lcov |8217934|7f20d8c21074ca5c2888c04488bd915f47eb357f4dd437b6d5f3efad3b79563f|
| Windows release.lcov |2505200|4576441cac40df3fc3787b20145d82c1e3d0a3a9fdce996f757a3b7113492e2e|
| per-file-verdict.json |50810|880f9658162e415416fcc912a5b86bb166e63eee78d3683d46a079735d652ad6|
