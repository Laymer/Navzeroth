# R4 — Lua evaluator and HUD foundation

Owner: Codex R4 worker, single writer. Claimed 2026-10-12.
Baseline: 08f6daa. Branch: feature/wave2-r4. Worktree: /tmp/navzeroth-r4.
Scope: addon pure modules and TOC, Lua harness tests. Shared manifests, contracts,
lockfiles, CI and status are owned by the coordinator and remain outside this task.
Acceptance: bounded decoder, all 192 predicate corpus cases, validated loaded-plan
state and injected HUD callbacks; affected and workspace locked checks, contract
checks, draft PR and hosted evidence. Live client bindings remain unverified.
Status: IMPLEMENTED for the pure synthetic scope; client behavior UNVERIFIED.

## Implementation

- `Core/Decode.lua`: pure bounded JSON data decoder. Defaults: 32768 input
  bytes, 32 container levels, 8192 values, 1024 entries/container and 4096 UTF-8
  bytes/string. Null sentinel, container kind and numeric lexeme metadata preserve
  absent/null, false/zero and integer/floating distinctions. Strict UTF-8,
  surrogate pairs, number grammar and duplicate key checks; fixed error codes.
- `Core/Predicate.lua`: all 14 closed operators with Rust-compatible truth,
  short-circuiting, bytewise modifier ordering, freshness, exact steps and ordered
  reasons/warnings. Full predicate and observation validation, party-copy
  normalization (defaults, ignored struct members), complete context checks.
- `Core/Plan.lua`: route-plan v1 import against a copied edition/build/interface,
  namespace and fixture boundary; dependency, step, participant, evidence,
  exclusion, timing, predicate, source and extension validation. Fixed production
  decoder budget; import failure retains current state; queries return copies.
- `Core/Hud.lua`: observation and rendering callbacks, fresh model per refresh,
  fixture label, declared alternatives, step/explanation and evaluation diagnostics.
  Definite true invalidation retains advice as invalidated; unknown invalidation
  or prerequisite suppresses ready. NA gates explicitly mean no requirement or
  invalidation (true/false respectively). No live APIs, client frames, event registrations or I/O.
- TOC follows existing namespace registration/load conventions. Core README
  documents the interfaces and explicitly leaves client import UI, automatic
  alternative selection and revalidation/advancement bindings for later work.
- Harness: all 192 unchanged Rust-anchored cases including 3 Lua-only decoder
  rejections; all 11 route-plan fixtures; boundary rejection, failed replacement,
  alias isolation, and injected HUD workflow cases. No schema/corpus changes.

## Verification performed

- `cargo fmt --all --check`: passed.
- `cargo clippy --locked --workspace --all-targets --exclude azeroth-desktop -- -D warnings`: passed.
- `cargo test --locked --workspace --exclude azeroth-desktop`: all binaries passed.
  After final addon/fixture-label refinements, affected harness tests and locked
  workspace Clippy passed again. Lua harness has 63 passing tests, including the
  new 2 decoder, 3 plan/HUD and 1 corpus tests.
- `cargo test --locked -p azeroth-lua-harness --test predicate_parity -- --nocapture`:
  192 cases passed, 3 Lua-only cases included (none skipped).
- Local luacheck via the existing scratch `run-luacheck.sh` (Lua 5.1.5,
  luacheck 0.26.1): zero warnings/errors across 11 addon source files.
- `pnpm install --frozen-lockfile`, `pnpm contracts:types`,
  `pnpm contracts:check`: passed; 73 exchange fixtures. `export-schemas` via
  documented azeroth-cli command and `git diff --exit-code -- schemas/v1`: no drift.
- `prepare-atlas --check-pointer`: the materialized worktree correctly refused
  actual PNGs. Extracted committed HEAD blobs using `git show` into
  `/tmp/navzeroth-r4-pointers` and the same command checked 4 LFS pointers.
  `git archive` materializes LFS content in this environment and could not be
  used for pointer validation. No content changes made.
- One attempted non-existent core `export_schemas` example failed before the
  documented CLI export command above succeeded; no generated artifacts from it.

## Decisions and limitations

No exchange schema, manifest, dependency, lockfile, CI or shared status changes.
Decoder budget overrides exist only for repository fixture loading; plan import
has no caller-controlled limits. The corpus expands declarative nest/wide
fixtures in the harness, so generated rule limits remain independent of JSON
container limits. Integer lexeme checks remain separate from numeric comparison.

This is a pure foundation, IMPLEMENTED_UNVERIFIED for client behavior. No
Forever client/build evidence, native window interaction, real quest transition,
TOC interface verification, frame rendering or live equipment observation ran.
The diagnostic entry point remains the only client binding. HUD alternatives
are declared data; stable automatic selection/stateful step advancement and
revalidation event dispatch are outside R4 (M4.6).

Next dependency-ready task: R11 bounded synthetic gear inputs and Rust/Lua
fixed-point scoring parity, stacked from feature/wave2-r4. R12 CI owner can use
the existing harness tests; normal workspace Rust CI already executes them.
Hosted PR and CI evidence: recorded below.

## Coordinating review follow-up

The initial HUD incorrectly treated true invalidation as completion/advance.
Plan section 6.5 defines per-step invalidation conditions; it supplies no
completion predicate. Corrected the HUD to retain the same step with
`invalidated` status and no completion claim. The no-applicable-step case is
`no_steps`. Focused regression keeps the invalidated step ID visible.

PlanStep HUD gate decision: `not_applicable` means no prerequisite (true) or no
invalidation (false), consistent with travel requirement NA semantics. Unknown,
unsupported and restricted remain unknown, retaining their observation status.
Quest-definition eligibility currently uses a conservative different missing-
requirement policy; this decision is local to explicit PlanStep gates. Focused
HUD tests cover both gates and all unavailable statuses.

Draft PR: https://github.com/Laymer/Navzeroth/pull/31.
Hosted run: https://github.com/Laymer/Navzeroth/actions/runs/38307428332 at
98e6a61: format, Lua static checks and pnpm Linux passed; Rust Linux/Windows,
pnpm Windows and desktop Windows were still pending at first inspection.

Hosted CI after the coordinating review correction: all seven checks passed at
117096c on [run 38307699996](https://github.com/Laymer/Navzeroth/actions/runs/38307699996):
Rust format, Rust Linux, Rust Windows, Lua static checks, pnpm Linux, pnpm
Windows and Desktop shell Windows. Desktop CI compiles/tests the shell; it does
not verify native UI interaction. Local post-review affected HUD tests,
formatting, harness Clippy and zero-warning luacheck also passed. Final follow-up
commit records this evidence only; executable sources are the checked revision.
