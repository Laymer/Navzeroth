# Source admission v1

Owner: Codex `/root` (implementation/integration ownership transferred from the
source-admission worker); original contract baseline `d59aa91`.
Continuation claim: `24646a6`, pipeline PR147.

The [registry](source-admission-v1.json) admits **four narrow source scopes**:
AtlasLootClassic original declarations (GPL2), Mapzeroth Retail EK node literals
(MIT), a WoWSims paladin implementation pointer (MIT), and four raw ATT Forever
Riverglades flight-point fields (MIT). Nine exact original
and licence objects are retained for the [offline pipeline](../data/SOURCE_PIPELINE.md).
Since 2026-10-19 it also admits **six generator sources** under
[ADR 0006 §14](../decisions/0006-v1-program.md) (below); thirteen retained
objects in all. Seven other candidate/exclusion entries remain unavailable to
acquisition.
The pure Rust `azeroth_core::source_admission` contract is a prerequisite
for every acquisition adapter. The release gate now binds accepted sources to
a pinned registry and local permission evidence (below). Audit-only Questie
staging and ordinary pack activation do not imply source admission.

`Registry::parse` checks the byte ceiling before strict deserialization, then
validates version, nested limits, duplicate identities and exact lists.
`Registry::authorize(source_id, file_path, fields, method)` revalidates even a
programmatically constructed registry and returns the exact file/source policy.
The CLI `source-sync` adapter now calls
this policy before acquisition, bounds streamed bytes to the remaining permitted
capacity, enforces the total request/byte budget and
minimum request interval, rejects redirects, verifies actual SHA-256 bytes, retains
only permitted immutable snapshots and applies the named reviewed transformation.
An authorization is not a consumable budget token: adapters must account for all
requests, including retries, themselves. Imported Lua must remain inert data.

Optional upstream metadata discovery has its own bounded
[access policy](source-updates-v1.json), bound to the registry hash. It cannot
fetch new source revisions or authorize content. Pinned source retrieval and
offline builds do not require that metadata service.

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
A third licence, `owner_accepted`, records that the owner explicitly accepts a
source's terms: it needs the same fields plus at least one evidence entry whose
`reviewed_scope` cites `ADR 0006 §14` (`OWNER_ACCEPTANCE_CITATION`); it is never
inferred. Admitted `owner_accepted` sources and sources retrieved by
`https_rate_limited` may list no files yet; `authorize` refuses any file that is
not listed, so nothing is usable until its generated files are registered.
Rights evidence must cover the exact file/field/transform scope; reviewers must
check that upstream can grant the rights and that attribution, share-alike,
retention and redistribution obligations are compatible. A hash identifies bytes;
it cannot establish authorship, legal authority or authenticity of a grant.
Neither admission nor raw imported counts establish 70374 applicability,
correction completeness, resolved conflicts or a complete target inventory.

## Current research disposition (2026-10-15)

- **ForeverDB is the preferred candidate**, replacing Wowhead at the user's request.
  Its advertised addon GPLv3 licence fits the selected licence families, but archive
  scope, exact file pins, acquisition permission and target validation are pending.
  Preserve Forever extraction, player observation and Classic inference separately.
  See the [provider review](foreverdb.md) and [audit metadata](foreverdb-review.json).

