//! Progression companions retain exact predecessor dependencies.
use super::{ContractKind, EnvelopeHeader, Payload, same_context};
use crate::{
    error::{Result, invalid},
    progression::ProgressionSnapshot,
};

impl Payload for ProgressionSnapshot {
    const KIND: ContractKind = ContractKind::ProgressionSnapshot;
    fn validate_payload(&self, header: &EnvelopeHeader) -> Result<()> {
        same_context(header, &self.binding.context)?;
        let deps = header.dependencies;
        if deps.dataset_revision.as_ref() != Some(&self.binding.source_revision)
            || deps.ruleset_version.is_some()
            || deps.score_model_version.is_some()
            || deps.snapshots.len() != 1
            || deps.snapshots[0].id != self.binding.snapshot
            || deps.snapshots[0].character != self.binding.character
            || deps.snapshots[0].context != self.binding.context
        {
            return Err(invalid(
                "Progression dependencies disagree with its snapshot binding",
            ));
        }
        if header.producer.session_id != self.capture.session_id
            || header.producer.sequence != self.capture.sequence
        {
            return Err(invalid(
                "Progression producer and capture identity disagree",
            ));
        }
        Ok(())
    }
}
