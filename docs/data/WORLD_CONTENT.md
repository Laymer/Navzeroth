# World content (`world_to_content.py`)

Task V2.8d ([ADR 0007](../decisions/0007-world-data-automation.md) §2-§4, §7, §8).
`data-generators/world_to_content.py` maps the generated world data shards
([WORLD_DATA.md](WORLD_DATA.md)) into one content v2 catalog for the whole world.
It uses the Python standard library only, parses imports as JSON and never runs them.

    python3 data-generators/world_to_content.py --input data-generators/output \
        --target release/v1/world-data-target.json --policy release/v1/accepted-origins.json \
        --authored content/authored/slice-v1 --out content/world/1.60.1.70450
    cargo run --locked -p azeroth-cli -- build-content-v2 --input content/world/1.60.1.70450 --output <new-dir>
    cargo run --locked -p azeroth-cli -- verify-pack --input <new-dir>

Tests: `python3 -m unittest data-generators/tests/test_world_to_content.py`
(offline, synthetic shards) and the content v2 tests in `crates/core/tests`.

## Output

| File | Contents |
| --- | --- |
| `catalog.json` | `navzeroth-reference-content` schema 2, compact JSON with sorted keys (`build-content-v2` writes the canonical pretty form into the pack) |
| `sources.json` | pack source registry (schema 1) for every source an evidence record cites |
| `conflicts.json` | every disagreement between inputs and how precedence settled it |
| `summary.json` | counts, dropped records with reasons, datasets not mapped, parameters, input and catalog SHA-256 |

Every record carries the target context from `--target` (edition, build,
interface; namespace `reference`, `fixture: false`). The revision is
`navzeroth-world-<build>-<first 12 hex of the input digest>`; the digest covers
every shard path and SHA-256, so new input gives a new revision. Collections are
sorted (numbers inside IDs numerically), keys are sorted and no clock or host path
is read: two runs on the same input give byte-identical files.

The files under `content/world/` are stored through Git LFS (`.gitattributes`).

## Origins and evidence

