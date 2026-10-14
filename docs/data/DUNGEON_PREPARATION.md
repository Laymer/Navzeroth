# Local dungeon preparation references

Dungeons inspects an explicitly selected imported character or local party.
No verified production Deadmines package is bundled. Missing packages, content,
member snapshots, fresh observations and source evidence remain visible gaps.
Preparation results are imported-state assessments, not live game state or a
statement that a dungeon is accessible or the party is ready.

Select the character or party and choose **Inspect preparation gaps**. An empty
package is useful: it reports current context/content and member capture gaps.
The advanced local reference editor accepts bounded JSON declarations for an
exact active content revision. Load synthetic preparation reference supplies the
engineering example in fixtures/desktop/preparation/synthetic-package.json.
It matches the demo-r1 content fixture, not arbitrary production content. Fixture
capture timestamps are fixed; native execution may correctly show them as stale.
The Rust replay tests supply an explicit nearby UTC for their positive cases.

Before-departure and before-entry checkboxes are manual, local to the displayed
report and unsaved. They do not mark a quest complete, verify equipment or perform
a game action. Run and formation ranges are declared estimates separate from
measured history. Role labels are declarations, not weights or inferred benefits.
Consumable possession cannot be inferred from current snapshot contracts.

## Version 1 contract

The command is get_dungeon_preparation with a text argument containing:

```json
{"schema_version":1,"subject":{"kind":"character","profile_id":"selected-stored-profile"},"package":null,"origin":null}
```

A party subject uses kind=party and party_id. The origin, when present, is a manual
shared NodeId. Package fields are defined in crates/app/src/dungeon_preparation.rs.
Every package identifies schema_version, id/title, full context, fixture status
and revision. Each section is a Declared value: observation with explicit status,
and evidence IDs. Entrance, access, quests, rewards, checks, formation/run ranges,
departure/entry checklists and encounter notes are required fields. Missing data
must use unknown/unsupported/restricted as appropriate; it is not automatically
not_applicable. Known empty lists and zero ranges remain known values.

Known references must resolve in the exact active content: entrance node/map,
quests, full item keys, acquisition source containing the item, registered evidence
and its source. Cited confidence and permissions remain separate from the local
package's unverified status. Evidence linkage alone does not independently verify
an authored assertion or grant redistribution permission. No upload or publishing
operation is provided.

Limits are 1 MiB input, 32 records per section, 32 evidence IDs per declaration,
1024 distinct evidence IDs, 40 members, 160-byte titles/check labels and 512-byte
notes. Duplicate JSON keys, unknown feature fields, future versions, incompatible
contexts and missing known references fail without echoing input. Existing core
observation/predicate validation and recursion limits also apply. The fully
calculated report must serialize within 4 MiB before publication.

Evaluation uses owned captured inputs outside the store lock. Initial capture and
final comparison use consistent SQLite transactions. Final validation checks
member order, current snapshot hashes, active content hash and capture freshness
classes; fresh members additionally use the existing typed-field expiry guard.
Nonfresh members remain in the report but are excluded from predicate evaluation,
including remote ParticipantState reads. A shared entrance route requires fresh
captures for every member and stateless, supported travel usable by every member.
Failed member gates are retained as route exclusions. Changing editor inputs,
selection or refresh invalidates old displayed results.

A first source-covered Deadmines package, actual gameplay/native acceptance and
real role/consumable/encounter data remain separate blocked work.
