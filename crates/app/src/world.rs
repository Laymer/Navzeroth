//! World browser and origin markers (ADR 0007 §3 and §9). Bounded server-side
//! search and pages over the active catalog's NPCs, objects, vendors, trainers,
//! quests and loot, plus a per-record origin summary derived from evidence →
//! source → editions and evidence confidence. Pure reads; no imported code runs.
use crate::{dungeon_preparation::PreparationSubject, snapshots::SnapshotApplication};
use azeroth_core::{
    content_v2::{AcquisitionFacts, AcquisitionOrigin, Cost, EntityKind, LootKind, Reward},
    entities::{CompletionSemantics, Confidence, ObjectiveTarget, ServiceKind},
    error::{AppError, ErrorCode, Result},
    observation::Observation,
    sources::SourceRecord,
    types::*,
};
use azeroth_storage::content::{ActiveContentRevision, ContentRevision, ContentRevisionV2};
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, HashMap},
    sync::{Arc, Mutex, Weak},
};

/// Rows per world page; each row carries bounded nested lists.
pub const WORLD_PAGE_ROWS: usize = 50;
/// Places, offers and objectives shown inside one row.
pub const MAX_ROW_LINKS: usize = 50;
/// Loot tables shown for one loot source (a creature has a few: loot,
/// pickpocketing, skinning) and rows shown for one table.
pub const MAX_SOURCE_TABLES: usize = 6;
pub const MAX_TABLE_ROWS: usize = 40;
pub const MAX_QUERY_CHARS: usize = 64;
pub const MAX_ORIGIN_RECORDS: usize = 200;
const MAX_REQUEST_BYTES: usize = 4096;
const MAX_ORIGIN_REQUEST_BYTES: usize = 128 * 1024;

fn invalid(message: &'static str) -> AppError {
    AppError::new(ErrorCode::InvalidInput, message, false)
}

