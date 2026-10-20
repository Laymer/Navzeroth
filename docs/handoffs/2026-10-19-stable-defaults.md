# STABLE-DEFAULTS handoff

Owner/sole writer: Codex `/root/tooling_review`; baseline `5d5fef9`;
branch `feat/stable-bundle-defaults`; claim recorded before code changes.

Implemented storage schema13, transactional content selection/travel choice,
revision-scoped durable acceptance, and version-triggered scope defaults. App
planning reads the persisted preference in its snapshot/content read transaction;
rollback restores retained revision preferences. App staging/configuration APIs
prepare validated composed sets without premature activation. Active inspections
and Content disclose actual reset history and persisted choice. See
[BUNDLED_DEFAULTS](../data/BUNDLED_DEFAULTS.md) for the loader interface and shared
scope semantics explicitly approved by the integrator.

The actual verified installed-resource loader remains separate shipment/receipt
closure work. No installation or shipped pack is invented. Fresh profiles share
scope state; extra profiles do not reset another profile's within-version choices.
Per-scope atomicity is guaranteed, not all-scopes-at-once configuration.

Validation discovered and corrected required integration points: recovery's table
whitelist, historical tests that manually roll databases back by dropping newer
tables, and latest-schema sentinels. A governance golden was regenerated with the
existing UPDATE_CONTENT_DISPOSITION_GOLDENS=1 command; its sole delta is the
explicit false travel preference in active inspection. Failed initial expanded
runs remain failures, not evidence of success.

Checks and final limitations are recorded below after completion.

Actual Linux validation:

- Locked offline pnpm installation, desktop typecheck and ESLint: passed.
- Desktop full Vitest suite with the existing per-file 90% line gate: 537 tests,
  66 files passed. ContentScreen executable statement-start lines: 80/86 (93.02%).
- Full app/storage LLVM run: 398 passed, zero failed, one existing ignored test.
  Changed production files measured: app content92.40%, snapshots91.08%, storage
  migrations95.52%, recovery92.35%; new preferences initially95.10%.
- App content-set integration: seven tests passed, including actual application
  close/reopen preserving true and false choices. Storage regression verifies
  first setup, same-version persistence, version reset, previous revision rollback,
  missing/invalid targets, late SQL failure atomicity and validated backup recovery.
- No coverage threshold was relaxed. These are native Linux measurements, not a
  hosted Windows/source-platform-union gate verdict.

Final clippy initially requested collapsing a nested if; the equivalent condition
was simplified, with a focused final preference coverage rerun below. No product
behavior changed for that lint fix. No native Windows/live game tests, installed
bundle verification or shipped-resource startup hook are claimed. Next: independent
review, integrate the verified loader interface and run refreshed protected checks.

Exact final validation: clippy --all-targets -D warnings passed for app/storage;
full app/storage LLVM rerun passed 398 tests (one existing ignored), with all five
measured changed production files still above 90% and final preferences96/101 =
95.05%. The focused 14-test coverage attempt alone reported partial per-file
coverage and is not counted as a gate pass; the complete final rerun supplies the
per-file evidence. Formatting and diff checks passed. Reports/logs remain outside
Git under /tmp/navzeroth-defaults-*; no private export or signing key was read.
