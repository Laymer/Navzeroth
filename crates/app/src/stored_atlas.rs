//! Selected immutable catalog Atlas; no demo fallback or filesystem lookup.
use crate::{
    DataMode,
    atlas::{AtlasView, ProjectedPosition},
    snapshots::{SnapshotApplication, resolve},
};
use azeroth_core::{
    atlas::ProjectionIndex,
    catalog::{Collection, PageRequest},
    error::{AppError, ErrorCode, Result},
    types::*,
};
use azeroth_storage::content::{ActiveContentRevision, ContentScope};
use serde::{Deserialize, Serialize};
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AtlasToken {
    pub profile_id: String,
    pub revision: DatasetRevision,
    pub catalog_sha256: Sha256Hex,
}
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct AtlasPage {
    pub token: AtlasToken,
    pub offset: usize,
    pub total: usize,
    pub next_offset: Option<usize>,
}
fn invalid(message: &'static str) -> AppError {
    AppError::new(ErrorCode::InvalidInput, message, false)
}
fn selected(
    store: &azeroth_storage::SqliteStore,
    id: Option<&str>,
) -> Result<(String, ContentScope, ActiveContentRevision)> {
    let id = id.ok_or_else(|| invalid("Select an imported profile to view its active atlas"))?;
    let key = resolve(store, id)?;
    let scope = ContentScope {
        context: key.context,
        fixture: key.fixture,
    };
    let content = store
        .active_catalog_revision(&scope)?
        .ok_or_else(|| invalid("No active atlas catalog for this profile"))?;
    Ok((id.to_owned(), scope, content))
}
fn token(
    store: &azeroth_storage::SqliteStore,
    profile: String,
    scope: &ContentScope,
    content: &ActiveContentRevision,
) -> Result<AtlasToken> {
    let hash = if content.content_v2().is_some() {
        store
            .catalog_first_page(scope, Collection::MapImages, 1)?
            .catalog_sha256
    } else {
        use sha2::{Digest, Sha256};
        Sha256Hex::new(format!(
            "{:x}",
            Sha256::digest(
                serde_json::to_vec(content).map_err(|_| invalid("Catalog cannot serialize"))?
            )
        ))?
    };
    Ok(AtlasToken {
        profile_id: profile,
        revision: content.revision().clone(),
        catalog_sha256: hash,
    })
}
impl SnapshotApplication {
    pub fn atlas_markers(
        &self,
        expected: Option<&AtlasToken>,
        now: Option<UtcMillis>,
    ) -> Result<Vec<crate::atlas::SnapshotMarker>> {
        let markers = self.markers(now)?;
        if let Some(expected) = expected {
            self.with_selected_store(|store, id| {
                let (id, scope, content) = selected(store, id)?;
                if &token(store, id, &scope, &content)? != expected {
                    return Err(invalid("Selected atlas revision changed; refresh"));
                }
                Ok(())
            })?;
        }
        Ok(markers)
    }