/// Where a fact came from, when that is not the Forever client (ADR 0007 §3).
/// No mark at all means Forever-origin, non-inferred or unattributed evidence.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize)]
pub enum OriginMark {
    #[serde(rename = "vanilla_1_12")]
    Vanilla112,
    #[serde(rename = "wow_classic_era")]
    ClassicEra,
    #[serde(rename = "inferred")]
    Inferred,
}
const VANILLA: u8 = 1;
const CLASSIC: u8 = 2;
const INFERRED: u8 = 4;
fn marks(bits: u8) -> Vec<OriginMark> {
    [
        (VANILLA, OriginMark::Vanilla112),
        (CLASSIC, OriginMark::ClassicEra),
        (INFERRED, OriginMark::Inferred),
    ]
    .into_iter()
    .filter(|(bit, _)| bits & bit != 0)
    .map(|(_, mark)| mark)
    .collect()
}
/// Each accepted non-Forever origin a source records marks its facts; the
/// Forever edition and an unknown edition list mark nothing.
fn source_bits(source: &SourceRecord) -> u8 {
    source.editions.value().map_or(0, |editions| {
        editions.iter().fold(0, |bits, e| {
            bits | match e.as_str() {
                "vanilla_1_12" => VANILLA,
                "wow_classic_era" => CLASSIC,
                _ => 0,
            }
        })
    })
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OriginCollection {
    Items,
    Quests,
    Acquisitions,
    Edges,
    Objectives,
    Nodes,
    Dungeons,
    Entities,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WorldView {
    Entities,
    Vendors,
    Trainers,
    Quests,
    Loot,
}

/// Entities, vendors and loot sources grouped by entity identity.
struct SourceGroup {
    id: SourceId,
    entity: Option<usize>,
    facts: Vec<usize>,
    /// Indices into `loot_tables` (loot sources only).
    tables: Vec<usize>,
    key: String,
}
/// Derived lookups for one immutable content revision.
struct WorldIndex {
    evidence: HashMap<String, u8>,
    records: HashMap<(OriginCollection, String), u8>,
    entities: Vec<usize>,
    entity_keys: Vec<String>,
    entity_by_id: HashMap<String, usize>,
    trainers: Vec<usize>,
    vendors: Vec<SourceGroup>,
    loot: Vec<SourceGroup>,
    quests: Vec<usize>,
    quest_offers: HashMap<String, Vec<usize>>,
    quest_objectives: HashMap<String, Vec<usize>>,
    /// Display names by item ID, from `EquipmentFacts.name`.
    item_names: HashMap<String, Text>,
    /// `QuestFacts` index and search key by quest ID.
    quest_fact_by_id: HashMap<String, usize>,
    quest_keys: HashMap<String, String>,
    /// Dungeons (indices into `dungeons`) whose bosses include the entity.
    boss_of: HashMap<String, Vec<usize>>,
    acquisition_by_id: HashMap<String, usize>,
    node_by_id: HashMap<String, usize>,
}
fn item_identity(item: &str, variant: &str) -> String {
    format!("{item}\u{0}{variant}")
}
impl WorldIndex {
    fn bits(&self, evidence: &[EvidenceId]) -> u8 {
        evidence.iter().fold(0, |bits, id| {
            bits | self.evidence.get(id.as_str()).copied().unwrap_or(0)
        })
    }
    fn new(content: &ActiveContentRevision) -> Self {
        let sources: HashMap<&str, u8> = content
            .sources()
            .iter()
            .map(|s| (s.id.as_str(), source_bits(s)))
            .collect();
        let data = content.reference();
        let evidence: HashMap<String, u8> = data
            .evidence
            .iter()
            .map(|e| {
                let mut bits = sources.get(e.source.as_str()).copied().unwrap_or(0);
                if e.confidence == Confidence::Inferred {
                    bits |= INFERRED;
                }
                (e.id.as_str().to_owned(), bits)
            })
            .collect();
        let mut index = WorldIndex {
            evidence,
            records: HashMap::new(),
            entities: vec![],
            entity_keys: vec![],
            entity_by_id: HashMap::new(),
            trainers: vec![],
            vendors: vec![],
            loot: vec![],
            quests: vec![],
            quest_offers: HashMap::new(),
            quest_objectives: HashMap::new(),
            item_names: HashMap::new(),
            quest_fact_by_id: HashMap::new(),
            quest_keys: HashMap::new(),
            boss_of: HashMap::new(),
            acquisition_by_id: HashMap::new(),
            node_by_id: HashMap::new(),
        };
        let mut records: HashMap<(OriginCollection, String), u8> = HashMap::new();
        let mut add = |index: &WorldIndex, c: OriginCollection, id: String, e: &[EvidenceId]| {
            *records.entry((c, id)).or_default() |= index.bits(e);
        };
        for r in &data.items {
            let id = item_identity(r.key.item_id.as_str(), r.key.variant.as_str());
            add(&index, OriginCollection::Items, id, &r.evidence);
        }
        for r in &data.quests {
            add(
                &index,
                OriginCollection::Quests,
                r.id.as_str().into(),
                &r.evidence,
            );
        }
        // Projected objectives retain their source quest's origin marks.
        for quest in &data.quests {
            if !quest.evidence.is_empty() {
                for stage in &quest.stages {
                    add(
                        &index,
                        OriginCollection::Objectives,
                        crate::planning_objectives::stage_identity(&quest.id, stage),
                        &quest.evidence,
                    );
                }
            }
        }
        for (i, r) in data.acquisitions.iter().enumerate() {
            add(
                &index,
                OriginCollection::Acquisitions,
                r.id.as_str().into(),
                &r.evidence,
            );
            index.acquisition_by_id.insert(r.id.as_str().into(), i);
        }
        for r in &data.edges {
            add(
                &index,
                OriginCollection::Edges,
                r.id.as_str().into(),
                &r.evidence,
            );
        }
        for (i, r) in data.objectives.iter().enumerate() {
            add(
                &index,
                OriginCollection::Objectives,
                r.id.as_str().into(),
                &r.evidence,
            );
            for target in &r.targets {
                if let ObjectiveTarget::QuestStage { quest, .. } = target {
                    let list = index
                        .quest_objectives
                        .entry(quest.as_str().into())
                        .or_default();
                    if list.last() != Some(&i) {
                        list.push(i);
                    }
                }
            }
        }
        for (i, r) in data.nodes.iter().enumerate() {
            index.node_by_id.insert(r.id.as_str().into(), i);
        }
        let mut quests: Vec<usize> = (0..data.quests.len()).collect();
        quests.sort_by(|a, b| data.quests[*a].id.cmp(&data.quests[*b].id));
        index.quests = quests;
        if let Some(v2) = content.content_v2() {
            for r in &v2.equipment {
                let id = item_identity(r.item.item_id.as_str(), r.item.variant.as_str());
                add(&index, OriginCollection::Items, id, &r.evidence);
                if let Some(name) = &r.name {
                    index
                        .item_names
                        .entry(r.item.item_id.as_str().into())
                        .or_insert_with(|| name.clone());
                }
            }
            for (i, r) in v2.quest_facts.iter().enumerate() {
                index.quest_fact_by_id.insert(r.quest.as_str().into(), i);
                add(
                    &index,
                    OriginCollection::Quests,
                    r.quest.as_str().into(),
                    &r.evidence,
                );
            }
            for r in &v2.acquisition_facts {
                add(
                    &index,
                    OriginCollection::Acquisitions,
                    r.acquisition.as_str().into(),
                    &r.evidence,
                );
            }
            for r in &v2.node_facts {
                add(
                    &index,
                    OriginCollection::Nodes,
                    r.node.as_str().into(),
                    &r.evidence,
                );
            }
            for (i, r) in v2.dungeons.iter().enumerate() {
                add(
                    &index,
                    OriginCollection::Dungeons,
                    r.id.as_str().into(),
                    &r.evidence,
                );
                for boss in &r.bosses {
                    index
                        .boss_of
                        .entry(boss.as_str().into())
                        .or_default()
                        .push(i);
                }
            }
            let quest_keys: HashMap<String, String> = data
                .quests
                .iter()
                .map(|q| {
                    let title = index
                        .quest_fact_by_id
                        .get(q.id.as_str())
                        .and_then(|i| v2.quest_facts[*i].title.as_ref())
                        .map_or("", Text::as_str);
                    (
                        q.id.as_str().into(),
                        format!("{} {}", title, q.id.as_str()).to_lowercase(),
                    )
                })
                .collect();
            index.quest_keys = quest_keys;
            for (i, r) in v2.entities.iter().enumerate() {
                add(
                    &index,
                    OriginCollection::Entities,
                    r.id.as_str().into(),
                    &r.evidence,
                );
                index.entity_by_id.insert(r.id.as_str().into(), i);
                let subname = r.subname.value().map_or("", Text::as_str);
                index.entity_keys.push(
                    format!("{} {} {}", r.name.as_str(), subname, r.id.as_str()).to_lowercase(),
                );
            }
            let order = |a: &usize, b: &usize| {
                let (x, y) = (&v2.entities[*a], &v2.entities[*b]);
                (x.name.as_str().to_lowercase(), &x.id)
                    .cmp(&(y.name.as_str().to_lowercase(), &y.id))
            };
            let mut entities: Vec<usize> = (0..v2.entities.len()).collect();
            entities.sort_by(order);
            index.trainers = entities
                .iter()
                .copied()
                .filter(|i| v2.entities[*i].services.contains(&ServiceKind::Trainer))
                .collect();
            let mut vendors: BTreeMap<SourceId, Vec<usize>> = BTreeMap::new();
            let mut loot: BTreeMap<SourceId, (Vec<usize>, Vec<usize>)> = BTreeMap::new();
            for (i, f) in v2.acquisition_facts.iter().enumerate() {
                match f.origin.value() {
                    Some(AcquisitionOrigin::Vendor { entity, .. }) => {
                        vendors.entry(entity.clone()).or_default().push(i)
                    }
                    Some(AcquisitionOrigin::Drop { entity, .. }) => {
                        loot.entry(entity.clone()).or_default().0.push(i)
                    }
                    Some(AcquisitionOrigin::Quest { quest }) => index
                        .quest_offers
                        .entry(quest.as_str().into())
                        .or_default()
                        .push(i),
                    _ => {}
                }
            }
            for i in &entities {
                let e = &v2.entities[*i];
                if e.services.contains(&ServiceKind::Vendor) {
                    vendors.entry(e.id.clone()).or_default();
                }
            }
            // Loot templates group under the entity they belong to; a template
            // whose source is not stated is its own source.
            for (i, table) in v2.loot_tables.iter().enumerate() {
                let owner = table.source.value().unwrap_or(&table.id);
                loot.entry(owner.clone()).or_default().1.push(i);
            }
            let group = |(id, facts, tables): (SourceId, Vec<usize>, Vec<usize>)| {
                let entity = index.entity_by_id.get(id.as_str()).copied();
                let name = entity.map_or("", |i| v2.entities[i].name.as_str());
                let key = entity.map_or_else(
                    || id.as_str().to_lowercase(),
                    |i| index.entity_keys[i].clone(),
                );
                SourceGroup {
                    id,
                    entity,
                    facts,
                    tables,
                    key: format!("{}\u{0}{key}", name.to_lowercase()),
                }
            };
            let sorted = |groups: BTreeMap<SourceId, (Vec<usize>, Vec<usize>)>| {
                let mut out: Vec<SourceGroup> = groups
                    .into_iter()
                    .map(|(id, (facts, tables))| group((id, facts, tables)))
                    .collect();
                out.sort_by(|a, b| (&a.key, &a.id).cmp(&(&b.key, &b.id)));
                out
            };
            let vendors = vendors
                .into_iter()
                .map(|(id, facts)| (id, (facts, vec![])))
                .collect();
            let (vendors, loot) = (sorted(vendors), sorted(loot));
            index.vendors = vendors;
            index.loot = loot;
            index.entities = entities;
        }
        index.records = records;
        index
    }
}

/// A weak handle keeps the allocation, so a cached pointer is never reused by
/// another revision, without keeping a replaced revision's data alive.
enum Indexed {
    V1(Weak<ContentRevision>),
    V2(Weak<ContentRevisionV2>),
}
fn handle(content: &ActiveContentRevision) -> Indexed {
    match content {
        ActiveContentRevision::V1(r) => Indexed::V1(Arc::downgrade(r)),
        ActiveContentRevision::V2(r) => Indexed::V2(Arc::downgrade(r)),
    }
}
fn same(a: &Indexed, b: &ActiveContentRevision) -> bool {
    match (a, b) {
        (Indexed::V1(x), ActiveContentRevision::V1(y)) => std::ptr::eq(x.as_ptr(), Arc::as_ptr(y)),
        (Indexed::V2(x), ActiveContentRevision::V2(y)) => std::ptr::eq(x.as_ptr(), Arc::as_ptr(y)),
        _ => false,
    }
}
static INDEX: Mutex<Option<(Indexed, Arc<WorldIndex>)>> = Mutex::new(None);
/// One derived index per active revision; a new revision replaces it.
fn index(content: &ActiveContentRevision) -> Arc<WorldIndex> {
    let mut cache = INDEX
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    if let Some((cached, index)) = cache.as_ref()
        && same(cached, content)
    {
        return index.clone();
    }
    let built = Arc::new(WorldIndex::new(content));
    *cache = Some((handle(content), built.clone()));
    built
}

#[derive(Clone, Debug, Serialize)]
pub struct ItemRef {
    pub item_id: ItemId,
    pub variant: VariantId,
    /// Display name from the item table, when the pack records one.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<Text>,
}
/// A quest giver or ender: the entity (or item) a pack names, with its name
/// when the pack describes it.
#[derive(Clone, Debug, Serialize)]
pub struct EntityLink {
    pub id: SourceId,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<Text>,
}
#[derive(Clone, Debug, Serialize)]
pub struct DungeonRef {
    pub id: SourceId,
    pub title: Text,
}
/// An item named by a loot row. The raw item ID may be absent from the item
/// table, so the name is optional.
#[derive(Clone, Debug, Serialize)]
pub struct LootItem {
    pub item_id: ItemId,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<Text>,
}
/// One row of a loot template, as the source states it. An absent
/// `probability` is unknown, never zero.
#[derive(Clone, Debug, Serialize)]
pub struct LootRowView {
    pub item: Option<LootItem>,
    /// A reference table of the catalog that is rolled `max_count` times.
    pub table: Option<SourceId>,
    pub probability: Option<Normalized>,
    pub min_count: u32,
    pub max_count: u32,
    pub group: Option<u32>,
    pub quest_only: bool,
    pub conditional: bool,
}
#[derive(Clone, Debug, Serialize)]
pub struct LootTableView {
    pub id: SourceId,
    pub kind: LootKind,
    pub rows: Vec<LootRowView>,
    pub rows_total: usize,
    pub origin: Vec<OriginMark>,
}
#[derive(Clone, Debug, Serialize)]
pub struct Place {
    pub node: NodeId,
    pub map_id: MapId,
    pub floor: FloorId,
    pub x: Normalized,
    pub y: Normalized,
}
#[derive(Clone, Debug, Serialize)]
pub struct EntityRow {
    pub id: SourceId,
    pub kind: EntityKind,
    pub name: Text,
    pub subname: Observation<Text>,
    pub levels: Observation<[Level; 2]>,
    pub services: Vec<ServiceKind>,
    pub reaction: Observation<SourceId>,
    pub places: Vec<Place>,
    pub places_total: usize,
    /// Dungeons this entity is a boss of.
    pub boss_of: Vec<DungeonRef>,
    pub origin: Vec<OriginMark>,
}
/// One acquisition: a vendor offer, a drop or a quest reward.
#[derive(Clone, Debug, Serialize)]
pub struct Offer {
    pub acquisition: AcquisitionId,
    pub items: Vec<ItemRef>,
    pub cost: Observation<Copper>,
    pub costs: Observation<Vec<Cost>>,
    /// Names of the items a cost asks for, by item ID.
    pub cost_item_names: BTreeMap<ItemId, Text>,
    pub probability: Observation<Normalized>,
    pub rewards: Observation<Vec<Reward>>,
    pub location: Option<NodeId>,
    pub dungeon: Option<SourceId>,
    pub origin: Vec<OriginMark>,
}
#[derive(Clone, Debug, Serialize)]
pub struct SourceRow {
    pub id: SourceId,
    pub entity: Option<EntityRow>,
    pub offers: Vec<Offer>,
    pub offers_total: usize,
    /// Loot templates of this source (loot view only).
    pub tables: Vec<LootTableView>,
    pub tables_total: usize,
}
#[derive(Clone, Debug, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum TargetRow {
    QuestStage { stage: ObjectiveId },
    Acquisition { source: AcquisitionId },
    Service { node: NodeId },
}
#[derive(Clone, Debug, Serialize)]
pub struct ObjectiveRow {
    pub id: ObjectiveId,
    pub targets: Vec<TargetRow>,
    pub origin: Vec<OriginMark>,
}
#[derive(Clone, Debug, Serialize)]
pub struct QuestRow {
    pub id: QuestId,
    /// Display title, when the pack records one.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<Text>,
    pub givers: Vec<EntityLink>,
    pub givers_total: usize,
    pub enders: Vec<EntityLink>,
    pub enders_total: usize,
    pub stages: Vec<ObjectiveId>,
    pub objectives: Vec<ObjectiveRow>,
    pub objectives_total: usize,
    pub rewards: Observation<Vec<ItemRef>>,
    pub offers: Vec<Offer>,
    pub offers_total: usize,
    pub completion: Observation<CompletionSemantics>,
    pub origin: Vec<OriginMark>,
}
#[derive(Clone, Debug, Serialize)]
#[serde(tag = "row", rename_all = "snake_case")]
pub enum WorldRow {
    Entity(EntityRow),
    Source(SourceRow),
    Quest(QuestRow),
}
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize)]
pub struct WorldCounts {
    pub entities: usize,
    pub vendors: usize,
    pub trainers: usize,
    pub quests: usize,
    pub loot: usize,
}
#[derive(Clone, Debug, Serialize)]
pub struct WorldPage {
    pub schema_version: u32,
    /// None when no catalog is active for the subject.
    pub revision: Option<DatasetRevision>,
    pub fixture: bool,
    pub view: WorldView,
    pub query: String,
    pub offset: usize,
    pub total: usize,
    pub next_offset: Option<usize>,
    pub page_size: usize,
    pub counts: WorldCounts,
    pub rows: Vec<WorldRow>,
}
#[derive(Clone, Debug, Serialize)]
pub struct RecordOrigins {
    pub revision: DatasetRevision,
    /// Aligned with the request; None when the record does not resolve.
    pub origins: Vec<Option<Vec<OriginMark>>>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct WorldRequest {
    subject: PreparationSubject,
    view: WorldView,
    #[serde(default)]
    query: String,
    #[serde(default)]
    offset: usize,
    #[serde(default)]
    revision: Option<DatasetRevision>,
}
#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OriginRef {
    pub collection: OriginCollection,
    pub id: String,
    /// Required for items, whose identity is the item and its variant.
    #[serde(default)]
    pub variant: Option<String>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct OriginsRequest {
    subject: PreparationSubject,
    revision: DatasetRevision,
    records: Vec<OriginRef>,
}

fn map_obs<T, U: Serialize>(o: &Observation<T>, f: impl FnOnce(&T) -> U) -> Observation<U> {
    match o.value() {
        Some(v) => Observation::known(f(v), o.metadata().clone()),
        None => Observation::unavailable(o.status(), o.metadata().clone()),
    }
    .unwrap_or_else(|_| Observation::unknown())
}

struct Ctx<'a> {
    content: &'a ActiveContentRevision,
    index: &'a WorldIndex,
}
impl Ctx<'_> {
    fn item_ref(&self, key: &ItemKey) -> ItemRef {
        ItemRef {
            item_id: key.item_id.clone(),
            variant: key.variant.clone(),
            name: self.index.item_names.get(key.item_id.as_str()).cloned(),
        }
    }
    /// `npc:…` and `object:…` resolve to entities, `item:…` to item names.
    fn link(&self, id: &SourceId) -> EntityLink {
        let name = match id.as_str().strip_prefix("item:") {
            Some(item) => self.index.item_names.get(item).cloned(),
            None => self
                .index
                .entity_by_id
                .get(id.as_str())
                .and_then(|i| Some(self.content.content_v2()?.entities[*i].name.clone())),
        };
        EntityLink {
            id: id.clone(),
            name,
        }
    }
    fn links(&self, ids: &[SourceId]) -> Vec<EntityLink> {
        ids.iter()
            .take(MAX_ROW_LINKS)
            .map(|id| self.link(id))
            .collect()
    }
    fn loot_table(&self, i: usize) -> LootTableView {
        let table = &self
            .content
            .content_v2()
            .expect("loot tables exist only in v2")
            .loot_tables[i];
        LootTableView {
            id: table.id.clone(),
            kind: table.table,
            rows: table
                .rows
                .iter()
                .take(MAX_TABLE_ROWS)
                .map(|row| LootRowView {
                    item: row.item.as_ref().map(|item| LootItem {
                        item_id: item.clone(),
                        name: self.index.item_names.get(item.as_str()).cloned(),
                    }),
                    table: row.table.clone(),
                    probability: row.probability,
                    min_count: row.min_count,
                    max_count: row.max_count,
                    group: row.group,
                    quest_only: row.quest_only,
                    conditional: row.conditional,
                })
                .collect(),
            rows_total: table.rows.len(),
            origin: marks(self.index.bits(&table.evidence)),
        }
    }
    fn entity(&self, i: usize) -> EntityRow {
        let v2 = self
            .content
            .content_v2()
            .expect("entities exist only in v2");
        let e = &v2.entities[i];
        let nodes = &self.content.reference().nodes;
        let places = e
            .nodes
            .iter()
            .filter_map(|n| self.index.node_by_id.get(n.as_str()).map(|i| &nodes[*i]))
            .take(MAX_ROW_LINKS)
            .map(|n| Place {
                node: n.id.clone(),
                map_id: n.position.map.map_id.clone(),
                floor: n.position.map.floor.clone(),
                x: n.position.x,
                y: n.position.y,
            })
            .collect();
        EntityRow {
            id: e.id.clone(),
            kind: e.kind,
            name: e.name.clone(),
            subname: e.subname.clone(),
            levels: e.levels.clone(),
            services: e.services.clone(),
            reaction: e.reaction.clone(),
            places,
            places_total: e.nodes.len(),
            boss_of: self
                .index
                .boss_of
                .get(e.id.as_str())
                .into_iter()
                .flatten()
                .take(MAX_ROW_LINKS)
                .map(|d| {
                    let d = &v2.dungeons[*d];
                    DungeonRef {
                        id: d.id.clone(),
                        title: d.title.clone(),
                    }
                })
                .collect(),
            origin: marks(self.index.bits(&e.evidence)),
        }
    }
    fn offer(&self, fact: &AcquisitionFacts) -> Offer {
        let acquisition = self
            .index
            .acquisition_by_id
            .get(fact.acquisition.as_str())
            .map(|i| &self.content.reference().acquisitions[*i]);
        let mut bits = self.index.bits(&fact.evidence);
        if let Some(a) = acquisition {
            bits |= self.index.bits(&a.evidence);
        }
        let (location, dungeon) = match fact.origin.value() {
            Some(AcquisitionOrigin::Vendor { location, .. }) => (Some(location.id.clone()), None),
            Some(AcquisitionOrigin::Drop { dungeon, .. }) => (None, dungeon.clone()),
            _ => (None, None),
        };
        Offer {
            acquisition: fact.acquisition.clone(),
            items: acquisition.map_or_else(Vec::new, |a| {
                a.items.iter().map(|i| self.item_ref(i)).collect()
            }),
            cost: acquisition.map_or_else(Observation::unknown, |a| a.cost.clone()),
            costs: fact.costs.clone(),
            cost_item_names: fact
                .costs
                .value()
                .into_iter()
                .flatten()
                .filter_map(|c| match c {
                    Cost::Item { item, .. } => self
                        .index
                        .item_names
                        .get(item.item_id.as_str())
                        .map(|n| (item.item_id.clone(), n.clone())),
                    _ => None,
                })
                .collect(),
            probability: acquisition.map_or_else(Observation::unknown, |a| a.probability.clone()),
            rewards: fact.rewards.clone(),
            location,
            dungeon,
            origin: marks(bits),
        }
    }
    fn offers(&self, facts: &[usize]) -> Vec<Offer> {
        // Fact indices only exist for v2 content; legacy catalogs have none.
        let rows = self
            .content
            .content_v2()
            .map_or(&[][..], |v2| &v2.acquisition_facts[..]);
        facts
            .iter()
            .take(MAX_ROW_LINKS)
            .map(|i| self.offer(&rows[*i]))
            .collect()
    }
    fn source(&self, group: &SourceGroup) -> SourceRow {
        SourceRow {
            id: group.id.clone(),
            entity: group.entity.map(|i| self.entity(i)),
            offers: self.offers(&group.facts),
            offers_total: group.facts.len(),
            tables: group
                .tables
                .iter()
                .take(MAX_SOURCE_TABLES)
                .map(|t| self.loot_table(*t))
                .collect(),
            tables_total: group.tables.len(),
        }
    }
    fn quest(&self, i: usize) -> QuestRow {
        let data = self.content.reference();
        let q = &data.quests[i];
        let none = Vec::new();
        let objectives = self
            .index
            .quest_objectives
            .get(q.id.as_str())
            .unwrap_or(&none);
        let offers = self.index.quest_offers.get(q.id.as_str()).unwrap_or(&none);
        let facts = self
            .index
            .quest_fact_by_id
            .get(q.id.as_str())
            .and_then(|i| Some(&self.content.content_v2()?.quest_facts[*i]));
        QuestRow {
            id: q.id.clone(),
            title: facts.and_then(|f| f.title.clone()),
            givers: facts.map_or_else(Vec::new, |f| self.links(&f.givers)),
            givers_total: facts.map_or(0, |f| f.givers.len()),
            enders: facts.map_or_else(Vec::new, |f| self.links(&f.enders)),
            enders_total: facts.map_or(0, |f| f.enders.len()),
            stages: q.stages.clone(),
            objectives: objectives
                .iter()
                .take(MAX_ROW_LINKS)
                .map(|o| {
                    let o = &data.objectives[*o];
                    ObjectiveRow {
                        id: o.id.clone(),
                        targets: o
                            .targets
                            .iter()
                            .map(|t| match t {
                                ObjectiveTarget::QuestStage { stage, .. } => {
                                    TargetRow::QuestStage {
                                        stage: stage.clone(),
                                    }
                                }
                                ObjectiveTarget::Acquisition { source } => TargetRow::Acquisition {
                                    source: source.clone(),
                                },
                                ObjectiveTarget::Service { node } => TargetRow::Service {
                                    node: node.id.clone(),
                                },
                            })
                            .collect(),
                        origin: marks(self.index.bits(&o.evidence)),
                    }
                })
                .collect(),
            objectives_total: objectives.len(),
            rewards: map_obs(&q.rewards, |r| {
                r.iter().map(|i| self.item_ref(i)).collect::<Vec<_>>()
            }),
            offers: self.offers(offers),
            offers_total: offers.len(),
            completion: q.completion.clone(),
            origin: marks(
                self.index
                    .records
                    .get(&(OriginCollection::Quests, q.id.as_str().into()))
                    .copied()
                    .unwrap_or(0),
            ),
        }
    }
}

