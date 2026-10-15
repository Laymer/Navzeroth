# Deterministic release inventory investigation

Run from the repository root:

```sh
cargo run --locked -p azeroth-cli -- inventory-report --root . --input release/v1/inventory-investigation.json
```

The command emits JSON without writing files, downloading sources or executing
imports. Exit 0 means a valid investigation report was generated, **not** complete
coverage or publication readiness. The report always declares
`publication_authorized: false`. The separate `release-readiness` gate remains
mandatory. Source admission and content-pack validation remain external checks.

The committed input has nine unknown domains, zero imported records and zero
accepted sources. No complete independently evidenced 70374 inventory is currently
available. Running automation cannot fill that external prerequisite.

## Input version 1

`schema_version`, `inventory`, `enumeration_reviews`, `records`, and `scenarios`
are required. `inventory` is the unchanged release manifest version 1 format.
Keep this investigation copy synchronized with the proposed release inventory;
the report's exact `input_sha256` binds both inventory and imported comparisons.
The tool does not write or automatically replace the authoritative release gate
inventory.

- `enumeration_reviews`: domain, exact target, reviewer,
  `method: independent_target_review`, and evidence file pins exactly matching the
  domain's enumeration evidence. Review must independently establish exhaustive
  target scope. Counts copied from staged records are not such evidence.
- `records`: domain, exact record identity, source ID and immutable revision,
  original edition/build (build may remain null), target, target validation status,
  evidence file pins, unresolved conflicts and unsupported corrections. Source
  pins must match inventory requirements. Inclusion declares neither rights nor
  successful validation. Any failed/unverified record, missing evidence, conflict
  or unapplied correction blocks that identity, even if another source passes.
- `scenarios`: ID, domain, operator procedure, required record identities,
  validation status and evidence. Dependencies can cross domains or regions;
  missing, outside-inventory or blocked dependencies enter the observation queue.
  Scenario IDs are investigation procedures, never gameplay-content substitutes.

Record identities are opaque exact strings here so investigation can precede new
content contracts. Final gate bindings must use the canonical JSON-array IDs
specified in `release/v1/README.md` and future reviewed v2 collections. An opaque
string accepted by this tool does not imply an existing gameplay record.

All nine domains must appear exactly once with at least one scope unit. Unknown
required identities are `null`; known empty is `[]`. A required count appears only
when every unit is enumerated, domain completeness is complete and independent
review exists. Unreviewed empty units stay unknown. The report distinguishes
imported, missing, unexpected and blocked identities. Per-source missing lists
mean absent from that source relative to the declared target; they do not imply
that each individual source should cover the entire target.

Reports sort identities, sources, scenarios and queue entries, preserving a fixed
nine-domain order. Input permutations produce the same comparison; the input hash
still records exact supplied bytes. Imported evidence cannot reuse the same hash
as independent target enumeration evidence. Different hashes cannot prove real
independence: the named reviewer must inspect the evidence and methodology.

The CLI verifies every referenced evidence file's size/hash beneath the selected
root, rejects symlink/reparse traversal and conflicting pins, and reads at most
1 GiB of evidence with a 256 MiB per-file bound. Input JSON is at most 16 MiB,
4,096 units, 250,000 required IDs/imported records/scenarios, 250,000 scenario
dependencies, and 2,500,000 required-ID/source comparison pairs. Oversize inputs
fail without truncation. These are investigation limits, not v2 catalog limits.
No personal exports belong in committed evidence; review and sanitize first.

## Frozen content-v2 requirements

The report emits explicit field-validation requirements for all nine domains:
quests; equipment; acquisition; geography; travel; dungeons; class/roles; party;
and reminders. Full enumeration must include each applicable combination, not
only representative records. Representative scenarios supplement enumeration.

The next content-contract PR must expose typed, provenance-bearing records for:

1. Class/role models: actual documented `RoleProfileData`, class/role/level
   applicability, proficiencies, layout/configuration facts and model exclusions.
   Gear must consume these records rather than a manually entered local profile
   serving as evidence of full support.
2. Dungeon packages: instance and entrance/map identity, access requirements,
   preparation quest chains, directed travel dependencies, acquisition rewards
   and evidence-backed checklists. Dungeon consumers must resolve these facts;
   a single dungeon acquisition row is not a dungeon package.
3. Quest eligibility: access, exclusive branches, reset/repeat and sharing facts,
   including prerequisites and context. Party must consume these records.
4. Acquisition: reward choices, quantities, currency/costs, vendor/crafting/drop
   conditions, availability/prerequisites and loot entitlement. Gear practical
   comparison must retain uncertainty and conditional availability.
5. Party coverage: versioned supported sizes/workflows and evidence, linked to
   concrete quest, model, acquisition and dungeon dependencies as applicable.
   Unequal progress, sharing, stale members, fairness, agreement, advancement and
   review require actual coverage. Synthetic scenarios/local profile IDs cannot
   replace those content records.

Use typed observation states for unknown, unsupported, restricted and stale;
known false, zero and empty retain separate meanings. Every record retains
origin and target context plus evidence/source provenance. One logical immutable
catalog must resolve all cross-domain/region references before activation.
Keep snapshot/plan exchange versions unchanged. Final new collection binding
names and support-matrix format require the coordinator's explicit contract review.

Neither this investigation contract nor a schema extension supplies actual game
facts, licences, observed APIs, full-content performance or Windows/live acceptance.
