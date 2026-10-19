//! Additive v2 content contract. Never an alternate snapshot/plan decoder.
//! Context, revision and fixture identity come exclusively from `reference`.
pub mod authored;
mod json;
pub mod records;
pub mod schema;
use crate::{
    contract::{CatalogDataset, Envelope, MAX_CATALOG_IDENTITIES},
    coverage::content_report,
    eligibility::{AccessRequirements, ChoiceTaken},
    entities::{
        AcquisitionKind, CompletionSemantics, Confidence, NodeRef, ServiceKind, bounded, unique,
    },
    error::{Result, Validate, invalid},
    gear::{ProficiencyTable, RoleProfile, SlotLayout, WeightSource},
    observation::Observation,
    predicate::{Field, Predicate, PredicateNode},
    types::*,
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

/// V2 encoded catalog bound; ordinary v1 envelopes retain their smaller bound.
pub const MAX_CONTENT_BYTES: usize = 576 * 1024 * 1024;
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ReferenceContent {
    pub format: String,
    pub schema_version: u32,
    pub reference: Envelope<CatalogDataset>,
    pub models: Vec<ClassRoleModel>,
    pub equipment: Vec<EquipmentFacts>,
    pub quest_facts: Vec<QuestFacts>,
    pub acquisition_facts: Vec<AcquisitionFacts>,
    pub dungeons: Vec<Dungeon>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub node_facts: Vec<NodeFacts>,
    /// Named NPCs and game objects (ADR 0007): who stands at which nodes.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub entities: Vec<EntityFacts>,
    /// Loot templates as the source states them (ADR 0007, task V2.8d).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub loot_tables: Vec<LootTable>,
}
/// Most rows one loot table may hold; the largest generated table has 1,056.
pub const MAX_LOOT_ROWS: usize = 4096;
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum LootKind {
    Creature,
    GameObject,
    Item,
    Reference,
    Fishing,
    Pickpocketing,
    Skinning,
    Disenchant,
}
/// One loot template with every row the source gives. A row names either an
/// item or a `reference` table rolled `max_count` times; references are never
/// expanded, so the catalog stays linear in the source rows.
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct LootTable {
    /// Stable identity such as `loot:creature:448`.
    pub id: SourceId,
    pub evidence: Vec<EvidenceId>,
    pub table: LootKind,
    /// What the template belongs to (`npc:448`, `item:…`, `area:…`), when known.
    pub source: Observation<SourceId>,
    pub rows: Vec<LootRow>,
}
fn is_false(value: &bool) -> bool {
    !value
}
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct LootRow {
    /// Client item ID. It may be absent from `items` when the item table of the
    /// pack's edition lacks it; the row keeps the source's statement.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub item: Option<ItemId>,
    /// A `reference` loot table of this catalog.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub table: Option<SourceId>,
    /// Chance per kill or roll. Absent means unknown (the source leaves it
    /// implicit), never zero; a stated zero is `0`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub probability: Option<Normalized>,
    pub min_count: u32,
    pub max_count: u32,
    /// Rows of one group drop at most one item between them.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub group: Option<u32>,
    /// Drops only while a quest needs it.
    #[serde(default, skip_serializing_if = "is_false")]
    pub quest_only: bool,
    /// The source attaches a condition this contract does not express.
    #[serde(default, skip_serializing_if = "is_false")]
    pub conditional: bool,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum EntityKind {
    Npc,
    GameObject,
}
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct EntityFacts {
    /// Stable identity such as `npc:448`; referenced by acquisition origins.
    pub id: SourceId,
    pub evidence: Vec<EvidenceId>,
    pub kind: EntityKind,
    pub name: Text,
    pub subname: Observation<Text>,
    /// Lowest and highest level.
    pub levels: Observation<[Level; 2]>,
    pub services: Vec<ServiceKind>,
    /// `alliance`, `horde`, `both` or `hostile`.
    pub reaction: Observation<SourceId>,
    /// Spawn nodes in the base graph; other spawns stay outside the travel graph.
    pub nodes: Vec<NodeId>,
}
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct NodeFacts {
    pub node: NodeId,
    pub evidence: Vec<EvidenceId>,
}
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ClassRoleModel {
    pub id: SourceId,
    pub evidence: Vec<EvidenceId>,
    pub class: SourceId,
    pub minimum_level: Level,
    pub maximum_level: Level,
    pub applicability: Observation<Predicate>,
    pub profile: RoleProfile,
    pub proficiency: Observation<ProficiencyTable>,
    pub layout: Observation<SlotLayout>,
    pub exclusions: Observation<Vec<Predicate>>,
}
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct EquipmentFacts {
    pub item: ItemKey,
    pub evidence: Vec<EvidenceId>,
    pub category: Observation<SourceId>,
    pub configuration: Observation<Predicate>,
    /// Display name from the item table, when recorded.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<Text>,
}
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct QuestFacts {
    pub quest: QuestId,
    pub evidence: Vec<EvidenceId>,
    pub access: Observation<Predicate>,
    pub access_requirements: Observation<AccessRequirements>,
    pub exclusive_groups: Observation<Vec<ExclusiveBranch>>,
    pub reset: Observation<ResetRule>,
    pub shareable: Observation<bool>,
    pub sharing_conditions: Observation<Predicate>,
    /// Display title, when recorded.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<Text>,
    /// Who or what starts the quest (`npc:…`, `object:…` or `item:…`).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub givers: Vec<SourceId>,
    /// Who or what accepts the turn-in (`npc:…` or `object:…`).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub enders: Vec<SourceId>,
}
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ExclusiveBranch {
    pub quests: Vec<QuestId>,
    pub taken_when: ChoiceTaken,
}
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum ResetRule {
    Never,
    Repeatable,
    Timed { period_ms: Millis },
    ServerReset { identifier: SourceId },
}
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct AcquisitionFacts {
    pub acquisition: AcquisitionId,
    pub evidence: Vec<EvidenceId>,
    pub origin: Observation<AcquisitionOrigin>,
    pub rewards: Observation<Vec<Reward>>,
    pub costs: Observation<Vec<Cost>>,
    pub availability: Observation<Predicate>,
    pub prerequisites: Observation<Predicate>,
    pub entitlement: Observation<LootEntitlement>,
}
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum AcquisitionOrigin {
    Quest {
        quest: QuestId,
    },
    Vendor {
        entity: SourceId,
        location: NodeRef,
    },
    Craft {
        recipe: SourceId,
        profession: SourceId,
    },
    Drop {
        entity: SourceId,
        dungeon: Option<SourceId>,
    },
}
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Reward {
    pub item: ItemKey,
    pub quantity: u32,
    /// None means unconditional; matching IDs identify mutually exclusive choices.
    pub choice_group: Option<SourceId>,
}
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum Cost {
    Copper { amount: Copper },
    Item { item: ItemKey, quantity: u32 },
    Currency { currency: SourceId, quantity: u32 },
}
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum LootEntitlement {
    Individual,
    SharedRoll,
    GroupChoice,
    MasterAssigned,
}
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Dungeon {
    /// Missing preserves the historical 1024 bound; v2 allows 4096 quest/reward refs.
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "present_reference_version"
    )]
    pub reference_contract_version: Option<u32>,
    pub id: SourceId,
    pub evidence: Vec<EvidenceId>,
    pub title: Text,
    pub map: MapKey,
    pub entrance: Observation<NodeRef>,
    pub access: Observation<Predicate>,
    pub preparation_quests: Observation<Vec<QuestId>>,
    pub travel_edges: Observation<Vec<EdgeId>>,
    pub rewards: Observation<Vec<AcquisitionId>>,
    pub checklist: Observation<Vec<DungeonCheck>>,
    /// Boss entities (`npc:…` identities of `entities`).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub bosses: Vec<SourceId>,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct DungeonCheck {
    pub id: SourceId,
    pub evidence: Vec<EvidenceId>,
    pub label: Text,
    pub condition: Observation<Predicate>,
}
fn present_reference_version<'de, D: serde::Deserializer<'de>>(
    deserializer: D,
) -> std::result::Result<Option<u32>, D::Error> {
    u32::deserialize(deserializer).map(Some)
}
impl Dungeon {
    pub fn reference_limit(&self) -> Result<usize> {
        match self.reference_contract_version.unwrap_or(1) {
            1 => Ok(1024),
            2 => Ok(4096),
            _ => Err(invalid("Unsupported dungeon reference contract version")),
        }
    }
}
pub fn decode(bytes: &[u8]) -> Result<ReferenceContent> {
    if bytes.len() > MAX_CONTENT_BYTES {
        return Err(invalid("V2 reference content exceeds byte limit"));
    }
    let content: ReferenceContent =
        json::typed(bytes).map_err(|_| invalid("Invalid v2 reference content"))?;
    content.validate()?;
    Ok(content)
}
fn refs<T: Ord>(values: &[T], available: &BTreeSet<&T>) -> Result<()> {
    refs_with_limit(values, available, 1024)
}
fn refs_with_limit<T: Ord>(values: &[T], available: &BTreeSet<&T>, limit: usize) -> Result<()> {
    if values.len() > limit {
        return Err(invalid("V2 reference list exceeds contract limit"));
    }
    crate::contract::unique_catalog(values.iter())?;
    if values.iter().any(|id| !available.contains(id)) {
        return Err(invalid("V2 reference does not resolve"));
    }
    Ok(())
}
struct Graph<'a> {
    context: DataContext,
    quests: BTreeSet<&'a QuestId>,
    objectives: BTreeSet<&'a ObjectiveId>,
    items: BTreeSet<&'a ItemKey>,
    nodes: BTreeMap<&'a NodeId, &'a MapKey>,
    maps: BTreeSet<&'a MapKey>,
    conflicted: BTreeSet<&'a EvidenceId>,
    evidence: BTreeMap<&'a EvidenceId, &'a crate::entities::EvidenceRecord>,
}
impl Graph<'_> {
    fn evidence(&self, ids: &[EvidenceId]) -> Result<()> {
        bounded(ids.len())?;
        unique(ids.iter())?;
        if ids.is_empty()
            || ids.iter().any(|id| {
                self.evidence.get(id).is_none_or(|e| {
                    self.conflicted.contains(id) || e.confidence == Confidence::DisputedOrStale
                })
            })
        {
            return Err(invalid(
                "V2 provenance is missing, unresolved or conflicted",
            ));
        }
        Ok(())
    }
    fn item(&self, item: &ItemKey) -> Result<()> {
        self.context.ensure_compatible(&item.context)?;
        if !self.items.contains(item) {
            return Err(invalid("V2 item does not resolve"));
        }
        Ok(())
    }
    fn node(&self, node: &NodeRef) -> Result<()> {
        self.context.ensure_compatible(&node.map.context)?;
        if self.nodes.get(&node.id) != Some(&&node.map) {
            return Err(invalid("V2 node does not resolve"));
        }
        Ok(())
    }
    fn predicate(&self, p: &Observation<Predicate>) -> Result<()> {
        if let Some(p) = p.value() {
            self.node_predicate(p)?;
        }
        Ok(())
    }
    fn node_predicate(&self, p: &PredicateNode) -> Result<()> {
        let field = |f: &Field| match f {
            Field::Quest { quest, .. } => self.quests.contains(quest),
            Field::Objective { objective } => self.objectives.contains(objective),
            _ => true,
        };
        let valid = match p {
            PredicateNode::All { predicates } | PredicateNode::Any { predicates } => {
                for p in predicates {
                    self.node_predicate(p)?;
                }
                true
            }
            PredicateNode::Not { predicate }
            | PredicateNode::ParticipantState { predicate, .. } => {
                self.node_predicate(predicate)?;
                true
            }
            PredicateNode::Known { field: f }
            | PredicateNode::Equals { field: f, .. }
            | PredicateNode::GreaterThanOrEqual { field: f, .. } => field(f),
            PredicateNode::QuestState { quest, .. } => self.quests.contains(quest),
            PredicateNode::ObjectiveState { objective, .. } => self.objectives.contains(objective),
            PredicateNode::LocationInSupportedArea { area } => {
                self.context.ensure_compatible(&area.map.context)?;
                self.maps.contains(&area.map)
            }
            PredicateNode::EquipmentMatches { item, .. } => {
                self.item(item)?;
                true
            }
            _ => true,
        };
        if !valid {
            return Err(invalid("V2 predicate reference does not resolve"));
        }
        Ok(())
    }
}
impl ReferenceContent {
    /// Matches release canonical record bindings; evidence has a separate budget.
    pub fn identity_count(&self) -> usize {
        self.reference.payload.identity_count()
            + self.models.len()
            + self.equipment.len()
            + self.quest_facts.len()
            + self.acquisition_facts.len()
            + self.dungeons.len()
            + self.node_facts.len()
            + self.entities.len()
            + self.loot_tables.len()
    }
}
impl Validate for ReferenceContent {
    fn validate(&self) -> Result<()> {
        if self.format != "navzeroth-reference-content" || self.schema_version != 2 {
            return Err(invalid("Unsupported v2 content or party matrix version"));
        }
        crate::contract::validate_catalog_capacity(
            self.identity_count(),
            self.reference.payload.evidence.len(),
        )?;
        self.reference.validate()?;
        if !self.reference.extensions.is_empty() {
            return Err(invalid("V2 content excludes untyped exchange extensions"));
        }
        if !content_report(&self.reference).errors.is_empty() {
            return Err(invalid("V2 base graph has unresolved references or cycles"));
        }
        for n in [
            self.models.len(),
            self.equipment.len(),
            self.quest_facts.len(),
            self.acquisition_facts.len(),
            self.dungeons.len(),
            self.node_facts.len(),
            self.entities.len(),
            self.loot_tables.len(),
        ] {
            if n > MAX_CATALOG_IDENTITIES {
                return Err(invalid("V2 catalog collection capacity exceeded"));
            }
        }
        crate::contract::unique_catalog(self.models.iter().map(|r| &r.id))?;
        crate::contract::unique_catalog(self.equipment.iter().map(|r| &r.item))?;
        crate::contract::unique_catalog(self.quest_facts.iter().map(|r| &r.quest))?;
        crate::contract::unique_catalog(self.acquisition_facts.iter().map(|r| &r.acquisition))?;
        crate::contract::unique_catalog(self.dungeons.iter().map(|r| &r.id))?;
        crate::contract::unique_catalog(self.node_facts.iter().map(|r| &r.node))?;
        crate::contract::unique_catalog(self.entities.iter().map(|r| &r.id))?;
        crate::contract::unique_catalog(self.loot_tables.iter().map(|r| &r.id))?;
        let data = &self.reference.payload;
        let g = Graph {
            context: self.reference.data_context(),
            quests: data.quests.iter().map(|q| &q.id).collect(),
            objectives: data
                .objectives
                .iter()
                .map(|o| &o.id)
                .chain(data.quests.iter().flat_map(|q| &q.stages))
                .collect(),
            items: data.items.iter().map(|i| &i.key).collect(),
            nodes: data
                .nodes
                .iter()
                .map(|n| (&n.id, &n.position.map))
                .collect(),
            maps: data.map_spaces.iter().map(|m| &m.key).collect(),
            conflicted: data
                .evidence
                .iter()
                .filter(|e| !e.conflicts.is_empty())
                .map(|e| &e.id)
                .chain(data.evidence.iter().flat_map(|e| &e.conflicts))
                .collect(),
            evidence: data.evidence.iter().map(|e| (&e.id, e)).collect(),
        };
        let quest_index: BTreeMap<_, _> = data.quests.iter().map(|q| (&q.id, q)).collect();
        let acquisition_index: BTreeMap<_, _> =
            data.acquisitions.iter().map(|a| (&a.id, a)).collect();
        let acquisitions: BTreeSet<_> = acquisition_index.keys().copied().collect();
        let dungeons: BTreeSet<_> = self.dungeons.iter().map(|d| &d.id).collect();
        let edges: BTreeSet<_> = data.edges.iter().map(|e| &e.id).collect();
        for facts in &self.node_facts {
            if !g.nodes.contains_key(&facts.node) {
                return Err(invalid("V2 node fact does not resolve"));
            }
            g.evidence(&facts.evidence)?;
        }
        for entity in &self.entities {
            g.evidence(&entity.evidence)?;
            bounded(entity.services.len())?;
            bounded(entity.nodes.len())?;
            if entity.nodes.iter().any(|n| !g.nodes.contains_key(n)) {
                return Err(invalid("V2 entity node does not resolve"));
            }
            if entity.levels.value().is_some_and(|[lo, hi]| lo > hi) {
                return Err(invalid("V2 entity level interval is inverted"));
            }
        }
        let tables: BTreeMap<_, _> = self.loot_tables.iter().map(|t| (&t.id, t.table)).collect();
        for table in &self.loot_tables {
            g.evidence(&table.evidence)?;
            if table.rows.is_empty() || table.rows.len() > MAX_LOOT_ROWS {
                return Err(invalid("V2 loot table row count is outside 1..=4096"));
            }
            for row in &table.rows {
                let target = match (&row.item, &row.table) {
                    (Some(_), None) => true,
                    (None, Some(id)) => tables.get(id) == Some(&LootKind::Reference),
                    _ => false,
                };
                if !target || row.min_count > row.max_count {
                    return Err(invalid(
                        "V2 loot row needs one item or reference table and an ordered count",
                    ));
                }
            }
        }
        let entities: BTreeSet<_> = self.entities.iter().map(|e| &e.id).collect();
        for m in &self.models {
            g.evidence(&m.evidence)?;
            m.profile.validate()?;
            if m.minimum_level > m.maximum_level {
                return Err(invalid("V2 model level interval is inverted"));
            }
            g.predicate(&m.applicability)?;
            if let Some(proficiency) = m.proficiency.value() {
                proficiency.validate()?;
                if !proficiency.classes.contains_key(&m.class) {
                    return Err(invalid("V2 model class missing from proficiency"));
                }
            }
            if let Some(layout) = m.layout.value() {
                layout.validate()?;
            }
            if let Some(exclusions) = m.exclusions.value() {
                bounded(exclusions.len())?;
                for p in exclusions {
                    g.node_predicate(p)?;
                }
            }
            match &m.profile.weight_source {
                WeightSource::DocumentedModel { source }
                | WeightSource::PermittedGuide { source } => {
                    if !m
                        .evidence
                        .iter()
                        .any(|id| g.evidence.get(id).is_some_and(|e| &e.source == source))
                    {
                        return Err(invalid("V2 weight source has no matching provenance"));
                    }
                }
                WeightSource::SyntheticTest if self.reference.fixture => {}
                _ => return Err(invalid("V2 model requires documented provenance")),
            }
        }
        for e in &self.equipment {
            g.evidence(&e.evidence)?;
            g.item(&e.item)?;
            g.predicate(&e.configuration)?;
        }
        for q in &self.quest_facts {
            g.evidence(&q.evidence)?;
            refs(std::slice::from_ref(&q.quest), &g.quests)?;
            if let (Some(reset), Some(completion)) = (
                q.reset.value(),
                quest_index
                    .get(&q.quest)
                    .and_then(|base| base.completion.value()),
            ) {
                let matches = matches!(
                    (reset, completion),
                    (ResetRule::Never, CompletionSemantics::Unrepeatable)
                        | (ResetRule::Repeatable, CompletionSemantics::Repeatable)
                        | (
                            ResetRule::Timed { .. } | ResetRule::ServerReset { .. },
                            CompletionSemantics::ResetBound
                        )
                );
                if !matches {
                    return Err(invalid("V2 reset contradicts base completion semantics"));
                }
            }
            g.predicate(&q.access)?;
            g.predicate(&q.sharing_conditions)?;
            for ids in [&q.givers, &q.enders] {
                bounded(ids.len())?;
                unique(ids.iter())?;
            }
            if let Some(access) = q.access_requirements.value() {
                access.validate()?;
            }
            if let Some(groups) = q.exclusive_groups.value() {
                bounded(groups.len())?;
                for group in groups {
                    refs(&group.quests, &g.quests)?;
                    if group.quests.len() < 2 || !group.quests.contains(&q.quest) {
                        return Err(invalid(
                            "V2 exclusive group must include this quest and an alternative",
                        ));
                    }
                }
            }
            if matches!(q.reset.value(), Some(ResetRule::Timed { period_ms }) if period_ms.get() == 0)
            {
                return Err(invalid("V2 timed reset must be positive"));
            }
        }
        for a in &self.acquisition_facts {
            g.evidence(&a.evidence)?;
            refs(std::slice::from_ref(&a.acquisition), &acquisitions)?;
            g.predicate(&a.availability)?;
            g.predicate(&a.prerequisites)?;
            if let Some(origin) = a.origin.value() {
                let base = acquisition_index
                    .get(&a.acquisition)
                    .expect("resolved acquisition");
                let matches = matches!(
                    (origin, base.kind),
                    (AcquisitionOrigin::Quest { .. }, AcquisitionKind::Quest)
                        | (AcquisitionOrigin::Vendor { .. }, AcquisitionKind::Vendor)
                        | (AcquisitionOrigin::Craft { .. }, AcquisitionKind::Crafting)
                        | (
                            AcquisitionOrigin::Drop { .. },
                            AcquisitionKind::Dungeon | AcquisitionKind::Other
                        )
                );
                if !matches {
                    return Err(invalid("V2 origin contradicts base acquisition kind"));
                }
                match origin {
                    AcquisitionOrigin::Quest { quest } => {
                        refs(std::slice::from_ref(quest), &g.quests)?
                    }
                    AcquisitionOrigin::Vendor { location, .. } => g.node(location)?,
                    AcquisitionOrigin::Drop {
                        dungeon: Some(id), ..
                    } => refs(std::slice::from_ref(id), &dungeons)?,
                    _ => {}
                }
            }
            if let Some(rewards) = a.rewards.value() {
                bounded(rewards.len())?;
                unique(rewards.iter().map(|r| &r.item))?;
                let base = acquisition_index
                    .get(&a.acquisition)
                    .expect("resolved acquisition");
                for r in rewards {
                    g.item(&r.item)?;
                    if !base.items.contains(&r.item) {
                        return Err(invalid("V2 reward absent from acquisition items"));
                    }
                }
            }
            if let Some(costs) = a.costs.value() {
                bounded(costs.len())?;
                for cost in costs {
                    if let Cost::Item { item, .. } = cost {
                        g.item(item)?;
                    }
                }
            }
        }
        for d in &self.dungeons {
            let reference_limit = d.reference_limit()?;
            g.evidence(&d.evidence)?;
            g.context.ensure_compatible(&d.map.context)?;
            if !g.maps.contains(&d.map) {
                return Err(invalid("V2 dungeon map does not resolve"));
            }
            if let Some(n) = d.entrance.value() {
                g.node(n)?;
            }
            g.predicate(&d.access)?;
            if let Some(q) = d.preparation_quests.value() {
                refs_with_limit(q, &g.quests, reference_limit)?;
            }
            if let Some(e) = d.travel_edges.value() {
                refs(e, &edges)?;
            }
            if let Some(r) = d.rewards.value() {
                refs_with_limit(r, &acquisitions, reference_limit)?;
            }
            refs(&d.bosses, &entities)?;
            if let Some(checks) = d.checklist.value() {
                bounded(checks.len())?;
                unique(checks.iter().map(|c| &c.id))?;
                for c in checks {
                    g.evidence(&c.evidence)?;
                    g.predicate(&c.condition)?;
                }
            }
        }
        Ok(())
    }
}