fn counts(index: &WorldIndex) -> WorldCounts {
    WorldCounts {
        entities: index.entities.len(),
        vendors: index.vendors.len(),
        trainers: index.trainers.len(),
        quests: index.quests.len(),
        loot: index.loot.len(),
    }
}
fn normalized_query(query: &str) -> Result<String> {
    if query.chars().count() > MAX_QUERY_CHARS {
        return Err(invalid("World search text exceeds 64 characters"));
    }
    Ok(query.trim().to_lowercase())
}

/// One bounded page of a world view. Search is a case-insensitive substring
/// match on names, subnames and identities, evaluated in Rust.
pub fn world_page(
    content: &ActiveContentRevision,
    view: WorldView,
    query: &str,
    offset: usize,
    revision: Option<&DatasetRevision>,
) -> Result<WorldPage> {
    if revision.is_some_and(|r| r != content.revision()) {
        return Err(invalid("World data changed; search again"));
    }
    let needle = normalized_query(query)?;
    let index = index(content);
    let ctx = Ctx {
        content,
        index: &index,
    };
    let matches = |key: &str| needle.is_empty() || key.contains(&needle);
    let data = content.reference();
    let hits: Vec<usize> = match view {
        WorldView::Entities => index
            .entities
            .iter()
            .copied()
            .filter(|i| matches(&index.entity_keys[*i]))
            .collect(),
        WorldView::Trainers => index
            .trainers
            .iter()
            .copied()
            .filter(|i| matches(&index.entity_keys[*i]))
            .collect(),
        WorldView::Vendors => (0..index.vendors.len())
            .filter(|i| matches(&index.vendors[*i].key))
            .collect(),
        WorldView::Loot => (0..index.loot.len())
            .filter(|i| matches(&index.loot[*i].key))
            .collect(),
        WorldView::Quests => index
            .quests
            .iter()
            .copied()
            .filter(|i| {
                let id = data.quests[*i].id.as_str();
                matches(
                    index
                        .quest_keys
                        .get(id)
                        .map_or(&id.to_lowercase(), |key| key),
                )
            })
            .collect(),
    };
    if offset > hits.len() {
        return Err(invalid("World page offset is invalid"));
    }
    let end = offset.saturating_add(WORLD_PAGE_ROWS).min(hits.len());
    let rows = hits[offset..end]
        .iter()
        .map(|i| match view {
            WorldView::Entities | WorldView::Trainers => WorldRow::Entity(ctx.entity(*i)),
            WorldView::Vendors => WorldRow::Source(ctx.source(&index.vendors[*i])),
            WorldView::Loot => WorldRow::Source(ctx.source(&index.loot[*i])),
            WorldView::Quests => WorldRow::Quest(ctx.quest(*i)),
        })
        .collect();
    Ok(WorldPage {
        schema_version: 1,
        revision: Some(content.revision().clone()),
        fixture: content.fixture(),
        view,
        query: query.to_owned(),
        offset,
        total: hits.len(),
        next_offset: (end < hits.len()).then_some(end),
        page_size: WORLD_PAGE_ROWS,
        counts: counts(&index),
        rows,
    })
}
fn empty_page(view: WorldView, query: &str) -> Result<WorldPage> {
    normalized_query(query)?;
    Ok(WorldPage {
        schema_version: 1,
        revision: None,
        fixture: false,
        view,
        query: query.to_owned(),
        offset: 0,
        total: 0,
        next_offset: None,
        page_size: WORLD_PAGE_ROWS,
        counts: WorldCounts::default(),
        rows: vec![],
    })
}
pub fn record_origins(
    content: &ActiveContentRevision,
    revision: &DatasetRevision,
    records: &[OriginRef],
) -> Result<RecordOrigins> {
    if revision != content.revision() {
        return Err(invalid("Catalog changed; refresh this view"));
    }
    if records.len() > MAX_ORIGIN_RECORDS {
        return Err(invalid("Origin request exceeds 200 records"));
    }
    let index = index(content);
    let origins = records
        .iter()
        .map(|r| {
            let id = match (r.collection, &r.variant) {
                (OriginCollection::Items, Some(variant)) => item_identity(&r.id, variant),
                (OriginCollection::Items, None) => return None,
                _ => r.id.clone(),
            };
            index
                .records
                .get(&(r.collection, id))
                .map(|bits| marks(*bits))
        })
        .collect();
    Ok(RecordOrigins {
        revision: revision.clone(),
        origins,
    })
}

