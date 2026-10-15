//! Immutable catalog row indexes and revision-bound pages. No storage or I/O.
use crate::{
    content_v2::ReferenceContent,
    error::{Result, Validate, invalid},
    types::{DatasetRevision, Sha256Hex},
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
pub const MAX_PAGE_ROWS: usize = 200;
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Collection {
    Items,
    Quests,
    Acquisitions,
    MapSpaces,
    Nodes,
    Edges,
    Objectives,
    Rules,
    MapImages,
    MapEmbeddings,
    Models,
    Dungeons,
    EquipmentFacts,
    QuestFacts,
    AcquisitionFacts,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PageRequest {
    pub revision: DatasetRevision,
    pub catalog_sha256: Sha256Hex,
    pub collection: Collection,
    pub offset: usize,
    pub limit: usize,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct CatalogRow {
    pub identity: String,
    pub record_index: usize,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct CatalogPage {
    pub revision: DatasetRevision,
    pub catalog_sha256: Sha256Hex,
    pub collection: Collection,
    pub total: usize,
    pub offset: usize,
    pub rows: Vec<CatalogRow>,
    pub next_offset: Option<usize>,
}
#[derive(Clone, Debug)]
pub struct CatalogIndex {
    revision: DatasetRevision,
    catalog_sha256: Sha256Hex,
    rows: BTreeMap<Collection, Vec<CatalogRow>>,
}
impl CatalogIndex {
    pub fn new(content: &ReferenceContent, catalog_sha256: Sha256Hex) -> Result<Self> {
        content.validate()?;
        let data = &content.reference.payload;
        let mut index = Self {
            revision: data.revision.clone(),
            catalog_sha256,
            rows: BTreeMap::new(),
        };
        macro_rules! add {
            ($collection:ident,$values:expr,$key:ident) => {{
                let mut rows = Vec::new();
                for (record_index, record) in $values.iter().enumerate() {
                    let identity = serde_json::to_string(&record.$key)
                        .map_err(|_| invalid("Catalog index identity cannot serialize"))?;
                    rows.push(CatalogRow {
                        identity,
                        record_index,
                    });
                }
                rows.sort_by(|a, b| a.identity.cmp(&b.identity));
                if rows.windows(2).any(|p| p[0].identity == p[1].identity) {
                    return Err(invalid("Catalog index repeats identity"));
                }
                index.rows.insert(Collection::$collection, rows);
            }};
        }
        add!(Items, data.items, key);
        add!(Quests, data.quests, id);
        add!(Acquisitions, data.acquisitions, id);
        add!(MapSpaces, data.map_spaces, key);
        add!(Nodes, data.nodes, id);
        add!(Edges, data.edges, id);
        add!(Objectives, data.objectives, id);
        add!(Rules, data.rules, id);
        add!(MapImages, data.map_images, id);
        add!(MapEmbeddings, data.map_embeddings, child);
        add!(Models, content.models, id);
        add!(Dungeons, content.dungeons, id);
        add!(EquipmentFacts, content.equipment, item);
        add!(QuestFacts, content.quest_facts, quest);
        add!(AcquisitionFacts, content.acquisition_facts, acquisition);
        Ok(index)
    }
    pub fn find(&self, collection: Collection, identity: &str) -> Option<usize> {
        let rows = self.rows.get(&collection)?;
        rows.binary_search_by(|row| row.identity.as_str().cmp(identity))
            .ok()
            .map(|i| rows[i].record_index)
    }
    pub fn first_page(&self, collection: Collection, limit: usize) -> Result<CatalogPage> {
        self.page(&PageRequest {
            revision: self.revision.clone(),
            catalog_sha256: self.catalog_sha256.clone(),
            collection,
            offset: 0,
            limit,
        })
    }
    pub fn page(&self, request: &PageRequest) -> Result<CatalogPage> {
        if request.revision != self.revision || request.catalog_sha256 != self.catalog_sha256 {
            return Err(invalid("Catalog page belongs to a different revision"));
        }
        if request.limit == 0 || request.limit > MAX_PAGE_ROWS {
            return Err(invalid("Catalog page size must be 1 through 200"));
        }
        let rows = self
            .rows
            .get(&request.collection)
            .ok_or_else(|| invalid("Catalog collection is unavailable"))?;
        if request.offset > rows.len() {
            return Err(invalid("Catalog page offset is outside collection"));
        }
        let end = request.offset.saturating_add(request.limit).min(rows.len());
        Ok(CatalogPage {
            revision: self.revision.clone(),
            catalog_sha256: self.catalog_sha256.clone(),
            collection: request.collection,
            total: rows.len(),
            offset: request.offset,
            rows: rows[request.offset..end].to_vec(),
            next_offset: (end < rows.len()).then_some(end),
        })
    }
}
