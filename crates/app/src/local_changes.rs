//! Opt-in selected-build metadata watcher. No archive/cache reads or activation.
use azeroth_core::local_data_workflow::{ChangePoller, PollStatus};
use azeroth_storage::local_data::SavedSelection;
use serde::Serialize;
use std::{
    path::PathBuf,
    sync::{Arc, Condvar, Mutex},
    thread::JoinHandle,
    time::{Duration, Instant},
};
#[derive(Clone, Debug, Serialize)]
pub struct View {
    pub format_version: u32,
    pub selection_id: String,
    pub status: String,
    pub candidate_sha256: Option<String>,
    pub auto_activation: bool,
}
pub(crate) struct Detector {
    pub baseline: String,
    pub preset: azeroth_core::local_data::PerformancePreset,
    cancel: Arc<(Mutex<bool>, Condvar)>,
    status: Arc<Mutex<View>>,
    join: Option<JoinHandle<()>>,
}
impl Detector {
    pub fn start(
        root: PathBuf,
        selected: SavedSelection,
        baseline: String,
    ) -> azeroth_local_data::Result<Self> {
        let preset = selected.preset;
        let mut poller = ChangePoller::new(&baseline)?;
        let cancel = Arc::new((Mutex::new(false), Condvar::new()));
        let state = Arc::new(Mutex::new(View {
            format_version: 1,
            selection_id: selected.selection_id.clone(),
            status: "waiting_for_stable_file".into(),
            candidate_sha256: None,
            auto_activation: false,
        }));
        let worker_cancel = cancel.clone();
        let worker_state = state.clone();
        let worker_baseline = baseline.clone();
        let join = std::thread::Builder::new()
            .name("local-build-change-watch".into())
            .spawn(move || {
                let started = Instant::now();
                let mut candidate = None;
                let mut observed_stamp = None;
                loop {
                    if *worker_cancel.0.lock().unwrap_or_else(|e| e.into_inner()) {
                        break;
                    }
                    let elapsed = started.elapsed().as_millis().min(u64::MAX as u128) as u64;
                    let status = match azeroth_local_data::changes::read_build_observation(
                        &selected.installation,
                    ) {
                        Ok((current, stamp)) => {
                            if observed_stamp.as_ref() != Some(&stamp) {
                                let _ = poller.poll(elapsed, None);
                                observed_stamp = Some(stamp);
                            }
                            match poller.poll(elapsed, Some(&current)) {
                                Ok(PollStatus::Ready) => {
                                    if *worker_cancel.0.lock().unwrap_or_else(|e| e.into_inner()) {
                                        break;
                                    }
                                    let existing = azeroth_local_data::lifecycle::index(&root)
                                        .ok()
                                        .and_then(|db| {
                                            db.change_candidate(&selected.selection_id)
                                                .ok()
                                                .flatten()
                                        })
                                        .filter(|id| {
                                            azeroth_local_data::changes::verify(&root, id)
                                                .is_ok_and(|c| {
                                                    c.current_sha256 == current
                                                        && c.previous_sha256 == worker_baseline
                                                })
                                        });
                                    let result = existing.map_or_else(
                                        || {
                                            azeroth_local_data::changes::stage(
                                                &root,
                                                &selected.installation,
                                                &selected.product,
                                                &selected.selection_id,
                                                &worker_baseline,
                                                &current,
                                                selected.preset,
                                            )
                                        },
                                        Ok,
                                    );
                                    match result {
                                        Ok(id) => {
                                            candidate = Some(id);
                                            poller.staged();
                                            "staged"
                                        }
                                        Err(_) => "retry_pending",
                                    }
                                }
                                Ok(PollStatus::Idle) => {
                                    candidate = None;
                                    "idle"
                                }
                                Ok(PollStatus::WaitingForStableFile) => {
                                    candidate = None;
                                    "waiting_for_stable_file"
                                }
                                Ok(PollStatus::Staged) => "staged",
                                Ok(PollStatus::RetryExhausted) => "retry_exhausted",
                                _ => "unreadable",
                            }
                        }
                        Err(_) => {
                            observed_stamp = None;
                            let _ = poller.poll(elapsed, None);
                            candidate = None;
                            "unreadable"
                        }
                    };
                    *worker_state.lock().unwrap_or_else(|e| e.into_inner()) = View {
                        format_version: 1,
                        selection_id: selected.selection_id.clone(),
                        status: status.into(),
                        candidate_sha256: candidate.clone(),
                        auto_activation: false,
                    };
                    let guard = worker_cancel.0.lock().unwrap_or_else(|e| e.into_inner());
                    let _ = worker_cancel.1.wait_timeout_while(
                        guard,
                        Duration::from_millis(500),
                        |cancelled| !*cancelled,
                    );
                }
            })
            .map_err(|_| "Cannot start selected-build detector")?;
        Ok(Self {
            baseline,
            preset,
            cancel,
            status: state,
            join: Some(join),
        })
    }
    pub fn view(&self) -> View {
        self.status
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clone()
    }
}
impl Drop for Detector {
    fn drop(&mut self) {
        *self.cancel.0.lock().unwrap_or_else(|e| e.into_inner()) = true;
        self.cancel.1.notify_all();
        if let Some(thread) = self.join.take() {
            let _ = thread.join();
        }
    }
}
