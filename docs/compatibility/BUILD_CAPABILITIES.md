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

## User-reported client 1.60.1.70338 / interface 16001 — 2026-10-13

Evidence: [sanitized report](../../fixtures/compatibility/forever-70338-user-diagnostic.json).
Operator: user in this session. Addon 0.1.0. Language: English; region-specific
locale code unknown. The report was copied from diagnostic_export in SavedVariables
after the instructed reload/logout sequence (specific transition not supplied).
GetBuildInfo returned version 1.60.1, build 70338 and interface 16001, and those
strings survived JSON encoding and retrieval. The edition remains the selected
addon target, not a GetBuildInfo observation.

One diagnostic SavedVariables retrieval is evidenced. General write timing,
snapshot persistence and restricted-context behavior remain unverified. UnitLevel,
inventory/item, quest and map function names were present; their values were not
read by that diagnostic and remain untested. CombatLogGetCurrentEventInfo was missing.

M2.1 adds an experimental, explicitly requested level-only capture for this tuple.
The guarded player-level read returns Known only after the actual call and shape
checks succeed; exceptions/invalid values stay Unknown and a missing getter is
Unsupported. This does not update the diagnostic catalogue to verified availability
for all game contexts. All other character fields remain unknown/absent. No real
snapshot result has yet been reported. The older dataset at 1.60.1.70205 is unchanged;
it is not silently treated as compatible with 70338.


### Manual snapshot follow-up — level 12

After the preview capture/import instructions, the user confirmed that the desktop
shows the correct level, **12**. This is one user-reported positive result for the
manual level capture, serialization/persistence, import and display flow. Preview
head: 6477f26; artifact run: 38387827829; implementation merged through PR #44 at
d5326c2. Installed files were not independently inspected. Combat state, zone,
and the exact reload/logout transition were not separately reported.

Level readability/serialization is verified by user report for this limited flow.
The earlier API-presence-only findings above remain the historical diagnostic
baseline. No other capability status is promoted. Clock trust and region-specific
locale remain unknown; equipment, quest, position and restricted-context checks
remain untested. This does not verify freshness or real planning.
