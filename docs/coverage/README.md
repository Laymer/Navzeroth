# Per-file coverage expansion (in progress)

The pre-expansion [inventory](2026-10-18-initial-inventory.json) binds source commit
`d053de0` and normalized source hashes. It includes 185 executable Rust and 67
executable TS/TSX files, 28 declaration-only sources, and explicit test/vendor/
generated exclusions. Build scripts, examples, native adapters, CLI/worker and
React entrypoints remain eligible. Declaration-only status is conservative and
syntax-checked; new executable declarations must produce a report. Source hashes
normalize CRLF to LF so native platforms share the same source lines.

The initial Linux all-target instrumented tests passed, but strict LLVM merging
failed on one truncated worker profile. Raw profiles remain retained locally; the
partial diagnostic export is **incomplete**, excludes native desktop coverage and
cannot pass the gate. The existing CLI tests intentionally kill an isolated worker;
the exact truncated producer/disposition still needs investigation. Do not delete
bad profiles to manufacture an accepted result. Initial TypeScript V8 tests all
passed (413 tests) but the 90% per-file threshold failed on 21 files. This is a
baseline, not an accepted final coverage result.

Toolchain: cargo-llvm-cov **0.9.1**, LLVM tools matching pinned Rust 1.99.0 (LLVM
23.1.1), and @vitest/coverage-v8 **5.0.3** matching Vitest. The Rust tool's default
filename exclusions hide examples, so collectors must disable those defaults and
explicitly account for test/vendor/generated files instead. Build-script reporting
must be enabled. The gate combines executable source lines across Linux/Windows
at one commit and uses integer covered*100 >= executable*90, without rounding.
Missing files and incomplete profiles fail. Raw report hashes and source hashes
are checked, and counts are reconstructed from raw reports, not claimed summaries.

Vitest loads its own configuration before enabling coverage. The separate
`configuration.mjs` collector executes that real module with Node precise V8,
TypeScript transpilation/source maps and the pinned Vitest AST converter. It checks
production CSP/asset contracts and retains raw V8 plus remapped Istanbul reports;
configuration code is not excluded. Declaration-only files remain explicit rows
with no executable lines. Coverage-ignore annotations are prohibited.

Current local commands:

```
node --test scripts/coverage/*.test.mjs
node scripts/coverage/inventory.mjs /tmp/new-inventory.json
node scripts/coverage/configuration.mjs /tmp/new-config-report-directory
pnpm --dir apps/desktop exec vitest run --coverage
```

Remaining work: native Linux/Windows collectors and bounded instrumented Tauri
smoke, complete eligible-source receipts, targeted Rust/UI test expansion, final
per-file ≥90% verdicts, baseline/final/raw artifacts and required Coverage gate
without removing any of the nine existing checks. Independent product defects
receive separate PRs (browser demo version correction: PR #201). Do not publish the
RC before completing this stage, version/notes, fresh exact-main checks and stability.
