# Lifecycle sidecar race correction — 2026-10-18

Owner/sole writer: Codex `/root`; baseline `ac114c8`, branch
`fix/lifecycle-sidecar-race`. Claim recorded before implementation.

Refreshed PR #201 Windows Rust CI run 39132767142 failed the ordinary opt-in
change-detector workflow: SQLite removed its journal between an existence check
and a second checked-path lookup. This is an independent product defect.

Validate each known database/sidecar leaf from one no-follow metadata observation.
Missing optional files remain permitted; other I/O failures are explicit refusals.
Observed links/reparse points, non-files and excessive sizes remain refused. Root
and ancestor checks remain in place. No cache/game read or activation change.

Actual checks:

- Four focused lifecycle unit tests passed: deterministic disappearance after
  metadata capture, missing/access errors, type/size boundaries and Unix symlink
  preservation. Windows reparse behavior awaits native CI.
- Reintroducing the redundant checked-path lookup made the deterministic test
  fail with `Selection is inaccessible`; the mutation was then removed and all
  four tests passed again.
- Both affected application workflow tests passed, including opt-in detection,
  staging without activation and opt-out.
- Local-data all-target Clippy with warnings denied passed.
- Two independent read-only reviewers reported no actionable findings; one also
  independently ran all four focused tests and diff whitespace verification.

Next: exact-head CI, native Windows packaging and both source platforms/parity;
merge only after successful checks and final review. Refresh PR #201 and the
coverage branch afterward. Coverage, development RC and Pages remain incomplete;
no publication or acceptance claim is made.
