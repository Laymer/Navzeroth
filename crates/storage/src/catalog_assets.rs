//! Verified immutable raster closure. Asset paths are inert identities, never disk reads.
use crate::{
    SqliteStore,
    content::{self, ContentRevisionV2, ContentScope, StageOutcome},
    content_catalog,
    error::{SqliteResult, invalid, storage},
};
use azeroth_core::{
    catalog::Collection,
    entities::ImageLayer,
    error::Result,
    types::{AssetPath, DatasetRevision, ImageId, LayerId, Sha256Hex},
};
use image::{ImageFormat, ImageReader, Limits};
use rusqlite::{OptionalExtension, TransactionBehavior, params};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{collections::BTreeMap, io::Cursor};
pub const MAX_ASSET_BYTES: usize = 64 * 1024 * 1024;
pub const MAX_ASSETS: usize = 2048;
pub const MAX_CLOSURE_BYTES: usize = 1024 * 1024 * 1024;
#[derive(Clone, Debug)]
pub struct CatalogAsset {
    pub file: AssetPath,
    pub bytes: Vec<u8>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ImageRequest {
    pub revision: DatasetRevision,
    pub catalog_sha256: Sha256Hex,
    pub image_id: ImageId,
    pub layer_id: LayerId,
}
fn verify_png(bytes: &[u8], layer: &ImageLayer) -> Result<()> {
    if bytes.is_empty()
        || bytes.len() > MAX_ASSET_BYTES
        || format!("{:x}", Sha256::digest(bytes)) != layer.sha256.as_str()
    {
        return Err(invalid("Catalog image bytes or hash differ"));
    }
    let mut limits = Limits::no_limits();
    limits.max_image_width = Some(8192);
    limits.max_image_height = Some(8192);
    limits.max_alloc = Some(256 * 1024 * 1024);
    let mut reader = ImageReader::with_format(Cursor::new(bytes), ImageFormat::Png);
    reader.limits(limits);
    let decoded = reader
        .decode()
        .map_err(|_| invalid("Catalog image is not a bounded PNG"))?;
    if decoded.width() != layer.width_px.get() || decoded.height() != layer.height_px.get() {
        return Err(invalid("Catalog image dimensions differ"));
    }
    Ok(())
}
fn closure_budget(revision: &impl Serialize, total: usize, limit: usize) -> Result<()> {
    struct Budget {
        used: usize,
        limit: usize,
    }
    impl std::io::Write for Budget {
        fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
            if bytes.len() > self.limit.saturating_sub(self.used) {
                return Err(std::io::Error::other("closure exceeds bound"));
            }
            self.used += bytes.len();
            Ok(bytes.len())
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }
    let used = total
        .checked_add(1)
        .filter(|n| *n <= limit)
        .ok_or_else(|| invalid("Catalog material closure exceeds capacity"))?;
    serde_json::to_writer_pretty(&mut Budget { used, limit }, revision)
        .map_err(|_| invalid("Catalog material closure exceeds capacity"))
}
impl SqliteStore {
    /// Stage every declared image-layer file with the immutable catalog in one transaction.
    /// This does not activate the catalog or turn source rights into a grant.
    pub fn stage_content_v2_bundle(
        &mut self,
        input: &ContentRevisionV2,
        assets: &[CatalogAsset],
    ) -> Result<StageOutcome> {
        if assets.len() > MAX_ASSETS {
            return Err(invalid("Catalog asset count exceeds capacity"));
        }
        let mut total = 0usize;
        let mut supplied = BTreeMap::new();
        for asset in assets {
            total = total
                .checked_add(asset.bytes.len())
                .ok_or_else(|| invalid("Catalog closure capacity overflow"))?;
            if asset.bytes.is_empty()
                || asset.bytes.len() > MAX_ASSET_BYTES
                || total > MAX_CLOSURE_BYTES
                || supplied.insert(&asset.file, &asset.bytes).is_some()
            {
                return Err(invalid(
                    "Catalog asset closure exceeds capacity or repeats a file",
                ));
            }
        }
        let revision = content_catalog::canonical(input)?;
        closure_budget(&revision, total, MAX_CLOSURE_BYTES)?;

        let mut expected = BTreeMap::<&AssetPath, &ImageLayer>::new();
        let mut layer_count = 0usize;
        for image in &revision.content.reference.payload.map_images {
            for layer in &image.layers {
                layer_count += 1;
                if layer_count > 250_000 {
                    return Err(invalid("Catalog layer mapping capacity exceeded"));
                }
                if let Some(previous) = expected.insert(&layer.file, layer)
                    && (previous.sha256 != layer.sha256
                        || previous.width_px != layer.width_px
                        || previous.height_px != layer.height_px)
                {
                    return Err(invalid("Catalog asset declarations conflict"));
                }
            }
        }
        if expected.len() != supplied.len() || expected.len() > MAX_ASSETS {
            return Err(invalid("Catalog requires its complete exact image closure"));
        }
        for (file, layer) in &expected {
            verify_png(
                supplied
                    .get(file)
                    .ok_or_else(|| invalid("Catalog image file is absent"))?,
                layer,
            )?;
        }
        let tx = self
            .conn
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .db()?;
        let outcome = content_catalog::stage_canonical(&tx, &revision)?;
        let id = content::find(&tx, &revision.key())?
            .ok_or_else(|| storage("Staged catalog identity missing"))?;
        for (file, layer) in expected {
            let bytes = supplied[file];
            let existing: Option<Vec<u8>> = tx
                .query_row(
                    "SELECT bytes FROM catalog_asset_blobs WHERE sha256=?1 AND length(bytes)<=?2",
                    params![layer.sha256.as_str(), MAX_ASSET_BYTES as i64],
                    |r| r.get(0),
                )
                .optional()
                .db()?;
            if let Some(existing) = existing {
                if existing != *bytes {
                    return Err(storage("Stored immutable asset bytes differ"));
                }
            } else {
                tx.execute(
                    "INSERT INTO catalog_asset_blobs(sha256,bytes) VALUES(?1,?2)",
                    params![layer.sha256.as_str(), bytes],
                )
                .db()?;
            }
        }
        for image in &revision.content.reference.payload.map_images {
            for layer in &image.layers {
                let previous:Option<String>=tx.query_row("SELECT sha256 FROM catalog_asset_layers WHERE revision_id=?1 AND image_id=?2 AND layer_id=?3",params![id,image.id.as_str(),layer.id.as_str()],|r|r.get(0)).optional().db()?;
                if let Some(previous) = previous {
                    if previous != layer.sha256.as_str() {
                        return Err(storage("Stored immutable layer differs"));
                    }
                } else {
                    tx.execute("INSERT INTO catalog_asset_layers(revision_id,image_id,layer_id,sha256) VALUES(?1,?2,?3,?4)",params![id,image.id.as_str(),layer.id.as_str(),layer.sha256.as_str()]).db()?;
                }
            }
        }
        let installed: i64 = tx
            .query_row(
                "SELECT count(*) FROM catalog_asset_layers WHERE revision_id=?1",
                [id],
                |r| r.get(0),
            )
            .db()?;
        if installed != layer_count as i64 {
            return Err(storage("Stored asset closure contains extra layers"));
        }
        tx.commit().db()?;
        Ok(outcome)
    }
    fn selected_image_layer(
        &self,
        scope: &ContentScope,
        request: &ImageRequest,
    ) -> Result<(i64, ImageLayer)> {
        let row = self.with_catalog_index(scope, |index| {
            let binding = index.first_page(Collection::MapImages, 1)?;
            if binding.revision != request.revision
                || binding.catalog_sha256 != request.catalog_sha256
            {
                return Err(invalid(
                    "Image request belongs to a different active catalog",
                ));
            }
            let identity = serde_json::to_string(&request.image_id)
                .map_err(|_| invalid("Invalid image identity"))?;
            index
                .find(Collection::MapImages, &identity)
                .ok_or_else(|| invalid("Image is absent from active catalog"))
        })?;
        let active = self
            .active_catalog_revision(scope)?
            .ok_or_else(|| invalid("No active catalog"))?;
        let image = active
            .reference()
            .map_images
            .get(row)
            .ok_or_else(|| storage("Catalog image index differs"))?;
        let layer = image
            .layers
            .iter()
            .find(|layer| layer.id == request.layer_id)
            .ok_or_else(|| invalid("Layer is absent from active catalog"))?
            .clone();
        let id = content::find(&self.conn, &active.key())?
            .ok_or_else(|| storage("Active catalog identity missing"))?;
        Ok((id, layer))
    }
    /// Installed-byte availability hint. Serving still rechecks bytes and PNG validity.
    pub fn catalog_image_present(
        &self,
        scope: &ContentScope,
        request: &ImageRequest,
    ) -> Result<bool> {
        let read = |store: &Self| {
            let (id, layer) = store.selected_image_layer(scope, request)?;
            let row:Option<(String,i64)>=store.conn.query_row("SELECT a.sha256,length(b.bytes) FROM catalog_asset_layers a JOIN catalog_asset_blobs b ON b.sha256=a.sha256 WHERE a.revision_id=?1 AND a.image_id=?2 AND a.layer_id=?3",params![id,request.image_id.as_str(),request.layer_id.as_str()],|r|Ok((r.get(0)?,r.get(1)?))).optional().db()?;
            match row {
                None => Ok(false),
                Some((sha, size))
                    if sha == layer.sha256.as_str()
                        && size > 0
                        && size <= MAX_ASSET_BYTES as i64 =>
                {
                    Ok(true)
                }
                _ => Err(storage("Installed catalog image metadata differs")),
            }
        };
        if self.conn.is_autocommit() {
            self.with_immediate_read(read)
        } else {
            read(self)
        }
    }
    pub fn active_catalog_image(
        &self,
        scope: &ContentScope,
        request: &ImageRequest,
    ) -> Result<Option<Vec<u8>>> {
        let read = |store: &Self| {
            let (id, layer) = store.selected_image_layer(scope, request)?;
            let row:Option<(String,Vec<u8>)>=store.conn.query_row("SELECT a.sha256,CASE WHEN length(b.bytes)<=?4 THEN b.bytes END FROM catalog_asset_layers a JOIN catalog_asset_blobs b ON b.sha256=a.sha256 WHERE a.revision_id=?1 AND a.image_id=?2 AND a.layer_id=?3",params![id,request.image_id.as_str(),request.layer_id.as_str(),MAX_ASSET_BYTES as i64],|r|Ok((r.get(0)?,r.get(1)?))).optional().db()?;
            match row {
                None => Ok(None),
                Some((sha, bytes)) => {
                    if sha != layer.sha256.as_str() {
                        return Err(storage("Installed catalog layer hash differs"));
                    }
                    verify_png(&bytes, &layer)?;
                    Ok(Some(bytes))
                }
            }
        };
        if self.conn.is_autocommit() {
            self.with_immediate_read(read)
        } else {
            read(self)
        }
    }
}

pub(crate) fn validate_revision_assets(
    conn: &rusqlite::Connection,
    id: i64,
    revision: &content::ActiveContentRevision,
) -> Result<()> {
    let count: i64 = conn
        .query_row(
            "SELECT count(*) FROM catalog_asset_layers WHERE revision_id=?1",
            [id],
            |r| r.get(0),
        )
        .db()?;
    if count == 0 {
        return Ok(());
    }
    let content = revision
        .content_v2()
        .ok_or_else(|| storage("V1 revision has unsupported asset rows"))?;
    let expected: usize = content
        .reference
        .payload
        .map_images
        .iter()
        .map(|i| i.layers.len())
        .sum();
    if expected > 250_000 || count != expected as i64 {
        return Err(storage(
            "Stored catalog asset closure is partial or oversized",
        ));
    }
    let mut files = BTreeMap::new();
    let mut total = 0usize;
    let mut verified = std::collections::BTreeSet::new();
    for image in &content.reference.payload.map_images {
        for layer in &image.layers {
            let (sha,bytes):(String,Vec<u8>)=conn.query_row("SELECT a.sha256,CASE WHEN length(b.bytes)<=?4 THEN b.bytes END FROM catalog_asset_layers a JOIN catalog_asset_blobs b ON b.sha256=a.sha256 WHERE a.revision_id=?1 AND a.image_id=?2 AND a.layer_id=?3",params![id,image.id.as_str(),layer.id.as_str(),MAX_ASSET_BYTES as i64],|r|Ok((r.get(0)?,r.get(1)?))).db()?;
            if sha != layer.sha256.as_str() {
                return Err(storage("Stored catalog asset binding differs"));
            }
            if let Some(previous) =
                files.insert(&layer.file, (sha.clone(), layer.width_px, layer.height_px))
            {
                if previous != (sha.clone(), layer.width_px, layer.height_px) {
                    return Err(storage("Stored asset declarations conflict"));
                }
            } else {
                total = total
                    .checked_add(bytes.len())
                    .ok_or_else(|| storage("Stored closure overflow"))?;
                if files.len() > MAX_ASSETS || total > MAX_CLOSURE_BYTES {
                    return Err(storage("Stored closure exceeds capacity"));
                }
            }
            if verified.insert((sha, layer.width_px.get(), layer.height_px.get())) {
                verify_png(&bytes, layer)?;
            }
        }
    }
    closure_budget(revision, total, MAX_CLOSURE_BYTES)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn closure_budget_counts_catalog_and_newline_without_allocating_assets() {
        assert!(closure_budget(&"x", 4, 8).is_ok());
        assert!(closure_budget(&"x", 5, 8).is_err());
        assert!(closure_budget(&"x", usize::MAX, 8).is_err());
    }
}
