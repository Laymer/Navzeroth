//! Bounded application-session jobs. Control never locks the store or recovery gate.
use azeroth_core::{
    error::{AppError, ErrorCode, Result},
    search_control::{SearchControl, SearchPhase},
};
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeMap,
    io::Write,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, AtomicU8, AtomicU64, Ordering},
    },
    time::{Duration, Instant},
};
pub const MAX_ACTIVE_JOBS: usize = 4;
pub const MAX_TERMINAL_JOBS: usize = 16;
pub const MAX_RESULT_BYTES: usize = 4 * 1024 * 1024;
pub const MAX_JOB_DURATION: Duration = Duration::from_secs(10);
const QUEUED: u8 = 0;
const RUNNING: u8 = 1;
const SUCCEEDED: u8 = 2;
const FAILED: u8 = 3;
const CANCELED: u8 = 4;
static REGISTRY_IDS: AtomicU64 = AtomicU64::new(1);
fn error(code: ErrorCode, message: &'static str) -> AppError {
    AppError::new(code, message, false)
}
fn canceled() -> AppError {
    error(ErrorCode::OperationCanceled, "Planning job canceled")
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PlanningKind {
    Solo,
    Joint,
}
#[derive(Clone, Debug, Serialize)]
pub struct JobAccepted {
    pub job_schema_version: u32,
    pub job_id: String,
}
#[derive(Clone, Debug, Serialize)]
pub struct JobStatus {
    pub job_schema_version: u32,
    pub job_id: String,
    pub kind: PlanningKind,
    pub state: &'static str,
    pub cancel_requested: bool,
    pub phase: Option<&'static str>,
    pub work_units: u64,
    pub result_available: bool,
    pub error: Option<AppError>,
}
#[derive(Debug, Serialize)]
pub struct JobResult {
    pub job_schema_version: u32,
    pub job_id: String,
    pub kind: PlanningKind,
    /// Serialized typed backend report, bounded independently of command input.
    pub report_json: String,
}
struct Entry {
    id: String,
    kind: PlanningKind,
    canceled: AtomicBool,
    state: AtomicU8,
    completed_order: AtomicU64,
    phase: AtomicU8,
    work: AtomicU64,
    outcome: Mutex<(Option<String>, Option<AppError>)>,
    pin: Mutex<Option<azeroth_storage::today::PlanDescriptor>>,
}
#[derive(Clone)]
pub struct JobTicket {
    entry: Arc<Entry>,
}
impl JobTicket {
    pub fn id(&self) -> &str {
        &self.entry.id
    }
    /// Terminal worker outcome remains available even after result retrieval/pruning.
    pub fn completion_result(&self) -> Result<()> {
        if self.entry.state.load(Ordering::Acquire) <= RUNNING {
            return Err(error(
                ErrorCode::ProviderUnavailable,
                "Planning job is not terminal",
            ));
        }
        let outcome = self.entry.outcome.lock().map_err(|_| {
            error(
                ErrorCode::ProviderUnavailable,
                "Planning job outcome unavailable",
            )
        })?;
        match &outcome.1 {
            Some(error) => Err(error.clone()),
            None => Ok(()),
        }
    }
}
pub struct PlanningJobs {
    registry: u64,
    next: AtomicU64,
    completed: AtomicU64,
    entries: Mutex<BTreeMap<u64, Arc<Entry>>>,
}
impl Default for PlanningJobs {
    fn default() -> Self {
        Self {
            registry: REGISTRY_IDS.fetch_add(1, Ordering::Relaxed),
            next: AtomicU64::new(1),
            completed: AtomicU64::new(1),
            entries: Mutex::new(BTreeMap::new()),
        }
    }
}
impl PlanningJobs {
    pub fn register(&self, kind: PlanningKind) -> Result<JobTicket> {
        let mut entries = self.entries.lock().map_err(|_| {
            error(
                ErrorCode::ProviderUnavailable,
                "Planning job registry unavailable",
            )
        })?;
        let active = entries
            .values()
            .filter(|e| e.state.load(Ordering::Acquire) <= RUNNING)
            .count();
        if active >= MAX_ACTIVE_JOBS {
            return Err(error(
                ErrorCode::ProviderUnavailable,
                "Planning job capacity reached",
            ));
        }
        let serial = self.next.fetch_add(1, Ordering::Relaxed);
        let entry = Arc::new(Entry {
            id: format!("planning-{}-{serial}", self.registry),
            kind,
            canceled: AtomicBool::new(false),
            state: AtomicU8::new(QUEUED),
            completed_order: AtomicU64::new(0),
            phase: AtomicU8::new(0),
            work: AtomicU64::new(0),
            outcome: Mutex::new((None, None)),
            pin: Mutex::new(None),
        });
        entries.insert(serial, entry.clone());
        Ok(JobTicket { entry })
    }
    fn entry(&self, id: &str) -> Result<Arc<Entry>> {
        self.entries
            .lock()
            .map_err(|_| {
                error(
                    ErrorCode::ProviderUnavailable,
                    "Planning job registry unavailable",
                )
            })?
            .values()
            .find(|e| e.id == id)
            .cloned()
            .ok_or_else(|| {
                error(
                    ErrorCode::InvalidInput,
                    "Planning job expired or belongs to another app session",
                )
            })
    }
    pub fn status(&self, id: &str) -> Result<JobStatus> {
        let entry = self.entry(id)?;
        let outcome = entry.outcome.lock().map_err(|_| {
            error(
                ErrorCode::ProviderUnavailable,
                "Planning job outcome unavailable",
            )
        })?;
        let state = entry.state.load(Ordering::Acquire);
        Ok(JobStatus {
            job_schema_version: 1,
            job_id: entry.id.clone(),
            kind: entry.kind,
            state: match state {
                QUEUED => "queued",
                RUNNING => "running",
                SUCCEEDED => "succeeded",
                FAILED => "failed",
                _ => "canceled",
            },
            cancel_requested: entry.canceled.load(Ordering::Acquire),
            phase: phase_name(entry.phase.load(Ordering::Relaxed)),
            work_units: entry.work.load(Ordering::Relaxed),
            result_available: outcome.0.is_some(),
            error: outcome.1.clone(),
        })
    }
    /// Linearized with publication; completed success remains available.
    pub fn cancel(&self, id: &str) -> Result<()> {
        let entry = self.entry(id)?;
        let _outcome = entry.outcome.lock().map_err(|_| {
            error(
                ErrorCode::ProviderUnavailable,
                "Planning job outcome unavailable",
            )
        })?;
        if entry.state.load(Ordering::Acquire) <= RUNNING {
            entry.canceled.store(true, Ordering::Release);
        }
        Ok(())
    }
    pub fn take_result(&self, id: &str) -> Result<JobResult> {
        let entry = self.entry(id)?;
        let mut outcome = entry.outcome.lock().map_err(|_| {
            error(
                ErrorCode::ProviderUnavailable,
                "Planning job outcome unavailable",
            )
        })?;
        let report_json = outcome.0.take().ok_or_else(|| {
            error(
                ErrorCode::InvalidInput,
                "Planning result is pending, unavailable or already retrieved",
            )
        })?;
        Ok(JobResult {
            job_schema_version: 1,
            job_id: entry.id.clone(),
            kind: entry.kind,
            report_json,
        })
    }
    /// Caller owns worker dispatch. Use guarded publication to check current
    /// input identities and publish while holding the short store lock.
    pub fn run<T: Serialize>(
        &self,
        ticket: JobTicket,
        call: impl FnOnce(&mut JobControl) -> Result<T>,
    ) {
        self.run_guarded(ticket, call, |publish| publish());
    }
    pub fn run_guarded<T: Serialize>(
        &self,
        ticket: JobTicket,
        call: impl FnOnce(&mut JobControl) -> Result<T>,
        validate_and_publish: impl FnOnce(&mut dyn FnMut() -> Result<()>) -> Result<()>,
    ) {
        self.run_guarded_inner(
            ticket,
            |control| call(control).map(|report| (report, None)),
            validate_and_publish,
        );
    }
    pub fn run_guarded_with_pin<T: Serialize>(
        &self,
        ticket: JobTicket,
        call: impl FnOnce(
            &mut JobControl,
        ) -> Result<(T, Option<azeroth_storage::today::PlanDescriptor>)>,
        validate_and_publish: impl FnOnce(&mut dyn FnMut() -> Result<()>) -> Result<()>,
    ) {
        self.run_guarded_inner(ticket, call, validate_and_publish);
    }
    /// Registry lookups are required even if a worker still owns a pruned ticket.
    pub(crate) fn pin_descriptor(
        &self,
        id: &str,
    ) -> Result<azeroth_storage::today::PlanDescriptor> {
        let entry = self.entry(id)?;
        if entry.state.load(Ordering::Acquire) != SUCCEEDED
            || entry.canceled.load(Ordering::Acquire)
        {
            return Err(error(
                ErrorCode::SnapshotOutdated,
                "Planning job cannot be pinned",
            ));
        }
        entry
            .pin
            .lock()
            .map_err(|_| error(ErrorCode::ProviderUnavailable, "Planning pin unavailable"))?
            .clone()
            .ok_or_else(|| error(ErrorCode::SnapshotOutdated, "Planning pin unavailable"))
    }
    fn run_guarded_inner<T: Serialize>(
        &self,
        ticket: JobTicket,
        call: impl FnOnce(
            &mut JobControl,
        ) -> Result<(T, Option<azeroth_storage::today::PlanDescriptor>)>,
        validate_and_publish: impl FnOnce(&mut dyn FnMut() -> Result<()>) -> Result<()>,
    ) {
        if self.entry(ticket.id()).is_err() {
            return;
        }
        if ticket
            .entry
            .state
            .compare_exchange(QUEUED, RUNNING, Ordering::AcqRel, Ordering::Acquire)
            .is_err()
        {
            return;
        }
        let mut control = JobControl {
            entry: ticket.entry.clone(),
            started: Instant::now(),
        };
        let computed = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            control.checkpoint(SearchPhase::ValidateInputs)?;
            let (result, pin) = call(&mut control)?;
            if let Some(p) = &pin {
                p.validate()?;
            }
            control.checkpoint(SearchPhase::Selection)?;
            let mut bytes = BoundedBytes(Vec::new());
            serde_json::to_writer(&mut bytes, &result).map_err(|_| {
                error(
                    ErrorCode::ImportTooLarge,
                    "Planning report exceeds retained result limit",
                )
            })?;
            control.check_time()?;
            String::from_utf8(bytes.0)
                .map(|report| (report, pin))
                .map_err(|_| {
                    error(
                        ErrorCode::InvalidInput,
                        "Planning report cannot be serialized",
                    )
                })
        }))
        .unwrap_or_else(|_| {
            Err(error(
                ErrorCode::ProviderUnavailable,
                "Planning worker failed",
            ))
        });
        match computed {
            Err(error) => {
                let _ = self.complete(&ticket, Err(error));
            }
            Ok((report, pin)) => {
                let mut report = Some(report);
                let mut pin = pin;
                let mut published = false;
                let mut publish = || {
                    control.check_time()?;
                    let report = report.take().ok_or_else(|| {
                        error(
                            ErrorCode::InvalidInput,
                            "Planning publication already completed",
                        )
                    })?;
                    self.complete_with_pin(&ticket, Ok(report), pin.take())?;
                    published = true;
                    Ok(())
                };
                let checked = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    validate_and_publish(&mut publish)
                }))
                .unwrap_or_else(|_| {
                    Err(error(
                        ErrorCode::ProviderUnavailable,
                        "Planning publication failed",
                    ))
                });
                if !published {
                    let failure = checked.err().unwrap_or_else(|| {
                        error(
                            ErrorCode::ProviderUnavailable,
                            "Planning publication was not validated",
                        )
                    });
                    let _ = self.complete(&ticket, Err(failure));
                }
            }
        }
    }
    fn complete(&self, ticket: &JobTicket, result: Result<String>) -> Result<()> {
        self.complete_with_pin(ticket, result, None)
    }
    fn complete_with_pin(
        &self,
        ticket: &JobTicket,
        result: Result<String>,
        pin: Option<azeroth_storage::today::PlanDescriptor>,
    ) -> Result<()> {
        // Registry -> outcome is the only nested order; status/cancel release
        // registry before locking outcome. Pruning and publication are atomic.
        let mut entries = self.entries.lock().map_err(|_| {
            error(
                ErrorCode::ProviderUnavailable,
                "Planning registry unavailable",
            )
        })?;
        let mut stored = ticket.entry.outcome.lock().map_err(|_| {
            error(
                ErrorCode::ProviderUnavailable,
                "Planning outcome unavailable",
            )
        })?;
        while entries
            .values()
            .filter(|e| e.state.load(Ordering::Acquire) > RUNNING)
            .count()
            >= MAX_TERMINAL_JOBS
        {
            let oldest = entries
                .iter()
                .filter(|(_, e)| e.state.load(Ordering::Acquire) > RUNNING)
                .min_by_key(|(_, e)| e.completed_order.load(Ordering::Relaxed))
                .map(|(id, _)| *id);
            if let Some(oldest) = oldest {
                entries.remove(&oldest);
            } else {
                break;
            }
        }
        let result = if ticket.entry.canceled.load(Ordering::Acquire) {
            Err(canceled())
        } else {
            result
        };
        ticket.entry.completed_order.store(
            self.completed.fetch_add(1, Ordering::Relaxed),
            Ordering::Relaxed,
        );
        match result {
            Ok(report) => {
                *ticket.entry.pin.lock().map_err(|_| {
                    error(ErrorCode::ProviderUnavailable, "Planning pin unavailable")
                })? = pin;
                *stored = (Some(report), None);
                ticket.entry.state.store(SUCCEEDED, Ordering::Release);
            }
            Err(error) => {
                let state = if error.code == ErrorCode::OperationCanceled {
                    CANCELED
                } else {
                    FAILED
                };
                *stored = (None, Some(error));
                ticket.entry.state.store(state, Ordering::Release);
            }
        }
        Ok(())
    }
}
struct BoundedBytes(Vec<u8>);
impl Write for BoundedBytes {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        if bytes.len() > MAX_RESULT_BYTES.saturating_sub(self.0.len()) {
            return Err(std::io::ErrorKind::FileTooLarge.into());
        }
        self.0.extend_from_slice(bytes);
        Ok(bytes.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}
pub struct JobControl {
    entry: Arc<Entry>,
    started: Instant,
}
impl JobControl {
    fn check_time(&self) -> Result<()> {
        if self.entry.canceled.load(Ordering::Acquire) {
            return Err(canceled());
        }
        if self.started.elapsed() >= MAX_JOB_DURATION {
            return Err(error(
                ErrorCode::OperationCanceled,
                "Planning job exceeded its ten second computation budget",
            ));
        }
        Ok(())
    }
}
impl SearchControl for JobControl {
    fn checkpoint(&mut self, phase: SearchPhase) -> Result<()> {
        if self.entry.canceled.load(Ordering::Acquire) {
            return Err(canceled());
        }
        let work = self.entry.work.fetch_add(1, Ordering::Relaxed);
        self.entry.phase.store(phase_id(phase), Ordering::Relaxed);
        if work.is_multiple_of(256) {
            self.check_time()?;
        }
        Ok(())
    }
}
fn phase_id(phase: SearchPhase) -> u8 {
    match phase {
        SearchPhase::ValidateInputs => 1,
        SearchPhase::ParticipantReadiness => 2,
        SearchPhase::EdgeAssessment => 3,
        SearchPhase::RouteExpansion => 4,
        SearchPhase::CandidateAssessment => 5,
        SearchPhase::Selection => 6,
    }
}
fn phase_name(phase: u8) -> Option<&'static str> {
    match phase {
        1 => Some("validate_inputs"),
        2 => Some("participant_readiness"),
        3 => Some("edge_assessment"),
        4 => Some("route_expansion"),
        5 => Some("candidate_assessment"),
        6 => Some("selection"),
        _ => None,
    }
}
