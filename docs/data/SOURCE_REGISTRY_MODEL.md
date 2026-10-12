# Source registry, evidence precedence and conflict model

Implements plan M3.2 (sections 8.3-8.5) as the pure module
`crates/core/src/sources.rs`. It performs no I/O, reads no clock and holds no
built-in source rows. Persistence (the `source_conflicts` table), envelope
integration, CLI and UI exposure are wave-2 work (M3.1, M3.6).

## Source registry

A `SourceRecord` covers the nine plan 8.3 field groups. Every policy field is an
`Observation`, so an unknown licence, schedule or offline flag stays distinct from
a recorded negative. Nothing defaults to "permitted": an unknown redistribution
observation must be treated as not permitted by callers.

`SourceRegistry` maps `SourceId` to `SourceRecord`. Registering an existing ID
fails; the first registration is kept. At most 1024 sources are accepted. Ingestion
requires every evidence `source` to be registered first (`DatasetIncomplete`
otherwise), and, when the source's known edition coverage does not include the
evidence's origin edition, fails with `BuildMismatch`. Unknown coverage is not
treated as a refusal. `ProviderCapability` keeps provider status; a source only
carries an optional `ProviderId` link.

## Precedence

`precedence_rank` orders the existing `Confidence` categories (no numeric
confidence, no new variants):

1. `verified_on_build`
2. `publisher_documented`
3. `limited_sample`
4. `cross_edition_unverified`
5. `inferred`
6. `disputed_or_stale`

`compare_precedence` ranks records by effective category, then by lower
`EvidenceId`, then by lower `SourceId`, so the order does not depend on input
order. Precedence only chooses the preferred record; every distinct evidence
record stays in the output.

## Keys, origin and isolation

Records are keyed by `RecordKey` (item or quest), which always carries the full
`DataContext` (edition, build, interface, namespace). The same item ID in another
context is a separate record and never a conflict (CASE-005). Each `Candidate`
pairs a payload (`ItemDefinition` or `QuestDefinition`) with the module-local
`origin` context and its `EvidenceRecord`. Validation requires that the payload
cites the evidence, that the evidence context equals the record context, that an
origin in the same edition equals the record context exactly, and that the origin
namespace equals the record namespace (demo and reference never mix, CASE-015).

## Conflict kinds

Conflicts are plain serde data (`SourceConflict { key, kind, preferred, others }`)
for wave-2 persistence. They are computed per key, never by last writer:

- `value_disagreement`: payloads differ (ignoring which evidence IDs they cite)
  and at least one disagreeing record is not in a strictly lower category than
  the preferred one.
- `category_downgrade`: payloads differ and every disagreeing record is strictly
  lower than the preferred one.
- `cross_edition_relabel`: the origin edition differs from the record's edition
  and the evidence claims more than `cross_edition_unverified`. The record is
  kept but ranked as `cross_edition_unverified` and never reported as verified
  (plan 8.5). A cross-edition record honestly labelled `cross_edition_unverified`
  (or lower) is not a relabel.

## Idempotence and errors

`NormalizedState::ingest` returns a new state. An identical evidence record is a
no-op. An evidence ID reused with different content is `InvalidInput`, never an
overwrite. More than 1024 candidates or total evidence records is rejected.

## Known limits

- The relabel check relies on the declared `origin`. A producer that also lies
  about the origin is not detectable here; that is a source-trust question for the
  registry's `record_class` and licence review, not for this module.
- `EvidenceRecord.conflicts` (declared conflicts) is not interpreted; computed
  conflicts are reported separately.
- Only items and quests are modelled.
