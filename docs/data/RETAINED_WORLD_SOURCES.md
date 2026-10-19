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

## Reviewed normalization contract1

Both retention and the admitted mapping pass require
`--normalization-policy release/v1/source-normalization-policy.json`. This is a
separate reviewed artifact, never written from a pack by the retainer. It pins
mapper v2, exact admitted upstream identities, original admission revisions,
canonical SHA256 of publisher/rights/retrieval declarations, actual historical
origins and the complete sorted generated field enumeration. The current list
was enumerated from the checked-in world sources and requires independent review.
Changing the mapper version, fields, origin or grant requires a reviewed policy
change. A generated pack cannot confer additional field rights. Repeated runs
accept the deterministic retained-file revision while keeping the same grant.
The admission's request and byte limits are ceilings; retention refuses excess
rather than expanding either limit. Permission proofs retain the8MiB bound.

The mapping pass verifies the policy's exact bytes, corresponding source's full
current pinned Git inventory (including the retainer, LICENSE, NOTICE and TALOD
permission), the exact code/terms hashes and immutable URLs, and the mapper/helper
license admission. Omitting an inventory member or substituting an unbound commit
fails. Existing original bytes, sizes, permissions and normalized revisions are
checked before any permission/revision is presented as usable. Every actual shard
source snapshot and origin must match the retained closure; Wago aggregate inputs
must reproduce its digest and CMaNGOS bytes its recorded Git blob. Extra or missing
originals and changed shard references fail. This binds source declarations to
retained bytes; source-platform regeneration/parity independently verifies the
transformation of the shard records themselves. It does not replace that gate.

The normal pipeline passes the policy to both commands. Run retention only after
committing reviewed generator code: dirty code cannot be assigned an immutable
source URL. After integrating mapper v3, explicitly review/update this policy's
version and any changed fields before regenerating. Missing public source URLs,
production receipts, Windows results or external review remain unverified.
