//! Additive v2 content contract. Never an alternate snapshot/plan decoder.
//! Context, revision and fixture identity come exclusively from `reference`.
mod json;
pub mod schema;
use crate::{
    contract::{CatalogDataset, Envelope, MAX_CATALOG_IDENTITIES},
    coverage::content_report,
    eligibility::{AccessRequirements, ChoiceTaken},
    entities::{Confidence, NodeRef, bounded, unique},
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
pub const MAX_CONTENT_BYTES: usize = 256 * 1024 * 1024;
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
}
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct DungeonCheck {
    pub id: SourceId,
    pub evidence: Vec<EvidenceId>,
    pub label: Text,
    pub condition: Observation<Predicate>,
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
    bounded(values.len())?;
    unique(values.iter())?;
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
impl Validate for ReferenceContent {
    fn validate(&self) -> Result<()> {
        if self.format != "navzeroth-reference-content" || self.schema_version != 2 {
            return Err(invalid("Unsupported v2 content or party matrix version"));
        }
        crate::contract::validate_catalog_capacity(
            self.reference.payload.identity_count() + self.models.len() + self.dungeons.len(),
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
        let acquisition_index: BTreeMap<_, _> =
            data.acquisitions.iter().map(|a| (&a.id, a)).collect();
        let acquisitions: BTreeSet<_> = acquisition_index.keys().copied().collect();
        let dungeons: BTreeSet<_> = self.dungeons.iter().map(|d| &d.id).collect();
        let edges: BTreeSet<_> = data.edges.iter().map(|e| &e.id).collect();
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
            g.predicate(&q.access)?;
            g.predicate(&q.sharing_conditions)?;
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
                refs(q, &g.quests)?;
            }
            if let Some(e) = d.travel_edges.value() {
                refs(e, &edges)?;
            }
            if let Some(r) = d.rewards.value() {
                refs(r, &acquisitions)?;
            }
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
