# Build capabilities

Capability evidence per plan section 22.4. Status values: verified available,
verified unavailable, restricted, failed, untested. An "untested" row records
what the reference content assumes, not a client fact.

## WoW Forever beta 1.60.1.70205 (interface 16001, assumed)

| Field | Value |
| --- | --- |
| Date | 2026-10-12 |
| Operator | none (no client session) |
| Game edition/build/interface | wow_forever / 1.60.1.70205 (poster footer) / 16001 (community exports, plan section 3.2) |
| OS/locale | not tested |
| Addon revision | none exists (M2) |
| Application revision | this branch |
| Documentation/source | docs/data/atlas-eastern-kingdoms.md; plan section 3.2 |

| Capability | Status | Notes | Affected tasks | Fallback |
| --- | --- | --- | --- | --- |
| Build and interface strings as recorded above | untested | The dataset `forever-atlas` is keyed to this tuple. A verified difference is a new dataset revision with evidence, never an in-place edit. | M6.5, M2, M0.4 | Snapshots from another build project only when that build is listed in `applicable_builds` with evidence |
| Continent-level normalized player position (`C_Map` style) | untested | Preferred position source (owner decision). | M0.4, M2.1, M6.5 | Zone-level position plus exported zone rectangle |
| Zone rectangle inside the continent map (`GetMapRectOnMap` style) | untested | Needed for the zone-level fallback through `map_embeddings`. | M0.4, M2.1, M6.5 | Marker not placed; schematic position only |
| uiMapID values (continent Eastern Kingdoms assumed `1415`) | untested | Classic-era convention used as a placeholder `map_id`. | M6.5 | `no_image_for_map` gap with an explicit "map identity unverified" explanation |
| Position axis orientation (top-left origin, x east, y south) | untested | Documented convention in core. | M6.5 | Calibration residuals would expose a flipped axis |
