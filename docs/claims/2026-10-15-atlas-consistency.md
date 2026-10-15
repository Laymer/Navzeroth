# Atlas consistent catalog reads
Owner /root/inventory; baseline8ee8ad4; branch fix/atlas-consistent-read.
Coordinator transfers stored_atlas.rs, snapshots marker block and catalog_pages.rs
for focused race correction. External SQLite activation must not pair old content
with new indexed rows/hash. Bind marker projection and expected token in the same
app lock and SQLite read transaction. Importer other blocks remain separate owner.
