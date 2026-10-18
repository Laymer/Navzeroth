# Selected-cache development check

Owner: Codex `/root`; receipt recorded 2026-10-18T10:35:47Z.
The operator initially deferred cache reads to continue playing, then explicitly
closed WoW and authorized the check. No account or unrelated SavedVariables paths
were scanned. Only five fixed allowlisted filenames in the selected beta product's
enUS ADB/WDB directories were checked; four existed, itemcache.wdb did not.

Native Windows PowerShell enumerated process names before, between and after
reads and found no `wow*` process. Selected files were opened with read-only access
and `FileShare.Read`, excluding concurrent writes/deletion; reparse ancestors and
quotas were checked. Original streams were hashed twice; private owner-ACL copies
matched their SHA-256 hashes before Rust parsing. Source files were never modified.

| File | Bytes | Actual disposition |
| --- | ---: | --- |
| DBCache.bin | 3,778,060 | XFTH v9, build 70450; 48,301 bounded envelope records; 197 duplicate table/record/push groups retained conservatively |
| questcache.wdb | 211,730 | Unsupported WDB layout; source build/record count unknown |
| creaturecache.wdb | 83,501 | Unsupported WDB layout; source build/record count unknown |
| gameobjectcache.wdb | 226,979 | Unsupported WDB layout; source build/record count unknown |

Total copied bytes: 4,300,270. All four private copies were removed successfully.
No paths, payloads, rows or personal exports were committed or published. enUS is
selected-directory evidence; the XFTH header does not establish embedded locale.
The duplicate groups are potential conflicts, not proof of differing payloads or
universal facts. Opaque hotfix payloads were not applied to archive data.

The pure Rust parser module matched commit
`3f90417e2739232982aba461720d54e0d2dd4697`, SHA-256
`973169c73d99f7a3bc28c3cfd9c87ab6ea6e07bacda7a24ee7e6e6b25c2a7cf9`. A temporary Linux runner linked the current core
build and exercised that unchanged parser over immutable copies; it did not call
`stage_cache_fixture` or relabel the actual cache as synthetic. Native capture and
pure parsing are development evidence. **The native Rust `cache-stage` end-to-end
path remains unverified**, as do native closed-game/sharing refusal behavior and
live addon locale capture. They remain TODOs; this does not clear acceptance gates.

Continuation 3's [handoff](2026-10-18-local-inputs.md) and
[PR #193](https://github.com/Laymer/Navzeroth/pull/193) record integration and
subsequent exact-head checks. The operator can resume play; any further mutable
source read requires a fresh closed-game check and operator opportunity.
