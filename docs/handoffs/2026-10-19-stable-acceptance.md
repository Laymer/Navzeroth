# STABLE-ACCEPTANCE: schema 4 core contract

Owner/sole writer: Codex `/root/tooling_review`; baseline `5d5fef9`;
branch `feat/stable-release-acceptance`, isolated worktree
`/tmp/navzeroth-stable-acceptance`. Claim was recorded before implementation.
Read ADR 0008 in the integration worktree and the V2.8 handoff.

Implemented an explicit schema 4 inventory/evidence/readiness contract. It retains
schema 3 target/origin/content semantics and requires eight distinct hosted_windows
rows plus OWNER-INSTALLER-SMOKE using interactive_windows. Every passed row must
bind the same candidate_installer_sha256 through its installer_sha256 field, with
existing tested-source / production fingerprint and artifact evidence requirements.
Actual bytes remain independently checked by the download verifier; executables
are not committed. Historical contracts reject the new populated bindings/method.
CLI policy loading, origin classification and material policy binding recognize
both schema 3 and 4. No content, receipt or admission requirement is relaxed.

Contract and remaining integration are documented in
[SCHEMA4_ACCEPTANCE](../releases/SCHEMA4_ACCEPTANCE.md). Historical repository
manifests remain schema 3 and observations are not automatically migrated or
promoted. Stable/publication remains false with pending owner smoke.

Actual Linux validation:

- Core release integration suites: 45 passed (15 schema1/general, 18 schema2,
  12 schema3/4), zero failures.
- `cargo check --locked -p azeroth-cli`: passed.
- `cargo test --locked -p azeroth-cli release::`: 6 passed, zero failed (filtered release-module tests).
- `cargo llvm-cov --locked -p azeroth-core --json --output-path
  /tmp/navzeroth-stable-acceptance-coverage.json`: 465 passed, zero failed,
  two existing ignored tests. Native Linux executable-line measurement for
  `crates/core/src/release.rs`: 812 / 827 = 98.1862%. This is a local LLVM
  measurement, not the cross-platform protected per-file coverage verdict.
- Formatting and diff checks passed. No coverage threshold changed.

Initial expanded testing found a historical test treating version 4 as future;
it now tests version 5. The first coverage attempt stopped on that same stale
test and is not counted as passing. New synthetic cases exercise complete schema4,
missing/failing/wrong-method and mismatched/absent installer identity, missing
source/fingerprint, pristine pending owner smoke, historical rows and historical
schema rejection. They do not assert real hosted or owner acceptance.

Outstanding: explicit two-phase build/artifact-ready versus publication-ready
adapter, packaging/tag/draft integration, reviewed schema4 manifest migration,
hosted evidence producers and actual hosted/owner results. Existing stable
artifact preparation still invokes full readiness, so the draft-before-owner-smoke
cycle remains blocked until that separate adapter is implemented. No waiver is
introduced. Native Windows, hosted coverage parity and real installation remain
unverified. No push, release, key or network pipeline action.

## Explicit preparation phases (follow-up)

Implemented core BuildPhase::Prebuild / Draft and separate CLI commands
release-build-readiness / release-draft-readiness. They run the same I/O verifier
then wrap its report with build_ready and publication_allowed=false (also false
inside the retained report). Only schema4 stable reports qualify. Prebuild accepts
pristine pending required observations; draft accepts only pristine pending owner
smoke. Initial source/fingerprint binding is never deferred by either phase.
Existing nonpending blockers, failed/partial claims and mismatched/stale bindings
remain failures. Full release-readiness behavior remains unchanged.

The earlier outstanding two-phase note now applies only to packaging and workflow
adapters, not the core/CLI contract. Root explicitly reserved those adapters to
avoid concurrent changes with STABLE-VERIFIER. Draft download verification must
choose draft readiness while public verification retains final readiness. No
claim of a usable end-to-end release workflow follows from these commands alone.

Actual follow-up checks: schema3/4 integration 14 passed; full core LLVM run 467
passed, zero failed, two ignored; release.rs 827/842 lines = 98.2185% local Linux
measurement. CLI filtered release-module tests: seven passed including wrapper
serialization/source-evidence retention and both publication flags false.
Formatting/diff checks passed. An initial CLI compile found the command arguments
are OsString rather than String; explicit comparisons corrected the guard, and
the final CLI test build passed. CLI/main cross-platform per-file coverage remains
for required hosted CI; no gate settings or threshold were changed.
