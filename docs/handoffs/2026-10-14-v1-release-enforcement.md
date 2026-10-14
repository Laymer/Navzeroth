# v1 stable-release enforcement

Owner / sole writer: Codex `/root/onboarding`.
Task: batch 2 CLI, packaging and tag enforcement for the full-content v1 release.
Baseline: `6db096b`; branch `feat/v1-release-enforcement`;
isolated worktree `/tmp/navzeroth-v1-enforcement`.
Claim recorded before implementation on 2026-10-14.

Read AGENTS.md, STATUS, final published-program handoff, implementation-plan
sections 19–20, and the coordinator's frozen release/v1 contract. Own CLI release
I/O module, entrypoint/library wiring, focused tests, packaging workflow and
Build-Windows.ps1. Core release types/validation remain coordinator-owned; source
adapters and acceptance records remain with their assigned workers. Own only the
record-binding/CLI details section in release/v1/README.md, as delegated by root.

Independent review of coordinator commit6db096b identified unsupported schemas
being ordinary prerelease blockers instead of errors, possible oversized-vector
CPU amplification after flagging bounds, and missing binding from declared
coverage IDs to actual pack records. Root owns core corrections. The frozen CLI
mapping uses canonical JSON-array strings [revision, collection, record identity]
from typed datasets; unsupported role/party/dungeon formats remain explicit
blockers until supported contracts exist. No invented models or game records.

Implementation is in draft PR [120](https://github.com/Laymer/Navzeroth/pull/120),
depending on core contract PR 115. The branch normally merged coordinator fix
`c6a5792`; it does not rewrite published history or change core release types.
Production commits are `6341839`, `f337689`, `ab44737`, `dff06e4`, and `1152b3b`.

## Implemented boundary

`azeroth-cli release-readiness --root . --version <version> [--evidence <path>]`
checks the frozen version-1 manifests, six source versions and seven internal
lockfile versions. The I/O layer bounds reads and Git output, rejects symlink or
reparse traversal, verifies exact evidence bytes/hashes, and proves tested-source
ancestry and the production Git-tree fingerprint. Unknown future schema versions
are errors even for prereleases. Product modifications and untracked product
files are blockers; documentation-only evidence commits preserve unchanged
production identity.

Ordinary packs go through the existing authoritative verifier, including source
distribution eligibility. Fixture packs, coverage gaps, wrong target metadata,
unpinned registry sources and mismatched dataset revisions block readiness.
The CLI binds coverage to actual typed records using the canonical JSON-array
identities documented in `release/v1/README.md`, rather than trusting declared
counts. All pack inputs must be tracked production files so excluded evidence
directories cannot change content without invalidating its tested identity.

Stable packaging calls the gate before output cleanup and again after building,
even with `-AddonOnly` or `-AllowDirty`. Both CLI and packaging inspect actual
recursive addon inputs: ignored/untracked files and links cannot enter a stable
ZIP or installer resource tree. Version classification preserves existing 0.x
and explicit-prerelease addon-only packaging without requiring Cargo. The tag
workflow reruns the CLI before creating a draft and downloads artifacts into
`dist/release-artifacts`, separate from checked-in `release/v1` contracts.

## Review and actual verification

- Coordinator core review at `c6a5792` found no remaining blocking or important
  finding within the pure contract. The earlier future-schema and bounds issues
  were corrected by the coordinator; actual record binding is this task's I/O
  responsibility.
- The complete CLI suite passed after the main implementation: 80 passed, one
  existing ignored integration check. After the ignored-input and asset-delivery
  changes, all 13 focused release integration tests passed again. They exercise
  malformed/oversized/future manifests, unknown fields, wrong versions/target,
  evidence hashes and paths, ancestry/tree identity, dirty/staged/ignored inputs,
  fixture pack refusal, and excluded pack payloads. Three new internal mapping
  tests check actual record/source bindings, omissions and unsupported domains.
- Strict CLI Clippy (`--all-targets -- -D warnings`) and formatting passed after
  the final production change. `git diff --check` passed.
- PowerShell 7 on Linux passed 27 synthetic packaging checks, including refusal
  before old artifacts are removed, AddonOnly/AllowDirty dispatch, malformed
  versions, preview packaging, ignored `.log` and ignored nested Lua files.
  This is not Windows PowerShell 5.1 or native Windows acceptance.
- A clean addon-only preview build at `dff06e40dcab53872a8fe5063ecd33c40fd3e5dd`
  succeeded. All 20 archive entries exactly matched tracked addon source bytes;
  all three checksums and the exact raw `SOURCE_COMMIT.txt` were verified. The
  actual CLI at that revision allowed 0.8.1 as a preview while reporting 67
  stable blockers, zero verified evidence files and zero verified content packs.
  At final production revision `1152b3b`, the real report still allows only the
  0.8.1 preview and reports 68 stable blockers, including asset delivery.
- Independent reviewer `/root/acceptance` found two important boundary issues:
  `release/*` included the new contract directory (fixed `ab44737`), and ignored
  addon inputs could escape Git status while entering recursive packages (fixed
  `dff06e4`). At exact `dff06e4`, the reviewer independently passed 13 release
  integration tests, nine library tests, 27 PowerShell 7 checks, whitespace and
  clean-status checks, and found no remaining blocker/important finding.
  The reviewer then independently checked `1152b3b`, passed the focused
  stable/0.x/rc regression and clean/whitespace checks, and confirmed no remaining
  blocker or important finding in the scoped implementation.
- The existing nine hosted required checks remain required on the exact merge
  candidate. Native Windows/PowerShell 5.1 and all live client acceptance remain
  unverified by this local Linux work. No stable tag or release was created.

The first hosted Windows package runs (including job `116080110491`) reached all
27 successful Windows PowerShell 5.1 assertions, then failed because the Actions
command wrapper propagated the last intentionally failing child's exit code.
The same wrapper invocation reproduced exit 1 locally despite 27 passing checks.
The harness now exits 0 explicitly only after every assertion and cleanup; an
exception still exits nonzero. The corrected wrapper invocation passes locally.
Injecting `/bin/false` as the synthetic child shell still fails its assertion and
returns exit 1, confirming the success exit does not mask a failed check.
Fresh hosted checks remain required; the earlier job did not reach packaging.

## Explicit remaining blockers and next task

Current manifests deliberately have incomplete mandatory inventory, no accepted
redistributable target source or verified offline pack, and missing actual
acceptance. Existing reference datasets have no complete class/role,
party-scenario or dungeon-preparation record contract; the CLI explicitly blocks
those domains rather than inventing game models or treating dungeon acquisition
rows as full dungeon support.

Independently of those missing domain contracts, **stable asset delivery remains
unsupported**. Current packaging produces only installer/addon and metadata. It
does not yet deliver verified offline packs, required upstream notices/licenses
or corresponding source materials. Commit `1152b3b` adds a dedicated unconditional
readiness blocker and regression assertion. It must remain until a separately
reviewed packaging integration defines and verifies the complete asset manifest,
including the accepted content/source/license set. Completing game records alone
must never make the current five-file packaging qualify for stable publication.

Next: fresh required hosted checks and normal integration by the coordinator.
Continue approved independent source adapters,
content/model gaps and user-guided build/Windows acceptance. A later packaging
task must implement and test the content/source asset delivery contract before
removing its explicit blocker. Never substitute these automated passes for the
recorded live and interactive Windows evidence.
