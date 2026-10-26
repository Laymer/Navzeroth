# Proven native coverage profile-output repair

Owner/sole writer `/root/tooling_review`; baseline
b689659844eeee8e3a9ee83769e2d9e1a5b50780; claim2caca720; immutable code
b62e5733986c5b251d3405517d45ee327fbab761.

Final-candidate coverage39413135940 used workflow headb689 and actual PR merge
checkoutffbf834ce9fca684d9dd3fcc357a240a81dd40e4. Windows job118595574952 failed
at11:31:25Z immediately after successful coverage-bundle-assembly. The bounded
per-command diagnostic proved the exact first dirty output:
`?? default_1146472104454006394_0_5104.profraw`.
Thus the earlier default-profile hypothesis is now an observed cause. Ordinary
CLI assembly runs in the collector's base environment, which omitted explicit
LLVM_PROFILE_FILE; an instrumented runtime emitted its default profile in the
repository working directory. The guard correctly refused this source dirt.
Native build/smoke were not reached in this run. The final gate failed collection,
not a measured per-file threshold. Raw job log is
/tmp/navzeroth-final-coverage-windows-39413135940.log, line419. Artifact12261296528
was not downloaded again because the completed job log already proves the cause.

The repair sets a controlled absolute LLVM_PROFILE_FILE under the external owned
build target for every base child environment, after rejecting inherited coverage
configuration and deleting inherited runtime settings. Linux retains
nav-%p-%c.profraw and runtime-counter-relocation; Windows retains nav-%p-%m.profraw.
LLVM_PROFILE_FILE_NAME remains identical. The existing show-env instrumentation
spread, same-owned-target checks, all per-command/final clean-source guards and
raw profile retention remain unchanged. Profiles are neither ignored nor deleted
to make source status clean; ordinary assembly and later verification children
now write where the existing retention scans and report collection expect them.

Actual local validation:

-12Node tests PASS across profile-output, clean-source, native-bundle,
 rust-environment and process-output suites.
-The new regression compiles and runs actual instrumented Rust. An ordinary
 launch emits a nonempty root default profile and the real Git clean-source
 check refuses it. A controlled launch emits a nonempty nav profile in the owned
 external target and leaves a second Git checkout clean. Both raw files remain
 present until the temporary fixture cleanup; no ignore/reset is used.
-Actual pinned cargo-llvm-cov0.9.1 show-env accepts the controlled base profile
 setting and preserves the exact owned profile/target/build paths. This is not
 merely an environment-string test. No whole-workspace compile was needed.
-Actionlint for coverage.yml and git diff --check PASS.

The three fast profile/clean-source/native-bundle suites are now wired into the
existing Linux/Windows coverage matrix after installing pinned llvm-cov and before
expensive collection. Local commands used explicit
RUSTC=/home/laymer/.cargo/bin/rustc and CARGO=/home/laymer/.cargo/bin/cargo because
these tools are outside the shell PATH; hosted runners use their installed tools.
Actual Windows execution of the new profile regression and fresh full coverage
remain UNVERIFIED until the next bound release candidate runs.

Separate results retained honestly: final-candidate Linux coverage receipt reports
exactffbf clean source,1607pass/0fail/8ignored across207 binaries; TypeScript has91
files all>=90%lines/global95.98%. Normal CI Windows39413135882 genuinely passed the
three file-identity replacement regressions; log
/tmp/navzeroth-b689-ci-windows.log. These are not a successful combined coverage
result and do not waive fresh Windows/native coverage or any installer/owner case.

Only tooling/workflow/test files changed. Rust product, TypeScript, packs, dataset
origins, raw/code source pins and factual coverage are unchanged. The existing
actual functional scenario proof retains its original tested-source identity;
this repair neither reruns nor restamps it. Release owner will renew production,
public review and signed receipt bindings after independent review. No keys,
signing, runtime/source normalization, production profile deletion, CI retry,
network acquisition, push or merge occurred in this task.

Next: independent review, normal focused repair PR targeting
release/stable-1.0.0-version, then release-owner V7 closure and fresh required
checks. Keep failed run39413135940 failed. All nine canonical acceptance rows
remain unverified, and publication remains blocked.
