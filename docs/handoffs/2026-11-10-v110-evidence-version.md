# V110.9b — version-isolated reviewed evidence import

Owner/sole writer Codex `/root`; baseline `d823b0df`; isolated PR #297.
Claim: `docs/claims/2026-11-10-v110-evidence-version.md`.

The inert evidence importer explicitly permits 1.0.0 and 1.1.0. Production helpers
require a version and read only `release/evidence/<version>/`. Unlisted/malformed
versions refuse before Git or destination creation. Both versions retain public-only
reviewed-commit scope, exclusion of evidence_run_id, byte/total/count/deadline bounds,
exact clean source checks, regular blobs and no archive execution. Data is copied
unchanged; Rust readiness must independently validate its claims. Without a reviewed
commit the existing CLI path remains a no-op, granting no acceptance.

Actual checks: Python transport suite 21 tests, 20 passed and one actual Windows
junction test skipped on Linux. Independent recovery_review reran the same suite
and whitespace check with the same outcome; no blocker found. Standard-library
trace measured 91.7% executable line coverage of import-reviewed-evidence.py;
report `/tmp/navzeroth-v110-evidence-trace/import-reviewed-evidence.cover` and
log `/tmp/navzeroth-v110-evidence-trace.log`. This is local Python instrumentation,
not the hosted Rust/TypeScript coverage receipt. `coverage.py` is not installed;
an initial trace invocation used `-m` instead of `--module`, then was corrected.

New regressions cover 1.1.0 CLI propagation, mixed-version archives, refusal to
inherit a 1.0.0-only archive and malformed/unlisted versions before transport.

## Hosted attempt preserved

[Windows package run 42836215251 attempt 1](https://github.com/Laymer/Navzeroth/actions/runs/42836215251)
tested PR head `d823b0df2ec012530daf57827f94fb16f08817a7` via merge
`84def34e05134a789a6668352294ca5ae10a5aa8` against `aaa73b94`.
Step 39 Build installer and addon package failed during assemble-bundled-content:
`Bundle production differs from candidate`. Application metadata remains 1.0.0
pending feature freeze; retained 1.0.0 evidence pins production a947d2f0 and cannot
cover changed source. Read-only recovery_review confirmed this expected refusal.
No installer/native acceptance ran. Subsequent missing-evidence failures are
consequences. No retry, relabeling of receipts or weakened limits/gates performed.

The same head/merge failed Windows Rust coverage in
[run42836215294, job129898987322](https://github.com/Laymer/Navzeroth/actions/runs/42836215294/job/129898987322),
attempt1. `coverage-bundle-assembly.log` from artifact13776290273 reports the same
`Bundle production differs from candidate` (CLI exit1), after successful compilation.
Frontend build and bundle-input hydration completed. No test/exporter failure was
identified. Independent recovery_review inspected the actual log; this is a failed
coverage collection, not a complete Windows coverage report or acceptance pass.
The retained artifact was independently verified at83,106,061 bytes, SHA-256
`128cc54699f5274f6240f8a326c11eefa5e09696ee3dd829de26254dbdcf9c1d`;
local copy `/tmp/navzeroth-coverage-windows-42836215294.zip`. Its24-command record
has only bundle assembly as an unexpected result; native smoke/report completion
is not established.

Next: typed gameplay catalog, four guidance workflows and saved dependencies;
remaining version-aware inventory/Windows tooling. Exact 70862 interface evidence
and refreshed content remain pending. Fresh 1.1.0 identities/signatures and complete
installer/draft acceptance follow feature/content freeze. No merge/tag/publication.
