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
