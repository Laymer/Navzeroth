//! Versioned private research deliveries; digest equality never grants availability.
use crate::{
    local_data::is_sha256,
    local_data_inputs::InputTuple,
    local_data_release::{AcceptanceProof, InventoryProof, SignerRole, TrustedSigner},
    release::Target,
};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Delivery {
    pub format_version: u32,
    pub tuple: InputTuple,
    pub catalog_sha256: String,
    pub projection_sha256: String,
    pub projector_profile_sha256: String,
    pub media_sha256: Vec<String>,
    pub guide_facts_sha256: Option<String>,
    pub input_sha256: Vec<String>,
    pub dependency_profile_sha256: String,
    pub dependencies_sha256: String,
    pub fixture: bool,
    pub local_only: bool,
    pub verified_for_planning: bool,
}
fn sorted_hashes(values: &[String], limit: usize) -> bool {
    values.len() <= limit
        && values.iter().all(|v| is_sha256(v))
        && values.windows(2).all(|p| p[0] < p[1])
}
impl Delivery {
    pub fn validate(&self) -> Result<(), &'static str> {
        self.tuple.validate()?;
        if self.format_version != 1
            || !self.local_only
            || self.verified_for_planning
            || [
                &self.catalog_sha256,
                &self.projection_sha256,
                &self.projector_profile_sha256,
                &self.dependency_profile_sha256,
                &self.dependencies_sha256,
            ]
            .into_iter()
            .any(|v| !is_sha256(v))
            || self
                .guide_facts_sha256
                .as_ref()
                .is_some_and(|v| !is_sha256(v))
            || !sorted_hashes(&self.media_sha256, 16)
            || !sorted_hashes(&self.input_sha256, 16)
        {
            return Err("Invalid research delivery");
        }
        Ok(())
    }
}
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Activation {
    pub format_version: u32,
    pub scope: String,
    pub generation: u64,
    pub delivery_sha256: Option<String>,
    pub verified_for_planning: bool,
}
#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum DevelopmentMatch {
    Unconfigured,
    Matched,
    Different,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ExpectedDevelopment {
    pub tuple: InputTuple,
    pub delivery_sha256: String,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct DeliveryPolicyV3 {
    pub format_version: u32,
    /// None is the pending post-launch target, never a wildcard for acceptance.
    pub target: Option<Target>,
    pub trusted_signers: BTreeMap<String, TrustedSigner>,
    pub development: Vec<ExpectedDevelopment>,
}
fn identity(s: &str) -> bool {
    !s.is_empty()
        && s.len() <= 80
        && s.bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'.'))
}
fn hex(s: &str, n: usize) -> bool {
    s.len() == n * 2
        && s.bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}
impl DeliveryPolicyV3 {
    pub fn validate(&self) -> Result<(), &'static str> {
        if self.format_version != 3
            || self.trusted_signers.len() > 64
            || self.development.len() > 64
        {
            return Err("Unsupported delivery policy");
        }
        if let Some(t) = &self.target
            && (t.edition != "wow_forever"
                || t.interface == 0
                || t.version.len() > 32
                || t.version.split('.').count() != 3
                || t.version
                    .split('.')
                    .any(|v| v.parse::<u32>().is_err() || v.len() > 1 && v.starts_with('0'))
                || t.build.parse::<u32>().is_err()
                || t.build.starts_with('0'))
        {
            return Err("Invalid frozen delivery target");
        }
        let mut keys = BTreeSet::new();
        let mut identities = BTreeSet::new();
        for (id, key) in &self.trusted_signers {
            if !identity(id)
                || !identity(&key.identity)
                || !hex(&key.public_key_hex, 32)
                || !keys.insert(&key.public_key_hex)
                || !identities.insert(&key.identity)
            {
                return Err("Invalid or duplicate independent signer");
            }
        }
        let mut tuples = BTreeSet::new();
        for expected in &self.development {
            expected.tuple.validate()?;
            let tuple = (
                &expected.tuple.product,
                &expected.tuple.version,
                expected.tuple.build,
                &expected.tuple.locale,
            );
            if !is_sha256(&expected.delivery_sha256) || !tuples.insert(tuple) {
                return Err("Invalid or duplicate development tuple");
            }
        }
        Ok(())
    }
    pub fn development_match(
        &self,
        tuple: &InputTuple,
        digest: &str,
    ) -> Result<DevelopmentMatch, &'static str> {
        self.validate()?;
        tuple.validate()?;
        if !is_sha256(digest) {
            return Err("Invalid development delivery identity");
        }
        Ok(match self.development.iter().find(|v| v.tuple == *tuple) {
            None => DevelopmentMatch::Unconfigured,
            Some(v) if v.delivery_sha256 == digest => DevelopmentMatch::Matched,
            Some(_) => DevelopmentMatch::Different,
        })
    }
}
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ReceiptPayloadV3 {
    pub format_version: u32,
    pub candidate_source_commit: String,
    pub production_sha256: String,
    pub release_policy_sha256: String,
    pub target: Target,
    pub delivery_sha256: String,
    pub binding: Delivery,
    pub independent_review_sha256: String,
    pub validation_results_sha256: String,
    pub scenario_dependencies_sha256: String,
    pub performance_workload_sha256: String,
    pub performance_results_sha256: String,
    pub domains: BTreeMap<String, InventoryProof>,
    pub acceptance: BTreeMap<String, AcceptanceProof>,
}
impl ReceiptPayloadV3 {
    pub fn validate(&self) -> Result<(), &'static str> {
        self.binding.validate()?;
        if self.format_version != 3
            || self.binding.fixture
            || !hex(&self.candidate_source_commit, 20)
            || self.target.edition != "wow_forever"
            || self.target.interface == 0
            || self.target.build != self.binding.tuple.build.to_string()
            || self.binding.tuple.version
                != format!("{}.{}", self.target.version, self.target.build)
            || [
                &self.production_sha256,
                &self.release_policy_sha256,
                &self.delivery_sha256,
                &self.independent_review_sha256,
                &self.validation_results_sha256,
                &self.scenario_dependencies_sha256,
                &self.performance_workload_sha256,
                &self.performance_results_sha256,
            ]
            .into_iter()
            .any(|v| !is_sha256(v))
        {
            return Err("Invalid future-target receipt bindings");
        }
        crate::local_data_release::validate_inventory_acceptance(&self.domains, &self.acceptance)
    }
}
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SignedReceiptV3 {
    pub format_version: u32,
    pub payload: ReceiptPayloadV3,
    pub signatures: Vec<crate::local_data_release::ReceiptSignature>,
}
/// Readiness is a separate authority: this research delivery never enables advice.
#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub struct ConsumerGuard {
    pub development_match: DevelopmentMatch,
    pub actionable_advice: bool,
    pub acceptance_ready: bool,
    pub limitations: Vec<&'static str>,
}
pub fn consumer_guard(state: DevelopmentMatch) -> ConsumerGuard {
    ConsumerGuard {
        development_match: state,
        actionable_advice: false,
        acceptance_ready: false,
        limitations: vec![
            "research_projection_not_availability_evidence",
            "relevant_build_bound_availability_evidence_required",
        ],
    }
}
/// Canonical signature order for the new format. Historical formats retain their rules.
pub const SIGNATURE_ROLES: [SignerRole; 2] = [SignerRole::Operator, SignerRole::Reviewer];
