# Continuation review, test expansion and per-file coverage

Owner/sole writer Codex `/root`, 2026-10-18. Branch
`test/continuation-coverage`, isolated worktree `Navzeroth-coverage`; baseline
`22338e0` (PR #200 pending). This is preparatory work; open the single substantial
review/test/CI PR only after engineering PR #200 reaches main and refresh to it.
Preserve the original checkout, all other worktrees and `.claude/`.

Inventory eligible first-party Rust and TS/TSX before expansion, explicitly account
for tests/generated/vendor/declaration-only sources, and fail missing report entries.
Enforce at least 90% executable-line coverage per source file with no rounding,
coverage-ignore annotations or concealment exclusions. Pin cargo-llvm-cov 0.9.1
and compiler-matching LLVM tools; aggregate Linux/Windows normalized executable
lines at the same commit. Include build scripts, examples, native adapters,
CLI/worker entrypoints and bounded Tauri smoke. Add matching Vitest V8 coverage,
including unloaded production files. Preserve all nine required checks and add
Coverage gate with baseline/final verdict/raw artifacts. Lua/PowerShell harnesses
remain required separately from the numeric Rust/TS gate.

Initial read-only baseline at dcd75c7 ran all Linux non-native tests successfully,
but strict LLVM merge failed on one truncated subprocess profile. Keep all raw
profiles and label the partial diagnostic export incomplete; it is not an
accepted coverage result. Native/Windows coverage and declaration accounting are
not yet measured. Diagnose lifecycle/process profile handling explicitly.

Read STATUS, latest performance/desktop handoffs and implementation plan sections
16/18 before changes. Add meaningful adversarial/domain/lifecycle/release/UI tests.
Any independent product defect gets a separate reviewed PR before integration.
Independent read-only reviewers review this PR/test quality; no new implementation
agents. Record actual checks, limits and next task. Version/notes, stability,
verified development RC publication and Pages remain subsequent stages.
