# Per-file coverage expansion — work in progress

Owner/sole writer Codex `/root`, branch `test/continuation-coverage`; initial
claim baseline `22338e0`, refreshed to main `ac114c8`. Original checkout and other
worktrees remain preserved. Engineering continuations 1–9 have merged.

Inventory and strict raw-report gate are implemented with pinned Vitest V8 and
LLVM tooling. No final coverage or release claim is made. The initial Linux tests
passed, but one truncated worker raw profile prevents strict LLVM merging; retain
it for investigation. Missing native Windows reports and native shell coverage
remain explicit failures. See docs/coverage/README.md for the exact boundary.

Actual current checks:

- 440 UI tests pass in the latest full V8 run; line coverage 89.02% overall,
  twelve source files below 90%. This is a failing coverage iteration.
- Eleven coverage-tool tests pass, including raw/source substitution, missing
  platform evidence and executable initializer classification regressions.
- Desktop typecheck and lint pass.
- Three strict JSON tests pass: duplicate/escaped keys, every truncation, quotas,
  invalid UTF-8/nonfinite/deep input, preserving false/zero and inert text.
- Three application research-boundary tests pass: synthetic partial delivery,
  unavailable totals/completeness, activation refusal preservation, opaque guide
  selections and invalid material kinds. No client/private source was imported.
- Configuration instrumentation was exercised separately; the combined clean-source
  TypeScript collector subsequently ran at `4435865`: 440 tests passed and
  threshold failure was preserved with exit 1 and raw/hash-bound receipts.

Independent interim reviews found and resolved source-hash/empty-native-receipt
and closure-inventory defects. Final collector/CI/test-quality review is pending.
PR #201 fixes a separately discovered hardcoded demo version. Its refreshed
Windows run exposed an independent SQLite sidecar race, now PR #202; both must
merge after exact-head checks and this branch must refresh before final coverage.

Next: exercise the clean-source collector, finish meaningful Rust/UI expansion,
diagnose strict subprocess profiling, implement Linux/Windows collectors including
build scripts/examples and bounded Tauri execution, and add required Coverage gate
while retaining all nine checks. At least 90% per eligible file is mandatory.
Version/notes, stability, verified development RC and Pages follow afterward.

Further verification: 58 focused gear/party UI tests pass, including uncertainty
reason mappings, disjoint ranking outcomes, paging, captured empty/matched/gaps,
unknown acquisition models and catalog failures. Typecheck/lint pass. Twelve
coverage-tool tests pass. The clean-source collector exposed a converter omission:
its pinned AST converter emits no statement for an export-default expression.
The collector now presents that expression with unchanged offsets/source maps as
an expression statement. A regression executes the actual configuration, requires
nonempty measured statements, then verifies all-zero V8 counts remain zero. This
adds executable accounting; it does not synthesize hits or exclude source.

Second clean-source collector run at `1f39b6e`: all 487 UI tests pass; V8 reports
90.93% overall lines and ten source files below 90%, so exits 1. The configuration
receipt now contains actual default-export statement coverage. Subsequent focused
media/input tests (seven) and facts-tool tests (four) pass, as do typecheck/lint:
revision-bound pagination, unsafe image URL refusal, saved selection scope,
forgetting selected paths and clearing derived views. Full rerun follows commit.

Third clean-source TypeScript run at `41e73ac`: 491 tests pass; seven UI files
remain below 90%. Raw-report gate now requires the complete evidence set for each
collector kind (LCOV; Istanbul+test-results; or Istanbul+precise V8), rejects omitted
or duplicate formats, and has omission regressions. This resolves the independent
reviewer's P2; final confirmation remains pending. Twelve tool tests pass. Existing
pnpm Linux/Windows checks now run those tool tests without replacing any check.

Further focused tests pass: six imports (promotion reason/CAS refusal, watcher
receipt dedup and timer cleanup), seven local-data panel tests (poll refusal and
forget without activation), five delivery tests (retained-reference inspection and
comparison), three transport tests (opaque handles/selected paths/false choices).
Desktop typecheck/lint pass. Next full TypeScript run follows this commit.

Linux continuous-profile investigation: a tiny instrumented process killed after
its ready marker retained nonzero profile data in five trials. The ten CLI
lifecycle tests then passed and strict profile merging succeeded. A subsequent
non-native all-target workspace run passed 1,319 tests (four ignored, 180 suites)
and strict LCOV export succeeded. This is diagnostic: source was not frozen for
the whole run, native shell/Windows are absent, and four zero-test harnesses emit
counter-bias runtime diagnostics and empty profiles. All 542 raw files are retained
(538 nonempty, 182,075,416 bytes total); do not treat this as final accepted coverage
or silently discard those four dispositions. The four harnesses are CLI main test
harness, schema-export example, search example and Lua callback example; their
production entrypoints must also execute and produce actual coverage. No baseline
profile was removed. Official LLVM documentation describes continuous recording
and runtime counter relocation, with Windows support needing verification:
https://clang.llvm.org/docs/SourceBasedCodeCoverage.html

Fourth clean-source UI run at `1809f10`: all 498 tests pass; only MapView,
demoAdapter and PlannerScreen remain below 90%. New focused checks now pass:
twelve demo boundary cases (native workflows refuse; one synthetic profile;
unknown identities and commands), map navigation/resize cleanup, and planner
export/clipboard failure and retry (eleven combined map/planner tests).
Typecheck/lint pass. Next clean-source full run follows commit. Independent review
confirmed the required-raw-evidence P2 resolved with no remaining delta findings.

The Linux diagnostic normalized production report has 185 eligible Rust files,
five missing and 46 measured files below 90%; inline test bodies are excluded by
exact cfg(test) ranges. This is not a combined Linux/Windows verdict. The largest
remaining gaps include native archive handling, extraction and app orchestration;
examples need real instrumented execution and native shell/build scripts need
platform collectors. Do not infer readiness from aggregate totals.
