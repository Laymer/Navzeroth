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

Second hosted Coverage run 39137046033, head `3bc9d18`, tested merge source
`8954b62`: TypeScript and Linux native collectors succeeded. Linux collected all
workspace tests, build scripts, schema export, debug refusals/verifications and
all three release examples, then strictly merged/exported debug/release profiles.
Windows workspace tests succeeded but its default environment output left native
backslashes/control separators unquoted; the POSIX-only decoder refused that
output. Windows collection and the combined gate correctly failed. Raw artifacts
for both platforms and TS remain retained under that exact source SHA.

Collector now requests cargo-llvm-cov show-env --sh explicitly on both platforms;
the inert parser accepts the export prefix and preserves native Windows paths,
spaces and encoded flags inside POSIX literals. Regression covers those values,
and actual pinned --sh output parsed with an owned target locally. Thirty-four
infrastructure/helper/build-guard tests pass. Native Windows confirmation awaits
the next hosted run; no shell environment output is executed.

Additional core checks: all 21 core unit tests pass, including new generic serde
owned-string/nonfinite/byte-value behavior and numeric-schema-format cleanup that
preserves zero bounds, false defaults, literal data and unknown string formats.
Three media manifest tests pass, exercising all read-refusal kinds, decoded pixel
limits, absent/altered lineage, calibration/gameplay promotion refusal, exact asset
count boundaries and duplicate/out-of-order IDs. Core all-target Clippy passes.
These are contract/boundary checks; no real image or gameplay evidence is implied.

Additional storage boundary expansion at baseline `d4707db`: two local-state JSON
tests preserve false, zero, negative/fractional numbers, null and inert Lua text;
reject nested escaped duplicate keys, truncation, trailing input, overflow, exact
byte-limit violations, ignored typed fields and wrong typed values. Generic serde
inputs additionally refuse nonfinite numbers and unsupported byte values. Both
tests pass with `cargo test -p azeroth-storage today::json::tests --lib`; storage
all-target Clippy passes. An initial filter `today_json::tests` matched zero tests;
the corrected module filter executed both tests. Formatting passes.

Hosted run 39138458699 at `d4707db`: TypeScript collection succeeded; both native
collectors remain in progress at this checkpoint. No combined coverage result or
release readiness is claimed. Next: inspect those native artifacts, expand the
remaining Rust gaps and complete independent review before version/publication.

PR #203 repair, owner `/root`, baseline `49eb29a`: hosted d4707db CI, source
parity and Windows packaging passed. Both native collectors also succeeded, but
the combined gate refused real Windows LCOV module paths containing internal
`tests/../` aliases. Lexical normalization now precedes containment and exact
inventory/hash validation. Compiler virtual `rustc/<40hex>/library/` paths are
recognized only when absent from the inventory; owned files always win. Unknown
source, escaping paths, ADS and missing/altered hashes remain refusals. Eight gate
tests pass, including captured Windows forms. All 32 coverage/native-smoke helper
tests pass. The original raw reports remain unchanged.

Reconstruction then correctly refused missing desktop-entrypoint execution. The
smoke ran successfully, but Tauri's `run` exits the process directly, preventing
Windows native profile flushing. The shell now uses `build().run_return()` and
returns its status through Rust's entrypoint, including both worker dispatches.
A unit test covers success/refusal/out-of-range exit codes; native execution is
awaiting hosted confirmation (local GTK/dbus prerequisites unavailable). Collector
now executes both native worker EOF refusals as well as the graceful GUI smoke.
No coverage threshold, source exclusion or acceptance requirement is relaxed.
Next: validate corrected native flushing and finish actual per-file Rust gaps.

Additional coverage tests: all four core gameplay tests pass, including exact
inventory-type equipment mappings (paired slots, non-equipment zero and unsupported
values). The preparation JSON boundary test passes for preserved false/zero/negative
values, unknown schemas and generic unsupported-byte refusal.

Next subtask claim: `/root`, sole writer, baseline `b2ff237`: expose a private
archive-opening boundary in extraction so synthetic ABI responses can exercise
real staging/checkpoint/refusal code. Production always opens the linked archive;
no runtime fixture selector or availability promotion is permitted. Tests remain
synthetic lifecycle evidence, separate from real archive/client acceptance.

