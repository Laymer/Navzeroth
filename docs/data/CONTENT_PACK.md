# Local reference pack format1

Frozen interface for section19.3 build-content/verify-pack, implementation owner Codex/root/atlas_dense,
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
fields and explicit collection/byte bounds. No generated packaging timestamps or host paths in the manifest. Imported evidence/
provenance timestamps remain intact in typed dataset/source documents.
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


## Implemented bounds and subset

Manifest and sources files:2MiB each; dataset:16MiB; each PNG:64MiB; all
pack files together:256MiB (including metadata). Independent cumulative input,
staging and verification budgets apply; canonical expansion is checked before
each staging file is created or written. At most128 datasets,1024
registered sources and2048 referenced assets across the pack. The exact tree
contains at most8192 file/directory entries. PNG decoding permits at most8192
pixels per axis and256MiB decoder allocation. Existing core bounds apply to
record lists,4096-byte text,256-byte IDs/asset paths and predicate depth.
Numbered output prefixes count toward the256-byte AssetPath bound.

JSON duplicate members and unknown typed fields are refused, including ignored
nested fields in existing exchange types. Nonempty exchange extensions are an
explicit unsupported pack subset; schema1 itself remains unchanged. The validated
schema1 dataset field `payload.map_embeddings: []` is accepted as its documented
typed empty default, equivalent to omission. Only the temporary comparison shape
recognizes that exact field; input bytes, identity/context/revision and the existing
canonical pack output remain unchanged. This exception does not apply to unknown
empty fields, another nesting path, null, or source/manifest documents. Other
null/empty optional fields omitted by typed serialization remain refused; omitted
defaults and canonical typed forms work. Arrays retain semantic order;
JSON object keys, source registry IDs, dataset identities and asset paths are
sorted. Input filenames do not determine output order. Verification recomputes
full typed coverage; the manifest stores its digest plus eleven bounded collection
counts and gap count, without converting gaps to errors or verification claims.

`distribution_eligible` checks declared registry metadata for every cited source:
explicit Permitted redistribution, recorded license/attribution/allowed-fields/
transformations and matching exact edition. Unknown, conditional, negative and
incomplete policy remain false. A dataset without cited sources also remains
false. The local_reference purpose never changes, and hashes/recorded policy do
not prove authenticity, source authority or legal permission. A synthetic positive
test declaration applies only to fixtures, never to missing production records.

## Commands and publication

For an explicitly prepared input directory:

```sh
cargo run --locked -p azeroth-cli -- build-content --input <dir> --output <new-dir>
cargo run --locked -p azeroth-cli -- verify-pack --input <pack-dir>
```

Both successful commands print an allowlisted report_version1 JSON containing
counts, byte total, coverage gaps, fixture count and declared distribution
eligibility; failures use static errors and nonzero exit codes without input echo.
Synthetic test preparation and successful command execution are covered by
`cargo test --locked -p azeroth-cli --test pack`. The test sidecar at
fixtures/content-pack/sources.json is explicitly synthetic, not a real provider
license or actual Forever-content permission record.

Portable publication reserves a create-new output directory, creates each child
exclusively, and writes manifest last. It is not an atomic directory rename and
does not promise crash durability; incomplete output cannot verify. Staging is a
create-new sibling and is verified before publication. Rollback deletes only
exclusively created files and removes directories only when empty. Existing
outputs and unrelated concurrently created files survive. Path checks reject
symlink/reparse components and parent traversal; files must be regular. Concurrent
hostile filesystem replacement during checks is outside this local-file workflow;
verification checks observed bytes/identities, not OS sandbox confinement.
