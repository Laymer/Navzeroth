# Source admission v1

Owner: Codex `/root/source_admission`; baseline `d59aa91`.

The [registry](source-admission-v1.json) intentionally admits **zero sources**.
The pure Rust `azeroth_core::source_admission` contract is an explicit prerequisite
for a future acquisition adapter. The release gate now binds accepted sources to
a pinned registry and local permission evidence (below). Audit-only Questie
staging and ordinary pack activation do not imply source admission.

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

Portable file paths reuse AssetPath character/reserved-name rules, preserving
upstream case while rejecting case-colliding names. Retrieval/evidence URLs use a
narrow ASCII HTTPS DNS-host subset on the default port: no user information,
explicit ports, query/fragment, percent encoding or trailing DNS dot. Sources that
need other access forms require a reviewed contract extension, not URL rewriting.


## Offline release binding

Release evidence may select `admission_registry` as a bounded `FileRef`; it is
mandatory when accepted sources exist. Both candidate and publication checks
require each accepted source ID/revision to be admitted under the compatible
license policy. Registry origin edition/build must exactly match the source
provenance declaration; target validation remains independent. Every rights and
access-proof digest must occur in checked permission evidence. Pack publisher,
license identifier, attribution and revision must match the reviewed admission,
and its declared allowed fields must be a subset of the selected original files'
permissions. Pack transformations must be known, nonempty, and drawn from the
selected permitted files’ exact `transformation` strings. Each string is a reviewed
transformation identifier/declaration; it is compared literally, never executed
or inferred. Multiple selected files may permit distinct transformation strings.
Missing or unreviewed admission, noncommercial/unknown policy and
mismatches block readiness.

For this binding, each `PermittedFile.path` is the exact repository-relative path
of its original source-material `FileRef`, with matching length and SHA-256.
Choose that staging destination explicitly during admission; no suffix matching,
implicit prefix rewrite or transformed-file substitution is permitted. Permitted
files not selected in the release do not supply field permissions. Generated
catalog and pack bytes are transformations and are not compared to raw source
bytes. Reviewers still verify the stated transformations and obligations.

The pinned registry is private review evidence and is excluded from public
release materials, including copies relabelled as license/attribution notices.
The existing production fingerprint excludes `docs/` and `release/` as before;
registry integrity is bound by the current evidence file reference and independent
review, just like permission proofs. No network retrieval or grant authentication
is performed by the gate. Preview repositories with no accepted sources retain
their existing behavior, and the committed registry still admits zero sources.
