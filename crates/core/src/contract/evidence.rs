//! Build capability profile, evidence and provider capabilities for one build.
//! Every evidence reference resolves inside the bundle; references across
//! bundles or datasets are content-pipeline work (M3).
use super::{
    ContractKind, EnvelopeHeader, Payload, mismatch, no_dependencies, same_context, unresolved,
};
use crate::{
    entities::{Confidence, EvidenceRecord, GameBuild, ProviderCapability, bounded, unique},
    error::{Result, Validate, Validated, invalid},
    types::EvidenceId,
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[schemars(rename = "EvidenceBundle")]
pub struct EvidenceBundleData {
    pub game_build: GameBuild,
    pub evidence: Vec<EvidenceRecord>,
    pub providers: Vec<ProviderCapability>,
}
pub type EvidenceBundle = Validated<EvidenceBundleData>;
impl Validate for EvidenceBundleData {
    fn validate(&self) -> Result<()> {
        bounded(self.evidence.len())?;
        bounded(self.providers.len())?;
        unique(self.evidence.iter().map(|e| &e.id))?;
        unique(self.providers.iter().map(|p| &p.provider))?;
        let ids: BTreeSet<&EvidenceId> = self.evidence.iter().map(|e| &e.id).collect();
        let resolves = |refs: &[EvidenceId]| refs.iter().all(|id| ids.contains(id));
        let capabilities_resolve = self
            .game_build
            .capabilities
            .value()
            .is_none_or(|checks| checks.values().all(|c| resolves(&c.evidence)));
        if !capabilities_resolve
            || !self.providers.iter().all(|p| resolves(&p.evidence))
            || !self.evidence.iter().all(|e| resolves(&e.conflicts))
        {
            return Err(unresolved(
                "Evidence reference does not resolve in the bundle",
            ));
        }
        Ok(())
    }
}
impl Payload for EvidenceBundle {
    const KIND: ContractKind = ContractKind::EvidenceBundle;
    fn validate_payload(&self, header: &EnvelopeHeader) -> Result<()> {
        no_dependencies(header)?;
        if self.game_build.key != header.context.build
            || self.game_build.locale != header.game.locale
        {
            return Err(mismatch("Evidence bundle build differs from its envelope"));
        }
        for e in &self.evidence {
            same_context(header, &e.context)?;
            if header.fixture && e.confidence == Confidence::VerifiedOnBuild {
                return Err(invalid(
                    "Fixture evidence cannot claim live build verification",
                ));
            }
        }
        for p in &self.providers {
            same_context(header, &p.context)?;
        }
        Ok(())
    }
}