- Wowhead is research-only. [Fanbyte terms](https://corp.fanbyte.com/legal/terms)
  sections 4 and 8 restrict redistribution and automated extraction. No applicable
  grant is recorded. The page was read during this task; no source rows retrieved.
  **Superseded 2026-10-19:** the owner admits Wowhead Classic/Forever for the
  generators and accepts those terms explicitly (ADR 0006 §14, below).
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
  **2026-10-19:** CMaNGOS classic-db is admitted for the generators
  (`cmangos-classic-db`, ADR 0006 §14); QuestieDB stays unaccepted.
- Original permitted observations and explicitly licensed contributions require
  the same grant/attribution, exact scope and target validation records. None are
  fabricated here.

Next: extend the existing narrow inert-data adapters only for further reviewed
grants, and obtain an independently evidenced target inventory. The accepted
historical scopes do not establish any verified70374 target facts. Unresolved corrections,
conflicts or permissions continue to block affected gameplay records.

Portable file paths reuse AssetPath character/reserved-name rules, preserving
upstream case while rejecting case-colliding names. Retrieval/evidence URLs use a
narrow ASCII HTTPS DNS-host subset on the default port: no user information,
explicit ports, query/fragment, percent encoding or trailing DNS dot. Sources that
need other access forms require a reviewed contract extension, not URL rewriting.


## Generator sources (ADR 0006 §14, 2026-10-19)

The owner admitted the data the generators in `data-generators/` download and
explicitly accepted the third-party terms (Wowhead/Fanbyte terms restrict
automated extraction and redistribution). Credits are in
[data-generators/NOTICE.md](../../data-generators/NOTICE.md).

| Registry id | Material | Licence | Origin edition | Retrieval |
| --- | --- | --- | --- | --- |
| `wowhead-classic` | www.wowhead.com/classic pages, nether.wowhead.com tooltips | `owner_accepted` | `wow_classic_era` | `https_rate_limited` |
| `wowhead-forever` | www.wowhead.com/forever pages scripts may read (list pages answer 403; not bypassed), tooltips | `owner_accepted` | `wow_forever` | `https_rate_limited` |
| `cmangos-classic-db` | cmangos/classic-db `Full_DB` dump (GPL v3 `LICENSE.md`, `COPYRIGHT.md` reservation) | `share_alike` GPL-3.0-only | `vanilla_1_12` | `https_rate_limited` |
| `wago-tools-wowf` | wago.tools DB2 CSV, product `wowf` (Blizzard client data) | `owner_accepted` | `wow_forever` | `https_rate_limited` |
| `wago-tools-classic-era` | wago.tools DB2 CSV, product `wow_classic_era` (Blizzard client data) | `owner_accepted` | `wow_classic_era` | `https_rate_limited` |
| `talod-generators` | the three TALOD generator scripts, NotInTheBand's written permission | `owner_accepted` | none | `manual_local` |

- `https_rate_limited` means opt-in, cached, rate-limited retrieval by the
  generators only. `source-sync` never fetches these sources (it refuses with
  "Generator-retrieved source material is retained from data-generators"), and
  builds and tests never use the network. Their rights and decision documents are
  retained under `content/source-materials/objects/` like every other licence object.
- Wowhead and wago.tools have no immutable upstream revision. Their `revision`
  pins the reviewed decision instead: the Git blob id of ADR 0006 at its §14
  amendment. Each shard records retrieval time and snapshot hash per source
  ([WORLD_DATA.md](../data/WORLD_DATA.md)). CMaNGOS keeps the audited commit
  `ec4f596…`; TALOD pins the commit that added `PERMISSION.md`.
- The CMaNGOS README and `LICENSE.md` are GPL **v3** (the owner's brief said
  GPL-2.0; the registry follows the published file).
- Origin builds stay null in the registry; each shard's envelope carries the actual
  origin build (or unknown). Non-Forever origins stay `cross_edition_unverified`.
- `files` lists are empty: the generated fact documents are registered later by the
  integrator (task V2.7f). Until then `authorize` refuses every file of these sources.
- Retrieval bounds are the contract maximums (1 GiB, 1,024 requests, one second).
  The generators' own per-host limits in `navzeroth_datagen/http.py` are the
  effective controls; aligning them with these declared bounds is an open item.

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


## Upstream paths with encoded spaces

Source HTTPS URLs admit canonical `%20` in the path, preserving upstream directory
names containing spaces. Other percent encodings, encoded authorities, literal
spaces, query/fragment/userinfo and encoded separators/dots/double escapes remain
refused. No URL path is decoded into a local filename. AssetPath and exchange
formats are unchanged. ATT's registry file path names its actual retained Git
object under `content/source-materials/objects/`; its immutable URL and NOTICE
preserve `.contrib/.db/forever/zones/eastern kingdoms/riverglades.lua` exactly.

This source is publisher-declared Forever content with unknown origin build.
Its four fields are a point/NPC/coordinate tuple/access symbol, not a directed
flight connection, calibrated position or resolved eligibility. The initially
investigated Ashenvale quest declaration was withdrawn upstream in bf974021;
it is excluded from admitted inputs, not declared invalid in the target client.
