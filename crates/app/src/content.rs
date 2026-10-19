//! Bounded local inspection DTOs. Checksums do not authenticate publishers.
use crate::catalog::CatalogView;
use azeroth_core::{
    coverage::ContentReport,
    error::{AppError, ErrorCode, Result},
    sources::{SourceConflict, SourceRecord},
    types::Sha256Hex,
};
use azeroth_storage::content::ContentRevisionKey;
use serde::{Deserialize, Serialize};
#[derive(Clone, Debug, Serialize)]
pub struct ContentInspection {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bundle_defaults: Option<azeroth_storage::content_preferences::BundleDefaultState>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub accepts_inferred_travel: Option<bool>,
    pub key: ContentRevisionKey,
    pub report: ContentReport,
    pub sources: Vec<SourceRecord>,
    pub conflicts: Vec<SourceConflict>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pages: Option<InspectionPages>,
}
#[derive(Clone, Debug, Serialize)]
pub struct InspectionPages {
    pub content_sha256: Sha256Hex,
    pub sources: usize,
    pub conflicts: usize,
    pub gaps: usize,
    pub errors: usize,
}
#[derive(Clone, Debug, Serialize)]
pub struct ContentRollback {
    pub changed: bool,
    pub active: Option<ContentInspection>,
}
pub(crate) fn inspect(content: impl CatalogView) -> Result<ContentInspection> {
    let mut report = content.report();
    let counts = (
        content.sources().len(),
        content.conflicts().len(),
        report.gaps.len(),
        report.errors.len(),
    );
    let pages = if content.content_v2().is_some()
        || [counts.0, counts.1, counts.2, counts.3]
            .into_iter()
            .any(|n| n > 200)
    {
        Some(InspectionPages {
            content_sha256: Sha256Hex::new(content.content_hash()?)?,
            sources: counts.0,
            conflicts: counts.1,
            gaps: counts.2,
            errors: counts.3,
        })
    } else {
        None
    };
    report.gaps.truncate(200);
    report.errors.truncate(200);
    Ok(ContentInspection {
        bundle_defaults: None,
        accepts_inferred_travel: None,
        key: content.key(),
        report,
        sources: content.sources().iter().take(200).cloned().collect(),
        conflicts: content.conflicts().iter().take(200).cloned().collect(),
        pages,
    })
}
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InspectionCollection {
    Sources,
    Conflicts,
    Gaps,
    Errors,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct InspectionRequest {
    key: ContentRevisionKey,
    content_sha256: Sha256Hex,
    collection: InspectionCollection,
    offset: usize,
}
#[derive(Serialize)]
pub struct InspectionPage {
    pub key: ContentRevisionKey,
    pub content_sha256: Sha256Hex,
    pub collection: InspectionCollection,
    pub offset: usize,
    pub total: usize,
    pub next_offset: Option<usize>,
    pub records: Vec<serde_json::Value>,
}
fn invalid(message: &'static str) -> AppError {
    AppError::new(ErrorCode::InvalidInput, message, false)
}
/// Members of one composed content set (for example the authored slice pack
/// and a generated world pack).
pub const MAX_CONTENT_SET: usize = 8;
/// Compose stored v2 revisions of one scope into one immutable revision.
///
/// Each member keeps its own evidence, sources and records; nothing is
/// relabelled or reconciled. The game context, namespace and fixture flag must
/// match. Collections are concatenated, byte-identical records are kept once,
/// and the composed catalog is decoded and validated again, so a duplicate
/// identity with different content or a dangling reference is refused. The
/// composed revision id is derived from the sorted member revisions. Imagery is
/// refused: member asset closures are not re-staged here.
pub(crate) fn compose(
    members: &[std::sync::Arc<azeroth_storage::content::ContentRevisionV2>],
) -> Result<azeroth_storage::content::ContentRevisionV2> {
    use serde_json::Value;
    use sha2::{Digest, Sha256};
    if members.len() > MAX_CONTENT_SET {
        return Err(invalid("Content set exceeds its member limit"));
    }
    // Member order never changes the composed bytes or identity.
    let mut members = members.to_vec();
    members.sort_by(|a, b| {
        a.content
            .reference
            .payload
            .revision
            .cmp(&b.content.reference.payload.revision)
    });
    let [first, rest @ ..] = members.as_slice() else {
        return Err(invalid("A content set needs at least one revision"));
    };
    let encode = |c: &azeroth_core::content_v2::ReferenceContent| {
        serde_json::to_value(c).map_err(|_| invalid("Content set member cannot be encoded"))
    };
    let header = |value: &Value| {
        let mut envelope = value["reference"].clone();
        if let Some(map) = envelope.as_object_mut() {
            for volatile in ["payload", "producer", "provenance"] {
                map.remove(volatile);
            }
        }
        (
            value["format"].clone(),
            value["schema_version"].clone(),
            envelope,
        )
    };
    let mut composed = encode(&first.content)?;
    let mut revisions = vec![first.content.reference.payload.revision.as_str().to_owned()];
    let mut sources: std::collections::BTreeMap<_, SourceRecord> = first
        .sources
        .iter()
        .map(|s| (s.id.clone(), s.clone()))
        .collect();
    let mut conflicts = first.conflicts.clone();
    fn merge(target: &mut Value, extra: &Value) -> Result<()> {
        let (Value::Object(target), Value::Object(extra)) = (target, extra) else {
            return Err(invalid("Content set member has an unexpected shape"));
        };
        for (key, value) in extra {
            match (target.get_mut(key), value) {
                (Some(Value::Array(into)), Value::Array(values)) => {
                    for value in values {
                        if !into.contains(value) {
                            into.push(value.clone());
                        }
                    }
                }
                (None, Value::Array(values)) => {
                    target.insert(key.clone(), Value::Array(values.clone()));
                }
                (Some(existing), value) if existing == value => {}
                _ => return Err(invalid("Content set members differ outside collections")),
            }
        }
        Ok(())
    }
    for member in rest {
        let value = encode(&member.content)?;
        if header(&value) != header(&composed) {
            return Err(invalid("Content set members differ in game context"));
        }
        let mut extensions = value.clone();
        let mut payload = Value::Null;
        if let Some(map) = extensions.as_object_mut() {
            payload = map.remove("reference").unwrap_or_default()["payload"].take();
            map.remove("format");
            map.remove("schema_version");
        }
        if let Some(map) = payload.as_object_mut() {
            map.remove("revision");
        }
        merge(&mut composed, &extensions)?;
        merge(&mut composed["reference"]["payload"], &payload)?;
        revisions.push(
            member
                .content
                .reference
                .payload
                .revision
                .as_str()
                .to_owned(),
        );
        for source in &member.sources {
            if sources
                .insert(source.id.clone(), source.clone())
                .is_some_and(|previous| previous != *source)
            {
                return Err(invalid(
                    "Content set members register one source differently",
                ));
            }
        }
        conflicts.extend(member.conflicts.iter().cloned());
    }
    revisions.sort();
    if revisions.windows(2).any(|w| w[0] == w[1]) {
        return Err(invalid("Content set repeats a revision"));
    }
    let digest = format!("{:x}", Sha256::digest(revisions.join("\n").as_bytes()));
    composed["reference"]["payload"]["revision"] = Value::String(format!("set-{}", &digest[..32]));
    if composed["reference"]["payload"]["map_images"]
        .as_array()
        .is_some_and(|images| !images.is_empty())
    {
        return Err(invalid("Content sets with imagery are not supported yet"));
    }
    let bytes = serde_json::to_vec(&composed).map_err(|_| invalid("Invalid content set"))?;
    Ok(azeroth_storage::content::ContentRevisionV2 {
        content: azeroth_core::content_v2::decode(&bytes)?,
        sources: sources.into_values().collect(),
        conflicts,
    })
}
impl crate::snapshots::SnapshotApplication {
    /// Activate several stored v2 revisions of the profile's scope as one
    /// composed revision (ADR 0007: the authored slice pack beside a generated
    /// world pack). A single revision is activated unchanged. With
    /// `accept_inferred_travel`, planning on exactly this active revision may
    /// use unverified edges whose evidence is all `inferred` (ADR 0007 §8);
    /// choices persist per immutable revision; rollback restores its saved choice.
    pub fn activate_content_set(
        &self,
        profile_id: &str,
        revisions: &[azeroth_core::types::DatasetRevision],
        accept_inferred_travel: bool,
    ) -> Result<ContentInspection> {
        if revisions.is_empty() || revisions.len() > MAX_CONTENT_SET {
            return Err(invalid("Content set member count is out of range"));
        }
        let key = self.with_profile_store_mut(profile_id, |store, profile| {
            let scope = azeroth_storage::content::ContentScope {
                context: profile.context.clone(),
                fixture: profile.fixture,
            };
            let mut members = vec![];
            for revision in revisions {
                let key = ContentRevisionKey {
                    scope: scope.clone(),
                    revision: revision.clone(),
                };
                match store.catalog_revision(&key)? {
                    Some(azeroth_storage::content::ActiveContentRevision::V2(content)) => {
                        members.push(content)
                    }
                    Some(_) if revisions.len() > 1 => {
                        return Err(invalid("Only v2 revisions can form a content set"));
                    }
                    Some(_) => {}
                    None => return Err(invalid("Content set revision is not stored")),
                }
            }
            let key = if revisions.len() == 1 {
                ContentRevisionKey {
                    scope,
                    revision: revisions[0].clone(),
                }
            } else {
                let composed = compose(&members)?;
                store.stage_content_v2_revision(&composed)?;
                azeroth_storage::content::ActiveContentRevision::V2(std::sync::Arc::new(composed))
                    .key()
            };
            store.activate_content_with_travel(&key, accept_inferred_travel)?;
            Ok(key)
        })?;
        let content = self
            .with_store_mut(|store| store.active_catalog_revision(&key.scope))?
            .filter(|c| c.key() == key)
            .ok_or_else(|| invalid("Content revision is not active"))?;
        inspect(content)
    }
}
impl crate::snapshots::SnapshotApplication {
    pub fn content_inspection_page(&self, text: &str) -> Result<InspectionPage> {
        if text.len() > 4096 {
            return Err(invalid("Inspection request exceeds bounds"));
        }
        let request: InspectionRequest =
            serde_json::from_str(text).map_err(|_| invalid("Invalid inspection page request"))?;
        self.with_store_mut(|store| {
            store.with_immediate_read(|store| {
                let content = store
                    .catalog_revision(&request.key)?
                    .ok_or_else(|| invalid("Inspected revision is not stored"))?;
                if content.content_hash()? != request.content_sha256.as_str() {
                    return Err(invalid("Inspected revision hash changed"));
                }
                fn rows<T: Serialize>(
                    values: &[T],
                    offset: usize,
                ) -> Result<(usize, Vec<serde_json::Value>)> {
                    if offset > values.len() {
                        return Err(invalid("Inspection offset outside revision"));
                    }
                    Ok((
                        values.len(),
                        values
                            .iter()
                            .skip(offset)
                            .take(200)
                            .map(serde_json::to_value)
                            .collect::<std::result::Result<Vec<_>, _>>()
                            .map_err(|_| invalid("Inspection record cannot serialize"))?,
                    ))
                }
                let (total, records) = match request.collection {
                    InspectionCollection::Sources => rows(content.sources(), request.offset)?,
                    InspectionCollection::Conflicts => rows(content.conflicts(), request.offset)?,
                    InspectionCollection::Gaps => rows(&content.report().gaps, request.offset)?,
                    InspectionCollection::Errors => rows(&content.report().errors, request.offset)?,
                };
                let end = request.offset + records.len();
                Ok(InspectionPage {
                    key: request.key,
                    content_sha256: request.content_sha256,
                    collection: request.collection,
                    offset: request.offset,
                    total,
                    next_offset: (end < total).then_some(end),
                    records,
                })
            })
        })
    }
    pub fn stage_verified_content_bundle(
        &self,
        root: &std::path::Path,
    ) -> Result<ContentInspection> {
        if root.as_os_str().as_encoded_bytes().len() > 4096 {
            return Err(invalid("Content pack directory exceeds bounds"));
        }
        let bundle = azeroth_ingest::pack::v2::read_verified(root).map_err(invalid)?;
        let content = azeroth_storage::content::ContentRevisionV2 {
            content: bundle.content,
            sources: bundle.sources,
            conflicts: vec![],
        };
        let assets = bundle
            .assets
            .into_iter()
            .map(|asset| azeroth_storage::catalog_assets::CatalogAsset {
                file: asset.path,
                bytes: asset.bytes,
            })
            .collect::<Vec<_>>();
        self.with_store_mut(|store| {
            store.stage_content_v2_bundle(&content, &assets)?;
            Ok(())
        })?;
        inspect(azeroth_storage::content::ActiveContentRevision::V2(
            std::sync::Arc::new(content),
        ))
    }
}

impl crate::snapshots::SnapshotApplication {
    /// Called only after the native loader verifies and stages the shipped packs.
    /// Keys may name an already validated composed revision. This API does not
    /// establish publisher authenticity or invent a shipped resource manifest.
    pub fn configure_bundled_defaults(
        &self,
        version: &str,
        keys: &[ContentRevisionKey],
    ) -> Result<Vec<bool>> {
        if keys.len() > MAX_CONTENT_SET || keys.is_empty() {
            return Err(invalid("Bundled default scope count is out of range"));
        }
        if keys
            .iter()
            .enumerate()
            .any(|(n, key)| keys[..n].iter().any(|other| other.scope == key.scope))
        {
            return Err(invalid("Bundled defaults repeat a scope"));
        }
        self.with_store_mut(|store| {
            keys.iter()
                .map(|key| store.apply_bundle_default(version, key))
                .collect()
        })
    }
}

impl crate::snapshots::SnapshotApplication {
    /// Prepare a deterministic bundle set without moving the active pointer.
    pub fn stage_content_set(&self, keys: &[ContentRevisionKey]) -> Result<ContentRevisionKey> {
        let Some(first) = keys.first() else {
            return Err(invalid("Content set is empty"));
        };
        if keys.len() > MAX_CONTENT_SET || keys.iter().any(|key| key.scope != first.scope) {
            return Err(invalid("Content set scopes differ or exceed bounds"));
        }
        self.with_store_mut(|store| {
            let mut members = Vec::new();
            for key in keys {
                match store.catalog_revision(key)? {
                    Some(azeroth_storage::content::ActiveContentRevision::V2(content)) => {
                        members.push(content)
                    }
                    _ => return Err(invalid("Bundled content set requires stored v2 revisions")),
                }
            }
            let composed = compose(&members)?;
            let key = composed.key();
            store.stage_content_v2_revision(&composed)?;
            Ok(key)
        })
    }
}
