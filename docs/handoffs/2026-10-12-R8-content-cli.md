# R8 content CLI

Owner: Codex root (single writer). Baseline: 08f6daa.
Branch: feature/wave2-r8; claimed 2026-10-12 before implementation.
Scope: bounded validate-content CLI and coverage documentation; reuse core coverage.
Checks and delivery evidence pending implementation.

Implemented file/directory coverage CLI, deterministic JSON/human output, fixed
input errors, and bounded regular-file reads. 128 files/16 MiB each/64 MiB total.
Directory discovery is direct datasets/*.json or direct *.json, no recursion.
Existing reference dataset passes with separate gaps (docs/data/COVERAGE.md).

Actual local checks: cargo fmt --all --check; locked workspace Clippy all-targets
excluding desktop (passed after correcting len_zero test lint); locked workspace
tests excluding desktop (passed, one pre-existing ignored generator); four CLI
regressions passed including deterministic output, semantic failure, option/input
failure, symlink/oversize rejection. Initial regression fixture path typo fixed.
pnpm frozen install, contracts:types/check passed, 73 fixtures; export-schemas
regenerated unchanged. No schema/lock changes. Native/live checks unverified.
Hosted CI pending draft PR. Next: R12 explicit CI content command.
