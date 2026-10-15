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
