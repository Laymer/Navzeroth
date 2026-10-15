# ForeverDB: preferred source candidate

Owner: Codex `/root`; reviewed 2026-10-15 at baseline `32c5e2d`.
The user selected **ForeverDB instead of Wowhead**. The earlier Wowhead request
is superseded and remains unsent. This review selects the next provider; it does
not certify a complete distributable 70374 catalog.

## Retrieved source

The publisher links to [CurseForge](https://www.curseforge.com/wow/addons/foreverdb)
and [Wago](https://addons.wago.io/addons/kGr9ReKy). Both advertise GPLv3. The ordinary
CurseForge download page returned HTTP 403 during this review. The Wago **Beta**
release list supplied the normal public download successfully; no account,
uploader, alternate identity or access-control workaround was used.

- Archive: `ForeverDB-0.34.0-store.zip`, 3,680,905 bytes.
- SHA-256: `197af97c02444077e8edd7883eeeb6328828cf9b2f06c540d5708d89894a17d0`.
- 176 archive members, 11,215,661 uncompressed bytes.
- `ForeverDB/Data/Meta.lua`: build `1.60.1.70338`, data revision
  `2026-10-12.1bcd12db`, addon `0.34.0`, cap `30`.
- TOC interface `16001`; Helper version `1.3.3`.

The publisher's [research metadata](https://foreverdb.net/llms.txt) independently
names 70338 with an extraction date of 2026-10-14. That changing website metadata
is distinct from the pinned archive's data revision. Neither verifies 70374.
The [member manifest](foreverdb-0.34.0-members.json) records exact file hashes and
sizes. [Review metadata](foreverdb-review.json) pins the researched pages by hash.
Raw downloaded Lua, executable code, images and fonts are not committed here.

## Candidate mapping

| Navzeroth domain | Archive material to evaluate | Required distinction |
| --- | --- | --- |
| Quests | `QuestForever.lua`, `QuestClassic.lua`, `QuestGuide.lua`, `Quests.lua`, `QuestXP.lua` | Client facts, observations and Classic inference; guide string references and unknown positions |
| Equipment | `Items.lua`, `Gear.lua`, `GearFinder.lua` | Tooltip facts versus actual stat/eligibility records; model assumptions |
| Acquisition | `GearSources.lua`, `Loot.lua`, `NPCs.lua`, `Recipes.lua` | Classic/observed drops, estimated rates, hidden versus revealed facts |
| Geography | `Zones.lua`, `RouteMaps.lua`, `DungeonMaps.lua` | Coordinate calibration and imagery rights separate from numeric facts |
| Travel | `Travel.lua`, `Routes_*.lua` | Client links, estimated flight times and Classic-derived danger; packed tables require an explicit inert decoder |
| Dungeons | `Dungeons.lua`, Helper `Data/Dungeons.lua`, `Data/Classic.lua` | Forever records versus Classic encounters, loot and quest details |
| Class/roles | `Gear.lua`, `Talents.lua`, `Spells.lua` | Published weights/models with evidence and limitations, not measured truth |
| Party | Helper dungeon/group declarations | Code/features do not establish complete supported sizes or workflows |
| Reminders | Data supporting individual rules | Each rule still needs a documented condition, provenance and suppression behavior |

These are mapping leads, not imported records or completeness denominators.
For example, `ns.qfQuests` and `ns.qcQuests` are different origin strata. Compact
pipe-delimited records, string dictionaries and packed tables need explicit
bounded transformations. Never run Lua to unpack them. Preserve zeros, unknown
coordinates, unavailable values, source labels and estimates independently.

## Admission still required

The [publisher's licence listing](https://www.curseforge.com/wow/addons/foreverdb/license)
provides an advertised compatible copyleft basis. `QuestClassic.lua`, `QuestXP.lua`
and Helper `Data/Classic.lua` include explicit GPL notices for CMaNGOS-derived
material. The archive has an OFL font notice but no root GPL licence text.
The [canonical GPLv3 text](licenses/GPL-3.0.txt) is now retained verbatim from the
[FSF source](https://www.gnu.org/licenses/gpl-3.0.txt), with a pinned hash in the
review metadata. No fresh permission is required for material already covered by
the GPL. Preserve copyright/attribution and licence notices; mark modifications
and their dates, apply the relevant same-licence terms to covered transformed
works, and deliver the applicable corresponding source in its preferred form for
modification. Several headers name absent generators, but that alone does not
establish which generator scripts or upstream inputs must be provided: assess
GPL section1's source-form distinction for each selected file. Missing root text
is not proof of an invalid grant. Exact covered file scope and compliant delivery
remain to be evidenced. No website or image licence is inferred.

The [publisher's provenance description](https://foreverdb.net/about) distinguishes
client extraction, player observations and CMaNGOS Classic fallback. Keep those
labels and the origin build when mapping to Navzeroth. Independently enumerate
the playable 70374 inventory and validate every required unit before claiming
coverage. Current source admission remains empty and release blockers remain.

The [robots policy](https://foreverdb.net/robots.txt) allows public-page research
but excludes `/api/`; no data API extraction was performed. The separately
authorized contact inquiry used the normal public contact form. Wago supplies expiring signed
download links. The existing automated admission URL subset rejects query URLs;
do not strip signatures or pretend these links are immutable retrieval URLs.
Use a reviewed manual-file acquisition declaration, or obtain a supported static
export/repository and then add a separately tested downloader if useful.

Read-only [quest staging](../data/FOREVERDB_STAGING.md) now verifies the selected
local member files and parses the supported namespace tables without executing
Lua or activating content. The pinned archive yielded 557 Forever-labelled and
3,491 Classic-derived source quest records, with 373 overlapping IDs; these are
source counts only. See the [result summary](foreverdb-staging-result.json).

Next: review corresponding source and specific data scope, admit a narrow pinned
file/field/transform list, implement an inert staging adapter, then compare it
against the independent target inventory. Do not install the upstream addon or
uploader as part of desktop import, and do not copy runtime telemetry or combat
workarounds into Navzeroth.


## Publisher inquiry

The prepared [source/export inquiry](permission-requests/foreverdb-source-request-unsent.md)
was sent through the public contact form on2026-10-15 with explicit user
authorization and public logging disabled. Server and visible confirmations
verified submission. No reply has been received; all admission and target
validation questions remain open.
