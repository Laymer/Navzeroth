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

#[test]
fn route_legacy_wrapper_matches_controlled_and_final_checkpoint_can_cancel() {
    use azeroth_core::travel::{RouteOptions, shortest_route, shortest_route_controlled};
    use azeroth_core::types::NodeId;
    let f = support::Fixture::new(8, 16, 1);
    let contexts = f.contexts();
    let origin = NodeId::new("n0000").unwrap();
    let target = NodeId::new("n0004").unwrap();
    let options = RouteOptions::default();
    let legacy =
        shortest_route(&f.nodes, &f.edges, &origin, &target, &contexts[0], &options).unwrap();
    let controlled = shortest_route_controlled(
        &f.nodes,
        &f.edges,
        &origin,
        &target,
        &contexts[0],
        &options,
        &mut NoopSearchControl,
    )
    .unwrap();
    assert_eq!(legacy, controlled);
    struct Count(usize);
    impl SearchControl for Count {
        fn checkpoint(&mut self, phase: SearchPhase) -> azeroth_core::error::Result<()> {
            if phase == SearchPhase::RouteExpansion {
                self.0 += 1;
            }
            Ok(())
        }
    }
    let mut count = Count(0);
    shortest_route_controlled(
        &f.nodes,
        &f.edges,
        &origin,
        &target,
        &contexts[0],
        &options,
        &mut count,
    )
    .unwrap();
    let error = shortest_route_controlled(
        &f.nodes,
        &f.edges,
        &origin,
        &target,
        &contexts[0],
        &options,
        &mut CancelAt {
            phase: SearchPhase::RouteExpansion,
            remaining: count.0 - 1,
        },
    )
    .unwrap_err();
    assert_eq!(error.code, ErrorCode::OperationCanceled);
}

#[test]
fn joint_cancels_during_member_edge_readiness_after_candidates() {
    let f = support::Fixture::new(8, 16, 2);
    assert_eq!(
        f.joint(
            4,
            &mut CancelAt {
                phase: SearchPhase::ParticipantReadiness,
                remaining: 11
            }
        )
        .unwrap_err()
        .code,
        ErrorCode::OperationCanceled
    );
}