Extraction seam implementation: production delegates only to linked Archive::open;
only cfg(test) code supplies synthetic ABI responses. Four orchestration tests pass
for immutable unsupported content/no implicit activation, interrupted checkpoint
resume, changed-source refusal, cancellation, corrupt hash versus layout mismatch
versus justified empty decoded tables, all 12 explicitly installed locales and
invalid selections before job creation. All-target local-data Clippy passes.
Independent read-only review found no actionable issues. A fresh isolated unit-only
LLVM diagnostic reports extraction.rs 607/690 lines and native.rs 215/216; it is
not clean-source/full-workspace evidence and cannot establish the final gate.
All raw diagnostic outputs remain local under /tmp/nav-extraction-boundary*.

Coverage normalization additionally rejects lexical checkout escapes explicitly
instead of treating them as external files; the eight gate tests pass.

Native IPC expansion prepared: 47 deliberately invalid application requests are
sent through real renderer-to-Rust dispatch, with structured AppError refusals and
unchanged current-reference assertions. The matrix never opens selection dialogs
or supplies private paths. Transport/unknown-command failures and unexpected
success fail the harness. Its local validation regression passes; actual native
execution remains pending hosted smoke. Existing package helper checks include it.

Missing native entrypoint counters now produce a full failing per-file verdict
instead of throwing before artifact creation. All 34 coverage/native-helper tests
pass. Reconstructing retained d4707db reports produces zero missing files, 37 Rust
files below threshold and the explicit missing-entrypoint error. This remains a
failing historical measurement; none of the thresholds or executable files changed.

Additional archive-boundary expansion (same sole writer, media extension of the
private opener seam): production media extraction always uses linked Archive::open.
Test-only orchestration exercises supported BLP decoding and all eight native read
refusals, selection/fingerprint refusal and preservation of an earlier media
revision. All 25 local-data unit tests pass; local-data/app all-target Clippy passes.
Oversized serialized reference metadata test passes. Three reminder tests pass
including absent-profile read/write refusal; three authored-content tests pass
including synthetic-provenance refusal. The retained Map definition source hash
is independently pinned and its focused test passes. Core/storage/app/local-data
all-target Clippy passed before the media extension; local-data/app were rerun
after it. The Windows process-name-only precondition test awaits native execution.
No game files are selected by these tests, and synthetic media is deleted with
the owned test store; it establishes no real-client or gameplay acceptance.

Further actual checks: five CLI argument tests and five content-validation tests
pass, including human doctor output preserving unverified native/live status,
schema output refusal preserving an existing file, malformed directory shapes,
129-file quota, inert malformed JSON diagnostics and semantic-error human output.
Repeated evidence pins deduplicate, while conflicting hashes refuse (focused unit
test passes). Two catalog-page tests pass across legacy collection bindings and
all v2 unresolved-index refusals. Three planning-job tests pass for poisoned
registry/outcome/pin locks, no partial success publication, real elapsed-budget
refusal without sleeping, cancellation distinction and all progress phases.
Imagery inventory provenance test passes for embeddings/pins and absent node
provenance, duplicate identities and unresolved evidence. Core content-v2 suite
passes 11 tests with one explicit fixture regeneration ignored, including recursive
quest/objective/map/equipment references and documented-model/guide bindings.
Core/app/CLI all-target Clippy passes. These remain synthetic/domain checks.

Fresh non-native Linux diagnostic at clean `ac76c10`: 1,362 tests pass, four
explicit regeneration tests ignored, 181 suites; strict LLVM export succeeds.
No native shell was included, so this is diagnostic evidence only. Raw profiles
remain in target/coverage-local-expansion; report/log/gaps in /tmp/nav-local-expansion*.
Twenty Linux files remain below 90%, including platform-specific files; fresh
Windows union is required. The 37-file retained historical report is superseded
for investigation only, not as accepted clean cross-platform evidence.

Addressed independent review P3: legacy catalog-page expected records/counts now
come independently from the fixture collection, preventing a wrong-collection
implementation from passing with only a matching label. Both page tests pass.
Media locale mask test and real CLI framed-worker refusal test pass. Acquisition
origin checks now verify quest/vendor/drop/crafting kinds against their base
acquisition identity; focused test passes. Core/app/CLI/local-data Clippy passes.

Next subtask claim: `/root`, sole writer, baseline `ac76c10`: make the thin
reference-window adapter generic over Tauri's Runtime and exercise dispatch through
the official mock runtime. Production continues using Wry; these tests establish
no ordinary-Windows focus/overlay/operator acceptance. Native hosted proof remains
separate. No permissions or game integration will be added.

