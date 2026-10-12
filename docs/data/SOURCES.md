# Source registry

Every provider is registered here before its data is used (plan section 8.3).
Public access never establishes redistribution permission. "Not recorded"
means exactly that; it is not a claim either way.

| Source ID | Publisher | Location | Edition/build coverage | Access | Licence / attribution / redistribution / retention | Allowed fields and transformations | Update schedule | Last retrieval | Offline | Record class |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| `src-wowforevermap-ek-terrain-2026-10-07` | WoWForeverMap.com | Rendered into the owner's composite poster | WoW Forever beta 1.60.1.70205 (poster footer) | Via the composite only | Licence not recorded; attribution kept verbatim in NOTICE; redistribution rests on the owner's composite statement; retained as long as that statement stands | Terrain and zone boundaries as drawn; crop, downscale | None (static poster) | Poster dated 2026-10-07; retrieval date to be filled by the owner | Yes | Third-party render (secondary) |
| `src-endgametools-ek-flight-2026-10-07` | EndgameTools.com | Rendered into the owner's composite poster | WoW Forever beta 1.60.1.70205 (poster footer) | Via the composite only | Licence not recorded; attribution kept verbatim in NOTICE; redistribution rests on the owner's composite statement | Flight-master names, faction access, drawn routes; pin tracing in image space | None (static poster) | As above | Yes | Third-party records (secondary) |
| `src-blizzard-map-artwork` | Blizzard Entertainment | Underlying map artwork inside the renders | WoW Forever | Via the composite only | Copyright Blizzard Entertainment (poster footer); licence not recorded; no claim of permission beyond the owner's composite statement | None directly | n/a | n/a | Yes | Publisher artwork |
| `src-owner-ek-composite-2026-10-07` | Repository owner (Igor K) | `content/forever/atlas/eastern-kingdoms/source/` | WoW Forever beta 1.60.1.70205 | Git LFS | Redistribution of this composite asserted by the owner on 2026-10-12 (docs/data/atlas-eastern-kingdoms.md); not covered by the repository BSD-3 licence | Crop panels, downscale overview, trace pins; roads are manual tracing, not level-safe routes | None | 2026-10-12 (added to the repository) | Yes | Manual composite of the sources above |
