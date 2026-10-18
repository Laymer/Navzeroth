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

Fifth clean-source UI run at `97c173d`: all 512 tests pass; Vitest per-file
thresholds pass, overall 95.73% lines. Strict raw reconstruction still failed one
eligible entry: ui/index.ts consists solely of named/type re-exports and has no
executable initializer or statement counters. The TypeScript AST classifier now
reports pure ExportDeclaration bindings as declaration-only, retaining default
expressions, variables, functions and class bodies as eligible. Regression tests
cover those distinctions. Independent read-only review confirmed this classification
and found no actionable issues. Original frozen baseline inventory is unchanged;
this is explicit no-executable-line accounting, not a source exclusion. All twelve
infrastructure tests pass. Recollect clean-source coverage after this commit before
claiming the TypeScript inventory passes; native Rust remains incomplete.

Sixth clean-source run at `9b28435`: 512 tests pass, command exit 0, all Vitest
per-file thresholds pass. Strict raw reconstruction confirms 66 measured TS/TSX
files pass and 22 declaration-only entries have zero executable lines. There are
no missing or failing TypeScript rows. Combined gate still fails 185 missing Rust
entries when given only the two TypeScript receipts; no combined readiness claim.
Draft Coverage workflow now collects/upload the current inventory, baseline,
raw V8/Istanbul/test results and hash/source receipts. Hosted result is pending;
native collectors and the required combined Coverage gate remain outstanding.

Refreshed onto PR #202/main `3288fa0`; resolved only the STATUS task-summary
conflict by preserving coverage progress and the independent race-fix receipt.
PR #202's reviewed head `26c1d1a` passed CI 39134364783 (all nine), package
39134364699 and source 39134364707. PR #201 is separately refreshed and requires
fresh exact-head checks. The coverage branch must refresh again after it merges.

Native collector/workflow implementation added (not yet executed end to end).
Pinned native LLVM identity, clean-source receipts, debug/release LCOV union,
all-target tests/build scripts/examples, isolated Windows synthetic app state and
bounded graceful Tauri shutdown are wired into Linux/Windows jobs. Raw profiles,
including empty/corrupt files on failure, logs and manifests are retained; build
outputs and synthetic desktop databases are excluded from artifacts. The combined
Coverage gate requires both collectors' success and reconstructs all normalized
lines. Existing nine checks remain; branch required-check settings are unchanged
until the new gate passes. Windows smoke requires an ephemeral hosted runner.

Both read-only reviewers found the same failure-preservation P2: a success receipt
could precede raw retention, and an always-running gate could ignore failed jobs.
Fixed both: emit Rust success receipt only after retention/manifest succeeds, and
require successful TypeScript/Rust collection jobs before accepting artifacts.
Final confirmation and first hosted collector runs remain pending. Actual local
checks: 14 coverage-tool tests plus 13 native-smoke helper tests pass, including
inert shell-assignment parsing, unknown/duplicate/truncated expressions and PID
bounds/normal-close construction. Three existing build-configuration guard tests
also pass. Native smoke and collector end-to-end remain unverified locally (Linux
lacks dbus development metadata; no native Windows Rust toolchain).

First hosted Coverage run 39136215077 at head `8d94b35` / synthetic merge source
`526c5d0`: TypeScript job passed. Linux all-target native workspace tests passed
1,325 tests, four ignored, 182 suites. Collector then failed because combined
stdout/stderr inserted cargo-llvm-cov's informational message into the inert
assignment parser. Raw profiles/logs/manifest were retained in artifact
`coverage-rust-Linux-526c5d05b2f60414cd1dd133455376e48aff0b64`; no Rust success
receipt or accepted coverage result was produced. Windows collector is still
running as this update is written. Separate required Windows Clippy failed on
an unused mutable binding in the synthetic research test directory helper.

Fixes: machine-readable environment output now uses a separate bounded stdout
pipe while stderr stays in retained logs; two subprocess regressions cover stderr
contamination and nonzero results. Actual pinned cargo-llvm-cov show-env now parses
successfully with an owned target directory. Directory creation is cfg-specific,
preserving Unix0700 without a Windows mutable binding. Three affected research
boundary tests and app all-target Clippy pass locally. Thirty-two tooling/helper/
build-guard tests pass; syntax, Rust formatting and whitespace checks pass.

Additional actual Rust expansion: two party assessment tests pass for unavailable,
restricted, unsupported, stale/uncertain reasons, phase order, blocked states and
preserved catch-up alternatives. Three session workflow tests pass, including
selected text/SavedVariables parity, zero duration, duplicate idempotence, invalid
key/global/missing file/inert-code refusal and unchanged snapshot/history.
Nineteen receipt tests pass, including four new format-three canonical roundtrip,
wrong/revoked signer keys, signature/role/duplicate assembly, unknown version,
altered independent production/source/delivery/frozen-target expectations and
pending-target refusal. Local-data receipt Clippy and app all-target Clippy pass.
These tests use synthetic fixtures/keys only and establish no operator acceptance.

Windows native collector also stopped at the same environment-parser failure
after its workspace-test command completed successfully (job 117701143049).
No successful native coverage collection is claimed. Both original raw artifacts
remain in run 39136215077. Refreshed this branch to main `02cbfd7`, preserving
coverage status and merge receipts; PR #201's package-metadata version correction
and regression are now included. Conflicts were limited to STATUS prose.
Next: fresh complete collector runs, resolve actual per-file Rust gaps, final
independent test-quality review, then the required gate rule/version/release work.

Archive FFI coverage subtask claimed at `4e4bee1` (baseline `3bc9d18`). The
private Calls table makes the Rust ABI/ownership boundary explicit. Production
Archive::open always selects LINKED; only cfg(test) code supplies synthetic
responses. Handles retain their matching table, serialization and thread affinity;
C++ shim and supported formats are unchanged. Tests exercise whole/empty/fallback,
null handles, quotas, encrypted gaps, malformed counts/layouts, read failures and
RAII cleanup. Encrypted spans are never reread or retained, and plain bytes remain
exact. A separate real linked-shim test rejects empty storage, leaves the source
directory empty and confirms the decryption guard. Both independent read-only
reviewers reported no actionable findings, with no evidence promotion or production
exclusions. Synthetic ABI tests do not establish real-archive verification.

Actual local checks: eight focused native tests; all twenty local-data unit tests
plus the one linked decryption integration test; local-data all-target Clippy;
formatting and whitespace checks pass. A new isolated instrumented diagnostic run
(not clean committed evidence) reports native.rs 215/216 production lines covered
(99.537%, no threshold rounding), excluding exact test-module ranges. Its lone
uncovered line is the poisoned-mutex refusal; no real OOM is induced. Fresh full
Linux/Windows source-bound reports still determine final coverage.

Collector identity validation also now requires the repository's exact Rust
release and exact numeric LLVM tool version (compiler-specific suffix allowed),
rejecting 23.1.10 when 23.1.1 is required and refusing inherited LLVM_COV/
LLVM_PROFDATA overrides. Thirty-three tooling/helper/build-guard tests pass.
Current hosted collectors remain at `3bc9d18` while these changes are kept local;
finish inspecting that run before pushing the next head.