Correction to the preceding page-test checkpoint: the first fixture-derived
comparison failed because optional empty imagery collections are omitted by
legacy serialization. Expected missing optional collections now mean the historical
empty default; present non-array values fail. The actual corrected result follows.

Corrected fixture-derived catalog tests now both pass. Reference-window Runtime
generic refactor and official mock-runtime dispatch test are prepared; formatting
and locked metadata resolution pass. Native compilation/execution is pending
because this local Linux environment lacks its GTK/dbus development prerequisites.
No permission/capability changes or new packages are introduced (Tauri's empty
`test` feature is enabled only as a dev dependency). Production remains Wry.

Next thin-adapter subtask, same owner and baseline `c411bd6`: consolidate identical
HTTP no-store/nosniff response construction in atlas_protocol and test its transport
headers/body/refusals. Asset authorization and integrity remain in application
services; synthetic response bytes cannot establish gameplay availability.

Hosted `b2ff237`, run 39149391518 / merge source `4fd076dd`: both native
collectors and TypeScript succeeded. Returning through Rust's native entrypoint
retains desktop counters; the combined verdict has no evidence errors or missing
files. It still correctly fails 34 Rust files below threshold, prior to the local
expansion commits. CI39149391500, source39149391517 and packaging39149391530
succeeded on that head. Original raw artifacts and full verdict remain retained;
local verdict /tmp/nav-coverage-fourth/verdict/per-file-verdict.json.

Thin atlas transport refactor preserves validated application providers, sharing
method/query refusal and no-store/nosniff response construction. Independent review
found a vacuous missing-asset query test; replaced its query claim with a positive
exact opaque-path provider response followed by same-path query404 and POST405
assertions that panic if the provider is consulted. Both production adapters use
the guard before their unchanged application integrity checks. Read-only reviewer
confirmed resolution and no remaining findings. Native compilation/execution of
new adapter tests remains pending hosted checks; no local native pass is claimed.

Next native adapter claim: `/root`, sole writer, baseline `d611fc5`. Reuse the
explicit invalid-request cases through the official Tauri mock IPC runtime as
well as real WebView2 smoke, binding argument keys to actual handler signatures.
Use only owned synthetic SQLite/local-data roots; no dialogs or user files. The
mock run tests Rust argument decoding/dispatch/refusals, not ordinary Windows
observations. Production registration and capabilities remain unchanged.

Hosted d611fc5: Linux native coverage, TypeScript and Windows packaging/native
WebView2 smoke passed. Windows shell unit execution failed before tests with
STATUS_ENTRYPOINT_NOT_FOUND (0xc0000139). The newly retained dialog imports are
consistent with a missing Common Controls v6 test-harness manifest; this diagnosis
is not yet confirmed by fresh native execution. Test-only COFF dependency metadata
now requests v6 in library/bin harnesses; production manifest is unchanged.
Read-only policy review found no actionable findings; native proof remains pending.

Shared synthetic refusal matrix expands to 82 requests and official mock IPC,
with actual-handler argument-key checks (two Node tests pass). App/CLI all-target
Clippy, Rust formatting and diff checks pass. New retrieval-publication, recipient
quest-stage/quotas and authored-content unknown-cost/empty-catalog tests passed
locally as recorded during development. No real network/game inputs are used.
Next: hosted loader/IPC verification, then remaining strict per-file deficits.

At 1e24cba, native coverage stopped at compilation: new IPC tests omitted direct
serde/serde_json dev dependencies. Added the already locked dependencies; lockfile
changes only desktop dependency edges. Local native check still stops at missing
dbus development prerequisites, so no native execution is claimed.

Additional focused tests actually passed: exact encrypted range lineage, staged
change artifacts/hash/schema substitutions, source policy distinctions, closed
fixture diagnostics, direct research services, finite/nonfinite float decode,
conflict-closure refusals, repeated/empty asset atomicity, historical snapshot and
guarded incomparable promotion, oversized/directory pending restore markers.
The snapshot test initially attempted a provably older promotion and correctly
failed; it now asserts that refusal and promotes a separate incomparable capture.
Read-only review caught the vacuous encrypted gap assertion; exact range assertion
replaced it and passed. Reviewer found no remaining findings in the batch reviewed.
Affected app/storage/ingest/local-data all-target Clippy passes after removing two
needless test clones. Native refusal Node tests, formatting/diff checks pass.
Next: fresh hosted native loader/IPC checks and clean LLVM diagnostic to identify
remaining deficits. Coverage gate is not passing; no merge/publication authorized
by evidence yet.

