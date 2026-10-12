//! Snapshot kinds carry validated domain snapshots unchanged. Per-field
//! provenance and each participant's timestamps are never merged or filled in.
use super::{ContractKind, EnvelopeHeader, Payload, no_dependencies, same_context};
use crate::{
    entities::{CharacterSnapshot, PartySnapshot},
    error::{Result, invalid},
};

impl Payload for CharacterSnapshot {
    const KIND: ContractKind = ContractKind::CharacterSnapshot;
    fn validate_payload(&self, header: &EnvelopeHeader) -> Result<()> {
        no_dependencies(header)?;
        same_context(header, &self.context)?;
        // A single producer exported this snapshot, so declared session facts must agree.
        let capture = &self.capture;
        if let (Some(producer), Some(captured)) = (&header.producer.session_id, &capture.session_id)
            && producer != captured
        {
            return Err(invalid("Producer and capture session IDs disagree"));
        }
        if let (Some(producer), Some(captured)) = (header.producer.sequence, capture.sequence)
            && producer != captured
        {
            return Err(invalid("Producer and capture sequences disagree"));
        }
        Ok(())
    }
}

impl Payload for PartySnapshot {
    const KIND: ContractKind = ContractKind::PartySnapshot;
    /// Participants may come from different producers; their capture metadata
    /// is checked individually by the domain record, not against the envelope.
    fn validate_payload(&self, header: &EnvelopeHeader) -> Result<()> {
        no_dependencies(header)?;
        same_context(header, &self.context)
    }
}
