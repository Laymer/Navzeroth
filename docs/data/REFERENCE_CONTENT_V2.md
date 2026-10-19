# Reference content v2 contract design

Owner: Codex `/root/source_admission`; coordinator approved 2026-10-15.

PR4 introduces a separate `navzeroth-reference-content` schema version 2 decoder.
It nests the unchanged validated v1 reference envelope as `reference`: that
contains the only context, dataset revision and fixture authority. Extensions
carry evidence IDs resolving into that same reference catalog. Snapshot and plan
exchange versions remain unchanged, as do existing v1 decoding and hashes.

Typed collections hold class-role models (validated gear weights, applicability,
proficiency, slot layout and exclusions), dungeon records, quest eligibility/
sharing/reset details, equipment category/configuration facts, acquisition
quantity/choice/cost/availability/entitlement details, with a separately owned release-layer party support matrix linked to concrete
quest/model/dungeon records. Missing values
use Observation, not fabricated defaults. Predicate and graph references resolve
across the entire logical catalog, including across authored regions. Conflicted
or disputed evidence cannot substantiate an extension record.

The first contract commit retains nested v1 collection bounds. PR5 must provide
v2-specific complete-catalog validation/indexing, preserving v1 bounds and hashes;
this intermediate implementation must not claim 250,000-record capacity. The
outer reader rejects oversized bytes and unknown versions before any activation.
A later pack v2 manifest must explicitly select this decoder. Storage must retain
original bytes/schema/revision and expose an active catalog view. Gear, Party and
Dungeon commands select records by ID from that active view and require same
context/revision. Existing manual v1 workflows remain available and distinct.

This document and core module are the contract portion of PR4 only. Pack v2,
app consumers, storage migration, generated schemas and UI selection are
coordinator-owned dependent work. No v2 pack or production corpus is shipped by
this contract foundation. Inventory IDs must bind to typed collections rather
than local profiles or synthetic scenario names. Unknown inventory remains an
external blocker independently of successful synthetic contract tests.

## Additive node provenance

`node_facts` is an optional v2 collection of `{ "node": "node-id", "evidence": ["evidence-id"] }`. Empty collections are omitted during serialization so existing v2 drafts retain their canonical bytes; v1 remains unchanged. Each node ID must resolve once into the nested reference catalog. Evidence must be nonempty, unique, resolved, and neither conflicted nor disputed/stale. These records count toward the shared 250,000-identity limit.

Only a node's own facts supply sources for its `[revision,"nodes",node]` release identity. The facts also have a `[revision,"node_facts",node]` identity in the travel domain. Nearby edges/maps and unrelated catalog records never lend a node their sources. No separate desktop list is needed for this provenance metadata. A draft may omit node facts, but full release binding requires a fact record for every node and rejects any claimed record with no source attribution, even when other records in its inventory unit have sources. This is a provenance contract, not new gameplay content or a rights grant.

## World content additions (task V2.8d)

Optional members, omitted when empty, so earlier catalogs keep their bytes:
`loot_tables` (complete loot templates, references unexpanded, at most 4,096 rows
each), `EquipmentFacts.name`, `QuestFacts.title`/`givers`/`enders`, and
`Dungeon.bosses` (resolving to `entities`). An `ItemDefinition` may have no slot
only when `two_handed` is `not_applicable` (not equippable). Authored packs refuse
loot tables. Mapping and limits: [WORLD_CONTENT.md](WORLD_CONTENT.md).
