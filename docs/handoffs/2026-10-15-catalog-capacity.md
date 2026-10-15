# Full-catalog capacity backend handoff

Owner: Codex /root/inventory. Baseline `d59aa91`; claim `65354b2`.
Branch/worktree: `feat/catalog-capacity`, `/tmp/navzeroth-catalog-capacity`.
Dependency PR4 core `f5d6ed6`, schema `1cdba15` and semantic `e17a551` merged normally.
Source worker explicitly transferred capacity/type-bound changes; app and desktop
writers received the concrete common-storage and page interfaces before migration.

Implemented separate v2 250k reference/evidence budgets, 256 MiB canonical catalog,
indexed complete graph validation, immutable caches and <=200 revision/hash pages.
Migration11 retains v1 bytes/hashes/history and the shared atomic activation,
rollback and disposition pointer. Common Arc-backed read APIs never silently fall
back to old v1. Details and exact identity-count definition are in
[the capacity guide](../data/CATALOG_CAPACITY_V2.md).

Actual verification:

- Full core/storage suite at `68c2923`: 431 passed, zero failed, one explicitly
  ignored fixture generator. Later semantic follow-up: 9 v2 tests passed with
  the same explicitly ignored fixture generator; schema artifact matches.
- Four catalog capacity/page tests: passed, including >2,048 records, v1 refusal,
  actual cross-region edge dependency, stale hash, <=200 pages and independent
  capacity boundaries.
- Three new storage tests: passed, including populated v1 migration byte/hash
  preservation, large v2 activation, Arc cache reuse, rollback across versions,
  stale pages, external tamper invalidation, quarantine fallback and failed stage.
- Existing 17 coverage and eight atlas tests pass after indexing; a 50,000-node
  chain/cycle test passes without recursion.
- Core/storage all-target strict Clippy, formatting and whitespace checks pass.

Development checks initially exposed a missing cache initializer, an index-macro
lifetime error, a test borrow error and Clippy style findings; all were fixed.
Old migration reconstruction fixtures needed to drop the new column before
pretending to be schema8/9, and future-schema tests needed the new latest version.
The initial new storage fixture had no items; the test now explicitly uses the
existing compatible synthetic item fixture. No failed run counts as acceptance.

Remaining PR5 integration: app consumer migration and desktop list pagination,
independent review, all nine hosted checks on final integration and main. Root
owns pack/material1GiB enforcement and source worker owns UI pages. Backend-only
engineering is not full-corpus acceptance. Actual licensed content, native Windows
performance and live target validation remain unavailable/unverified.

UI coordination follow-up adds cached `catalog_first_page`, so an initial page
request does not require serializing the full catalog to obtain its hash. The
storage page test now verifies the returned first-page identity; all three storage
v2 tests and strict core/storage Clippy pass after this change.

Recovery follow-up: generic backup row bounds previously remained 32 MiB, which
would refuse a valid large v2 catalog on restore. Revision/manifest rows now allow
256 MiB plus bounded identity-column overhead; other rows keep their prior bound.
A targeted regression checks the separate limits; affected recovery tests rerun.

Integration verification: merged core v2/main dependencyfb6303b normally into
837de5e, preserving catalog, release_inventory and source_admission registrations.
Repeated four catalog-capacity and three storage-v2 tests: all passed. PR133 base
changed to main for hosted checks; core PR131 remains a merge prerequisite.
