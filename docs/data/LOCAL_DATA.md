# Private local client data

The standalone [implementation prompt](../../localdata.md) defines the complete
intended subsystem. This first implementation supports read-only installation
metadata inspection and preliminary resource planning. It does not extract or
activate client data. Desktop, native archive reader, isolated extraction worker,
table/cache/media decoders and private-catalog release policy remain future work.
The existing bundled-content release gates have not changed.

After provisioning the repository's pinned Rust toolchain/dependencies:

```sh
cargo run --locked -p azeroth-cli -- local-data inspect --installation <selected-directory> --product wow_classic_beta
cargo run --locked -p azeroth-cli -- local-data plan --installation <selected-directory> --product wow_classic_beta --destination <existing-private-directory> --preset max
```

Replace angle-bracket placeholders with explicitly selected paths; quote spaces.
Use native paths on Windows and mounted paths under WSL. No installation search,
account scan, cache reads, downloads, process access, installation writes or output
creation occur. Plans require an existing destination separate from the source;
neither may contain the other. No directory permission probes write files.

Both operations emit structured JSON. Exit codes: `0` successful inspection;
`1` refused/invalid/inaccessible/changing input; `2` unsupported command/options;
`3` a valid preliminary plan with unresolved extraction prerequisites. Exit3 is
deliberate: a useful preliminary report cannot authorize extraction. The remaining
commands listed in the prompt fail with usage rather than simulating success.
No background watcher, worker or extraction is started by either command.

Inspection reads only `.build.info`, at most256KiB and64 rows, for one explicitly
selected active product. Unknown columns remain inert; required column types,
build keys/version/build and row structure are validated. Duplicate columns or
multiple active matches refuse. It rechecks bytes and filesystem metadata after
a200ms interval. `build_info_stable` means exactly that file was stable, not that
archive/config/root/index files have passed consistency checks. The metadata
fingerprint is not a complete storage identity. Locale tags describe declarations,
not proof that each locale's files are installed/readable. Product identity alone
does not verify edition applicability. Symlinks and Windows reparse points in
selected paths/ancestors are refused. These preflight checks are not a race-proof
worker sandbox; extraction needs handle-based checks and isolation first.

The four presets expose whole-worker memory ceilings and total-storage ceilings
(originals, derivatives, indexes and temporary staging). Max is the default.
This implementation starts zero workers and reports actual worker count0.
Required/available storage remain `null`, never fabricated zero estimates.
Destination writability and free-space checks remain explicitly unverified.

Core includes format1 metadata structural validation. The I/O verifier additionally
requires an application-owned reviewed bundle digest, verifies every retained
SHA-256 object/size and refuses transforms absent from the compiled allowlist.
Bundle JSON cannot introduce trust or executable transforms. Duplicate keys,
unknown fields/versions, ambiguous table/layout IDs, absent licence-object
references, path-like identities and excessive closure sizes refuse. This is a
foundation for source admission, not a claim that a referenced licence grants
rights or that a DBD/table decoder exists. No production metadata bundle, transform
or upstream source has been admitted in this change. Pin constants describe the
required future revisions; no build-time download or native dependency was added.

The user-selected installation's `.build.info` was actually inspected on
2026-10-16: `wow_classic_beta`, `1.60.1.70425`, declared `enUS`, stable over200ms.
That is metadata-only installation evidence, not live acceptance or70374 data.
The authorized release target remains Forever1.60.1/build70374/interface16001.
All nine independent target inventories and operator acceptance requirements
remain unchanged. Private inputs and absolute installation paths are not committed.
