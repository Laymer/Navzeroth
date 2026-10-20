# Independent source-knowledge pipeline

Owner: Codex `/root`. Contract version: 1. Reviewed: 2026-10-15.

The project retains approved, immutable source files and their licence evidence
in Git. Building the source-knowledge catalog needs neither a publisher reply
nor a publisher website. It is a research/interchange catalog, separate from
reference-content v2 and playable content packs. It cannot authorize publication
or activate desktop content. Existing snapshot, plan and release gates remain.

## Build and acquisition

After provisioning the pinned Rust toolchain and dependencies (`cargo fetch
--locked`), run from the repository with PowerShell 7:

```powershell
./scripts/Build-SourceKnowledge.ps1 -OutputDirectory dist/source-knowledge
```

The script compiles the CLI with `--locked --offline`, verifies retained inputs,
and builds without transport. An existing output directory is refused. A prepared
CLI can be passed as `-Executable`. This does not vendor the Rust toolchain or
Cargo dependencies; those are provisioned separately from gameplay sources.

The underlying commands are:

```sh
cargo run --locked --offline -p azeroth-cli -- source-sync --registry docs/sources/source-admission-v1.json --cache content/source-materials/objects
cargo run --locked --offline -p azeroth-cli -- source-build --registry docs/sources/source-admission-v1.json --plan content/source-build-v1.json --cache content/source-materials/objects --output dist/source-knowledge
```

`source-sync` is offline unless `--fetch` is explicit. For a new mirror/cache,
that flag permits only approved pinned HTTPS files and licence evidence at
`raw.githubusercontent.com`. It enforces each admitted source's byte, request and
rate bounds. Hash mismatch, missing offline material, unsafe paths, symlinks,
reparse points and changed files fail rather than replacing retained bytes.
Unreviewed/rejected candidates are reported and never fetched. Lua is parsed as
bounded data; imported code is never executed. No addon installation, game-data
API scraping, uploader or credential is needed.

A separate optional metadata permission permits four public GitHub latest-commit
queries, with 1-second spacing, 256 KiB per response and 768 KiB total:

```sh
target/debug/azeroth-cli source-sync --registry docs/sources/source-admission-v1.json --cache content/source-materials/objects --fetch --check-updates docs/sources/source-updates-v1.json
```

The policy is bound to the registry hash and admitted repositories. Metadata
failure means unknown freshness. Observing a new revision never downloads or
admits that revision's content. Review its rights, paths, fields, origin and
adapter before updating immutable pins. The AtlasLoot adapter additionally
requires the exact reviewed source hash, so a layout change cannot silently
change its meaning. Source updates are automated discovery, followed by reviewed
admission; ordinary catalog builds remain independent of discovery.

## Uniform facts and precedence

The versioned `source-build-v1.json` plan declares its target and an explicit
as-of UTC millisecond value. `knowledge_v1` inputs carry a domain, record key,
field key, tagged datum, confidence, evidence date and clock trust. The other
adapters produce that same contract. Every candidate retains its immutable source
revision, file hash and original edition/build.

Convergence groups the same domain/identity/field within the same known origin.
Different editions/builds remain separate. Unknown-origin inputs remain isolated
by source/revision; they cannot gain target applicability by agreement.
Candidates rank by the existing confidence reliability order, then newest actual
observation with a trusted clock, then stable provenance keys. A download time,
commit SHA or uncertain clock does not establish freshness. Stale values rank
below usable evidence. All competing values remain available; a conflicting
entry cannot become target-usable solely because one candidate ranked first.
Only exact known target identity with suitable confidence and a known datum can
count as target-usable. Inference or cross-edition evidence is never promoted.

