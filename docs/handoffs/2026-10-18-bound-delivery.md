# Continuation 6 — bound delivery and guarded consumers

Owner/sole writer: Codex `/root`; 2026-10-18. Initial baseline `ea47b57`,
media PR #195. Claim: [bound delivery](../claims/2026-10-18-bound-delivery.md).
Integrate resulting main before final PR review/merge.

Implemented immutable version-1 research delivery binding catalog/tuple,
projection/projector, media, selected observations, optional guide facts and
source/dependency profile hashes. Full deterministic closure verification,
explicit diff and transactional CAS activation/rollback use an additive private
SQLite schema-2 delivery history, preserving catalog/job state. Partial lookup
uses bounded ordered pages, source states/reasons and nullable unavailable counts.

CLI and typed Tauri/React expose reference preparation, inspection, explicit
selection and reviewed rollback. Embedded pending policy produces an unconfigured
development indicator; even a matched digest cannot enable advice or acceptance.
Research data is not injected into planning datasets. Format-3 policy/receipts
support pending/future frozen targets, complete evidence bindings and independent
signatures. Historical formats 1/2 retain original 70374/signature rules.
[Contract and command documentation](../data/LOCAL_DELIVERY.md).

Actual checks: 1,299 Rust workspace tests passed, four ignored, 178 result suites;
subsequent CLI delivery build/page/CAS/active integration passed all ten CLI local-
data tests. All 393 desktop unit tests passed; typecheck, production build, ESLint,
Rust formatting and workspace Clippy with warnings denied passed. No ordinary
Windows or interactive desktop claim; exact-head native/hosted checks pending.

Two independent read-only reviewers found whole-domain JSON expansion before
pagination and unavailable tables represented as zero records. Fixed with borrowed
slice serialization, source/gap metadata and null-versus-zero counts. Regressions
cover 10,000 typed records serializing only two requested rows; missing,
unsupported and justified empty tables; durable v1 migration/history isolation;
failed/stale activation preservation; rollback history; delivery substitutions;
future-target signatures, revocation, fixture refusal and pending current target;
stale/mismatched UI pages and rollback generation refusal. Reviewers confirmed
resolved, with no remaining actionable findings in reviewed scopes, including
new receipt CLI/pending config and guarded UI.

Limitations: desktop preparation currently binds the projection only; CLI supports
explicit media/input/guide bindings. A durable prior-revision picker and complete
input/media presentation follow in continuation 8. Changed analyzer profiles
require rebuilding artifacts; no silent trust upgrade. The launch target remains
pending, trusted V3 signers/expected development digests are empty, and current-
release V3 verification refuses. Imported counts are not reviewed completeness.
No new availability, signing, calibration, live or ordinary-Windows evidence was
obtained. Coverage expansion and publication are separate later PRs.

Next: continuation 7 — connect bound research facts to authored verification,
inspect the owner's private guide designs as inert source, design class-specific
solo/duo routes only where facts permit, and enumerate exact available record IDs
across all nine domains with independent completeness kept separate.
