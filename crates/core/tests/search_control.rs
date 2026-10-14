#[path = "../examples/support/mod.rs"]
mod support;
use azeroth_core::{
    error::{AppError, ErrorCode},
    search_control::*,
};
struct CancelAt {
    phase: SearchPhase,
    remaining: usize,
}
impl SearchControl for CancelAt {
    fn checkpoint(&mut self, phase: SearchPhase) -> azeroth_core::error::Result<()> {
        if phase == self.phase {
            if self.remaining == 0 {
                return Err(AppError::new(
                    ErrorCode::OperationCanceled,
                    "Search canceled",
                    false,
                ));
            }
            self.remaining -= 1;
        }
        Ok(())
    }
}
#[test]
fn solo_cancellation_propagates_from_every_search_phase() {
    let f = support::Fixture::new(8, 16, 1);
    for phase in [
        SearchPhase::ValidateInputs,
        SearchPhase::EdgeAssessment,
        SearchPhase::RouteExpansion,
        SearchPhase::CandidateAssessment,
        SearchPhase::Selection,
    ] {
        for remaining in [0, 1] {
            assert_eq!(
                f.solo(4, &mut CancelAt { phase, remaining })
                    .unwrap_err()
                    .code,
                ErrorCode::OperationCanceled
            );
        }
    }
}
#[test]
fn joint_cancels_participants_and_nested_routes() {
    let f = support::Fixture::new(8, 16, 2);
    for phase in [
        SearchPhase::ParticipantReadiness,
        SearchPhase::RouteExpansion,
        SearchPhase::CandidateAssessment,
        SearchPhase::EdgeAssessment,
    ] {
        assert_eq!(
            f.joint(
                4,
                &mut CancelAt {
                    phase,
                    remaining: 3
                }
            )
            .unwrap_err()
            .code,
            ErrorCode::OperationCanceled
        );
    }
}
#[test]
fn noop_preserves_repeatable_results_and_input_order_independence() {
    let mut f = support::Fixture::new(8, 16, 2);
    let solo = serde_json::to_value(f.solo(4, &mut NoopSearchControl).unwrap()).unwrap();
    let joint = serde_json::to_value(f.joint(4, &mut NoopSearchControl).unwrap()).unwrap();
    f.nodes.reverse();
    f.edges.reverse();
    assert_eq!(
        solo,
        serde_json::to_value(f.solo(4, &mut NoopSearchControl).unwrap()).unwrap()
    );
    assert_eq!(
        joint,
        serde_json::to_value(f.joint(4, &mut NoopSearchControl).unwrap()).unwrap()
    );
}