    pub fn stored_atlas(
        &self,
        image: Option<&str>,
        expected: Option<&AtlasToken>,
        offset: usize,
    ) -> Result<AtlasView> {
        if offset > 250_000 || (offset > 0 && expected.is_none()) {
            return Err(invalid(
                "Atlas page requires exact revision and bounded offset",
            ));
        }
        self.with_selected_store(|store, id| {
            let (id, scope, content) = selected(store, id)?;
            let current = token(store, id, &scope, &content)?;
            if expected.is_some_and(|v| v != &current) {
                return Err(invalid("Selected atlas revision changed; refresh"));
            }
            let dataset = content.reference();
            let (indices, total, next) = if let Some(image) = image {
                if offset != 0 {
                    return Err(invalid("Single image cannot have page offset"));
                }
                let index = dataset
                    .map_images
                    .iter()
                    .position(|i| i.id.as_str() == image)
                    .ok_or_else(|| invalid("No such atlas image"))?;
                (vec![index], 1, None)
            } else if content.content_v2().is_some() {
                let page = store.catalog_page(
                    &scope,
                    &PageRequest {
                        revision: current.revision.clone(),
                        catalog_sha256: current.catalog_sha256.clone(),
                        collection: Collection::MapImages,
                        offset,
                        limit: 200,
                    },
                )?;
                (
                    page.rows.into_iter().map(|r| r.record_index).collect(),
                    page.total,
                    page.next_offset,
                )
            } else {
                let mut indices: Vec<_> = (0..dataset.map_images.len()).collect();
                indices.sort_by_key(|i| &dataset.map_images[*i].id);
                let total = indices.len();
                if offset > total {
                    return Err(invalid("Atlas offset outside revision"));
                }
                let indices = indices
                    .into_iter()
                    .skip(offset)
                    .take(200)
                    .collect::<Vec<_>>();
                let end = offset + indices.len();
                (indices, total, (end < total).then_some(end))
            };
            let images = indices
                .into_iter()
                .map(|i| {
                    let mut image = crate::atlas::stored_image(
                        dataset,
                        &content.context(),
                        &dataset.map_images[i],
                    )?;
                    if content.content_v2().is_some() {
                        for layer in &mut image.layers {
                            let request = azeroth_storage::catalog_assets::ImageRequest {
                                revision: current.revision.clone(),
                                catalog_sha256: current.catalog_sha256.clone(),
                                image_id: image.id.clone(),
                                layer_id: layer.id.clone(),
                            };
                            if store.catalog_image_present(&scope, &request)? {
                                layer.url = Some(asset_url(&StoredImageRequest {
                                    profile_id: current.profile_id.clone(),
                                    request,
                                })?);
                                layer.availability = crate::atlas::LayerAvailability::Available;
                            }
                        }
                    }
                    Ok(image)
                })
                .collect::<Result<_>>()?;
            Ok(AtlasView {
                data_mode: DataMode::Stored,
                fixture: content.fixture(),
                revision: content.revision().clone(),
                namespace: content.context().namespace,
                images,
                page: Some(AtlasPage {
                    token: current,
                    offset,
                    total,
                    next_offset: next,
                }),
            })
        })
    }
    pub fn stored_project_positions(
        &self,
        positions: &[Position],
        expected: &AtlasToken,
    ) -> Result<Vec<ProjectedPosition>> {
        if positions.is_empty() || positions.len() > crate::atlas::MAX_POSITIONS {
            return Err(invalid("Provide between 1 and 64 positions"));
        }
        self.with_selected_store(|store, id| {
            let (id, scope, content) = selected(store, id)?;
            if &token(store, id, &scope, &content)? != expected {
                return Err(invalid("Selected atlas revision changed; refresh"));
            }
            let index = ProjectionIndex::new(content.reference());
            Ok(positions
                .iter()
                .map(|p| match index.project(p, &content.context()) {
                    Ok(p) => ProjectedPosition::Placed(p),
                    Err(g) => ProjectedPosition::Gap(g),
                })
                .collect())
        })
    }
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct StoredImageRequest {
    profile_id: String,
    request: azeroth_storage::catalog_assets::ImageRequest,
}
fn asset_url(request: &StoredImageRequest) -> Result<String> {
    use std::fmt::Write;
    let bytes = serde_json::to_vec(request).map_err(|_| invalid("Image token cannot serialize"))?;
    if bytes.len() > 4096 {
        return Err(invalid("Image token exceeds bounds"));
    }
    let mut hex = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        let _ = write!(hex, "{byte:02x}");
    }
    let origin = if cfg!(windows) {
        "http://atlas.localhost"
    } else {
        "atlas://localhost"
    };
    Ok(format!("{origin}/stored/{hex}"))
}
impl SnapshotApplication {
    /// Exact token to immutable installed bytes. No path is interpreted as a file.
    pub fn stored_atlas_asset(&self, path: &str) -> Result<Option<Vec<u8>>> {
        let hex = path
            .strip_prefix("/stored/")
            .ok_or_else(|| invalid("Invalid stored image token"))?;
        if hex.is_empty()
            || hex.len() > 8192
            || !hex.len().is_multiple_of(2)
            || !hex
                .bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
        {
            return Err(invalid("Invalid stored image token"));
        }
        let bytes = (0..hex.len())
            .step_by(2)
            .map(|i| {
                u8::from_str_radix(&hex[i..i + 2], 16)
                    .map_err(|_| invalid("Invalid stored image token"))
            })
            .collect::<Result<Vec<_>>>()?;
        let text =
            std::str::from_utf8(&bytes).map_err(|_| invalid("Invalid stored image token"))?;
        let request: StoredImageRequest =
            serde_json::from_value(crate::dungeon_preparation::json::strict_value(text)?)
                .map_err(|_| invalid("Invalid stored image token"))?;
        self.with_selected_store(|store, selected| {
            if selected != Some(request.profile_id.as_str()) {
                return Err(invalid("Selected atlas profile changed"));
            }
            let key = resolve(store, &request.profile_id)?;
            store.active_catalog_image(
                &ContentScope {
                    context: key.context,
                    fixture: key.fixture,
                },
                &request.request,
            )
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    #[test]
    fn stored_uri_serves_verified_bytes_and_refuses_malformed_or_changed_identity() {
        use azeroth_storage::{
            catalog_assets::CatalogAsset,
            content::{ContentRevision, ContentRevisionV2},
        };
        let app = SnapshotApplication::in_memory().unwrap();
        let old: ContentRevision = serde_json::from_str(include_str!(
            "../../../fixtures/desktop/planning/synthetic-revision.json"
        ))
        .unwrap();
        let reference: serde_json::Value = serde_json::from_str(include_str!(
            "../../../fixtures/contracts/v1/reference-dataset-imagery.json"
        ))
        .unwrap();
        let document = json!({"format":"navzeroth-reference-content","schema_version":2,"reference":reference,"models":[],"equipment":[],"quest_facts":[],"acquisition_facts":[],"dungeons":[]});
        let content = ContentRevisionV2 {
            content: azeroth_core::content_v2::decode(&serde_json::to_vec(&document).unwrap())
                .unwrap(),
            sources: old.sources,
            conflicts: vec![],
        };
        let assets = [
            (
                "overview",
                include_bytes!(
                    "../../../content/demo/atlas/synthetic-continent/layers/overview.png"
                )
                .as_slice(),
            ),
            (
                "roads",
                include_bytes!("../../../content/demo/atlas/synthetic-continent/layers/roads.png")
                    .as_slice(),
            ),
            (
                "flight",
                include_bytes!("../../../content/demo/atlas/synthetic-continent/layers/flight.png")
                    .as_slice(),
            ),
        ]
        .into_iter()
        .map(|(name, bytes)| CatalogAsset {
            file: AssetPath::new(format!("atlas/synthetic-continent/layers/{name}.png")).unwrap(),
            bytes: bytes.to_vec(),
        })
        .collect::<Vec<_>>();
        app.with_store_mut(|store| {
            store.stage_content_v2_bundle(&content, &assets)?;
            store.activate_content_revision(&content.key())?;
            Ok(())
        })
        .unwrap();
        app.import_text(
            &format!(
                "ANAV1:{}",
                include_str!("../../../fixtures/contracts/v1/character-snapshot.json")
            ),
            None,
        )
        .unwrap();
        let view = app.stored_atlas(None, None, 0).unwrap();
        let layer = &view.images[0].layers[0];
        let url = layer.url.as_ref().unwrap();
        let path = &url[url.find("/stored/").unwrap()..];
        let bytes = app.stored_atlas_asset(path).unwrap().unwrap();
        use sha2::{Digest, Sha256};
        assert_eq!(
            format!("{:x}", Sha256::digest(&bytes)),
            layer.sha256.as_str()
        );
        for bad in [
            "/stored/",
            "/stored/%2f",
            "/stored/abc",
            "/stored/zz",
            "/stored/../x",
        ] {
            assert!(app.stored_atlas_asset(bad).is_err());
        }
        let mut token: StoredImageRequest = serde_json::from_slice(
            &(path
                .strip_prefix("/stored/")
                .unwrap()
                .as_bytes()
                .chunks(2)
                .map(|c| u8::from_str_radix(std::str::from_utf8(c).unwrap(), 16).unwrap())
                .collect::<Vec<_>>()),
        )
        .unwrap();
        token.request.catalog_sha256 = Sha256Hex::new("f".repeat(64)).unwrap();
        let changed = asset_url(&token).unwrap();
        assert!(
            app.stored_atlas_asset(&changed[changed.find("/stored/").unwrap()..])
                .is_err()
        );
    }
}
