# Schema 5 downloaded-draft release contract

Owner tooling_review. Baseline 9cc774e4f2656f26d544071d6bd174367ccca16a;
claim 71d48438. Isolated worktree /tmp/navzeroth-schema5-contract.
Production implementation cbf2b70a, hardened by 8bf6317e and 40956118;
63190bde only strengthens negative tests with type-preserving corruptions.
No canonical release/v1 manifests, keys, signing, workflow dispatch or release
publication changed. Root owns PowerShell, producer and download-workflow adapters.

## Contract and interfaces

[Schema 5](../releases/SCHEMA5_ACCEPTANCE.md) defines exactly eight existing
hosted rows plus DRAFT-INSTALLER-SMOKE / automated_draft_windows. SCHEMA_STABLE
stays 4, whose OWNER-INSTALLER-SMOKE / interactive_windows semantics remain intact.
Schemas 1–3, their target/origin rules and 16 MiB limits are unchanged. Stable
schemas 4 and 5 share existing 64 MiB limits, admitted materials/source rules,
receipts and source fingerprints. Historical results are not promoted.

Prebuild permits nine pristine pending rows. Draft permits only the final pristine
pending row after eight exact-installer hosted passes. Full publication readiness
requires all nine and structured verified evidence. Preparation never authorizes
publication. Failed/partial/missing/wrong-method/stale-source/hash results block.

Schema 5 alone accepts optional nonnull draft_installer_smoke with raw_result,
provenance and independent_review FileRefs. All three must bind to the final
row's artifacts. CLI release-readiness verifies bounded typed payloads, native
UI/detail pins, existing download receipt, exact source/installer/production,
trusted production policy and completed run/job/release API snapshots. The API
native test step must occur once and pass; installer asset ID/name must each be
unambiguous. Existing download receipt run/attempt strings must exactly match.
The source dispatch must use the exact tag, not a newer workflow branch.

The policy path is fixed at config/release-draft-smoke-v1.json. Workflow reads
require .github/workflows/ plus a single traversal-free AssetPath filename.
Repository, workflow/job, test-step, Windows label and runner group are policy
pins, not evidence-selected authority. Three JSON documents and nested JSON/API
refs are each <=1 MiB; native details are 1–16 distinct pins <=8 MiB each. Parsed
bytes are rehashed after bounded reads, including the verify-cache/reopen boundary.
No imported code executes. Public/generated structural schemas are regenerated
from Rust, while runtime assessment remains authoritative for full semantics.

After the draft exists, the coordinator fetches actual completed API results,
independently reviews the actual downloaded installer/native evidence and builds
an external overlay. These acceptance files do not enter or mutate signed bundle
resources or tagged source. Offline JSON checks prove recorded consistency, not
GitHub authenticity or reviewer independence from a string. Both receipt keys
remain operator-controlled. Human smoke is explicitly not performed.

## Actual verification

[Compact current-source coverage/checks](../reviews/2026-10-23-schema5-contract-checks.json).
Linux Rust 1.99.0 / cargo-llvm-cov 0.9.1: 574 passed, zero failed, four ignored
across 59 test binaries: core 497/2 ignored, CLI 77/2 ignored. Core production
bytes are unchanged since cbf2b70a; CLI profiles were freshly rerun on 63190bde.
The eight new I/O tests include many type-preserving mutations, invalid source,
phase/IDs/attempts/assets/reviews/API states, skipped/duplicate native steps,
missing/altered/bounded resources, native UI/check/detail failures and malformed
policy paths. Core tests preserve historical rows and cover all phases, strict
schema shapes and generated schema artifacts. Python preparation: nine passed.
Clippy all targets, fmt and diff check passed. Production coverage, excluding
only existing inventory-defined test ranges and using exact current source paths:

- core release.rs: 932/948 (98.31%).
- CLI release.rs: 1062/1173 (90.54%).
- CLI release/bundled.rs: 267/273 (97.80%).
- CLI release/materials.rs: 476/501 (95.01%).
- CLI release/draft_smoke.rs: 194/195 (99.49%).

The unchanged gate/inventory modules were byte-compared before extracting these
counts. No threshold, rounding, inventory or missing-file checks changed.
Coordinator independently reviewed cbf2/8bf/409 and executed all eight frozen CLI
unit tests using the author's binary; this is not a separate compiler build.

Recorded development/setup failures: cargo-llvm-cov rejected combining --no-report
and --no-clean (no tests executed), then the supported --no-report command passed.
The initial coverage checker import targeted an older checkout without those
modules; retry used byte-identical current modules and available TypeScript.
A direct AssetPath policy experiment correctly failed the positive test because
.github is not an asset path; the prefix-plus-filename validation repaired it,
after which all tests passed. These are not installer or release acceptance runs.

## Next

Root integrates the independently reviewed schema 5 core/CLI and separate
PowerShell/workflow/ADR changes, migrates canonical release manifests deliberately,
and renews source/review/receipts. Fresh protected checks, source parity, stability,
all eight exact-installer hosted cases and independently reviewed downloaded-draft
native UI acceptance remain required. Actual Windows execution, human smoke and
publication were not performed by this task. No time limits, live-client claims,
content exclusions or ordinary-PC performance claims changed.
