# Continuation 3 — selected cache and observation inputs

Owner/sole writer: Codex `/root`; 2026-10-18.
Initial baseline: `7f5587a`, reviewed lifecycle branch. PR #192 subsequently
merged as `1cb6bb90382a55acbebb2f036859fb2bb7f66007`; integrated before this PR.
Branch: `feat/continuation-local-inputs`; isolated worktree.
[Claim](../claims/2026-10-18-local-inputs.md).

## Implemented

Pure versioned input contracts and bounded cache envelope parser; explicit
unsupported dispositions and conflicts; private immutable source/analysis/manifest
closure with source-closed analyzer identity; bounded revision pages; Windows
closed-game/process-name and write-excluding handle boundary; fail-closed WSL
mutable reads; separate synthetic fixture operation; shared CLI/app entry points.
Existing inert snapshot/session importers preserve observation states and context.
Compatible session v2 adds recorded locale when observable, preserving v1 unknown
locale and existing recorder/reload semantics. [Supported scope](../data/LOCAL_INPUTS.md).

## Decisions and limitations

No automatic cache discovery or hotfix overlay. Opaque payloads and unsupported
formats remain research-only; version 8 is deliberately unsupported. Cache locale
is directory evidence, never an embedded-header claim. Source edition/interface
is retained in addon analysis; product-to-edition applicability stays unverified.
A turn-in cannot establish universal prerequisites or eligibility.

The operator initially chose synthetic fixtures while playing, then closed WoW
and authorized a bounded native capture. [Actual development receipt](2026-10-18-cache-development-check.md):
70450/enUS XFTH v9 parsed with 48,301 envelope records; three WDB layouts explicitly
unsupported; private copies removed. This tested native PowerShell capture plus
Linux pure Rust parsing, not the native Rust staging path. Native Rust end-to-end,
closed-game/write-sharing refusals and live addon locale remain unverified TODOs,
separate from hosted synthetic CI.
Desktop presentation follows in continuation 8; staged inputs cannot grant gameplay
availability or alter an active catalog.

## Review and validation

Independent read-only reviewer `/root/review_policy` found one actionable gap:
the initial analyzer fingerprint omitted transitive snapshot validation modules.
It now inventories complete first-party source directories, with a regression for
changed validation and newly added modules. A second review identified missing workspace/dependency manifests; those and
crate-root build/support modules are now included and tested. Final read-only
review reported no remaining actionable findings.

Final local workspace tests: 174 suites, 1,268 passed, zero failed, four ignored
(`cargo test --locked --workspace --exclude azeroth-desktop`, Linux/WSL,
incremental/debug info disabled). This includes cache bounds/truncation/conflicts,
source/analysis/manifest tampering, revision pages, source/dependency fingerprint
closure, CLI round trip, session v1/v2 and 13 Lua recorder harness tests. Native
local-data checks passed (73 tests before the final additional manifest regression).

The first Clippy run found two needless closure borrows, and a later run found
an unused test import; corrected both. The first workspace run found two new Lua
lines exceeded 100 columns; reformatted them, and the final full suite passes.
Formatting, final Clippy and exact hosted receipts are recorded with the PR.
Hosted exact-head checks remain required before merge.

## Next

Continuation 4: complete reviewed gameplay mappings and guide facts using verified
projections and separate permitted observation evidence. Keep unknown applicability
and unsupported payload semantics explicit. Media/tooltips follow it.

Subsequent exact-head hosted results and merge receipt are recorded in
[PR #193](https://github.com/Laymer/Navzeroth/pull/193); pending checks are not passes.
