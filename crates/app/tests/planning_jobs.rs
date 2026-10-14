use azeroth_app::planning_jobs;
use azeroth_core::{
    error::ErrorCode,
    search_control::{SearchControl, SearchPhase},
};
use planning_jobs::*;
use std::sync::{Arc, Barrier};
#[test]
fn cancel_before_dispatch_does_not_run_compute_and_has_no_result() {
    let jobs = PlanningJobs::default();
    let ticket = jobs.register(PlanningKind::Solo).unwrap();
    let id = ticket.id().to_owned();
    jobs.cancel(&id).unwrap();
    jobs.cancel(&id).unwrap();
    jobs.run(ticket, |_| -> azeroth_core::error::Result<u32> {
        panic!("canceled work must not run")
    });
    let status = jobs.status(&id).unwrap();
    assert_eq!(status.state, "canceled");
    assert!(!status.result_available);
    assert_eq!(status.error.unwrap().code, ErrorCode::OperationCanceled);
    assert!(jobs.take_result(&id).is_err());
}
#[test]
fn cancellation_is_reachable_during_compute_and_does_not_cancel_another_job() {
    let jobs = Arc::new(PlanningJobs::default());
    let ticket = jobs.register(PlanningKind::Solo).unwrap();
    let id = ticket.id().to_owned();
    let other = jobs.register(PlanningKind::Joint).unwrap();
    let other_id = other.id().to_owned();
    let barrier = Arc::new(Barrier::new(2));
    let worker_jobs = jobs.clone();
    let worker_barrier = barrier.clone();
    let worker = std::thread::spawn(move || {
        worker_jobs.run(ticket, |control| {
            worker_barrier.wait();
            loop {
                control.checkpoint(SearchPhase::RouteExpansion)?;
            }
            #[allow(unreachable_code)]
            Ok(0u32)
        })
    });
    barrier.wait();
    assert_eq!(jobs.status(&id).unwrap().state, "running");
    jobs.cancel(&id).unwrap();
    worker.join().unwrap();
    assert_eq!(jobs.status(&id).unwrap().state, "canceled");
    jobs.run(other, |_| Ok(42u32));
    assert_eq!(jobs.status(&other_id).unwrap().state, "succeeded");
    assert_eq!(jobs.take_result(&other_id).unwrap().report_json, "42");
    assert!(jobs.take_result(&other_id).is_err());
    jobs.cancel(&other_id).unwrap();
    assert_eq!(jobs.status(&other_id).unwrap().state, "succeeded");
}
#[test]
fn capacity_does_not_evict_active_jobs_and_terminal_retention_is_bounded() {
    let jobs = PlanningJobs::default();
    let mut tickets = Vec::new();
    for _ in 0..MAX_ACTIVE_JOBS {
        tickets.push(jobs.register(PlanningKind::Solo).unwrap());
    }
    assert!(jobs.register(PlanningKind::Solo).is_err());
    let active_id = tickets[0].id().to_owned();
    for ticket in tickets.drain(1..) {
        jobs.run(ticket, |_| Ok(true));
    }
    let first_terminal = jobs.register(PlanningKind::Joint).unwrap();
    let first_id = first_terminal.id().to_owned();
    jobs.run(first_terminal, |_| Ok(false));
    for _ in 0..MAX_TERMINAL_JOBS {
        let ticket = jobs.register(PlanningKind::Solo).unwrap();
        jobs.run(ticket, |_| Ok(1u32));
    }
    assert!(jobs.status(&first_id).is_err());
    assert_eq!(jobs.status(&active_id).unwrap().state, "queued");
    let elsewhere = PlanningJobs::default();
    assert!(elsewhere.cancel(&active_id).is_err());
    jobs.run(tickets.pop().unwrap(), |_| Ok(0u32));
}
#[test]
fn panic_failure_and_oversized_result_have_no_partial_success() {
    let jobs = PlanningJobs::default();
    let ticket = jobs.register(PlanningKind::Solo).unwrap();
    let id = ticket.id().to_owned();
    jobs.run(ticket, |_| -> azeroth_core::error::Result<bool> {
        panic!("synthetic worker failure")
    });
    assert_eq!(jobs.status(&id).unwrap().state, "failed");
    assert_eq!(
        jobs.status(&id).unwrap().error.unwrap().code,
        ErrorCode::ProviderUnavailable
    );
    let ticket = jobs.register(PlanningKind::Solo).unwrap();
    let id = ticket.id().to_owned();
    jobs.run(ticket, |_| Ok("x".repeat(MAX_RESULT_BYTES)));
    assert_eq!(
        jobs.status(&id).unwrap().error.unwrap().code,
        ErrorCode::ImportTooLarge
    );
    assert!(jobs.take_result(&id).is_err());
}
#[test]
fn progress_is_work_count_and_phase_without_an_invented_percentage() {
    let jobs = PlanningJobs::default();
    let ticket = jobs.register(PlanningKind::Joint).unwrap();
    let id = ticket.id().to_owned();
    jobs.run(ticket, |control| {
        control.checkpoint(SearchPhase::ParticipantReadiness)?;
        Ok(serde_json::json!({"kind":"synthetic","value":0}))
    });
    let status = jobs.status(&id).unwrap();
    assert_eq!(status.phase, Some("selection"));
    assert_eq!(status.work_units, 3);
    let result = jobs.take_result(&id).unwrap();
    assert_eq!(result.kind, PlanningKind::Joint);
    assert_eq!(result.job_id, id);
    assert_eq!(
        serde_json::from_str::<serde_json::Value>(&result.report_json).unwrap()["value"],
        0
    );
    let value = serde_json::to_value(status).unwrap();
    assert!(value.get("percentage").is_none());
}