impl SnapshotApplication {
    pub fn world_page(&self, text: &str) -> Result<WorldPage> {
        if text.len() > MAX_REQUEST_BYTES {
            return Err(invalid("World page request exceeds bounds"));
        }
        let r: WorldRequest =
            serde_json::from_str(text).map_err(|_| invalid("Invalid world page request"))?;
        let content = self.with_store_mut(|store| {
            store.with_immediate_read(|store| {
                crate::authored_content::selected_catalog(store, &r.subject)
            })
        })?;
        match content {
            Some(content) => world_page(&content, r.view, &r.query, r.offset, r.revision.as_ref()),
            None if r.revision.is_some() || r.offset > 0 => {
                Err(invalid("World data changed; search again"))
            }
            None => empty_page(r.view, &r.query),
        }
    }
    pub fn record_origins(&self, text: &str) -> Result<RecordOrigins> {
        if text.len() > MAX_ORIGIN_REQUEST_BYTES {
            return Err(invalid("Origin request exceeds bounds"));
        }
        let r: OriginsRequest =
            serde_json::from_str(text).map_err(|_| invalid("Invalid origin request"))?;
        let content = self
            .with_store_mut(|store| {
                store.with_immediate_read(|store| {
                    crate::authored_content::selected_catalog(store, &r.subject)
                })
            })?
            .ok_or_else(|| invalid("No active catalog"))?;
        record_origins(&content, &r.revision, &r.records)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use azeroth_core::observation::ObservationStatus;
    fn legacy() -> ActiveContentRevision {
        ActiveContentRevision::V1(Arc::new(
            serde_json::from_str(include_str!(
                "../../../fixtures/desktop/content/demo-revision.json"
            ))
            .unwrap(),
        ))
    }
    #[test]
    fn legacy_catalogs_list_quests_and_have_no_entities() {
        let content = legacy();
        for view in [
            WorldView::Entities,
            WorldView::Vendors,
            WorldView::Trainers,
            WorldView::Loot,
        ] {
            let page = world_page(&content, view, "", 0, None).unwrap();
            assert_eq!(page.total, 0);
        }
        let quests = world_page(&content, WorldView::Quests, "", 0, None).unwrap();
        assert_eq!(quests.counts.quests, content.reference().quests.len());
        let WorldRow::Quest(row) = &quests.rows[0] else {
            panic!("quest row expected")
        };
        assert!(row.offers.is_empty());
        let other = legacy();
        assert!(!same(&handle(&content), &other));
        assert!(same(&handle(&content), &content));
    }
    #[test]
    fn marks_and_observation_mapping_keep_status_distinct() {
        assert_eq!(
            marks(VANILLA | CLASSIC | INFERRED),
            [
                OriginMark::Vanilla112,
                OriginMark::ClassicEra,
                OriginMark::Inferred
            ]
        );
        assert!(marks(0).is_empty());
        let restricted: Observation<u32> =
            Observation::unavailable(ObservationStatus::Restricted, Default::default()).unwrap();
        assert_eq!(
            map_obs(&restricted, |v| v + 1).status(),
            ObservationStatus::Restricted
        );
        assert!(empty_page(WorldView::Loot, &"z".repeat(65)).is_err());
    }
}
