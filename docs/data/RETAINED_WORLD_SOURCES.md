# Retaining exact world-source inputs

`data-generators/retain_world_sources.py` performs offline retention only. It reads
an explicitly selected cache and generated shards, verifies source identities and
retains exact raw bytes. It neither fetches inputs nor grants third-party rights.

Invoke with explicit `--root`, `--world`, `--cache`, `--pack-sources`, `--registry`
and `--output`. `--source-commit` accepts a full reviewed commit object. Without
that flag, the pin is the latest commit touching selected generator code, LICENSE,
NOTICE or TALOD permission. Output-only commits cannot move that pin. Every selected
working-tree file must exactly match its pinned Git blob; dirty/untracked code is
refused rather than attributed to a nonexistent immutable URL.

The mapper transformation identity comes from the pinned mapper's literal VERSION,
so mapper version3 produces `world_to_content_v3`. No stale v2 label is invented.
Only world_to_content.py and navzeroth_datagen/bounded.py enter the project-original
BSD source admission. The complete generator correspondence is retained separately
with project LICENSE, generator NOTICE and actual TALOD permission, explicitly
classified as mixed terms. In particular, sqldump.py documents derivation from
TALOD gen_fishing; this retention does not relicense it as BSD. Required downstream
material packaging must retain that distinction and the recorded terms.

Wago source snapshots are aggregate hashes, not individual CSV hashes. The retainer
reconstructs the exact sorted (URL, body-SHA256) list using gen_world_db2.TABLES for
each dataset, AreaTable for gen_fishing, and the six actually consumed auxiliary
CMaNGOS tables: UiMap, UiMapAssignment, FactionTemplate, AreaTable, Map, AreaTrigger.
It reproduces the generator's JSON digest exactly. Missing, failed, wrong-build,
changed or inconsistent constituents remain unresolved and stop retention. Failed
responses are not invented as available CSV bytes. CMaNGOS input is additionally
bound to its recorded dump filename and Git blob identity.

Reads use the existing bounded, regular-file, UTF-8/strict-JSON helpers. Raw bodies
are capped at64MiB each and128MiB aggregate (selected measured inputs approximately
24.5MB); snapshots at256, with the existing finite directory/file/record scans.
No concurrent ancestor-replacement protection is claimed. Content-addressed objects
use exclusive creation and exact existing-byte verification; corruption and final
symlinks are refused. This never silently repairs retained historical objects.

The schema2 output includes source files, aggregate constituent bindings,
source_commit, transformation identity and the separately classified corresponding
source closure. Source admission remains schema2 and retains existing upstream
rights evidence. Hashes establish byte identity; independent review, permission
scope, signed content receipts and release-material validation remain separate.
