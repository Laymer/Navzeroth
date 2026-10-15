# ForeverDB quest staging v1

`stage-foreverdb` reads three explicitly selected local data files from the
reviewed 0.34.0 layout. It checks their exact bytes against a selected member
manifest, parses the complete inert wrappers, and reports source quest IDs and
namespace table sizes. This is diagnostic staging, not an admitted gameplay pack.

```sh
cargo run --locked -p azeroth-cli -- stage-foreverdb \
  --input /private/extracted-addon-root \
  --manifest docs/sources/foreverdb-0.34.0-members.json
```

The selected directory must contain `ForeverDB/Data/Meta.lua`,
`ForeverDB/Data/QuestForever.lua` and `ForeverDB/Data/QuestClassic.lua`. Obtain the
archive through the [reviewed publisher-linked download](../sources/foreverdb.md),
check its exact archive SHA-256, and prepare those files locally. Extraction and
downloads are separate operator actions; the command performs neither. Do not
run imported Lua or the addon to unpack data. Keep the local full report private
until reviewed; only counts and hashes are recorded in the repository.

The file manifest is a bounded JSON array of `{path,bytes,sha256}` declarations.
Unknown/duplicate fields, case-colliding paths, unsafe names and invalid hashes
fail. Only the three exact supported paths are read, each at most 8 MiB. The
manifest must be at most 1 MiB (the reused local-file read boundary has an 8 MiB
read ceiling); it may declare at most 1,024 members. Every selected path component
must be free of symlinks/reparse points and parent traversal. Failed reads, changed
files and mismatched hashes return an error with no partial staging result.

Metadata must declare addon `0.34.0`, a `1.60.1.<numeric-build>` origin, and the
supported six literal fields. Unknown versions and schema changes need review.
Quest files must contain their complete expected sets of namespace assignments,
with literal tables only. The parser rejects calls, functions, variable values,
operators, duplicate keys/assignments, unknown assignments and executable suffixes.
Each file is bounded to 1,000,000 literal values, depth 32 and 64 KiB strings;
quest tables are bounded to 100,000 positive u32 IDs. Compact source quest rows
retain their eight Forever or fourteen Classic fields without interpreting them.

The raw staging type preserves `nil`, false, zero, empty strings and absence
separately and cannot be serialized as a gameplay catalog. The public JSON report
contains metadata and IDs, with no quest text. Overlapping IDs remain in both
source inventories. A matching member manifest establishes byte integrity against
that selected manifest, not its authenticity or the identity of a complete ZIP.

The bundle's origin build is separate from per-record builds, which are unknown.
In particular, a Forever file can combine client facts with player observations
whose capture builds are absent. Classic facts keep `wow_classic` as their
declared edition and do not inherit the Forever bundle build. Changing metadata
to 70374 never sets target validation to verified.

Both rights admission and target validation stay unverified/unreviewed. Required
target quest IDs stay null; source-empty tables do not establish an empty target.
The command always reports publication disallowed, and has no output-directory,
activation, download or release-gate override. Exit0 means diagnostic staging
succeeded, not content or release readiness. Exit1 means refusal, exit2 invalid
options. A closed output pipe is handled as a failed report write.

On the pinned archive, the adapter parsed 557 Forever-labelled and 3,491
Classic-derived source quest records, with 373 overlapping IDs. See the
[actual result summary](../sources/foreverdb-staging-result.json). These counts
are not a complete target inventory. Other content domains, compact-field
normalization, missing corresponding source and 70374 validation remain work.
