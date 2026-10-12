//! Route plans are declarative data for the addon. Section 7.1 limits apply.
use super::{ContractKind, EnvelopeHeader, Payload, same_context};
use crate::{
    entities::RoutePlan,
    error::{Result, invalid},
    predicate::RuleLimits,
};

pub const MAX_PLAN_STEPS: usize = 100;
pub const MAX_STEP_ALTERNATIVES: usize = 8;
pub const MAX_PLAN_RULE_DEPTH: usize = 12;

impl Payload for RoutePlan {
    const KIND: ContractKind = ContractKind::RoutePlan;
    fn validate_payload(&self, header: &EnvelopeHeader) -> Result<()> {
        same_context(header, &self.context)?;
        let deps = header.dependencies;
        if deps.dataset_revision.as_ref() != Some(&self.dataset_revision)
            || deps.ruleset_version.as_ref() != Some(&self.ruleset_version)
            || deps.score_model_version.as_ref() != Some(&self.score_model_version)
            || deps.snapshots != self.inputs
        {
            return Err(invalid("Plan dependencies disagree with the plan payload"));
        }
        if self.steps.len() > MAX_PLAN_STEPS {
            return Err(invalid("Plan exceeds the addon step limit"));
        }
        let limits = RuleLimits::new(MAX_PLAN_RULE_DEPTH, 1024, 1024)?;
        for step in &self.steps {
            if step.alternatives.len() > MAX_STEP_ALTERNATIVES {
                return Err(invalid("Plan step exceeds the alternative limit"));
            }
            for rule in [&step.prerequisite, &step.invalidation] {
                if let Some(predicate) = rule.value() {
                    predicate.validate_with(limits)?;
                }
            }
        }
        Ok(())
    }
}
