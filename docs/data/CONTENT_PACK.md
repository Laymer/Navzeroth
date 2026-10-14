# Local reference pack format1

Frozen interface for section19.3 build-content/verify-pack, owner Codex/root,
baseline v0.4.0. This internal container does not alter exchange schema1.

Input: one selected content directory with datasets/*.json and sources.json.
Sources document: {"schema_version":1,"sources":[SourceRecord,...]} using the
existing validated SourceRegistry (unknown policy remains unknown). Every evidence
and map-image source must be registered. No raw provider scraping or live API.

Output: create-new directory, deterministic manifest.json plus one numbered folder
per exact context/fixture/revision. Each folder contains canonical typed dataset.json
and referenced layer assets at their original bounded AssetPath, with supplied hash
and dimensions verified. Sources canonicalized in sources.json. No arbitrary source
files, scripts, personal exports or directory recursion; symlink components refused.

Manifest schema_version1, generator_version, purpose local_reference, sources digest,
and sorted entries containing dataset path/bytes/digest, exact DataContext/fixture/
revision and sorted asset path/bytes/digest metadata. Every object has deny-unknown
fields and explicit collection/byte bounds. No timestamps or host paths in output.
Verify checks the exact file set, byte/hash metadata, typed source/dataset/coverage,
context identity and image hash/dimensions; refuse future manifest versions, duplicate
identities, missing/extra files or symlinks. Hashes prove integrity, not authenticity.

Build stages in a create-new sibling, verifies before publishing, refuses existing
output and never replaces a last-valid pack. Cleanup removes only its own staging.
Independent exact contexts are kept in separate folders, never silently merged.

Sources with unknown/non-permitted/conditional redistribution never count as permitted.
The validation result exposes distribution_eligible only when every cited source has
explicit Permitted policy, with recorded license/attribution/allowed-fields/transformations,
and exact edition coverage. A local pack can preserve unknown source metadata; it is
not evidence of a permitted production-content pipeline or full focused release.
Current real content/source gaps remain blocked. Synthetic examples are labeled fixture.