#[test]
fn foreign_registry_cannot_dispatch_a_ticket() {
    let jobs = PlanningJobs::default();
    let foreign = PlanningJobs::default();
    let ticket = jobs.register(PlanningKind::Solo).unwrap();
    let id = ticket.id().to_owned();
    foreign.run(ticket.clone(), |_| Ok(99u32));
    assert_eq!(jobs.status(&id).unwrap().state, "queued");
    jobs.run(ticket, |_| Ok(1u32));
    assert_eq!(jobs.take_result(&id).unwrap().report_json, "1");
}

#[test]
fn simultaneous_completions_obey_terminal_cap_and_evict_by_completion_age() {
    let jobs = PlanningJobs::default();
    let oldest_active = jobs.register(PlanningKind::Solo).unwrap();
    let oldest_active_id = oldest_active.id().to_owned();
    let mut terminal_ids = Vec::new();
    for _ in 0..15 {
        let ticket = jobs.register(PlanningKind::Solo).unwrap();
        terminal_ids.push(ticket.id().to_owned());
        jobs.run(ticket, |_| Ok(0u32));
    }
    let mut active = Vec::new();
    for _ in 0..3 {
        active.push(jobs.register(PlanningKind::Joint).unwrap());
    }
    let active_ids: Vec<_> = active.iter().map(|t| t.id().to_owned()).collect();
    for ticket in active {
        jobs.run(ticket, |_| Ok(1u32));
    }
    assert_eq!(jobs.status(&oldest_active_id).unwrap().state, "queued");
    jobs.run(oldest_active, |_| Ok(2u32));
    let retained = terminal_ids
        .iter()
        .chain(active_ids.iter())
        .chain(std::iter::once(&oldest_active_id))
        .filter(|id| jobs.status(id).is_ok())
        .count();
    assert_eq!(retained, MAX_TERMINAL_JOBS);
    for id in active_ids.iter().chain(std::iter::once(&oldest_active_id)) {
        assert!(jobs.status(id).unwrap().result_available);
    }
    assert!(terminal_ids[..3].iter().all(|id| jobs.status(id).is_err()));
}

#[test]
fn publication_guard_rejects_obsolete_success_and_serialization_precedes_guard() {
    let jobs = PlanningJobs::default();
    let ticket = jobs.register(PlanningKind::Solo).unwrap();
    let id = ticket.id().to_owned();
    jobs.run_guarded(
        ticket,
        |_| Ok(42u32),
        |_publish| {
            Err(azeroth_core::error::AppError::new(
                ErrorCode::SnapshotOutdated,
                "Snapshot changed",
                false,
            ))
        },
    );
    assert_eq!(
        jobs.status(&id).unwrap().error.unwrap().code,
        ErrorCode::SnapshotOutdated
    );
    assert!(!jobs.status(&id).unwrap().result_available);
    let ticket = jobs.register(PlanningKind::Solo).unwrap();
    let id = ticket.id().to_owned();
    jobs.run_guarded(
        ticket,
        |_| Ok("x".repeat(MAX_RESULT_BYTES)),
        |_| panic!("oversized output must fail before guard"),
    );
    assert_eq!(
        jobs.status(&id).unwrap().error.unwrap().code,
        ErrorCode::ImportTooLarge
    );
}
