# V110.4d — application progression commands

Owner/sole writer Codex `/root`; baseline `0bed7f4e`; isolated PR #297 branch.
Claim: `docs/claims/2026-11-10-v110-progression-application.md`.

## Implemented

Three application/native commands: get_progression_view, import_progression_text,
save_manual_progression. Views obtain predecessor and both origins in one consistent
read. Both mutations check the expected exact binding; storage rechecks selected
predecessor atomically. Wrong-profile/stale UI/companion identities refuse.

Manual DTOs contain values and completeness, never caller-supplied provenance.
The service assigns manual source/capture metadata (synthetic for fixture profiles),
preserves unknown/zero/false and separate bags/bank/equipment scopes, and validates
all resulting domain invariants. Missing trusted boundary time stays uncertain.
Records remain immutable; refreshing a manual capture requires a new companion ID.
No imported observation is overwritten or promoted by manual input.

New progression-api schema/TypeScript are generated from Rust. The existing TS
generator now regenerates both artifacts. Rust tests enforce schema drift; AJV
checks manual provenance refusal and unknown/complete semantics. Native handlers
use blocking database-host dispatch; command-map wrappers preserve safe errors.
Browser demo explicitly refuses persistent progression commands.

## Actual checks

- Five progression app tests plus ten snapshot/recovery regression tests passed.
- Desktop: 557 unit tests, typecheck and lint passed.
- 74 exchange fixtures and new progression API schema constraints passed.
- Existing core contract-schema tests passed; exchange v1 output unchanged.
- App all-target Clippy, formatting and whitespace passed.
- Focused llvm-cov: manual.rs 92/97 lines (94.85%), mod.rs 173/176 (98.30%).
  Schema exporter explicitly executed with instrumentation and added to the hosted
  coverage collector. Full release coverage/Windows acceptance is not claimed.
- Read-only reviewer app_progression_review found no blocking logic issue and
  identified missing schema-exporter coverage execution; repaired in collector.
  Reviewer inspected native registration/wrappers and whitespace, did not run tests.
- Coverage tooling tests: five passed after sandbox EPERM spawning Node was rerun
  with escalation. An initial regression command named nonexistent test `boundary`;
  corrected to the actual progression/snapshot_workflow/local_state_recovery targets.

Initial compile corrected wrong depth-helper module and SchemaVersion constructor
before passing checks. New schemars dependency uses already-pinned version1.2.2;
no dependency version changed. Native Windows new handlers remain pending hosted
verification; no interactive/live-game verification.

## Next

Expose these commands in progression/manual-input UI, then implement saved plan
binding and the four guidance services with versioned gameplay facts. Exact70862
interface evidence/content refresh remain pending. No merge/tag/publication.
