# Selected private cache and observation inputs

Continuation 3 adds bounded research staging alongside immutable archive catalogs.
It does not apply hotfix overlays or make records playable. Archive, cache and
addon evidence remain distinct, bound by hashes and an exact selected catalog.
All retained bytes, analyses and manifests stay in the private local-data store.
No source paths or imported text appear in default reports.

## Supported scope

Explicit cache kinds are `hotfix` (`DBCache.bin`), `quest` (`questcache.wdb`),
`creature` (`creaturecache.wdb`), `game-object` (`gameobjectcache.wdb`) and `item`
(`itemcache.wdb`). The selected filename must match exactly, inside the requested
locale directory. That locale must be declared by the selected catalog.
Nothing searches account directories, other SavedVariables, chat or caches.

The bounded parser supports XFTH/HTFX envelope versions 1–7 and 9, including
header/build, extended header, per-record metadata, original order and bounded
opaque payload ranges. Versions 2–6 require matching per-record versions and
zero reserved flags. Version 8 has ambiguous historical layouts and is explicitly
unsupported; unknown versions/formats and legacy WDB payload layouts are retained
with `unsupported` and unknown record counts. They are not decoded as zero rows.
Known headers must match the selected catalog build exactly. Unsupported formats
cannot establish a recorded build or locale; `catalog_tuple` is selected context.

Reference: [pinned DBCD HTFX reader](https://github.com/wowdev/DBCD/blob/e732093f8864240fc5884bd1bba6b02f3dfc0d56/DBCD.IO/Readers/HTFXReader.cs)
and [record layouts](https://github.com/wowdev/DBCD/blob/e732093f8864240fc5884bd1bba6b02f3dfc0d56/DBCD.IO/Common/HTFXStructs.cs).
No new upstream runtime dependency or upstream code is vendored.

Payloads are not interpreted through guessed table definitions. Valid nonempty
records are upsert candidates; add versus replace remains unknown. Other flags
are retained as invalidation of unknown meaning. They cannot establish universal
deletions. Duplicate table/record/push tuples remain conflicts with every original
record retained. No global newest-wins rule is applied. Field layout interpretation
and reviewed overlay applicability remain explicit gaps. No encrypted key/table
payloads are interpreted or used for decryption.

Limits: 64 MiB per cache; 100,000 records; 1 MiB per payload; 32 MiB analysis;
64 KiB manifest; 200 metadata rows per revision-bound page. The normal private
store quota and destination writer lock apply. Partial writes cannot seal an
input manifest referring to absent objects. Existing active catalogs are untouched.

## Closed-game boundary

Production cache staging requires native Windows process enumeration to establish
that no `wow*.exe` is running before reading and again before staging. It reads
executable names only, never game memory or process contents. Enumeration errors
and excessive process inventories refuse the operation. Nothing closes or controls
the game. Selected cache handles deny sharing for writes/deletion throughout the
read/staging interval. Existing writers prevent opening. The source is read and
hashed twice, and its handle/path identity and size are rechecked.

Linux/WSL production cache reads fail closed because a reliable Windows-game
precondition is unavailable there. Synthetic staging is a separate explicitly
named API/CLI operation and always marks its result `fixture: true`. Desktop
application services expose only guarded production cache staging. Synthetic
fixture staging never counts as real-cache acceptance.

The owner deferred real-cache verification on 2026-10-18 to continue playing.
**TODO:** select exact cache files and verify them only after WoW is closed.
No installed mutable cache was read for this continuation.

## Permitted addon observations

`observation-stage` reuses existing bounded inert `ANAV1:` character-snapshot and
`ANAVSESSION1:` session decoders. It accepts explicitly selected transport files;
existing SavedVariables importers remain available separately. No imported Lua is
executed. Typed analyses preserve all metadata, unknown/unsupported/restricted
states, false, zero and justified empty values. Real snapshots require addon
producer identity; synthetic/demo inputs retain fixture identity.

Snapshot build (exact full version or exact numeric build) and locale must match
the catalog. Source edition/interface/namespace remain in the typed analysis;
product-to-edition applicability remains unverified and cannot enable gameplay.
Session version 1 remains readable with locale explicitly unrecorded. Version 2
adds a required known locale; the addon emits it only when permitted `GetLocale`
returns a recognized locale at manual session start. Otherwise it emits historical
version 1. Version 1 with a locale, version 2 without one and unknown versions are
refused. Both use the existing bounded framing. Reload never resumes recording.

A quest turn-in is contextual evidence only. It cannot establish prerequisite,
sharing, reset, branch, rate or universal eligibility facts. No additional queries,
automated interactions or background observation were added.

## CLI

```text
azeroth-cli local-data cache-stage --destination PRIVATE --catalog SHA --input SELECTED/enUS/DBCache.bin --kind hotfix --locale enUS --preset small
azeroth-cli local-data cache-stage-fixture --destination PRIVATE --catalog SHA --input SYNTHETIC/enUS/DBCache.bin --kind hotfix --locale enUS
azeroth-cli local-data observation-stage --destination PRIVATE --catalog SHA --input EXPORT --kind snapshot --locale enUS
azeroth-cli local-data observation-stage --destination PRIVATE --catalog SHA --input EXPORT --kind session --locale enUS
azeroth-cli local-data input-verify --destination PRIVATE --input SHA
azeroth-cli local-data input-page --destination PRIVATE --input SHA --offset 0 --limit 200
```

`inputs/<sha256>` manifests reference retained originals and deterministic typed
analyses in `objects/<sha256>`. Verification checks hashes, catalog closure, tuple,
version/disposition, analyzer profile and a regenerated analysis. The analyzer
fingerprint includes the complete first-party Rust source directories used for
validation/storage plus build/dependency inputs, with normalized line endings.
Unknown profiles are rejected; they are never silently interpreted as current.
Research activation and catalog/projection formats retain their existing readers.
