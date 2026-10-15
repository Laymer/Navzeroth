# Verified pack import and bounded Content inspection

Owner /root/acceptance; baseline8cd85ac; claimb511bd0. App/native Content import
explicitly accepts a local extracted pack directory. It calls the shared ingest
v2 read_verified verifier once, retains those exact verified bytes and atomically
stages catalog/registry/assets through storage. No activation, rights inference,
network retrieval, imported code execution or parallel verifier is introduced.
Existing explicit activation and rollback remain separate.

ContentInspection keeps validation summaries and at most200 errors/gaps/source/
conflict rows per collection. Counts and immutable revision hash are explicit.
Further pages bind full ContentRevisionKey+hash, including staged revisions before
activation, under one SQLite read transaction. Hash includes source registry and
conflicts, unlike the canonical reference-only catalog hash. The full report is
computed internally for validation, never sent whole when oversized. Content UI
pages rows and rejects stale result identities; activation does not depend on
which inspection page is visible. Small v1 DTO payloads retain original shape.

Checks: full app suite passes; two focused integration tests exercise205sources
as200+5/stalehash/offset refusal and a real synthetic image pack build->verify->
stage->activate->stored image retrieval, with catalog tamper rejection and no
implicit activation. All204desktop tests pass, including explicit directory
staging and hash-bound inspection navigation. App strict Clippy, desktop typecheck
and lint pass (test non-null assertion lint corrected); formatting checked after
final edits. Native compilation remains unverified due missing Linux DBus dev
package. Windows70374/live/full-corpus performance remain unverified.

Atlas concurrency correction is assigned to inventory worker: stored_atlas.rs,
catalog_pages.rs and snapshots marker block frozen to them. This task only changes
snapshot Content stage/active/rollback inspect-result plumbing, so merge stays
separate. Coordinator owns frozen Atlas PR136/main integration; source worker owns
Dungeon travel followup. Next: merge their reviewed corrections and latest pack
contracts, then rerun affected checks and hand off draft PR.

Follow-up integration: preserved coordinator's shared verifier PNG regression by
normal Atlas merge (it was added after this task's implementation baseline).
Merged reviewed authored Dungeon travel fix e274b0c, combining both additive tests;
all8 authored/inspection integration checks pass. Merged consumer browser
mock/narrow-width fix dc0783f; all40 Playwright browser tests pass (24.5seconds).
Directory paths now have an explicit4096byte app boundary. Independent coordinator
review of bd3d4a4 found no important issue. Pending only coordinated Atlas consistent
read correction before final integrated test run and draft handoff.