Read-only policy review found start_snapshot_watch was incorrectly classified as
an immediate refusal: missing selected files start a watcher and asynchronously
report Missing. Removed it from the shared refusal fixture; official mock IPC now
asserts owned missing-path startup, Missing/no import, explicit stop and unchanged
reference state. Added nine typed refusals and explicit read-only/absent-party
success responses. The shared matrix now has 90 cases. Independent review found
no remaining argument/semantics findings; native hosted execution remains required.

CLI synthetic empty-catalog round trip passed: coverage/verify/status, projection,
guide facts, tooltip, delivery build/read/explicit activation/rollback/diff, inventory
and all nine domain pages preserve unknown completeness and research status. CLI
all-target Clippy passes. Coverage/native Node helper tests: 22 passed.

Clean non-native Linux run at 25549a5 passed 1,380 tests, four explicit regenerators
ignored, 182 suites. LLVM export failed on a corrupt profile: the manual diagnostic
used LLVM_PROFILE_FILE instead of cargo-llvm-cov's LLVM_PROFILE_FILE_NAME and hence
did not select continuous profiles. No counts from that failed export are accepted;
raw logs/profiles retained. Next diagnostic uses the collector's actual environment
contract and clears its owned prior coverage artifacts before starting.

Hosted 25549a5 Windows shell job117752054067 confirms the loader repair: seven
Rust unit tests executed, six passed and the IPC test failed specifically on the
already-reviewed accepted watcher startup. The same assertion failed on Linux.
Correction is pushed at 6e9affb; fresh checks are running. No loader failure remains
in that observed execution.

Clean non-native Linux diagnostic at 6e9affb exported successfully using the
continuous-profile environment (1,381 tests passed, four explicit regenerators
ignored). Current-source inventory filtering leaves ten below-threshold files;
this excludes native Tauri and needs Windows union, so it is not the final gate.
Artifacts: /tmp/nav-local-expansion-third.{log,lcov}, matching inventory/gaps in
/tmp/nav-local-expansion-third-inventory.json and /tmp/nav-local-third-gaps.json.

Next focused batch passed locally: eight authored-content tests (exact dungeon
choice fields); service recreation and worker-entry refusal (one); watcher workflow
including immutable-candidate reuse after restart (two); idempotent asset install
and subsequent tamper refusal (one); source retrieval plus upstream metadata states
(one); two selected-input/CLI round-trip tests; historical and format3 synthetic
receipt CLI signing/assembly/crypto check with exact production-target refusal
(one). The real CLI extraction subprocess reaches a durable failure, then explicit
resume/cancel/forget, and keeps research activation separate. A separate real
media subprocess refuses an undiscovered ID. Guide replay remains unverified and
non-clean over the empty synthetic catalog. Read-only lifecycle review found no
actionable findings. Production metadata transport still uses https_get; its new
private callback only permits bounded synthetic transport testing.
App/CLI/storage all-target Clippy, formatting and diff checks pass for that batch.
Next: clean source diagnostic followed by native union and exact-head CI review.

At 6e9affb, native Windows shell CI passes the corrected mock IPC lifecycle and
90-refusal matrix. Source Linux/Windows/parity pass. Packaging failed before build
because DEPENDENCIES.json still recorded the prior Cargo.lock hash. Regenerated
with the canonical notices tool: only cargo_lock_sha256 changes; --check verifies
all 506 files and four notice-helper tests pass. No packages/licenses changed.

Clean local diagnostic c353525: 1,383 passed, four explicit regenerators ignored,
182 suites; LLVM export succeeds. Four Linux-only files remain below threshold:
CLI local_data 734/827, media 456/534, and the two platform-specific path/isolation
files awaiting Windows union. Current-source inventory was used, including moved
unit-test ranges; no rounding/exclusions were added. Reports/log/inventory/gaps in
/tmp/nav-local-expansion-fourth* and /tmp/nav-local-fourth-gaps.json.

Focused follow-up passes: media discovery and decode refusal test (signed IDs,
zero/unknown/missing/unsupported/duplicates, exact gaps, unsupported/truncated/
overpixel BLP, unsupported layouts/undecoded lineage, earlier immutable revision).
The undecoded gap initially omitted its reason suffix; test now declares an exact
synthetic reason and verifies the complete gap. CLI roundtrip now rejects selected
inert snapshot/session text and invalid guide-fact input while retaining prior
facts. Read-only review found no actionable findings. Local-data/CLI all-target
Clippy and formatting/diff checks pass; rerun complete clean diagnostic next.
