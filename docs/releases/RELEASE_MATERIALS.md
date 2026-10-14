# Verified release materials

The release-only material manifest is separate from the version-1 exchange,
source registry and content-pack contracts. It does not accept a source or
establish live gameplay or Windows acceptance. Current inventory and evidence
still block stable publication.

```sh
cargo run --locked -p azeroth-cli -- release-materials --root . --version 0.8.1
cargo run --locked -p azeroth-cli -- release-materials --root . --version 0.8.1 --output /existing/parent/new-material-directory
```

The first command emits a deterministic JSON plan without writing files. The
second validates the same plan, then stages only selected regular files into a
new directory. It never overwrites an existing output. An empty preview selection
does not create an output directory. An optional `--evidence <relative-file>`
selects another bounded manifest under the repository; normal packaging always
uses the canonical release evidence.

Selection reuses the existing authoritative pack verifier, frozen target,
accepted source revisions and FileRef hashes. Pack registries must explicitly
permit offline use. Raw files, license files and attribution files come only from
the matching `SourceEvidence` declarations. Required permission and target proofs
are verified locally, while incomplete live/Windows acceptance remains visible
in `release-readiness`. No imported Lua or SQL is executed.

All selected inputs must be tracked production files outside `docs/` and
`release/`, have the declared bytes/hash and remain unchanged in the working tree.
Symlinks/reparse paths, unknown editions, fixture packs, coverage gaps, unsupported
schemas, unresolved redistribution permission, unpinned revisions and duplicate
pack/source identities refuse selection. Inputs are bounded to 256 MiB each,
16,384 selected files and the release verifier's cumulative 1 GiB budget. Existing
pack and manifest limits remain stricter where applicable.

The public schema-version-1 material manifest records the candidate commit,
production fingerprint, inventory hash, target, pack manifests/revisions,
source IDs/revisions/origin metadata and each exact input/archive path and hash.
Files are sorted by portable archive path. Private enumeration, coverage,
acceptance, permission and target proofs are never selected automatically. A
selected file that overlaps a private proof by path **or exact content hash** is
refused, so the public-material and private-proof roles must be disjoint. Only
declared source/license/attribution fields enter the public source metadata.

The archive keeps the original terms and bytes in two separate trees:

- `content-packs/<repository path>` contains ordinary pack manifests, datasets,
  registries and their complete asset closures.
- `source-materials/<repository path>` contains inert raw source, license and
  attribution files. These files are not copied into the addon or loaded as
  executable content. They are not relabeled as project-owned BSD material.

When the verified selection is nonempty, Windows packaging additionally emits
`AzerothNavigator-materials-<version>.zip`, `RELEASE_MATERIALS.json`, and
`RELEASE_ASSETS.json`. The material manifest is also embedded in the ZIP. ZIP
entries are sorted with a fixed source timestamp. The asset manifest records the
exact release file set, candidate commit, sizes and hashes; checksums cover both
public manifests, the ZIP and the ordinary installer/addon/metadata files.
Schema-version changes require explicit compatibility work; current verification
compares the canonical manifests to a freshly generated expected selection.

Existing empty-content 0.x or named-prerelease packages retain their original
file layout and addon-only use without Cargo. They do not acquire playable data
or a stable-readiness claim. A nonempty selection requires the locked Rust CLI.

```powershell
./scripts/windows/Verify-ReleaseArtifacts.ps1 -Version 0.8.1 -Directory dist/windows
```

Use `-AddonOnly` when verifying an addon-only package. The verifier recomputes the
material selection from the checked-out source, validates every ZIP entry and
its bounded decoded bytes, rejects extra/missing/duplicate/traversal/link entries,
compares the sidecar and embedded manifests, checks source-commit metadata and
release notes, and requires the exact asset/checksum file set. Rewriting outer
checksums cannot hide a changed material payload. It runs after packaging and
again against downloaded artifacts before creating a tag draft.

This delivery support replaces the temporary generic “asset delivery is not
implemented” blocker. Actual mandatory coverage, source permissions, missing
class/role/party/dungeon contracts and recorded client/Windows acceptance still
block stable readiness. No upstream source or raw dataset is accepted by this
implementation or its synthetic transport tests.
