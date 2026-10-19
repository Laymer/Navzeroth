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
fn dungeon_category(
    dungeon: &azeroth_core::content_v2::Dungeon,
    sources: &std::collections::BTreeMap<azeroth_core::types::SourceId, SourceRecord>,
    evidence: &std::collections::BTreeMap<
        azeroth_core::types::EvidenceId,
        azeroth_core::entities::EvidenceRecord,
    >,
) -> Option<bool> {
    use azeroth_core::entities::Confidence;
    let (mut authored, mut historical) = (false, false);
    for id in &dungeon.evidence {
        let proof = evidence.get(id)?;
        let source = sources.get(&proof.source)?;
        if !proof.conflicts.is_empty() || proof.confidence == Confidence::DisputedOrStale {
            return None;
        }
        if source
            .access
            .value()
            .is_some_and(|access| *access == azeroth_core::sources::AccessMethod::Authored)
        {
            authored = true;
        } else if proof.confidence == Confidence::Inferred {
            // Derived estimates do not outrank either source category.
        } else if proof.confidence == Confidence::CrossEditionUnverified
            && proof.origin.as_ref().map_or_else(
                || {
                    source.editions.value().is_some_and(|editions| {
                        editions.iter().any(|edition| {
                            matches!(edition.as_str(), "vanilla_1_12" | "wow_classic_era")
                        }) && editions.iter().all(|edition| {
                            edition == &proof.context.build.edition
                                || matches!(edition.as_str(), "vanilla_1_12" | "wow_classic_era")
                        })
                    })
                },
                |origin| matches!(origin.edition.as_str(), "vanilla_1_12" | "wow_classic_era"),
            )
        {
            historical = true;
        } else {
            // In particular, Forever client facts have higher precedence than
            // authored content. This narrow enrichment never overrides them.
            return None;
        }
    }
    if authored {
        Some(true)
    } else if historical {
        Some(false)
    } else {
        None
    }
}

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
        let envelope = value["reference"]
            .as_object()
            .map(|map| {
                Value::Object(
                    map.iter()
                        .filter(|(key, _)| {
                            !matches!(key.as_str(), "payload" | "producer" | "provenance")
                        })
                        .map(|(key, value)| (key.clone(), value.clone()))
                        .collect(),
                )
            })
            .unwrap_or(Value::Null);
        (
            value["format"].clone(),
            value["schema_version"].clone(),
            envelope,
        )
    };
    let mut composed = encode(&first.content)?;
    let mut revisions = vec![first.content.reference.payload.revision.as_str().to_owned()];
    let mut sources = std::collections::BTreeMap::new();
    let mut evidence = std::collections::BTreeMap::new();
    for member in &members {
        for source in &member.sources {
            if sources
                .insert(source.id.clone(), source.clone())
                .is_some_and(|old| old != *source)
            {
                return Err(invalid(
                    "Content set members register one source differently",
                ));
            }
        }
        for proof in &member.content.reference.payload.evidence {
            if evidence
                .insert(proof.id.clone(), proof.clone())
                .is_some_and(|old| old != *proof)
            {
                return Err(invalid(
                    "Content set members declare one evidence identity differently",
                ));
            }
        }
    }
    let mut conflicts = first.conflicts.clone();
    fn merge(
        target: &mut Value,
        extra: &Value,
        sources: &std::collections::BTreeMap<azeroth_core::types::SourceId, SourceRecord>,
        evidence: &std::collections::BTreeMap<
            azeroth_core::types::EvidenceId,
            azeroth_core::entities::EvidenceRecord,
        >,
        conflicts: &mut Vec<SourceConflict>,
    ) -> Result<()> {
        let (Value::Object(target), Value::Object(extra)) = (target, extra) else {
            return Err(invalid("Content set member has an unexpected shape"));
        };
        for (key, value) in extra {
            match (target.get_mut(key), value) {
                (Some(Value::Array(into)), Value::Array(values)) => {
                    if key == "dungeons" {
                        let mut indices: std::collections::BTreeMap<_, _> = into
                            .iter()
                            .enumerate()
                            .map(|(index, value)| {
                                (value["id"].as_str().unwrap_or_default().to_owned(), index)
                            })
                            .collect();
                        for value in values {
                            let id = value["id"]
                                .as_str()
                                .ok_or_else(|| invalid("Dungeon identity is missing"))?;
                            if let Some(index) = indices.get(id).copied() {
                                if into[index] == *value {
                                    continue;
                                }
                                let a: azeroth_core::content_v2::Dungeon =
                                    serde_json::from_value(into[index].clone())
                                        .map_err(|_| invalid("Invalid dungeon member"))?;
                                let b: azeroth_core::content_v2::Dungeon =
                                    serde_json::from_value(value.clone())
                                        .map_err(|_| invalid("Invalid dungeon member"))?;
                                let (authored, historical) = match (
                                    dungeon_category(&a, sources, evidence),
                                    dungeon_category(&b, sources, evidence),
                                ) {
                                    (Some(true), Some(false)) => (&a, &b),
                                    (Some(false), Some(true)) => (&b, &a),
                                    _ => {
                                        return Err(invalid(
                                            "Dungeon overlap lacks an unambiguous authored/historical precedence",
                                        ));
                                    }
                                };
                                let (merged, disagreements) =
                                    azeroth_core::dungeon_composition::enrich(
                                        authored, historical,
                                    )?;
                                for conflict in disagreements {
                                    let (preferred, others) = if conflict.generated_preferred {
                                        (&historical.evidence, &authored.evidence)
                                    } else {
                                        (&authored.evidence, &historical.evidence)
                                    };
                                    conflicts.push(SourceConflict {
                                        key: azeroth_core::sources::RecordKey::Dungeon {
                                            context: authored.map.context.clone(),
                                            dungeon: authored.id.clone(),
                                            field: conflict.field,
                                        },
                                        kind:
                                            azeroth_core::sources::ConflictKind::CategoryDowngrade,
                                        preferred: preferred[0].clone(),
                                        others: others
                                            .iter()
                                            .filter(|id| **id != preferred[0])
                                            .cloned()
                                            .collect(),
                                    });
                                }
                                into[index] = serde_json::to_value(merged)
                                    .map_err(|_| invalid("Dungeon enrichment cannot be encoded"))?;
                            } else {
                                indices.insert(id.to_owned(), into.len());
                                into.push(value.clone());
                            }
                        }
                        continue;
                    }
                    // Index hashes, not cloned records. Compare full values within
                    // each bucket so collisions never discard distinct content.
                    use std::hash::{DefaultHasher, Hash, Hasher};
                    let fingerprint = |value: &Value| {
                        let mut hasher = DefaultHasher::new();
                        value.hash(&mut hasher);
                        hasher.finish()
                    };
                    let mut positions: std::collections::HashMap<u64, Vec<usize>> =
                        std::collections::HashMap::new();
                    for (index, value) in into.iter().enumerate() {
                        positions.entry(fingerprint(value)).or_default().push(index);
                    }
                    for value in values {
                        let bucket = positions.entry(fingerprint(value)).or_default();
                        if !bucket.iter().any(|index| into[*index] == *value) {
                            bucket.push(into.len());
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
        let mut extensions = value;
        let mut payload = Value::Null;
        if let Some(map) = extensions.as_object_mut() {
            payload = map.remove("reference").unwrap_or_default()["payload"].take();
            map.remove("format");
            map.remove("schema_version");
        }
        if let Some(map) = payload.as_object_mut() {
            map.remove("revision");
        }
        merge(
            &mut composed,
            &extensions,
            &sources,
            &evidence,
            &mut conflicts,
        )?;
        merge(
            &mut composed["reference"]["payload"],
            &payload,
            &sources,
            &evidence,
            &mut conflicts,
        )?;
        revisions.push(
            member
                .content
                .reference
                .payload
                .revision
                .as_str()
                .to_owned(),
        );
        conflicts.extend(member.conflicts.iter().cloned());
    }
    revisions.sort();
    if revisions.windows(2).any(|w| w[0] == w[1]) {
        return Err(invalid("Content set repeats a revision"));
    }
    // Policy v2 binds actual composed bytes and provenance, not only member
    // labels. Streaming avoids another world-sized serialization buffer.
    struct DigestWriter<'a>(&'a mut Sha256);
    impl std::io::Write for DigestWriter<'_> {
        fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
            self.0.update(bytes);
            Ok(bytes.len())
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }
    conflicts.sort();
    if conflicts.windows(2).any(|pair| pair[0] == pair[1]) {
        return Err(invalid("Content set repeats a source conflict"));
    }
    composed["reference"]["payload"]["revision"] = Value::String(String::new());
    let mut identity = Sha256::new();
    identity.update(b"navzeroth-content-set-policy-2\n");
    serde_json::to_writer(DigestWriter(&mut identity), &revisions)
        .map_err(|_| invalid("Invalid content set identity"))?;
    serde_json::to_writer(DigestWriter(&mut identity), &composed)
        .map_err(|_| invalid("Invalid content set identity"))?;
    serde_json::to_writer(DigestWriter(&mut identity), &sources)
        .map_err(|_| invalid("Invalid content set provenance"))?;
    serde_json::to_writer(DigestWriter(&mut identity), &conflicts)
        .map_err(|_| invalid("Invalid content set conflicts"))?;
    let digest = format!("{:x}", identity.finalize());
    composed["reference"]["payload"]["revision"] = Value::String(format!("set-{}", &digest[..32]));
    if composed["reference"]["payload"]["map_images"]
        .as_array()
        .is_some_and(|images| !images.is_empty())
    {
        return Err(invalid("Content sets with imagery are not supported yet"));
    }
    let bytes = serde_json::to_vec(&composed).map_err(|_| invalid("Invalid content set"))?;
    // The strict decoder owns its parsed representation. Release the large
    // composition tree before constructing that representation.
    drop(composed);
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
    /// any other activation or revision keeps the strict default.
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
            store.activate_content_revision(&key)?;
            Ok(key)
        })?;
        self.set_inferred_travel(accept_inferred_travel.then(|| key.clone()))?;
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

#[cfg(test)]
mod composition_checks {
    use super::*;
    use std::{collections::BTreeMap, sync::Arc};

    #[test]
    fn dungeon_category_refuses_missing_conflicting_or_stale_proofs() {
        let content = azeroth_core::content_v2::decode(include_bytes!(
            "../../../fixtures/contracts/v2/reference-content.json"
        ))
        .unwrap();
        let dungeon = &content.dungeons[0];
        let proof = content.reference.payload.evidence[0].clone();
        let mut evidence = BTreeMap::from([(proof.id.clone(), proof.clone())]);
        let mut source_value: serde_json::Value = serde_json::from_str::<serde_json::Value>(
            include_str!("../../../fixtures/content-pack/sources.json"),
        )
        .unwrap()["sources"][0]
            .clone();
        source_value["id"] = serde_json::json!(proof.source);
        source_value["access"] = serde_json::json!({"status":"unknown"});
        let source: SourceRecord = serde_json::from_value(source_value.clone()).unwrap();
        let mut sources = BTreeMap::from([(source.id.clone(), source.clone())]);
        assert!(dungeon_category(dungeon, &sources, &BTreeMap::new()).is_none());
        assert!(dungeon_category(dungeon, &BTreeMap::new(), &evidence).is_none());
        let mut value = serde_json::to_value(&proof).unwrap();
        for confidence in ["disputed_or_stale", "limited_sample"] {
            value["confidence"] = serde_json::json!(confidence);
            evidence.insert(
                proof.id.clone(),
                serde_json::from_value(value.clone()).unwrap(),
            );
            assert!(dungeon_category(dungeon, &sources, &evidence).is_none());
        }
        value["confidence"] = serde_json::json!("publisher_documented");
        value["conflicts"] = serde_json::json!(["other-proof"]);
        evidence.insert(
            proof.id.clone(),
            serde_json::from_value(value.clone()).unwrap(),
        );
        assert!(dungeon_category(dungeon, &sources, &evidence).is_none());
        value["conflicts"] = serde_json::json!([]);
        value["confidence"] = serde_json::json!("inferred");
        evidence.insert(
            proof.id.clone(),
            serde_json::from_value(value.clone()).unwrap(),
        );
        assert!(dungeon_category(dungeon, &sources, &evidence).is_none());
        source_value["access"] = serde_json::json!({"status":"known","value":"authored"});
        let source: SourceRecord = serde_json::from_value(source_value).unwrap();
        sources.insert(source.id.clone(), source);
        assert_eq!(dungeon_category(dungeon, &sources, &evidence), Some(true));
    }

    #[test]
    #[ignore = "Requires explicit public full-world and authored catalog directories"]
    fn actual_world_and_authored_dungeons_compose_with_warnings_and_conflicts() {
        fn load(path: &std::path::Path) -> Arc<azeroth_storage::content::ContentRevisionV2> {
            let content = azeroth_core::content_v2::decode(
                &std::fs::read(path.join("catalog.json")).unwrap(),
            )
            .unwrap();
            let sources: serde_json::Value =
                serde_json::from_slice(&std::fs::read(path.join("sources.json")).unwrap()).unwrap();
            Arc::new(azeroth_storage::content::ContentRevisionV2 {
                content,
                sources: serde_json::from_value(sources["sources"].clone()).unwrap(),
                conflicts: vec![],
            })
        }
        let loading = std::time::Instant::now();
        let world = load(std::path::Path::new(
            &std::env::var("NAVZEROTH_WORLD_CONTENT_INPUT").unwrap(),
        ));
        let authored = load(std::path::Path::new(
            &std::env::var("NAVZEROTH_AUTHORED_CONTENT_INPUT").unwrap(),
        ));
        let expected = world.content.dungeons.len();
        println!("Public catalog load: {:?}", loading.elapsed());
        let composing = std::time::Instant::now();
        let result = compose(&[world, authored]).unwrap();
        println!("Public catalog compose: {:?}", composing.elapsed());
        assert_eq!(result.content.dungeons.len(), expected);
        let dungeon = result
            .content
            .dungeons
            .iter()
            .find(|d| d.id.as_str() == "dungeon:the-deadmines")
            .unwrap();
        assert_eq!(dungeon.map.map_id.as_str(), "instance:36");
        assert_eq!(
            dungeon.entrance.value().unwrap().id.as_str(),
            "node:dungeon-entrance:36"
        );
        assert!(
            dungeon
                .preparation_quests
                .value()
                .unwrap()
                .iter()
                .any(|q| q.as_str() == "quest:166")
        );
        assert!(
            dungeon
                .checklist
                .value()
                .unwrap()
                .iter()
                .any(|check| check.label.as_str().contains("ghostEntranceX"))
        );
        assert!(result.conflicts.iter().any(|c| matches!(
            c.key,
            azeroth_core::sources::RecordKey::Dungeon {
                field: azeroth_core::dungeon_composition::DungeonField::Map,
                ..
            }
        )));
        println!(
            "Actual public catalogs composed: {} dungeons, {} sources, {} conflicts, revision {}",
            expected,
            result.sources.len(),
            result.conflicts.len(),
            result.content.reference.payload.revision.as_str()
        );
    }
}