Known false, zero and empty text remain values. Unknown, unsupported, restricted,
stale and not-applicable remain distinct tags; known null is refused. Strict JSON
rejects duplicate members, unsupported versions, future dates, unsafe numbers,
excessive depth/size and identity collisions. Limits include 8 MiB and 250,000
facts per input, 6,000,000 facts per build, 64 KiB per value, 8 GiB catalog
encoding and 2 GiB retained material closure; the values and their measurement
are in [WORLD_DATA.md](WORLD_DATA.md#source-pipeline-limits). Overflow fails;
records are never truncated. The catalog is streamed to its output file, so a
build does not hold a second encoded copy in memory.

## Generated fact sources

`data-generators/register_facts.py` (ADR 0007 §7) registers every fact document
`to_facts.py` writes: one admitted `gen-<generator>-<edition>-<build>` source per
generator and origin, `manual_local`, with each document's exact hash, size and
`domain.field` list, rights copied from the upstream sources the generator used,
and ADR 0007 as access evidence. It copies the documents to
`content/source-materials/objects/<sha256>`, upserts the plan's `knowledge_v1`
inputs, removes sources and objects for documents that no longer exist, updates
`registry_sha256` in `source-updates-v1.json` and rewrites the generated sections
of the [material notice](../../content/source-materials/NOTICE.md) and of
`.gitattributes`. That block stores exactly the generated objects in Git LFS;
licence and evidence objects stay plain blobs, so CI without LFS still builds
every hand-admitted source.

## Retained scopes and current gaps

| Scope | Original applicability | Derived facts |
| --- | --- | ---: |
| AtlasLootClassic selected original data declarations (GPLv2) | Classic, build unknown | 66 area/instance declarations and 718 NPC/normal-loot declarations |
| Mapzeroth Eastern Kingdoms node literals (MIT) | Retail, build unknown | 896 map/coordinate/interior facts |
| WoWSims paladin implementation pointer (MIT) | Classic, build unknown | 1 source reference; no coefficients or simulation |
| AllTheThings Riverglades point declarations (MIT) | Forever, build unknown | 4 raw point/NPC/coordinate/access-symbol fields; no inferred flight edge |

Total: 1,685 reference facts, including 258 unknown and 26 unsupported fields.
The generated fact sources (below) add 2,862,735 facts: 2,566,760 from CMaNGOS
(`vanilla_1_12` 1.12.1) and 295,975 from wago.tools `wowf` (`wow_forever`
1.60.1.70245). None is target-usable for the plan's target (the detected client
build, ADR 0007 §6; 1.60.1.70450 when this was written): their origin differs from
the target, and convergence never relabels it. The release gate accepts such
origins through `release/v1/accepted-origins.json` (ADR 0007 §3), not through this
catalog.
There are **zero verified target-usable facts** in this research catalog. The
catalog reports all nine domains with unknown independently required inventory
counts. Quests, equipment, party and reminders have no admitted facts in
this plan. An upstream source group is not proof of an available Forever dungeon;
legacy area IDs are not modern UI map IDs. Loot declarations establish no drop
rates, quantities, availability, prerequisites or player entitlement.

The registry also records ForeverDB, CMaNGOS, QuestieDB, Forever GuideMate,
Lodestar, pfQuest and VMaNGOS as unreviewed scoped candidates, and Forever Guides
as excluded. These entries preserve investigation leads without fetching or
converging material outside an accepted grant. Wowhead remains a research
reference. Published compatible grants replace the need for permission email
only for the exact works they cover. Broad database ancestry and target coverage
still require evidence. See the [source strategy](../sources/independent-source-strategy.md).

## Outputs, redistribution and automation

A successful build publishes a fresh directory atomically: `catalog.json`,
`build.json`, `closure.json`, exact `registry.json` and `plan.json`, `NOTICES.txt`
and the nine original/licence objects in `materials/objects`. The closure is
sufficient for another offline build. The build report binds input, catalog,
material and selected generator-source hashes. `publication_authorized` is always
false. This artifact cannot pass the stable release gate as a playable pack.

Retain source licences and attribution with derived material. AtlasLoot-derived
knowledge retains applicable GPLv2 terms and a dated modification notice. The
CI artifact additionally bundles the complete repository source revision for the
transformation, alongside exact original sources; see the
[material notice](../../content/source-materials/NOTICE.md). The repository BSD
licence does not relicense third-party material.

The `Independent source knowledge` workflow builds twice on Linux and Windows,
compares outputs locally and across platforms, then retains complete artifacts.
It pulls only `content/source-materials/objects/*` from Git LFS (the generated
fact objects). Source archives disable Git LFS smudge/process filters and verify
exact pointer bytes before upload, preserving the committed source without
downloading imagery.
It runs on relevant PRs/main pushes and can run manually or weekly. Only the
scheduled/manual Linux job performs the separately permitted metadata check.
Stable packaging still requires full release readiness. Real permitted addon
observations, independently evidenced target inventories and operator-driven
live/native acceptance remain necessary to close production gaps.


The current ATT revision bf974021 explicitly reverts the initially investigated
quest-data commit `649f90`. Bounded metadata discovery caught that change before
admission. Its removed Ashenvale declaration is not included. The selected
Riverglades point remains byte-identical at the newer revision. This establishes
source retention, not 70374 validation or a gameplay observation date. Immutable
source changes still require admission/adapter review; builds retain their pins.
