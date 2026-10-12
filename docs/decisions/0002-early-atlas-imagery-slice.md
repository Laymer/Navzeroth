# ADR 0002: early atlas imagery slice with a snapshot location marker

Date: 2026-10-12
Status: accepted by the owner during planning
Tasks: M6.5 (early slice), M6.1, M6.2, M6.4, M6.6 (partial), M10.1, M10.3,
M10.4, M12.1; prerequisite M1.4; M6.3 not addressed

## Context
The owner supplied a two-panel Eastern Kingdoms travel atlas poster (terrain
with traced roads, and the same terrain with the flight network and a 31-entry
flight-master index) for WoW Forever beta build 1.60.1.70205, and asked for it
to become the interactive map base with a marker for the player's current
location. The plan schedules imagery under M6 (after M3 and M4) and the desktop
reference overlay under M10 (after M9). No desktop shell (M1.4), content
directory, image tooling or Git LFS configuration exists yet.

## Options considered
Wait for M3/M4 before any imagery; ship a schematic-only atlas first; or pull
an explicitly scoped imagery slice forward. The owner selected the early slice.

## Decision
- The poster is committed under Git LFS in `content/forever/atlas/` with a
  path-scoped NOTICE; the repository BSD-3 licence does not cover it. The owner
  states redistribution rights for this composite (docs/data record). Third
  party licences are recorded as "not recorded", never inferred.
- Additive schema-version-1 dataset members: `map_images` (layers with hashes,
  image-space pins, a calibration observation with evidence, explicit
  cross-build applicability) and `map_embeddings` (a child map's rectangle in
  its parent). Flight masters are image-space pins; `nodes`/`edges` stay empty
  until positions are measured.
- `MapImage.applicable_builds` is an explicit, evidence-backed exception to the
  strict `DataContext` equality rule: a position from a listed build of the
  same edition may be projected onto the image with a visible notice. Anything
  else is a `build_mismatch` gap. Embeddings require exact context equality.
- Projection is a pure core function returning explicit gaps, never a guessed
  marker. The desktop marker is always the last imported snapshot position with
  its capture age; it is never called live and no addon bridge is added.
- Before in-game calibration exists, an owner-declared provisional transform
  (evidence confidence `inferred`) draws the marker labelled "approximate,
  uncalibrated" with an uncertainty ring. `limited_sample` needs at least three
  in-game control points; `verified_on_build` needs a section 22.4 client
  record in docs/compatibility/.
- `map_id` values are the addon-exported uiMapID as a decimal string. Until
  M0.4 verifies them, reference-content map and build/interface values are
  documented placeholders; a verified difference is a new dataset revision,
  never an in-place edit.
- One atlas dataset per edition grows by revision; Kalimdor (M12.1) is a
  revision bump plus its own rights record.
- Imagery is served by a custom Tauri URI scheme restricted to dataset-declared,
  hash-verified layers. Commands deviate from plan section 13: `query_atlas`,
  `project_positions`, `get_snapshot_markers`, `toggle_overlay`,
  `solve_calibration`.
- The M10 overlay window is the "explicitly scoped independent UI experiment"
  the plan allows, started before M6 and M9 are complete.
- Content validation runs inside the existing Rust CI jobs (pointer and
  consistency checks that need no LFS objects) until a data job is justified.

## Consequences
Image decoding and hashing live in the CLI only; core stays pure. CI never
downloads LFS objects. M6.5 stays IN_PROGRESS or IMPLEMENTED_UNVERIFIED until
calibration evidence from the recorded client build exists. Desktop work waits
for M1.4.

## Evidence
Source file SHA-256
`48ad4c6a36ca13343420d63328fc55d45c2733b06e95968ab983700690313228`, 17,511,553
bytes, 4492x5385 RGB. The poster footer names WoWForeverMap.com,
EndgameTools.com, Blizzard artwork and "beta build 1.60.1.70205". The owner's
rights statement is in docs/data/atlas-eastern-kingdoms.md. No client evidence
exists for positions, map IDs or the interface number.

Delivery: one branch and draft PR per slice, task-prefixed commits, one writer.
