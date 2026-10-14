# Public-source audit for the v1 content inventory

Audit date: 2026-10-14. Owner: Codex `/root/public_sources`. Project baseline:
`bb3782cbac13b5fb3e6ca8196278ba7fcc427474`. Requested target: Forever
`1.60.1.70374`, Interface `16001`. This is source-discovery evidence, not a
release-acceptance record. The coordinator owns the versioned release inventory
and evidence contract; this audit does not replace them.

Neither audited source is accepted for a shipped Forever content pack. QuestieDB
offers useful candidate data, but the inspected revision does not establish a
data redistribution grant or complete 70374 coverage. CMaNGOS is a historical
comparison with separate copyright exclusions. Unknown target inventories remain
release blockers, even if an adapter can decode every available source row.

The accompanying [source audit JSON](v1-source-audit.json) contains immutable
identities, byte counts, SHA-256 values for fully retrieved files, Git blob IDs
for the broader input inventory, field-index maps and mandatory coverage units.
It contains no copied gameplay tables and is not an activatable dataset.

## QuestieDB, audited first

Pinned commit:
[`9bc909a464ce0af80af83ce8a4f959b5f911fe88`](https://github.com/Questie/QuestieDB/tree/9bc909a464ce0af80af83ce8a4f959b5f911fe88),
committed 2026-10-14T05:34:47Z; Git tree
`56f5b3a4b70b50b877b2ca1de1be9b42099fd122`. The GitHub recursive-tree response
contains 694 entries and reports `truncated=false`.

[PROVENANCE.md](https://github.com/Questie/QuestieDB/blob/9bc909a464ce0af80af83ce8a4f959b5f911fe88/PROVENANCE.md)
identifies Questie ancestry and the historical migration reference
`Questie/Questie@454b9d072965ee8f1a881429260fcf1fac8d60f7`; it is not a promise
that current files equal that historical revision. The complete pinned tree has
no LICENSE, COPYING or COPYRIGHT filename. Its
[third-party notices](https://github.com/Questie/QuestieDB/blob/9bc909a464ce0af80af83ce8a4f959b5f911fe88/tools/lua-binary/THIRD_PARTY_NOTICES.txt)
identify bundled Lua interpreters, not a gameplay-data license. The historical
Questie tree has licenses for specific libraries/icons but no root data license
found by the same filename search. No general grant was found in the inspected
README/provenance material. This is an unresolved permission finding, not a
claim that no permission could exist elsewhere. Keep all derived data excluded
from public assets until the applicable grant and attribution obligations are
recorded. Do not apply Navzeroth's BSD license to upstream-derived records.

The [Forever data history](https://github.com/Questie/QuestieDB/blob/9bc909a464ce0af80af83ce8a4f959b5f911fe88/docs/forever-data.md)
records initial coordinate migration from Era `1.15.9.69722` to Forever
`1.60.1.69893` and later map-support additions from `1.60.1.70291`.
Coordinate conversion assumes unchanged world positions; it does not discover
moved/new content. Compatibility maps, unresolved areas and inherited support
remain separately qualified. Those earlier builds must remain attached to their
evidence. They do not verify the requested 70374 baseline.

The [generated delta-base documentation](https://github.com/Questie/QuestieDB/blob/9bc909a464ce0af80af83ce8a4f959b5f911fe88/docs/forever-delta-base.md)
describes additional candidate relationships, levels, categories, restrictions
and selected fields from cached source evidence, including Wowhead references.
It withholds ambiguous evidence and some large item-source lists, and explicitly
leaves gameplay gaps. Its linked
`src/corrections/Forever/generated/provenance.json` is absent from the pinned
public tree. The corresponding local reports/cache are not public inputs in this
audit. A secondhand cached-source reference is not a redistribution grant or a
reproducible completeness proof. No Wowhead scraping was performed.

The [client acceptance section](https://github.com/Questie/QuestieDB/blob/9bc909a464ce0af80af83ce8a4f959b5f911fe88/docs/forever.md#client-support-and-acceptance)
also leaves Forever source-loader and placement checks pending. Upstream offline
test reports are attributed upstream results; this task did not run those tests
or adopt them as Navzeroth live evidence.

### Source inventory and correction dependencies

The following counts are **raw source ID-line counts**, measured on the complete
hash-verified bytes. Each file uses one column-zero `[decimal ID] =` line per
raw record, and no duplicate IDs occurred under that count. This is a lexical
inventory, not a Lua parse, correction-composed count, reference-validation pass,
availability finding or denominator for all Forever content.

| Raw file under `data/Forever/` | Bytes | Distinct ID lines |
| --- | ---: | ---: |
| `foreverQuestDB.lua` | 1,057,548 | 4,244 |
| `foreverNpcDB.lua` | 2,033,176 | 10,119 |
| `foreverItemDB.lua` | 2,181,706 | 14,889 |
| `foreverObjectDB.lua` | 1,011,387 | 6,645 |

The [configuration](https://github.com/Questie/QuestieDB/blob/9bc909a464ce0af80af83ce8a4f959b5f911fe88/src/config.lua)
selects four raw tables, eighteen owned Forever correction providers and nine
Forever support files, plus shared schemas, enum definitions and derived passes.
The order is raw tables, six legacy providers, four generated delta providers,
four generated trace providers, then four authored providers. Dynamic providers
and derived transformations also affect effective values. The public tree pins
these inputs; only the files marked `full_bytes_verified` in the JSON received
complete byte retrieval and SHA-256 verification in this task. Unread providers
must not be described as audited literal-only content.

The raw form is a deferred Lua string, for example the exact wrapper
`QuestieDB.questData = [[return {`. Parse that table as data. Do not invoke
`QuestieLoader`, `loadstring`, `dofile`, the upstream generator or its validator.
Corrections use symbolic field/zone/race/class names, arithmetic sums, positional
`nil` holes and table `_add`/`_remove` operations. Some files also contain module
calls, side effects or runtime branches. Static means independent of character
state; it does not mean safe to execute or supported by a literal parser.

[Schema metadata](https://github.com/Questie/QuestieDB/tree/9bc909a464ce0af80af83ce8a4f959b5f911fe88/src/meta)
has 36 quest, 15 NPC, 16 item and 7 object fields. Getter normalization can turn
absent numbers into zero. Read literal presence instead. Preserve explicit zero,
false, absent/nil and empty tables independently. Raw race masks include values
above `u32`, including `4294967373` and `8589934770`; never truncate them.

### Accepted mapping design, pending data acceptance

These are source-to-staging mappings. They authorize adapter engineering, not
source registration as permitted content or promotion to playable records.

| Source fields | Destination and validation |
| --- | --- |
| Quest 1, 4–7, 18–20, 30–32, 35 | Preserve name/levels/restriction facts and missingness. Translate to `QuestDefinition.eligibility` only when every relevant condition and exact-build enum is understood; retain unsupported restrictions as blockers. |
| Quest 12–16, 22, 25, 27–28, 33–34, 36 | Preserve prerequisite, exclusion, breadcrumb and active/completed distinctions. Construct predicates only with reviewed semantics; signed IDs and group-vs-alternative relationships cannot become a flat prerequisite list. Check cycles and references. |
| Quest 2–3, 8–11, 17, 21, 23–24, 26, 29 | Stage giver/finisher/objective/source-item/category/flag/reputation/extra-objective facts. Positive AreaID and negative QuestSort are different identities. Text or flags alone cannot invent stages, objective counts, sharing, timed/escort behavior or item-reward choices. |
| NPC 1, 4–15; Object 1–7 | Stage entity identity, levels, spawns, waypoints, quest links and faction/service clues. NPC 2–3 are deprecated compatibility health placeholders. These contracts have no generic NPC/object collection; retain source staging instead of inventing `TravelNode` or `Objective` semantics. |
| Item 1–6, 9–16 | Stage item identity, levels, categories, source relationships, start/related quests and taught spell. Candidate quest/vendor/drop links may support later `AcquisitionSource` records with unknown price/probability/conditions. They do not establish guaranteed rewards, quantities, stock, loot entitlement or equip eligibility. |
| Item 7–8 | Preserve original flags/food category with exact schema meaning; do not map food or class to an equipment slot. The source lacks the full slot, handedness, stats and effects required for reliable `ItemDefinition` scoring. |
| Forever zone/support mappings and spawn coordinates | Candidate `MapSpace`/`Position` evidence only after AreaID/UiMapID/floor/frame validation. Percent coordinates need explicit unit conversion; sentinels and inherited dungeon aliases are not outdoor points. Do not transform already converted coordinates again. |
| XP/drop/dungeon support | Versioned secondary facts with their original provenance. Presence does not prove active 70374 content, drop probability, a navigable entrance, valid player route, encounter advice or a preparation model. |

The existing `QuestDefinition` requires nonempty stages, and `ItemDefinition`
requires a slot. The adapter must not fabricate either to satisfy validation.
Keep partial source rows in an internal, bounded staging/report type. Reuse
`DataContext`, observation states, evidence/source policies and the current
content validation pipeline when a complete domain mapping becomes justified.
Do not change exchange schema v1 solely to fit these incomplete imports.

## CMaNGOS Classic DB historical comparison

Pinned commit:
[`ec4f596146be6467ea93c57397858e329e2db852`](https://github.com/cmangos/classic-db/tree/ec4f596146be6467ea93c57397858e329e2db852),
committed 2026-09-22T18:45:58Z; Git tree
`f8bfbbe1d155c560fc954db36c2c835fb2df67b0`. Its complete API tree contains 432
entries, including 387 SQL files under `Updates/`. The base archive
`Full_DB/ClassicDB_1_12_1_z2815.sql.gz` is 12,959,882 bytes. Its Git blob is pinned
in the JSON; the archive was not downloaded or decompressed in this audit.
Therefore there are no claimed SQL row counts or complete table mappings.

The [README](https://github.com/cmangos/classic-db/blob/ec4f596146be6467ea93c57397858e329e2db852/README.md)
targets patch 1.12 clients, including builds 5875/6005/6141. It specifies GPLv3
while separately identifying copyrighted material. The
[LICENSE](https://github.com/cmangos/classic-db/blob/ec4f596146be6467ea93c57397858e329e2db852/LICENSE.md)
and [COPYRIGHT](https://github.com/cmangos/classic-db/blob/ec4f596146be6467ea93c57397858e329e2db852/COPYRIGHT.md)
must be assessed together: the latter reserves Blizzard material and describes
the project's intended fair-use context. This audit does not establish rights
for Navzeroth's redistribution of those excluded materials. Historical candidate
facts remain `cross_edition_unverified`; never replace a Forever fact merely
because the Classic row appears more complete.

If a defined subset is later accepted, preserve applicable license/copyright
notices, author attribution, immutable original source and transformation inputs
and provide required corresponding source with the distributed derivative.
Record any exceptions by file/field. Do not combine a GPL label with a claim that
all content is unrestricted or project-owned BSD.

The inspected [precision update](https://github.com/cmangos/classic-db/blob/ec4f596146be6467ea93c57397858e329e2db852/Updates/0000_precision_fix.sql)
contains `ALTER TABLE`, and the
[instance documentation](https://github.com/cmangos/classic-db/blob/ec4f596146be6467ea93c57397858e329e2db852/Updates/Instances/README.txt)
describes later override order and variable-based IDs. A literal INSERT reader
cannot claim to reproduce the current database by ignoring update programs.
Keep an explicit base-plus-update inventory; reject unimplemented statements,
variables, expressions and server scripts with coverage impact. Never run
`InstallFullDB.sh`, feed SQL into SQLite/MySQL, or treat NPC movement paths as
validated player roads. A later comparison task can inspect documented quest,
creature, object, item, loot and vendor tables after bounded archive/schema
inspection establishes their actual column contracts.

## Full-content inventory and release blockers

At the project baseline, the real `forever-atlas-r1` payload is scoped to
`1.60.1.70205`: 0 quests, items, acquisitions, travel nodes, edges, objectives or
rules; 1 map space, 1 image, 0 embeddings and 6 evidence records. It is not 70374
content. The existing source registry leaves image redistribution unresolved.
Synthetic demo records and role weights do not increase real-content coverage.

The following mandatory units must appear independently in the coordinator's
inventory. Every complete target denominator is currently unknown. The named
focused units are starting partitions from the implementation plan, not the
complete frozen-world list.

| Unit family | Required enumeration and acceptance |
| --- | --- |
| Quests | Every available quest/stage by region, faction, class/race, level and reset/event context; giver/finisher links, objectives, prerequisites, choices and reward records. Start with Westfall, Redridge, Duskwood and Wetlands, then explicitly enumerate remaining 70374 regions. |
| NPCs/services | Quest participants, vendors, repair/training/travel services; exact identities, positions, access, service capabilities and build evidence. A flag is a clue pending interpretation. |
| Objects | Quest/objective/interaction objects, spawn frames and item/quest links; no invented interaction or completion behavior. |
| Equipment | Every supported equippable variant/slot/level range, proficiency, uniqueness, handedness, stats, effects and restrictions. Candidate item names alone do not count. |
| Acquisition | Quest rewards including choices, vendors/stock/prices, dungeon drops/loot rules, crafting/professions and other available sources; references, restrictions, costs and uncertainty. |
| Geography/atlas | Actual 70374 continents, regions, outdoor/instance map spaces and floors, transformations, imagery permission, positions and coverage boundaries. Map symbols do not establish playability. |
| Travel | Individually directed walking/flight/boat/tram/return and supported teleport links, endpoints, geometry, unlock/access/cooldown constraints, measured or declared time uncertainty and sensible stopping points. Stormwind/Ironforge links need separate evidence. |
| Dungeons/preparation | Every available dungeon and any in-scope raid/instance workflow: entrance/path/access, quests, rewards, per-participant eligibility, role/consumable checks and permitted encounter notes. Deadmines is the initial partition; enumerate the remainder before full-content claims. |
| Classes/roles | Every playable class/race/faction/level/scenario combination with documented model provenance and representative gear/planning cases. Initially paladin duo, paladin tank, mage duo and mage damage; neither audited source supplies accepted weights. |
| Parties | Supported group sizes and class/role combinations; separate prerequisites, sharing versus assistance, divergent progress, stale/missing participants, agreement and fairness. A general algorithm is not evidence for every game mechanic. |
| Reminders | Every declared reminder rule and its required service, observation, predicate, evidence, cooldown/dismissal/expiry and fallback behavior. No evidence-free generic training/repair/consumable assertions. |
| Sessions/end-to-end | Real progression and timing coverage for declared scenarios, including the required two-character plan/opportunity/advancement/deviation/review/next-plan flow. Preserve recorded-client/environment evidence separately from source fixtures. |

Optional official API, addon party messaging, combat-log analysis and hosting
deferrals must remain explicit; they cannot excuse missing mandatory local
party, content or acceptance units. Freeze a target inventory only after an
authoritative or otherwise permitted exact-build catalog identifies available
content. When that catalog is incomplete, record an unknown denominator, not
zero missing records or a percentage based on the imported subset.

## Dependency-ready adapter tasks

1. Build a deterministic source-audit/staging adapter for the exact four raw
   QuestieDB wrappers. Enforce byte/token/string/depth/record limits, UTF-8 and
   finite-number checks; reject duplicate IDs/keys, unknown enum fields,
   executable constructs, mixed editions and unexpected wrappers. Preserve
   ordered arrays and nil holes. Emit byte hashes, parsed counts, unsupported
   inputs and missing references; emit no activatable dataset while corrections,
   permission or build acceptance are incomplete.
2. Independently inventory all eighteen providers plus enum/derived/support
   dependencies. A restricted interpreter may be designed for explicitly
   allowed literal tables and named constant expressions; it must never execute
   Lua. Version the allowlist, implement exact table-operation semantics and
   precedence, and report every unsupported function/branch/pass. An unsupported
   provider can affect the whole entity family when affected IDs are unknown.
   Do not silently apply only the easy corrections or erase raw records when a
   later row is withheld.
3. Add conflict/distribution checks against the coordinator's coverage/evidence
   contract. Pin source revision plus file hash and mapping version; keep
   contradictory records and original contexts. Missing source permission,
   required corrections, target denominator, mandatory records or exact-build
   evidence must prevent stable publication. Partial audit output is useful
   even when distribution remains false.
4. Use synthetic fixtures to test malformed/oversized data, unknown fields,
   duplicate IDs/keys, nil-vs-empty-vs-zero, wide race masks, edition mixing,
   correction omissions/ordering, conflicts and deterministic output. Real
   upstream byte counts are a performance input, not a coverage pass. Complete
   a CMaNGOS parser only if a concrete permitted historical-comparison need
   remains after QuestieDB staging; do not expand into a general SQL engine.

No game-content bytes, copied runtime code, source licenses or source registries
were added to shipped assets by this audit. Source eligibility, exact-build
completeness, complete correction composition and all unperformed client checks
remain unverified or blocked as recorded above.
