//! A reference dataset revision for one build and namespace. Local references
//! resolve inside the dataset. Not resolved here (M3 content work): quest or
//! objective IDs used only inside predicates, travel-graph cycles (legitimate),
//! and references to other datasets.
use super::{ContractKind, EnvelopeHeader, Payload, no_dependencies, same_context, unresolved};
use crate::{
    entities::*,
    error::{Result, Validate, Validated},
    types::*,
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[schemars(rename = "ReferenceDataset")]
pub struct ReferenceDatasetData {
    pub revision: DatasetRevision,
    pub evidence: Vec<EvidenceRecord>,
    pub items: Vec<ItemDefinition>,
    pub acquisitions: Vec<AcquisitionSource>,
    pub quests: Vec<QuestDefinition>,
    pub map_spaces: Vec<MapSpace>,
    pub nodes: Vec<TravelNode>,
    pub edges: Vec<TravelEdge>,
    pub objectives: Vec<Objective>,
    pub rules: Vec<ContextRule>,
}
pub type ReferenceDataset = Validated<ReferenceDatasetData>;

impl ReferenceDatasetData {
    fn contexts(&self) -> impl Iterator<Item = &DataContext> {
        let e = self.evidence.iter().map(|r| &r.context);
        let i = self.items.iter().map(|r| &r.key.context);
        let a = self.acquisitions.iter().map(|r| &r.context);
        let q = self.quests.iter().map(|r| &r.context);
        let m = self.map_spaces.iter().map(|r| &r.key.context);
        let n = self.nodes.iter().map(|r| &r.position.map.context);
        let g = self.edges.iter().map(|r| &r.context);
        let o = self.objectives.iter().map(|r| &r.context);
        let r = self.rules.iter().map(|r| &r.context);
        e.chain(i)
            .chain(a)
            .chain(q)
            .chain(m)
            .chain(n)
            .chain(g)
            .chain(o)
            .chain(r)
    }
    fn evidence_refs(&self) -> impl Iterator<Item = &EvidenceId> {
        let e = self.evidence.iter().flat_map(|r| &r.conflicts);
        let i = self.items.iter().flat_map(|r| &r.evidence);
        let a = self.acquisitions.iter().flat_map(|r| &r.evidence);
        let q = self.quests.iter().flat_map(|r| &r.evidence);
        let m = self.map_spaces.iter().flat_map(|r| &r.evidence);
        let g = self.edges.iter().flat_map(|r| &r.evidence);
        let o = self.objectives.iter().flat_map(|r| &r.evidence);
        let r = self.rules.iter().flat_map(|r| &r.evidence);
        e.chain(i)
            .chain(a)
            .chain(q)
            .chain(m)
            .chain(g)
            .chain(o)
            .chain(r)
    }
}

impl Validate for ReferenceDatasetData {
    fn validate(&self) -> Result<()> {
        for len in [
            self.evidence.len(),
            self.items.len(),
            self.acquisitions.len(),
            self.quests.len(),
            self.map_spaces.len(),
            self.nodes.len(),
            self.edges.len(),
            self.objectives.len(),
            self.rules.len(),
        ] {
            bounded(len)?;
        }
        unique(self.evidence.iter().map(|r| &r.id))?;
        unique(self.items.iter().map(|r| &r.key))?;
        unique(self.acquisitions.iter().map(|r| &r.id))?;
        unique(self.quests.iter().map(|r| &r.id))?;
        unique(self.map_spaces.iter().map(|r| &r.key))?;
        unique(self.nodes.iter().map(|r| &r.id))?;
        unique(self.edges.iter().map(|r| &r.id))?;
        unique(self.objectives.iter().map(|r| &r.id))?;
        unique(self.rules.iter().map(|r| &r.id))?;

        let evidence: BTreeSet<_> = self.evidence.iter().map(|r| &r.id).collect();
        let items: BTreeSet<_> = self.items.iter().map(|r| &r.key).collect();
        let acquisitions: BTreeSet<_> = self.acquisitions.iter().map(|r| &r.id).collect();
        let quests: BTreeMap<_, _> = self.quests.iter().map(|r| (&r.id, &r.stages)).collect();
        let maps: BTreeSet<_> = self.map_spaces.iter().map(|r| &r.key).collect();
        let nodes: BTreeMap<_, _> = self
            .nodes
            .iter()
            .map(|r| (&r.id, &r.position.map))
            .collect();
        let node_resolves = |n: &NodeRef| nodes.get(&n.id) == Some(&&n.map);

        let resolved = self.evidence_refs().all(|id| evidence.contains(id))
            && self
                .acquisitions
                .iter()
                .flat_map(|a| &a.items)
                .all(|k| items.contains(k))
            && self
                .quests
                .iter()
                .filter_map(|q| q.rewards.value())
                .flatten()
                .all(|k| items.contains(k))
            && self
                .quests
                .iter()
                .filter_map(|q| q.locations.value())
                .flatten()
                .all(|p| maps.contains(&p.map))
            && self.nodes.iter().all(|n| maps.contains(&n.position.map))
            && self
                .edges
                .iter()
                .all(|e| node_resolves(&e.from) && node_resolves(&e.to))
            && self
                .objectives
                .iter()
                .flat_map(|o| &o.targets)
                .all(|t| match t {
                    ObjectiveTarget::QuestStage { quest, stage } => quests
                        .get(quest)
                        .is_some_and(|stages| stages.contains(stage)),
                    ObjectiveTarget::Acquisition { source } => acquisitions.contains(source),
                    ObjectiveTarget::Service { node } => node_resolves(node),
                });
        if !resolved {
            return Err(unresolved("Dataset reference does not resolve"));
        }
        Ok(())
    }
}
impl Payload for ReferenceDataset {
    const KIND: ContractKind = ContractKind::ReferenceDataset;
    fn validate_payload(&self, header: &EnvelopeHeader) -> Result<()> {
        no_dependencies(header)?;
        for context in self.contexts() {
            same_context(header, context)?;
        }
        if header.fixture
            && self
                .evidence
                .iter()
                .any(|e| e.confidence == Confidence::VerifiedOnBuild)
        {
            return Err(crate::error::invalid(
                "Fixture evidence cannot claim live build verification",
            ));
        }
        Ok(())
    }
}
