# V110.9b — reviewed evidence version isolation

Owner/sole writer Codex `/root`; baseline `d823b0df`; isolated PR #297 branch.

Scope: explicitly permit 1.0.0/1.1.0 reviewed public-evidence imports, select only
the requested version's inert Git subtree, and reject malformed/unlisted versions
before transport. Preserve public-only scope, source cleanliness, bounds, no code
execution and Rust readiness authority. Do not copy/relabel historical receipts.

Validation: Python transport regression tests, negative cross-version cases,
focused coverage and independent read-only review. Remaining Windows workloads,
inventory and release workflow versioning remain separate required work.

Trigger recorded: Windows package run 42836215251 attempt 1, head d823b0df,
checkout merge 84def34e against aaa73b94, stopped at assemble-bundled-content with
`Bundle production differs from candidate`. Existing 1.0.0 receipt pins a947d2f0;
the source changed. Installer/acceptance never ran. Read-only recovery_review
confirmed this expected binding refusal. No rerun or gate weakening is authorized
by this failure; fresh 1.1.0 closure follows feature/content freeze.
