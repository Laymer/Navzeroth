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

## v0.2.0 clock verification boundary

The level-12 report does not establish GetServerTime or GetTime behavior. v0.2.0
offers an explicit guarded clock probe and an operator confirmation after trusted
UTC comparison. Missing/invalid values remain uncertain; observed UTC is distinct
from trusted UTC. Stability guards detect reversals/resets/jumps and reload clears
trust. Actual client probe/confirmation, restrictions and displayed capture age
remain UNVERIFIED until reported under recorded conditions.

### Wave-3 reported clock follow-up

The user-supplied delivery plan reports successful trusted-age display after clock
confirmation and uncertain-age display after reload for the previously recorded
1.60.1.70338 / interface 16001 client. This establishes two limited user-reported
flow results. No raw exports, inspected addon revision, exact displayed values,
combat/zone conditions or broader restriction evidence were supplied here.
Earlier unverified statements remain historical; other bindings remain unverified.

## Client 1.60.1.70425 development admission

On 2026-10-16 the owner's selected installation's `.build.info` reported product
`wow_classic_beta`, version `1.60.1.70425` (see
[LOCAL_DATA.md](../data/LOCAL_DATA.md)). That is **installation metadata only, with no
addon-run evidence**: no `GetBuildInfo` result, probe export, capture or live check on
70425 has been recorded. `Capabilities.isCaptureTarget` and `Capabilities.admission`
therefore admit the exact tuple `1.60.1` / `70425` / `16001` for **development capture
only**, labelled `development_unverified` in the addon and its window.

- The admission enables the same guarded experimental reads as the other admitted
  builds; it verifies no capability. Every probe row stays `untested` and no status in
  this document is promoted.
- Every export and snapshot records the exact build string, so 70425 data is never
  relabelled 70374 and the two are never mixed silently.
- The 1.0 release target and all LIVE acceptance rows stay bound to 1.60.1.70374.
  `azeroth-cli capture` keeps reporting `target_tuple_matches=false` for 70425, which is
  correct. The guided acceptance kit still requires 70374.
- Neighbouring builds (70424, 70426) are rejected; only exact tuples are admitted.

## Client 1.60.1.70450 development admission

On 2026-10-17 the owner's installation `.build.info` and Blizzard's public version list
(`wow_classic_beta`, build config `84bea4233331e390fda8724c6f91ffa5`, issue #182; see
[BUILD_WATCH.md](BUILD_WATCH.md#acknowledged-builds)) reported `1.60.1.70450`. The owner
reported its interface number as `16001`. No `GetBuildInfo` result, probe export,
capture or live check on 70450 has been recorded, so the addon admits the exact tuple
`1.60.1` / `70450` / `16001` exactly like 70425: **development capture only**, labelled
`development_unverified`. Every rule of the 70425 section applies: no capability is
promoted, exports keep the exact build, it is never the 1.0.0 target (ADR 0006), and
neighbouring builds (70449, 70451) stay unadmitted.
