# Data generators: credits and terms

The scripts in this folder download public game reference data and write
[navzeroth-world-data v1](../docs/data/WORLD_DATA.md) shards. The repository's
BSD-3-Clause licence covers Navzeroth's own code only. It does not relicense the
TALOD scripts or any downloaded data. The admission decision is
[ADR 0006 §14](../docs/decisions/0006-v1-program.md); the registry entries are in
[source-admission-v1.json](../docs/sources/source-admission-v1.json).

Navzeroth is not affiliated with or endorsed by TALOD or its author, Wowhead or
Fanbyte, the CMaNGOS project, wago.tools or Blizzard Entertainment.

## TALOD generators

Data generators based on work by NotInTheBand (TALOD), used with permission.

`TALOD-data-generators/tools/gen_enhancements.py`, `gen_professions.py` and
`gen_fishing.py` (and Navzeroth's modifications of them) are used under
NotInTheBand's written permission of 18 October 2026,
[TALOD-data-generators/PERMISSION.md](TALOD-data-generators/PERMISSION.md)
(SHA-256 `08abb624e55bd7383eefca9d14a02f5309c44c0012d865a78d01fc45736a25f3`).
Its conditions, in short:

- keep the credit above to the original author;
- don't present the generators as official TALOD tools or imply that TALOD or its
  author endorses Navzeroth;
- Navzeroth only: use in another project needs separate permission;
- non-commercial: selling the generators or their output, or putting them behind
  a paywall, needs separate permission;
- the permission covers these three scripts only, not the rest of TALOD (addon,
  Lua code, art, branding, documentation, other tools) and not the data the
  scripts download;
- the scripts come as is, without warranty; the author may revoke the permission
  for future use by written notice if its conditions are broken.

## Wowhead

Data from Wowhead (<https://www.wowhead.com/classic>, <https://www.wowhead.com/forever>
and `nether.wowhead.com` tooltips). Wowhead is operated by Fanbyte; its
[terms](https://corp.fanbyte.com/legal/terms) restrict automated extraction and
redistribution. The owner of Navzeroth has read those terms and explicitly accepts
that risk for this project (ADR 0006 §14). Retrieval is opt-in, cached and
rate-limited. Wowhead `/forever` list pages refuse scripts (HTTP 403); that block
is not bypassed.

## CMaNGOS classic-db

Data from the CMaNGOS Classic-DB (<https://github.com/cmangos/classic-db>), © the
CMaNGOS Classic-DB contributors, released under the GNU General Public License
version 3 (its `LICENSE.md`). Its `COPYRIGHT.md` states that World of Warcraft
content and materials are trademarks and copyrights of Blizzard or its licensors
and are not part of the GPL; the owner accepts that reservation's risk (ADR 0006
§14). Derived shards keep the dump name and hash, origin `vanilla_1_12` and
confidence `cross_edition_unverified`. Redistributing derived data keeps the GPL v3
notice and the corresponding source (these generators).

## wago.tools

Data from wago.tools DB2 CSV exports (<https://wago.tools>) for the products `wowf`
(WoW Forever 1.60.1) and `wow_classic_era`. The tables are Blizzard Entertainment
client data: World of Warcraft and Blizzard Entertainment are trademarks or
registered trademarks of Blizzard Entertainment, Inc. The owner accepts the
wago.tools and Blizzard terms risk for this project (ADR 0006 §14). Every shard
keeps its product and build.

## Other cited references

`gen_fishing.py` contains a few hand-entered fish schedules whose `source` text
cites Warcraft Wiki (<https://warcraft.wiki.gg>) and Blizzard Watch pages. Those
pages are not downloaded.
