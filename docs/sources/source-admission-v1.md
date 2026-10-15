# Source admission v1

Owner: Codex `/root/source_admission`; baseline `d59aa91`.

The [registry](source-admission-v1.json) intentionally admits **zero sources**.
The pure Rust `azeroth_core::source_admission` contract is an explicit prerequisite
for a future acquisition adapter. It does not change existing audit-only Questie
staging, source policies, pack activation or the independent stable release gate.
Do not treat this additive API as proof those consumers enforce admission yet.

`Registry::parse` checks the byte ceiling before strict deserialization, then
validates version, nested limits, duplicate identities and exact lists.
`Registry::authorize(source_id, file_path, fields, method)` revalidates even a
programmatically constructed registry and returns the exact file/source policy.
No downloader is supplied while no source is admitted. Future adapters must call
this before I/O, bound streamed bytes, enforce the total request/byte budget and
minimum request interval, reject redirects, verify actual SHA-256 bytes, retain
only permitted immutable snapshots and apply the named reviewed transformation.
An authorization is not a consumable budget token: adapters must account for all
requests, including retries, themselves. Imported Lua must remain inert data.

The contract limits the JSON to 1 MiB, sources to 128, files per source to 1,024,
fields per file to 256, rights evidence entries to 64 and strings to 4,096 bytes.
Each source snapshot declares a full 40-character Git SHA-1 or 64-character digest;
files and evidence require lowercase SHA-256 pins. Files are at most 8 MiB and a
retrieval closure at most 1 GiB. Access is serial, at least one second apart and
at most 1,024 requests. Exact relative file paths and fields have no wildcards.
This describes source staging limits, not the later full-catalog v2 limits.

Rights status is separate from source edition/build; absent origin remains null.
Only reviewed permissive/share-alike grants with a license identifier, reviewer,
attribution, obligations, scope-bearing immutable evidence and access-method
evidence can be admitted. Noncommercial, unknown and rejected grants fail closed.
Rights evidence must cover the exact file/field/transform scope; reviewers must
check that upstream can grant the rights and that attribution, share-alike,
retention and redistribution obligations are compatible. A hash identifies bytes;
it cannot establish authorship, legal authority or authenticity of a grant.
Neither admission nor raw imported counts establish 70374 applicability,
correction completeness, resolved conflicts or a complete target inventory.

## Current research disposition (2026-10-15)

- Wowhead is research-only. [Fanbyte terms](https://corp.fanbyte.com/legal/terms)
  sections 4 and 8 restrict redistribution and automated extraction. No applicable
  grant is recorded. The page was read during this task; no source rows retrieved.
- RestedXP remains requirements-only under the selected policy. Its pinned
  [license](https://github.com/RestedXP/RXPGuides/blob/78e11d001e32b1432e29fc4bf8df61743a24ac68/LICENSE)
  is noncommercial. No route or guide content is admitted.
- Warcraft Wiki needs contribution-level rights and separate media review.
  The [copyright policy](https://warcraft.wiki.gg/wiki/Warcraft_Wiki%3ACopyrights)
  could not be retrieved by this task's browser; no current license verification
  or admission is claimed.
- QuestieDB and CMaNGOS retain the unaccepted status and immutable identities in
  the [prior audit](v1-public-source-audit.md). Public repository availability,
  executable corrections and historical edition lineage are not permission or
  target validation. This task does not repeat or supersede that audit.
- Original permitted observations and explicitly licensed contributions require
  the same grant/attribution, exact scope and target validation records. None are
  fabricated here.

Next: obtain reviewed grants and independently evidenced target inventory, then
add narrowly scoped inert-data acquisition adapters. Unresolved corrections,
conflicts or permissions continue to block affected gameplay records.
