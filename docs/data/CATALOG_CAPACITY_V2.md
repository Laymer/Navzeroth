# Immutable v2 catalog capacity and pages

Ordinary snapshot/plan/reference v1 decoding remains unchanged, including the
1,024 collection bound. Reference-content v2 uses `Envelope<CatalogDataset>`;
the transparent payload wrapper applies a separate complete-graph capacity policy
before normal context, evidence and reference validation. It cannot be decoded as
a large v1 envelope.

V2 allows at most 250,000 reference identities and independently 250,000 evidence
records, within 256 MiB of encoded canonical content. Reference identities count
all ordinary dataset collections except evidence, nested quest stages and map
pins, plus class/role models, equipment facts, quest facts, acquisition facts and
dungeons. `ReferenceContent::identity_count()` is the shared definition. The pack
boundary separately limits verified material closure to 1 GiB. Per-record vectors,
predicates, planning selections and imported snapshots keep their smaller bounds.
Oversize inputs fail; no records are truncated.

Dataset validation indexes cross-region and cross-domain references. Coverage
uses indexed objective owners, iterative prerequisite cycle traversal and one
`atlas::ProjectionIndex` per projection batch. Map embedding depth is refused
immediately beyond the existing bound. A 50,000-node synthetic chain/cycle test
checks iterative traversal; it is not full-content Windows performance evidence.

## Storage and version compatibility

SQLite migration 11 adds `content_version` with default 1 to immutable revision
rows. It preserves existing documents, hashes, source records, dispositions and
activation history. Both formats share the same context-qualified active and
previous pointers; activation, rollback and quarantine fallback remain atomic.
An immutable revision name cannot be reused for different bytes or a new format.

`ActiveContentRevision::{V1,V2}` contains immutable `Arc` values and exposes
reference data, context, fixture flag, revision, sources, conflicts, key and
optional full v2 content. `active_catalog_revision`, `usable_catalog_revision`
and `validate_usable_catalog_guard` are the common read APIs. Legacy v1 reads
explicitly refuse an active v2 value; they never silently read an obsolete v1
pointer. Application consumers must migrate to the common API before release.

The one-entry catalog cache and one-entry row-index cache bind SQLite row identity,
`PRAGMA data_version` and the connection's change counter. External writes or
local mutations invalidate cached validation. Disposition and active-pointer
checks still run on every use. Caches do not retain retired catalogs indefinitely.
Database corruption is an error, not permission to substitute older content.

V1 stored hashes retain their exact old serialization. V2 revision hashes bind
content, source declarations and conflicts. The separate catalog content SHA is
exactly the pack's sorted-object pretty JSON plus trailing newline; it does not
include party evidence and therefore avoids a circular support-matrix hash.

## Page API

`catalog::PageRequest` requires the exact revision and content SHA, collection,
offset and limit. Limits are 1–200 rows; out-of-range offsets and stale identities
fail. `CatalogPage` reports the same identity, total, offset, rows and an optional
next offset. Each row supplies its deterministic serialized identity and original
record index; the application resolves only those rows into display DTOs.

`SqliteStore::catalog_page` validates the active v2 scope and uses the cached index.
Supported collections include all ordinary top-level gameplay collections,
models, dungeons and equipment/quest/acquisition facts. Evidence is validation
material, not an interactive gameplay selection. Stable key ordering makes pages
repeatable within one revision; an activation invalidates old tokens.

Desktop integration is a dependent writer's task. Backend pages alone do not
establish that every UI list is bounded. Full accepted corpus measurement,
ordinary Windows cached-lookup/focused-planning budgets and real source content
remain unverified until the licensed corpus and operator environment exist.
