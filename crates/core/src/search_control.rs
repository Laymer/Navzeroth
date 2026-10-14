//! Caller-controlled cooperative cancellation and progress for bounded searches.
//! Core performs no I/O and reads no clock. Controllers may reject a checkpoint
//! with OperationCanceled; the error propagates instead of a partial success.
use crate::error::Result;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SearchPhase {
    ValidateInputs,
    ParticipantReadiness,
    EdgeAssessment,
    RouteExpansion,
    CandidateAssessment,
    Selection,
}

pub trait SearchControl {
    fn checkpoint(&mut self, phase: SearchPhase) -> Result<()>;
}

#[derive(Default)]
pub struct NoopSearchControl;
impl SearchControl for NoopSearchControl {
    fn checkpoint(&mut self, _phase: SearchPhase) -> Result<()> {
        Ok(())
    }
}
