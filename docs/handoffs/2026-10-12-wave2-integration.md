# Wave-2 integration handoff

Owner: Codex root, coordinating three implementation workers.
Claim: 2026-10-12; original baseline/main 08f6daa.
Final documentation branch: docs/wave2-integration, base feature/wave2-r12.
Combined code checked locally and hosted: a08f5d7. R12 head 6f7fbb6 adds only
coordination documentation; no code difference from the checked implementation.
Original worktree stays at 08f6daa with only its pre-existing untracked .claude/.
No PR/main merge, force push, release, packaging or live gameplay action occurred.

## Delivered review units

| Task | PR / dependency | Checked code or head | Hosted evidence |
| --- | --- | --- | --- |
| Coordination | [24](https://github.com/Laymer/Navzeroth/pull/24) | 6630ad0 | [38305840809](https://github.com/Laymer/Navzeroth/actions/runs/38305840809), seven jobs passed |
| R1 timing disposition | [25](https://github.com/Laymer/Navzeroth/pull/25) | 2a9abf3 | [38307149382](https://github.com/Laymer/Navzeroth/actions/runs/38307149382), seven passed |
| R2 selected-file watcher | [26](https://github.com/Laymer/Navzeroth/pull/26) | d3daf4c | [38306962276](https://github.com/Laymer/Navzeroth/actions/runs/38306962276), seven passed |
| R3 desktop workflow | [33](https://github.com/Laymer/Navzeroth/pull/33), base R2 | a4a0b83 | [38309934719](https://github.com/Laymer/Navzeroth/actions/runs/38309934719), seven passed |
| R4 Lua evaluator/plan/HUD | [31](https://github.com/Laymer/Navzeroth/pull/31) | 94760c1 | [38308575668](https://github.com/Laymer/Navzeroth/actions/runs/38308575668), seven passed |
| R5 greedy planner | [30](https://github.com/Laymer/Navzeroth/pull/30) | 4683ac5 | [38308762436](https://github.com/Laymer/Navzeroth/actions/runs/38308762436), seven passed |
| R6 acquisitions | [34](https://github.com/Laymer/Navzeroth/pull/34) | 2da2a01 | [38308662161](https://github.com/Laymer/Navzeroth/actions/runs/38308662161), seven passed |
| R7 content persistence | [32](https://github.com/Laymer/Navzeroth/pull/32) | f0eb9ff | [38308579143](https://github.com/Laymer/Navzeroth/actions/runs/38308579143), seven passed |
| R8 content CLI | [27](https://github.com/Laymer/Navzeroth/pull/27) | 505dee7 | [38306556536](https://github.com/Laymer/Navzeroth/actions/runs/38306556536), seven passed |
| R9 analytics | [28](https://github.com/Laymer/Navzeroth/pull/28), base R1 | fe2c42e code; 7135203 evidence-only head | [38306694055](https://github.com/Laymer/Navzeroth/actions/runs/38306694055), seven passed |
| R10 atlas search | [29](https://github.com/Laymer/Navzeroth/pull/29) | d6d9323 | [38309945300](https://github.com/Laymer/Navzeroth/actions/runs/38309945300), seven passed |
| R11 Lua gear scoring | [35](https://github.com/Laymer/Navzeroth/pull/35), base R4 | f91f3cc | [38308970271](https://github.com/Laymer/Navzeroth/actions/runs/38308970271), seven passed |
| R12 integration CI/browser | [36](https://github.com/Laymer/Navzeroth/pull/36), base integration/wave2-foundations | a08f5d7 code; 6f7fbb6 docs prerequisite | [38310280809](https://github.com/Laymer/Navzeroth/actions/runs/38310280809), eight passed |

All PRs remain draft/reviewable. The integration prerequisite is a feature branch
combining the task histories; it does not replace the individual PRs. Its sole
implementation merge conflict was AtlasScreen: retain R3 external refreshKey and
R10 manual refresh. R12 adds only its CI/browser files against this prerequisite.
Final documentation contains only documentation against R12.

## Actual combined checks

Linux/WSL portable checks, run from the R12 worktree on combined code:

- `cargo fmt --all --check`: passed.
- `cargo clippy --locked --workspace --all-targets --exclude azeroth-desktop -- -D warnings`: passed.
- `cargo test --locked --workspace --exclude azeroth-desktop`: 463 passed, 0 failed, 1 existing ignored poster generator.
- `cargo run --locked -p azeroth-cli -- validate-fixtures`: 73 contract and 33 transport fixtures passed, including declared rejection cases.
- `cargo run --locked -p azeroth-cli -- validate-content --input content/forever --json`: passed with nonfatal gaps; deterministic existing coverage is in docs/data/COVERAGE.md.
- `cargo test --locked -p azeroth-lua-harness --test predicate_parity --test gear_parity`: passed; 192 predicate cases including three Lua-only rejection cases, 32 score/component/certainty comparisons and 157 arithmetic vectors.
- `cargo run --locked -p azeroth-cli -- export-schemas --output schemas/v1`, `pnpm contracts:types`, schema diff check: unchanged contracts.
- `pnpm install --frozen-lockfile`, `pnpm contracts:check`: passed, 73 schema/TypeScript cases.
- Desktop typecheck, lint, build and `test:unit`: passed, 64 unit tests.
- `pnpm --dir apps/desktop test:e2e`: four Chromium workflows passed, exercising command mocks, failure recovery, unchanged-current outcomes, opaque profiles, selected-file/watch cancellation, dense atlas filtering/keyboard zoom/pan/refresh and periodic backend freshness queries.
- `prepare-atlas --check-pointer`: four committed LFS pointers passed in a temporary git cat-file copy. Worktrees/git archive materialize images and correctly fail pointer-only checks; no imagery change committed.
- `git diff --check`: passed; original working-tree changes preserved.

Hosted evidence includes Linux/Windows Rust, Lua static checks, pnpm Linux/Windows
and native Windows compilation/tests. R12 adds explicit fixture/content/Lua steps
and a Linux mocked Chromium job. A hosted native build is not a native interaction
or packaging test. Local browser server/Chromium execution used approved sandbox
escalation; no account or release action was needed.

## Reviews and corrections actually performed

Root read actual timing, Lua plan/HUD, persistence, analytics and Lua gear diffs.
Worker A reviewed R5/R6; worker B reviewed R10. These were focused implementation
reviews, not a claim of exhaustive independent security review.

- R1 retains legitimate earlier observations across monotonic resets and clock
  changes; future sequence and contradictory same-capture timing now fail.
- R4 true invalidation retains an invalidated step rather than declaring it
  complete. HUD not_applicable gates explicitly mean no prerequisite/invalidation;
  unknown/restricted/unsupported remain uncertain. Regression cases added.
- R5 unknown/excluded candidates cannot become definite selections. Added replay
  cases for intermediate stateful rejection, final stopping action, disconnected
  stops and collection/step bounds. Exact ratios/ties and final service costs tested.
- R6 explicitly separates caller spending ceilings from captured money. Stale or
  uncertain observed money cannot produce a guaranteed assessment under Reject;
  AllowStale retains uncertainty. Nonzero insufficient funds and duplicate credit tested.
- R7 rejects malformed self/empty disagreements, validates requested revision
  identity, seals immutable children and keeps prior activation on failure. Tests
  cover v1 migration, corrupt identity, rollback and a consistent backup including
  committed WAL while excluding uncommitted writes. Snapshot hash unchanged.
- R10 preserves selection/search/type/layers through failed refresh and retry;
  completed-refresh tests wait for changed revision. Removed pins and vanished
  types are explicitly displayed; ArrowUp/Down wrap and Home/End are tested.
- R3 open views requery backend age every 60 seconds; freshness math stays in Rust.

## Decisions, limitations and next tasks

Exchange schemas stayed frozen. Planner/analytics/scoring compiler types remain
library types; commands use explicit application DTOs. Character profiles isolate
full edition/build/interface/namespace/fixture identity. Import validation/storage
ordering preserves current state for malformed/duplicate/older/incomparable input.

Desktop storage supports character snapshots only. Party import is explicitly
rejected; selection is session-persistent. Atlas imagery remains the labelled demo
reference dataset, so incompatible stored locations return explicit gaps. The
watcher bounds retries (three per unchanged candidate); exact same-metadata ABA
replacement between reads remains a documented platform limitation, not a claim
of perfect filesystem change detection.

The greedy planner consumes caller clusters, disjoint reward IDs, known completion
estimates and normalized goal values; it does not simulate prerequisite unlocks,
shareable kills, personalized utility/fairness or beam search. Budget feasibility
uses expected time rather than guaranteeing worst-case ranges. Acquisition
probability formulas require explicitly declared independent player trial semantics;
source drop probability alone never becomes guaranteed player acquisition.

Lua HUD/scoring uses injected callbacks and synthetic catalogs; no permitted-client
bindings or equipment observations are claimed. Analytics preserves clock uncertainty
and covered monotonic time; it has no combat metrics or personalized calibration.

Live Forever build observations, quest transitions, native window interaction and
packaging remain UNVERIFIED. No forbidden game access or live socket bridge exists.
Next dependency-ready work: review these PRs/stacks; party snapshot persistence;
planner-to-export/UI and analytics/gear screens; permitted real quest/acquisition
content and explicit model profiles. Client checks require sanitized evidence from
the recorded build. PR/main merges and releases require separate authorization.

Final hosted outcome: combined run 38310280809 completed SUCCESS with all eight jobs, including native Windows compilation/tests and mocked Chromium. Documentation-only prerequisite/final changes do not change the checked source.
