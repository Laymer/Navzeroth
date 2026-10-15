-- Verified immutable PNG bytes are shared by hash, never arbitrary disk paths.
CREATE TABLE catalog_asset_blobs (
    sha256 TEXT PRIMARY KEY CHECK(length(sha256)=64),
    bytes BLOB NOT NULL CHECK(length(bytes)>0 AND length(bytes)<=67108864)
);
CREATE TABLE catalog_asset_layers (
    revision_id INTEGER NOT NULL REFERENCES dataset_revisions(id),
    image_id TEXT NOT NULL,
    layer_id TEXT NOT NULL,
    sha256 TEXT NOT NULL REFERENCES catalog_asset_blobs(sha256),
    PRIMARY KEY(revision_id,image_id,layer_id)
);
CREATE TRIGGER catalog_asset_blobs_no_update BEFORE UPDATE ON catalog_asset_blobs
BEGIN SELECT RAISE(ABORT,'immutable catalog asset'); END;
CREATE TRIGGER catalog_asset_blobs_no_delete BEFORE DELETE ON catalog_asset_blobs
BEGIN SELECT RAISE(ABORT,'immutable catalog asset'); END;
CREATE TRIGGER catalog_asset_layers_no_update BEFORE UPDATE ON catalog_asset_layers
BEGIN SELECT RAISE(ABORT,'immutable catalog layer'); END;
CREATE TRIGGER catalog_asset_layers_no_delete BEFORE DELETE ON catalog_asset_layers
BEGIN SELECT RAISE(ABORT,'immutable catalog layer'); END;
CREATE TRIGGER catalog_asset_layers_v2 BEFORE INSERT ON catalog_asset_layers
WHEN NOT EXISTS(SELECT 1 FROM dataset_revisions WHERE id=NEW.revision_id AND content_version=2)
BEGIN SELECT RAISE(ABORT,'catalog assets require v2'); END;