The origin edition is never relabelled. One evidence record exists per
(source, origin edition, dataset): `ev:<source>:<origin>:<dataset>`, for example
`ev:cmangos-classic-db:vanilla_1_12:world.npcs`. Its `source` is always a source of
the record's true origin, so the source's `editions` read `[<origin>, <pack
edition>]` (V2.8h). Its confidence is the shard's (`cross_edition_unverified` for
1.12 data, `publisher_documented` for Forever client tables); its notes name the
origin edition and build, the shard paths and the source revision and snapshot hash;
`observed_at_utc_ms` is the earliest retrieval time the shards record, otherwise
`unknown`. A 1.12 shard that also read Forever client tables (spawn positions from
the client's map layout, map and area names, NPC reactions from `FactionTemplate`)
cites only the CMaNGOS evidence; the wago.tools source, build and snapshot are named
in that evidence's notes ("The generator also read wago-tools-wowf ..."), so an
origin marker never shows Forever for 1.12 records. A source not listed in the
mapping's source table is taken as the origin's own.

Derived values cite `ev:navzeroth-world-content:inferred:<rule>` with confidence
`inferred`: `travel.walk`, `travel.flight-time` and `loot.reference-expansion`.

`sources.json` gives each upstream source `editions = [<true origin edition>,
<pack edition>]`, e.g. `["vanilla_1_12", "wow_forever"]` for CMaNGOS: the storage
layer requires the pack edition, and the origin marker reads the other entry. The
derived source `navzeroth-world-content` has `editions: not_applicable`. Licence,
attribution and redistribution follow
[source admission](../sources/source-admission-v1.md): CMaNGOS and wago.tools are
`conditional`, the derived source is `permitted`, so `distribution_eligible` is false.

## Policy and precedence

`--policy` (`release/v1/accepted-origins.json`) names the origins accepted per domain;
a shard from another, unaccepted origin is skipped and listed under
`skipped_origins`. When two inputs state the same record (same dataset and key), the
origin earlier in `precedence` wins: owner observations, `wow_forever`, authored
content, `vanilla_1_12`, `wow_classic_era`. Every field of the losing record that
differs is a `value_disagreement` in `conflicts.json`, with both values and origins;
for items only `name`, `quality`, `class_id`, `subclass_id`, `inventory_type`,
`item_level` and `required_level` are compared (Forever stats are computed from
stat budgets without armor or ratings, so stats and prices differ by design).
Other reports there:

- `missing_in_higher_precedence`: an item the Forever item table lacks that the
  1.12 `item_template` defines; the 1.12 item is used (chosen origin
  `vanilla_1_12`) and `references` counts its vendor, quest-reward and loot uses.
- `missing_item`: an item referenced by 1.12 data that no item table defines, with
  the reference counts. Such references are dropped from acquisitions and quest
  rewards (an `ItemKey` must resolve) and kept as raw IDs in loot tables.
- `prerequisite_cycle`: a quest prerequisite that closed a cycle and was dropped.
- Authored content: the Deadmines title "The Deadmines" (authored) outranks
  "Deadmines" (1.12); area names from the Forever `AreaTable` outrank the 1.12
  level table.

## Mapping

### Map spaces

One map space per uiMap (`map_id` = uiMapID as a decimal string, floor `"0"`, no
instance). A uiMap with exactly one `UiMapAssignment` gets a `CoordinateTransform`
in `yards`; one spanning several (Azeroth, 947) or none stays `unknown`. The client
map's x grows east, which is world -Y, and its y grows south, world -X:

    nx = u0 + (u1 - u0) * (maxY - Y) / (maxY - minY)
    ny = v0 + (v1 - v0) * (maxX - X) / (maxX - minX)

so `target_x = scale_x * nx + offset_x` is world **Y** and `target_y` is world **X**,
with `scale_x = -(maxY - minY) / (u1 - u0)`, `offset_x = maxY + u0 * (maxY - minY) / (u1 - u0)`
and likewise for y. Hogger (-9947.88, 594.77) maps to (0.271, 0.868) on Elwynn
Forest (1429). Dungeon instance maps have no uiMap: map space `instance:<MapID>`,
transform `unknown`.

### Nodes, entities

- Spawn nodes `node:npc-spawn:<guid>` / `node:object-spawn:<guid>` at the spawn's
  known uiMap position, for: NPCs with a role (all their spawns, at most 1,024),
  quest givers and enders (NPCs and objects, all spawns), and quest-objective
  entities: kill targets, `object` objective targets and creatures dropping a
  quest-only item that a quest objective needs, at most 3 spawns each (lowest guid,
  then farthest apart). Unsupported positions (instances) give no node.
- `node:taxi:<TaxiNodes ID>`, `node:graveyard:<id>`,
  `node:dungeon-entrance:<MapID>` (entrance located on its most specific uiMap).
- Service: flight, dungeon, trainer, vendor, repair, then `other` (quest givers,
  innkeepers, bankers, graveyards ...); objective-only nodes are `not_applicable`.
- `node_facts` for every node with its position evidence.
- `entities`: every NPC (`npc:<entry>`) and game object (`object:<entry>`) with name,
  subname (absent: `unknown`), levels (`unknown` when 0), services from roles,
  reaction (objects: `not_applicable`) and its nodes; a flight master also lists its
  taxi node.

### Travel edges

Both kinds have status `unverified` (only `Verified` needs evidence; the planner's
treatment of unverified inferred edges is the policy task's), `cooldown_ms` and
`wait_ms` `not_applicable`, `uncertainty` `unknown`.

- Flight `edge:flight:<TaxiPath ID>` between taxi nodes: `expected_ms` = path length
  (sum of 3D segment lengths between consecutive points on one map) / **31 yards/s**
  (a commonly quoted taxi speed; take-off and landing are not modelled).
  Requirements stay `unknown` (known flight points and faction are not expressible),
  `consumes_resource` `unknown`. The taxi cost has no edge field and is not mapped.
- Walking `edge:walk:<from>:<to>`: on one uiMap, each service/quest/flight/graveyard/
  entrance node joins its **4** nearest such nodes within **1,000 yards**, and each
  objective-only node its **1** nearest such node; every pair is written in both
  directions. `expected_ms` = straight-line world distance / **7 yards/s** (run
  speed). Requirements known-empty (`all []`), `consumes_resource` false. Terrain,
  water and hostile creatures are ignored.

### Quests

- `quest:<id>`, stages `quest:<id>:objective:<n>` (one per objective) or
  `quest:<id>:complete`. No `objectives` records are written: an `Objective` needs
  participants.
- Eligibility: `level_at_least min_level`; `min_level` 0 is known "no minimum".
- Prerequisites: positive `prev_quest` = turned in, negative = accepted; quests whose
  `next_quest` is this quest add "turned in" (any member of a positive exclusive
  group). An absent reference is 0 in CMaNGOS and means none (known-empty); a
  reference to a quest outside the data makes prerequisites `unknown`.
- Locations: positions of giver and ender nodes (at most 64). Rewards: reward and
  choice items the item table defines.
- `quest_facts`: access (same predicate), `access_requirements` from the race/class
  masks (`race:<name>`, `class:<name>`; 0 or all = no restriction), positive
  exclusive groups (`taken_when: turned_in`), shareable from flag 0x8, plus the
  optional `title`, `givers` and `enders` (`npc:`, `object:` or `item:` IDs).
  Reset, completion and sharing conditions stay `unknown`.

### Items

Every Forever item, then every 1.12 `item_template` item the Forever table lacks
(`item_id` = item ID, variant `base`; the 1.12 items cite the CMaNGOS evidence).
wago.tools serves one `wowf` build (1.60.1.70245) whose `ItemSparse` has 19,227
named items; its `Item` table has 31,824 rows, but the 12,597 without an
`ItemSparse` row carry no name, level or stats, and wago.tools lists no other
`wowf` build (checked 2026-10-19).
Equippable items get slots
from `InventoryType` (the `inventory_type_slots` mapping), `two_handed` true for
type 17, attributes `stat:<name>` from the generator's stats (Forever: armor and
combat ratings are not in the client table; 1.12: `stat_typeN` under the same
names plus `armor` and `<school>_resistance`), requirements `level_at_least` or
`unsupported` when the class or race mask restricts. Non-equippable items
(type 0: quest items, reagents, consumables) are kept with **no slot and
`two_handed: not_applicable`**: the core contract accepts an empty slot list only
with that explicit marker, so vendors, loot and quest rewards can reference them.
`equipment` facts carry the category (`item_class:<class>:<subclass>` or
`non_equippable`) and the optional display `name`.

### Acquisitions

- Vendor `vendor:<npc>:<item>`: cost and `costs` = the item's buy price in copper,
  probability 1, origin `vendor {entity, location}` at the NPC's first node
  (`unknown` without a located spawn), entitlement individual.
- Quest reward `quest-reward:<quest>`: all reward items; choice items share
  `choice_group` `quest:<id>:choice`; probability 1 without choices, otherwise
  `not_applicable`.
- Drop `drop:<table>:<entry>:<item>` with probability on the base record: kept for
  equippable items, quest-only items and dungeon-boss loot; other rows live only in
  `loot_tables`. Probability is |chance| / 100; a zero-chance row in a group gets
  its equal share of the group's remaining chance; otherwise `unknown`. Quest-only
  rows (negative chance) have conditions "a quest needing the item is accepted";
  rows with a CMaNGOS condition are `unsupported`. Origin `drop {entity, dungeon}`
  where the entry names an entity (creature, pickpocketing and skinning entries are
  the creature entry by the classic-db convention; item containers `item:<id>`,
  fishing `area:<id>`; game objects `object:<id>`: the generator writes
  `gameobject_loot_template` once per chest or fishing hole whose `data1` names the
  loot ID). Disenchant loot IDs (`item_template.DisenchantID`) are each shared by
  several items (up to 877), so they name no single entity and the origin stays
  `unknown`. Kind `dungeon` for bosses, otherwise `other`.
- Boss reference loot `drop:creature:<boss>:ref:<table>:<item>` (depth 1):
  p = 1 - (1 - p_roll * p_item)^rolls, occurrences combined as 1 - prod(1 - p).

### Loot tables (`loot_tables`)

The complete loot data without expansion: one `loot:<table>:<entry>` per template,
`table` kind, `source` (as for drops) and every row: `item` (raw client ID, which may
be absent from `items`) or `table` (a reference table of the catalog), optional
`probability` (absent = unknown, never 0), `min_count`, `max_count`, optional
`group`, `quest_only`, `conditional`. A reference stays one row, so the size is
linear in the source rows (229,646 rows instead of over a million expanded).

### Dungeons

`dungeon:<slug of the title>` (`dungeon:the-deadmines` joins the authored record),
map `instance:<MapID>`, title (authored first), entrance node or `unknown`, optional
`bosses` (entities), preparation quests = quests whose zone is the dungeon's area
(`unknown` without an area), rewards = boss drop acquisitions, access, travel edges
and checklist `unknown` (the authored pack keeps the checklist).

### Not mapped

`trainers.spells`, `spells.spells`, `talents.talents` (no content v2 type yet),
`fishing.*` except fishing loot, taxi costs. `summary.json` lists them.

## Measured world pack (1.60.1.70450, V2.8h)

| Collection | Count |
| --- | --- |
| items / equipment | 24,359 (19,227 Forever, 5,132 from 1.12 `item_template`) |
| acquisitions / acquisition_facts | 58,712 |
| quests / quest_facts / stages | 4,245 / 4,245 / 6,039 |
| nodes / node_facts | 8,800 |
| edges (32,265: 315 flight) | 32,265 |
| entities | 21,043 |
| loot_tables (rows) | 8,196 (240,722; every item row resolves to an item) |
| map_spaces / dungeons / evidence | 87 / 27 / 24 |

`catalog.json` input 152,853,500 bytes; canonical pack catalog about 367 MB
(`verify-pack` total 367,728,145 bytes). Conflicts: 5,132 items the Forever table
lacks (filled from 1.12), 2,801 value disagreements (mostly item class fields).
Drop origins now resolved for 18,922 game object drops (665 loot tables);
disenchant (37 tables) and reference tables stay `unknown`.

On the development machine (WSL2, release build): generator 4.8 s, 0.87 GiB;
`build-content-v2` 34 s, peak RSS 4.8 GiB; `verify-pack` 13 s, peak RSS 3.9 GiB.
Decoding holds the strict JSON value, a typed copy and a comparison value at once,
so memory is about 14 times the canonical size. Consumers that load the pack (the
desktop) pay the same.

## Limits

| Limit | Value | Rationale |
| --- | --- | --- |
| `contract::MAX_CATALOG_IDENTITIES` | 500,000 (was 250,000) | 186,665 measured, about 2.7x |
| `contract::MAX_CATALOG_EVIDENCE` | 250,000 (unchanged) | 27 measured |
| `content_v2::MAX_CONTENT_BYTES` | 576 MiB (was 256 MiB) | 273.5 MB canonical, about 2.2x |
| `ingest::pack::v2::MAX_CATALOG_BYTES` | 576 MiB (was 256 MiB) | same as the decoder |
| `ingest::pack::v2::MAX_CLOSURE_BYTES` | 1 GiB (unchanged) | build reads 114 MB, verify 274 MB |
| `content_v2::MAX_LOOT_ROWS` | 4,096 rows per table | largest table 1,056 |

`release::MAX_RECORDS` is 2,000,000 since the release gate task (V2.8e, ADR 0007),
so the whole-world pack and the authored pack fit under it with headroom.

## Application use (V2.8g)

**Activation.** The Content screen keeps the packs staged in the session. Two or more
staged packs of the selected context activate together (`activate_content_set`,
for example the authored slice pack and the world pack) as one composed revision
`set-<sha>`. A checkbox accepts inferred travel (ADR 0007 §5, §8): `activate_content_set`
with `accept_inferred_travel`. The choice is held in memory for exactly the active
revision (not persisted: the settings document is a versioned, validated schema that
recovery also reads, so a field there needs its own migration task); a restart, a plain
activation or a rollback returns to the strict default, and the user activates again.

**Travel on the world pack.** `travel::MAX_TRAVEL_NODES` (32,768) and
`MAX_TRAVEL_EDGES` (131,072) replace the old 1,024 bound (measured: 8,800 nodes and
32,265 edges; no uiMap holds more than 395 nodes or 1,638 edges, and walking edges
never leave their uiMap; 288 flight edges join them, so whole-graph search is cheap). `TravelGraph::prepare` validates
and assesses the graph once; `route` runs Dijkstra and stops when the target settles.
Measured on the real pack (release build, WSL2): load 1.4 s (JSON), prepare 52 ms,
each query 0.5 to 30 ms (12 hops, 1.08 Ms: 13 ms; 27 hops: 30 ms) against 52 to
106 ms for the one-shot `shortest_route`; peak RSS 2.1 GiB with the parsed catalog
(`cargo test --release -p azeroth-core --test travel_world -- --ignored --nocapture`
with `NAVZEROTH_WORLD_CATALOG`).

**Unknown requirements.** Under the accepting activation an inferred edge whose
requirement is unknown, unsupported or restricted (all 315 flight edges) is usable;
its hop carries `requirement_assumed: true` and the route is an estimate. Authored,
verified and estimated edges keep the strict rule. The planning report states the
assumption. Joint planning, route acquisition and dungeon preparation use the same
options as solo planning. Without the activation, generated inferred edges are left out
of the search and the report says so.

**Evidence.** A generated flight edge cites the documented taxi path and the inferred
duration; `inferred_edges` accepts evidence of `inferred` plus `publisher_documented` or
`verified_on_build`, never `cross_edition_unverified` or `limited_sample`.

**World browser.** Quest titles, givers and enders (resolved to entity or item
names), stages from the quest records, item names (`EquipmentFacts.name`), dungeon
bosses on entities and loot templates per source (rows: item or reference table,
chance (absent is unknown), counts, group, quest-only, conditional; at most 6 tables
per source and 40 rows per table are listed, with totals). Real pack: first loot page
109 ms (index build), later pages under 2 ms.

## Known limits (review of V2.8)

- Route search keeps a full edge path in every heap label (the label order breaks ties
  by path). The measured 0.5 to 30 ms are for reachable targets; an unreachable target
  searches the whole graph (up to 131,072 edges) and its memory grows with
  pushes times path length. Predecessor pointers would change the tie order and need
  their own equivalence tests.
- Activating the world pack composes members through typed values and a decode, so the
  desktop peak is a multiple of the 367 MB canonical size (3.9 to 4.8 GiB measured for
  the world pack alone); the merge uses linear `contains` per element.
- Quest `givers`/`enders` and `LootTable.source` are checked for uniqueness and shape
  but not resolved against entities.
- Evidence for wago `wowf` shards (build 1.60.1.70245) is stamped with the target
  build (1.60.1.70450) in its context; the shard build is only in the evidence notes.
