# Verified catalog imagery storage

Owner: Codex /root/inventory; claim baseline908aef2; separate branch
`feat/catalog-assets`, dependent on PR133. Acceptance worker owns Atlas consumer
and coordinator owns the shared pack verifier/import integration.

Migration12 adds immutable SHA-256 blobs and revision/image/layer bindings.
`stage_content_v2_bundle` stages a complete exact declared closure in one SQLite
transaction with the catalog. Every PNG is hash checked, fully decoded with
8192-pixel and256MiB decode bounds, and matched to declared dimensions.
Paths remain inert catalog identities. Bounds:2048 files,64MiB/file,1GiB total
material including serialized revision,250000 layer mappings. Shared files
must agree on hash/dimensions. JSON-only catalog staging remains supported;
its missing image bytes are explicitly unavailable.

`catalog_image_present` is a cheap availability hint for an exact active scope,
revision/hash/image/layer. `active_catalog_image` rechecks hash and PNG before
returning bytes. Stale bindings and unknown identifiers refuse. Activation and
rollback use the existing single atomic pointer; immutable bytes follow history.
`catalog_revision` exposes a read-only enum for staged content inspection without
changing disposition. No filesystem image paths or addon bridge are introduced.

Recovery applies the same closure bounds, rejects orphan blobs, conflicting
file declarations and tampered PNGs. Dataset/coverage row limit stays256MiB plus
16KiB; blob row limit64MiB plus16KiB. Backup file bound increases from512MiB to
4GiB to accommodate a1GiB closure plus SQLite overhead and retained history.
This remains a hard bound: long histories can require independent archival;
no rows or assets are silently truncated.

Checks: full storage suite122 tests passed before review follow-up. Following
review, five asset integration tests and one closure-budget unit pass, including
atomic failure, explicit missing imagery, exact active binding, staged v2 read,
wrong PNG/dimensions, tampered serving/recovery, orphan recovery refusal, and
backup reconstruction. Strict storage all-target Clippy passes. Synthetic PNGs
only; licensed production imagery, native Atlas display and full Windows corpus
performance remain unverified. Coordinator reviews recovery changes and handles
hosted checks/merges; next task is independent shared-pack verifier review.
