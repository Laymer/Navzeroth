//! Bounded desktop pages over an immutable active catalog. No imported code runs.
use crate::{dungeon_preparation::PreparationSubject, snapshots::SnapshotApplication};
use azeroth_core::{
    catalog::{Collection, MAX_PAGE_ROWS, PageRequest},
    error::{AppError, ErrorCode, Result},
    types::{DatasetRevision, Sha256Hex},
};
use azeroth_storage::{SnapshotStore, SqliteStore, content::ActiveContentRevision};
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
fn invalid(message: &'static str) -> AppError {
    AppError::new(ErrorCode::InvalidInput, message, false)
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PageInfo {
    pub revision: DatasetRevision,
    pub catalog_sha256: Sha256Hex,
    pub collection: Collection,
    pub offset: usize,
    pub total: usize,
    pub next_offset: Option<usize>,
}
pub type Pages = BTreeMap<Collection, PageInfo>;
#[derive(Clone, Debug, Serialize)]
pub struct RecordPage {
    #[serde(flatten)]
    pub page: PageInfo,
    pub records: Vec<Value>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Request {
    subject: PreparationSubject,
    collection: Collection,
    revision: DatasetRevision,
    catalog_sha256: Sha256Hex,
    offset: usize,
}
fn value<T: Serialize>(record: &T) -> Result<Value> {
    serde_json::to_value(record).map_err(|_| invalid("Catalog record cannot serialize"))
}
fn record(content: &ActiveContentRevision, collection: Collection, index: usize) -> Result<Value> {
    let data = content.reference();
    macro_rules! row {
        ($rows:expr) => {
            $rows
                .get(index)
                .ok_or_else(|| invalid("Catalog row does not resolve"))
                .and_then(value)
        };
    }
    match collection {
        Collection::Items => row!(data.items),
        Collection::Quests => row!(data.quests),
        Collection::Acquisitions => row!(data.acquisitions),
        Collection::Nodes => row!(data.nodes),
        Collection::Edges => row!(data.edges),
        Collection::Objectives => row!(data.objectives),
        Collection::Rules => row!(data.rules),
        Collection::MapSpaces => row!(data.map_spaces),
        Collection::MapImages => row!(data.map_images),
        Collection::MapEmbeddings => row!(data.map_embeddings),
        _ => {
            let v2 = content
                .content_v2()
                .ok_or_else(|| invalid("This collection requires v2 content"))?;
            match collection {
                Collection::Models => row!(v2.models),
                Collection::Dungeons => row!(v2.dungeons),
                Collection::EquipmentFacts => row!(v2.equipment),
                Collection::QuestFacts => row!(v2.quest_facts),
                Collection::AcquisitionFacts => row!(v2.acquisition_facts),
                _ => unreachable!(),
            }
        }
    }
}
pub(crate) fn page(
    store: &SqliteStore,
    content: &ActiveContentRevision,
    collection: Collection,
    request: Option<&PageRequest>,
) -> Result<RecordPage> {
    if content.content_v2().is_some() {
        let scope = content.key().scope;
        let indexed = match request {
            Some(r) => store.catalog_page(&scope, r)?,
            None => store.catalog_first_page(&scope, collection, MAX_PAGE_ROWS)?,
        };
        if indexed.revision != *content.revision() {
            return Err(invalid("Catalog changed while loading the page"));
        }
        let records = indexed
            .rows
            .iter()
            .map(|r| record(content, collection, r.record_index))
            .collect::<Result<Vec<_>>>()?;
        return Ok(RecordPage {
            page: PageInfo {
                revision: indexed.revision,
                catalog_sha256: indexed.catalog_sha256,
                collection,
                offset: indexed.offset,
                total: indexed.total,
                next_offset: indexed.next_offset,
            },
            records,
        });
    }
    // Legacy documents retain their smaller bounds and original representation.
    let ActiveContentRevision::V1(v1) = content else {
        unreachable!()
    };
    let hash = Sha256Hex::new(format!(
        "{:x}",
        Sha256::digest(
            serde_json::to_vec(&v1.envelope)
                .map_err(|_| invalid("Legacy catalog cannot serialize"))?
        )
    ))?;
    let data = content.reference();
    let mut rows: Vec<(String, usize)> = vec![];
    macro_rules! index {
        ($records:expr,$key:ident) => {
            for (i, r) in $records.iter().enumerate() {
                rows.push((
                    serde_json::to_string(&r.$key)
                        .map_err(|_| invalid("Catalog identity cannot serialize"))?,
                    i,
                ));
            }
        };
    }
    match collection {
        Collection::Items => index!(data.items, key),
        Collection::Quests => index!(data.quests, id),
        Collection::Acquisitions => index!(data.acquisitions, id),
        Collection::Nodes => index!(data.nodes, id),
        Collection::Edges => index!(data.edges, id),
        Collection::Objectives => index!(data.objectives, id),
        Collection::Rules => index!(data.rules, id),
        Collection::MapSpaces => index!(data.map_spaces, key),
        Collection::MapImages => index!(data.map_images, id),
        Collection::MapEmbeddings => index!(data.map_embeddings, child),
        _ => {}
    }
    rows.sort_by(|a, b| a.0.cmp(&b.0));
    let offset = request.map_or(0, |r| r.offset);
    if request.is_some_and(|r| {
        r.revision != *content.revision()
            || r.catalog_sha256 != hash
            || r.collection != collection
            || r.limit != MAX_PAGE_ROWS
    }) || offset > rows.len()
    {
        return Err(invalid("Catalog page changed or offset is invalid"));
    }
    let end = offset.saturating_add(MAX_PAGE_ROWS).min(rows.len());
    let records = rows[offset..end]
        .iter()
        .map(|(_, i)| record(content, collection, *i))
        .collect::<Result<Vec<_>>>()?;
    Ok(RecordPage {
        page: PageInfo {
            revision: content.revision().clone(),
            catalog_sha256: hash,
            collection,
            offset,
            total: rows.len(),
            next_offset: (end < rows.len()).then_some(end),
        },
        records,
    })
}
pub(crate) fn first<T: DeserializeOwned>(
    store: &SqliteStore,
    content: Option<&ActiveContentRevision>,
    collection: Collection,
    pages: &mut Pages,
) -> Result<Vec<T>> {
    let Some(content) = content else {
        return Ok(vec![]);
    };
    let records = page(store, content, collection, None)?;
    pages.insert(collection, records.page);
    records
        .records
        .into_iter()
        .map(|v| serde_json::from_value(v).map_err(|_| invalid("Catalog page DTO does not match")))
        .collect()
}
impl SnapshotApplication {
    pub fn catalog_page(&self, text: &str) -> Result<RecordPage> {
        if text.len() > 4096 {
            return Err(invalid("Catalog page request exceeds bounds"));
        }
        let r: Request =
            serde_json::from_str(text).map_err(|_| invalid("Invalid catalog page request"))?;
        self.with_store_mut(|store| {
            store.with_immediate_read(|store| {
                let content = crate::authored_content::selected_catalog(store, &r.subject)?
                    .ok_or_else(|| invalid("No active catalog"))?;
                if r.collection == Collection::Objectives
                    && let PreparationSubject::Character { profile_id } = &r.subject
                {
                    let key = crate::snapshots::resolve(store, profile_id)?;
                    let snapshot = store
                        .current_snapshot(&key)?
                        .ok_or_else(|| invalid("Profile has no current snapshot"))?;
                    return crate::planning_objectives::page(
                        store,
                        &content,
                        &snapshot.payload.character,
                        Some(&PageRequest {
                            revision: r.revision,
                            catalog_sha256: r.catalog_sha256,
                            collection: r.collection,
                            offset: r.offset,
                            limit: MAX_PAGE_ROWS,
                        }),
                    );
                }
                page(
                    store,
                    &content,
                    r.collection,
                    Some(&PageRequest {
                        revision: r.revision,
                        catalog_sha256: r.catalog_sha256,
                        collection: r.collection,
                        offset: r.offset,
                        limit: MAX_PAGE_ROWS,
                    }),
                )
            })
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    #[test]
    fn legacy_pages_bind_every_collection_and_refuse_stale_or_out_of_range_requests() {
        let revision: azeroth_storage::content::ContentRevision = serde_json::from_str(
            include_str!("../../../fixtures/desktop/content/demo-revision.json"),
        )
        .unwrap();
        let content = ActiveContentRevision::V1(Arc::new(revision));
        let store = SqliteStore::open_in_memory().unwrap();
        let fixture = serde_json::to_value(content.reference()).unwrap();
        for (collection, field) in [
            (Collection::Items, "items"),
            (Collection::Quests, "quests"),
            (Collection::Acquisitions, "acquisitions"),
            (Collection::Nodes, "nodes"),
            (Collection::Edges, "edges"),
            (Collection::Objectives, "objectives"),
            (Collection::Rules, "rules"),
            (Collection::MapSpaces, "map_spaces"),
            (Collection::MapImages, "map_images"),
            (Collection::MapEmbeddings, "map_embeddings"),
        ] {
            let first = page(&store, &content, collection, None).unwrap();
            assert_eq!(first.page.collection, collection);
            // Historical documents omit optional empty imagery collections.
            let expected = match fixture.get(field) {
                Some(Value::Array(rows)) => rows.clone(),
                None => vec![],
                _ => panic!("Fixture collection must be an array"),
            };
            assert_eq!(first.page.total, expected.len());
            let mut actual: Vec<String> = first
                .records
                .iter()
                .map(|r| serde_json::to_string(r).unwrap())
                .collect();
            let mut expected: Vec<String> = expected
                .iter()
                .map(|r| serde_json::to_string(r).unwrap())
                .collect();
            actual.sort();
            expected.sort();
            assert_eq!(actual, expected);
            assert_eq!(first.records.len(), first.page.total);
            assert_eq!(first.page.next_offset, None);
            let mut request = PageRequest {
                revision: first.page.revision.clone(),
                catalog_sha256: first.page.catalog_sha256.clone(),
                collection,
                offset: 0,
                limit: MAX_PAGE_ROWS,
            };
            assert_eq!(
                page(&store, &content, collection, Some(&request))
                    .unwrap()
                    .records,
                first.records
            );
            request.offset = first.page.total + 1;
            assert!(page(&store, &content, collection, Some(&request)).is_err());
            request.offset = 0;
            request.catalog_sha256 = Sha256Hex::new("f".repeat(64)).unwrap();
            assert!(page(&store, &content, collection, Some(&request)).is_err());
            assert!(record(&content, collection, usize::MAX).is_err());
        }
    }
    #[test]
    fn every_v2_record_collection_refuses_an_unresolved_index() {
        let content =
            ActiveContentRevision::V2(Arc::new(azeroth_storage::content::ContentRevisionV2 {
                content: azeroth_core::content_v2::decode(include_bytes!(
                    "../../../fixtures/contracts/v2/reference-content.json"
                ))
                .unwrap(),
                sources: vec![],
                conflicts: vec![],
            }));
        for collection in [
            Collection::Items,
            Collection::Quests,
            Collection::Acquisitions,
            Collection::Nodes,
            Collection::Edges,
            Collection::Objectives,
            Collection::Rules,
            Collection::MapSpaces,
            Collection::MapImages,
            Collection::MapEmbeddings,
            Collection::Models,
            Collection::Dungeons,
            Collection::EquipmentFacts,
            Collection::QuestFacts,
            Collection::AcquisitionFacts,
        ] {
            assert_eq!(
                record(&content, collection, usize::MAX)
                    .unwrap_err()
                    .to_string(),
                "Catalog row does not resolve"
            );
        }
    }
}
