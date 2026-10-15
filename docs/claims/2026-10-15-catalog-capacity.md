# Full-catalog capacity claim

Owner/sole writer: Codex /root/inventory. Baseline `d59aa91`.
Branch/worktree: `feat/catalog-capacity`, `/tmp/navzeroth-catalog-capacity`.
User plan PR5, dependent on reviewed PR4 content-v2 contracts. Coordinator
explicitly delegates dataset graph capacity helpers, core immutable catalog indexes
and bounded revision pages, storage v2 migration/atomic activation/cache APIs.
Source worker owns content_v2.rs until explicit field-type transfer after freeze;
acceptance worker owns app/desktop consumers; coordinator owns pack/release binding.

Preserve ordinary v1 decoding, immutable bytes/hashes and rollback history. V2
limits: 250,000 reference identities and 250,000 evidence records, 256 MiB encoded
catalog; verified material closure is 1 GiB in pack boundary. Per-record/planning
bounds remain unchanged. Never silently fall back to stale v1 active content.
Checks: >1024 catalogs, capacity refusal, graph dependencies, bounded revision
pages/indexes, migration preserving v1 bytes/history, atomic activation/rollback,
and affected consumer tests. Actual full-corpus Windows performance unverified.
