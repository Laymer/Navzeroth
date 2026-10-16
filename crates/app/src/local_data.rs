//! Application orchestration for private research catalogs. Native decoding stays
//! in a self-worker. Frontends receive opaque selections and bounded inventory pages.
use azeroth_core::{
    error::{AppError, ErrorCode},
    local_data::PerformancePreset,
    local_data_catalog::{ExtractionSummary, TableInventory},
};
use azeroth_local_data::{
    Inspection,
    worker::{ExtractionRequest, ExtractionWorker, WorkerControl, WorkerEvent},
};
use serde::Serialize;
use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
    sync::{
        Arc, Mutex,
        atomic::{AtomicU64, Ordering},
    },
};
type Result<T> = std::result::Result<T, AppError>;
fn error(message: &'static str) -> AppError {
    AppError::new(ErrorCode::InvalidInput, message, false)
}

#[derive(Clone, Serialize)]
pub struct SelectionView {
    pub selection_id: String,
    pub inspection: Inspection,
}
#[derive(Clone, Serialize)]
pub struct JobView {
    pub job_id: String,
    pub state: String,
    pub completed: u32,
    pub total: u32,
    pub summary: Option<ExtractionSummary>,
    pub error: Option<String>,
}
#[derive(Serialize)]
pub struct InventoryPage {
    pub revision: String,
    pub offset: u32,
    pub total: u32,
    pub rows: Vec<TableInventory>,
    pub next_offset: Option<u32>,
}
struct Selection {
    path: PathBuf,
    inspection: Inspection,
}
struct Job {
    view: JobView,
    control: WorkerControl,
}
pub struct LocalDataService {
    root: PathBuf,
    selections: Mutex<BTreeMap<String, Selection>>,
    jobs: Arc<Mutex<BTreeMap<String, Job>>>,
    serial: AtomicU64,
}
impl LocalDataService {
    pub fn new(root: PathBuf) -> Self {
        Self {
            root,
            selections: Mutex::default(),
            jobs: Arc::default(),
            serial: AtomicU64::new(1),
        }
    }
    fn store(&self) -> Result<PathBuf> {
        if !self.root.exists() {
            let mut builder = std::fs::DirBuilder::new();
            builder.recursive(true);
            #[cfg(unix)]
            {
                use std::os::unix::fs::DirBuilderExt;
                builder.mode(0o700);
            }
            builder
                .create(&self.root)
                .map_err(|_| error("Cannot create private local-data directory"))?;
        }
        azeroth_local_data::checked_path(&self.root).map_err(error)
    }
    pub fn select(&self, path: &Path) -> Result<SelectionView> {
        let path = azeroth_local_data::checked_path(path).map_err(error)?;
        let inspection = azeroth_local_data::inspect(&path, "wow_classic_beta").map_err(error)?;
        let mut selections = self
            .selections
            .lock()
            .map_err(|_| error("Selection registry unavailable"))?;
        if selections.len() >= 16 {
            return Err(error(
                "Selection capacity reached; restart to clear unused selections",
            ));
        }
        let id = format!(
            "installation-{}",
            self.serial.fetch_add(1, Ordering::Relaxed)
        );
        selections.insert(
            id.clone(),
            Selection {
                path,
                inspection: inspection.clone(),
            },
        );
        Ok(SelectionView {
            selection_id: id,
            inspection,
        })
    }
    pub fn preview(
        &self,
        id: &str,
        preset: PerformancePreset,
    ) -> Result<azeroth_local_data::ExtractionPlan> {
        let selections = self
            .selections
            .lock()
            .map_err(|_| error("Selection registry unavailable"))?;
        let selection = selections
            .get(id)
            .ok_or_else(|| error("Unknown installation selection"))?;
        azeroth_local_data::plan(
            &selection.path,
            &selection.inspection.metadata.product,
            &self.store()?,
            preset,
        )
        .map_err(error)
    }
    pub fn start(&self, id: &str, preset: PerformancePreset) -> Result<JobView> {
        let path = {
            let selections = self
                .selections
                .lock()
                .map_err(|_| error("Selection registry unavailable"))?;
            selections
                .get(id)
                .ok_or_else(|| error("Unknown installation selection"))?
                .path
                .clone()
        };
        let mut jobs = self
            .jobs
            .lock()
            .map_err(|_| error("Job registry unavailable"))?;
        if jobs.len() >= 8 || jobs.values().any(|j| j.view.state == "running") {
            return Err(error(
                "A local-data job is running or job capacity was reached",
            ));
        }
        let job_id = format!("local-data-{}", self.serial.fetch_add(1, Ordering::Relaxed));
        let request = ExtractionRequest {
            protocol_version: 1,
            request_id: job_id.clone(),
            installation: path,
            product: "wow_classic_beta".into(),
            destination: self.store()?,
            preset,
            tables: Vec::new(),
        };
        let mut worker = ExtractionWorker::start(&request).map_err(error)?;
        let view = JobView {
            job_id: job_id.clone(),
            state: "running".into(),
            completed: 0,
            total: 0,
            summary: None,
            error: None,
        };
        jobs.insert(
            job_id.clone(),
            Job {
                view: view.clone(),
                control: worker.control(),
            },
        );
        drop(jobs);
        let jobs = self.jobs.clone();
        std::thread::spawn(move || {
            loop {
                let event = worker.next_event();
                let Ok(mut jobs) = jobs.lock() else {
                    break;
                };
                let Some(job) = jobs.get_mut(&job_id) else {
                    break;
                };
                if job.view.state == "cancelled" {
                    break;
                }
                match event {
                    Ok(WorkerEvent::Progress {
                        completed, total, ..
                    }) => {
                        job.view.completed = completed;
                        job.view.total = total;
                    }
                    Ok(WorkerEvent::Complete { summary, .. }) => {
                        job.view.state = "complete".into();
                        job.view.summary = Some(summary);
                        break;
                    }
                    Ok(WorkerEvent::Failed { code, .. }) => {
                        job.view.state = "failed".into();
                        job.view.error = Some(code);
                        break;
                    }
                    Err(code) => {
                        job.view.state = "failed".into();
                        job.view.error = Some(code.into());
                        break;
                    }
                }
            }
        });
        Ok(view)
    }
    pub fn status(&self, id: &str) -> Result<JobView> {
        self.jobs
            .lock()
            .map_err(|_| error("Job registry unavailable"))?
            .get(id)
            .map(|j| j.view.clone())
            .ok_or_else(|| error("Unknown local-data job"))
    }
    pub fn cancel(&self, id: &str) -> Result<JobView> {
        let control = {
            let mut jobs = self
                .jobs
                .lock()
                .map_err(|_| error("Job registry unavailable"))?;
            let job = jobs
                .get_mut(id)
                .ok_or_else(|| error("Unknown local-data job"))?;
            if job.view.state != "running" {
                return Ok(job.view.clone());
            }
            job.view.state = "cancelled".into();
            job.control.clone()
        };
        control.cancel().map_err(error)?;
        self.status(id)
    }
    pub fn inventory(&self, revision: &str, offset: u32, limit: u32) -> Result<InventoryPage> {
        if limit == 0 || limit > 200 {
            return Err(error("Inventory page limit must be 1 through 200"));
        }
        let manifest =
            azeroth_local_data::verify_catalog(&self.store()?, revision).map_err(error)?;
        let total = manifest.tables.len() as u32;
        if offset > total {
            return Err(error("Inventory offset exceeds catalog"));
        }
        let end = offset.saturating_add(limit).min(total);
        Ok(InventoryPage {
            revision: revision.into(),
            offset,
            total,
            rows: manifest.tables[offset as usize..end as usize].to_vec(),
            next_offset: if end < total { Some(end) } else { None },
        })
    }
}
impl Drop for LocalDataService {
    fn drop(&mut self) {
        if let Ok(jobs) = self.jobs.lock() {
            for job in jobs.values() {
                if job.view.state == "running" {
                    let _ = job.control.cancel();
                }
            }
        }
    }
}
