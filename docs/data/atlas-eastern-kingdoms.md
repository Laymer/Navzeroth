# Eastern Kingdoms travel atlas: rights and source record

Asset: `content/forever/atlas/eastern-kingdoms/source/Eastern-Kingdoms-Travel-Atlas.png`
SHA-256: `48ad4c6a36ca13343420d63328fc55d45c2733b06e95968ab983700690313228`
Size: 17,511,553 bytes, 4492 x 5385 pixels, 8-bit RGB PNG
Registry: `src-owner-ek-composite-2026-10-07` in [SOURCES.md](SOURCES.md)

## Owner statement (2026-10-12)
The repository owner selected committing this composite via Git LFS and
confirmed holding the rights to redistribute this composite poster in this
repository. The owner may amend this wording; the date of any amendment is
recorded here. This statement covers the composite only. It records no licence
for WoWForeverMap.com terrain, EndgameTools.com flight records or Blizzard
artwork; those remain "not recorded" in the registry, and the footer attribution
is kept verbatim in `content/forever/atlas/NOTICE` and in the application.

## What the poster shows
Two panels of the same terrain: 01 "Main roads & crossings" (gold walking
corridors, white dashed Deeprun Tram, labelled towns) and 02 "Flight network"
(blue Alliance, coral Horde, violet tower routes, 31 numbered flight masters;
34 faction records). Notes on the poster: 23 outdoor zones drawn including
Riverglades; walking lines simplify the main roads and are not level-safe
routes; the tram line is schematic; flight paths must be discovered in-game.
Footer: "Reference data: beta build 1.60.1.70205. Routes and access may change
during beta."

## Derived files
`prepare-atlas` crops each panel to a shared frame and writes a half-resolution
overview; hashes and dimensions are recorded in `atlas.generated.json` next to
the manifest. Derived PNGs are also LFS-tracked.

## Not claimed
- Routes drawn on the poster are not verified travel edges and are not
  level-safe.
- Flight masters and routes are reference geography, not a character's unlocks.
- No calibration between image pixels and in-game coordinates is verified. The
  provisional transform in the reference dataset has confidence `inferred`.
- Map and build identifiers in the reference dataset are placeholders until the
  M0.4 client checks exist (see docs/compatibility/BUILD_CAPABILITIES.md).

## Coverage
The poster draws 23 zones. Supported coverage for the focused release (plan
section 2.3) is Westfall, Redridge Mountains, Duskwood and Wetlands with
Stormwind/Ironforge links. Imagery outside that set is browsable reference
geography only and is labelled as outside supported coverage.
