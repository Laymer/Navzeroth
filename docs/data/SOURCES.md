# Source registry

Every provider is registered here before its data is used (plan section 8.3).
Public access never establishes redistribution permission. "Not recorded"
means exactly that; it is not a claim either way.

The matching typed schema-version-1 sidecar is
[content/forever/sources.json](../../content/forever/sources.json). It registers
these four IDs for local pack preparation without supplying missing permissions.
Third-party redistribution remains Unknown; the owner's composite-only assertion
is Conditional and does not grant permission for the underlying works. All
licenses and verified revisions remain Unknown. Date-only retrieval/addition
records are not invented exact UTC timestamps. Blizzard's direct allowed fields,
transformations and retrieval are NotApplicable, matching the table's `None
directly`/`n/a`. Offline composite access has no authentication or conditional HTTP
requests; its unrecorded access method stays Unknown for the embedded sources.
All four records are SecondaryInference, including publisher artwork and the manual
composite: neither is promoted to publisher documentation or game observation.
Local packs remain `distribution_eligible=false`; this sidecar introduces no source
permission or build-specific verification.

| Source ID | Publisher | Location | Edition/build coverage | Access | Licence / attribution / redistribution / retention | Allowed fields and transformations | Update schedule | Last retrieval | Offline | Record class |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| `src-wowforevermap-ek-terrain-2026-10-07` | WoWForeverMap.com | Rendered into the owner's composite poster | WoW Forever beta 1.60.1.70205 (poster footer) | Via the composite only | Licence not recorded; attribution kept verbatim in NOTICE; redistribution rests on the owner's composite statement; retained as long as that statement stands | Terrain and zone boundaries as drawn; crop, downscale | None (static poster) | Poster dated 2026-10-07; retrieval date to be filled by the owner | Yes | Third-party render (secondary) |
| `src-endgametools-ek-flight-2026-10-07` | EndgameTools.com | Rendered into the owner's composite poster | WoW Forever beta 1.60.1.70205 (poster footer) | Via the composite only | Licence not recorded; attribution kept verbatim in NOTICE; redistribution rests on the owner's composite statement | Flight-master names, faction access, drawn routes; pin tracing in image space | None (static poster) | As above | Yes | Third-party records (secondary) |
| `src-blizzard-map-artwork` | Blizzard Entertainment | Underlying map artwork inside the renders | WoW Forever | Via the composite only | Copyright Blizzard Entertainment (poster footer); licence not recorded; no claim of permission beyond the owner's composite statement | None directly | n/a | n/a | Yes | Publisher artwork |
| `src-owner-ek-composite-2026-10-07` | Repository owner (Igor K) | `content/forever/atlas/eastern-kingdoms/source/` | WoW Forever beta 1.60.1.70205 | Git LFS | Redistribution of this composite asserted by the owner on 2026-10-12 (docs/data/atlas-eastern-kingdoms.md); not covered by the repository BSD-3 licence | Crop panels, downscale overview, trace pins; roads are manual tracing, not level-safe routes | None | 2026-10-12 (added to the repository) | Yes | Manual composite of the sources above |

## Generator sources (2026-10-19)

[ADR 0006 §14](../decisions/0006-v1-program.md) admits the data that the generators
in `data-generators/` download: Wowhead Classic/Forever (`wowhead-classic`,
`wowhead-forever`), CMaNGOS classic-db (`cmangos-classic-db`, GPL v3) and wago.tools
DB2 exports (`wago-tools-wowf`, `wago-tools-classic-era`; Blizzard client data), plus
the TALOD generator scripts under NotInTheBand's written permission
(`talod-generators`). The owner explicitly accepts the third-party terms; the
admission registry records that as licence `owner_accepted`. These entries live in
the [admission registry](../sources/source-admission-v1.json) (see
[source admission](../sources/source-admission-v1.md)), not in the pack sidecar above.
Retrieval is opt-in and rate-limited, run by the generators only; Wowhead `/forever`
list pages answer 403 to scripts and are not fetched. Output is
[navzeroth-world-data v1](WORLD_DATA.md), edition-tagged and never relabelled.
Credits: [data-generators/NOTICE.md](../../data-generators/NOTICE.md).
