//! Durable private extraction orchestration. UI receives opaque identities only.
use azeroth_core::{
    error::{AppError, ErrorCode},
    local_data::PerformancePreset,
    local_data_catalog::TableInventory,
    local_data_lifecycle::{Activation, JobState},
};
use azeroth_local_data::{
    lifecycle,
    worker::{ExtractionRequest, ExtractionWorker, WorkerControl, WorkerEvent},
};
use serde::Serialize;
use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
};
type Result<T> = std::result::Result<T, AppError>;
fn error(message: &'static str) -> AppError {
    AppError::new(ErrorCode::InvalidInput, message, false)
}
pub use azeroth_local_data::lifecycle::{SavedSelectionView, SelectionView};
pub type JobView = azeroth_core::local_data_lifecycle::JobStatus;
#[derive(Serialize)]
pub struct InventoryPage {
    pub revision: String,
    pub offset: u32,
    pub total: u32,
    pub rows: Vec<TableInventory>,
    pub next_offset: Option<u32>,
}
pub struct LocalDataService {
    root: PathBuf,
    jobs: Arc<Mutex<BTreeMap<String, WorkerControl>>>,
}
impl LocalDataService {
    pub fn build_research_inventory(
        &self,
        delivery: &str,
        authored_pack: Option<&Path>,
        preset: PerformancePreset,
    ) -> Result<String> {
        azeroth_local_data::inventory::build(&self.store()?, delivery, authored_pack, preset)
            .map_err(error)
    }
    pub fn research_inventory_page(
        &self,
        revision: &str,
        domain: &str,
        offset: u32,
        limit: u32,
        authored_pack: Option<&Path>,
    ) -> Result<azeroth_local_data::inventory::Page> {
        azeroth_local_data::inventory::page(
            &self.store()?,
            revision,
            domain,
            offset,
            limit,
            authored_pack,
        )
        .map_err(error)
    }
    pub fn review_bound_guide(
        &self,
        delivery: &str,
        guide: &Path,
        character: azeroth_ingest::rxp::Character,
        mode: azeroth_core::local_data_inventory::Mode,
    ) -> Result<azeroth_local_data::guide_review::Review> {
        azeroth_local_data::guide_review::verify(&self.store()?, delivery, guide, character, mode)
            .map_err(error)
    }
    pub fn prepare_delivery(&self, catalog: &str, locale: &str) -> Result<String> {
        let root = self.store()?;
        let projection = azeroth_local_data::projection::project_catalog(&root, catalog, locale)
            .map_err(error)?;
        azeroth_local_data::delivery::build(
            &root,
            &projection.projection_sha256,
            &[],
            &[],
            None,
            PerformancePreset::Max,
        )
        .map_err(error)
    }
    pub fn delivery_page(
        &self,
        revision: &str,
        domain: &str,
        offset: u32,
        limit: u32,
    ) -> Result<azeroth_local_data::delivery::Page> {
        let policy = azeroth_local_data::delivery_receipts::policy(include_bytes!(
            "../../../config/release-delivery-v3.pending.json"
        ))
        .map_err(error)?;
        azeroth_local_data::delivery::page(
            &self.store()?,
            revision,
            domain,
            offset,
            limit,
            Some(&policy),
        )
        .map_err(error)
    }
    pub fn delivery_active(
        &self,
        revision: &str,
    ) -> Result<azeroth_core::local_data_delivery::Activation> {
        azeroth_local_data::delivery::active(&self.store()?, revision).map_err(error)
    }
    pub fn activate_delivery(
        &self,
        revision: &str,
        expected: u64,
        rollback: bool,
    ) -> Result<azeroth_core::local_data_delivery::Activation> {
        azeroth_local_data::delivery::activate(&self.store()?, revision, expected, rollback)
            .map_err(error)
    }
    pub fn delivery_diff(
        &self,
        before: &str,
        after: &str,
    ) -> Result<azeroth_local_data::delivery::Diff> {
        azeroth_local_data::delivery::diff(&self.store()?, before, after).map_err(error)
    }
    pub fn media_discover(
        &self,
        catalog: &str,
        locale: &str,
        offset: u32,
        limit: u32,
    ) -> Result<azeroth_local_data::media::DiscoveryPage> {
        azeroth_local_data::media::discovery_page(&self.store()?, catalog, locale, offset, limit)
            .map_err(error)
    }
    pub fn media_extract(
        &self,
        selection: &str,
        catalog: &str,
        locale: &str,
        ids: Vec<u32>,
        preset: PerformancePreset,
    ) -> Result<String> {
        let selected = self.selection(selection, preset)?;
        let request = azeroth_local_data::media::Request {
            format_version: 1,
            installation: selected.installation,
            destination: self.store()?,
            catalog: catalog.into(),
            locale: locale.into(),
            ids,
            preset,
        };
        let executable =
            std::env::current_exe().map_err(|_| error("Media executable unavailable"))?;
        azeroth_local_data::media::extract_worker(&executable, &request).map_err(error)
    }
    pub fn media_page(
        &self,
        revision: &str,
        offset: u32,
        limit: u32,
    ) -> Result<azeroth_local_data::media::Page> {
        azeroth_local_data::media::page(&self.store()?, revision, offset, limit).map_err(error)
    }
    pub fn media_asset(&self, revision: &str, id: &str) -> Result<Vec<u8>> {
        azeroth_local_data::media::asset(&self.store()?, revision, id).map_err(error)
    }
    pub fn tooltip(
        &self,
        projection: &str,
        spell: u32,
    ) -> Result<azeroth_core::tooltip::Evaluation> {
        azeroth_local_data::media::tooltip(&self.store()?, projection, spell).map_err(error)
    }
    pub fn new(root: PathBuf) -> Self {
        Self {
            root,
            jobs: Arc::default(),
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
    /// Explicit file selection only. Cache reads always use the native closed-game
    /// guard; synthetic cache staging is intentionally absent from desktop commands.
    pub fn stage_input(
        &self,
        catalog: &str,
        path: &Path,
        kind: azeroth_core::local_data_inputs::InputKind,
        locale: &str,
        preset: PerformancePreset,
    ) -> Result<azeroth_local_data::inputs::StagedInput> {
        let root = self.store()?;
        if kind.cache_filename().is_some() {
            azeroth_local_data::inputs::stage_cache(&root, catalog, path, kind, locale, preset)
        } else {
            azeroth_local_data::inputs::stage_observation(
                &root, catalog, path, kind, locale, preset,
            )
        }
        .map_err(error)
    }
    pub fn verify_input(
        &self,
        digest: &str,
    ) -> Result<azeroth_core::local_data_inputs::ResearchInput> {
        azeroth_local_data::inputs::verify(&self.store()?, digest).map_err(error)
    }
    pub fn input_page(
        &self,
        digest: &str,
        offset: u32,
        limit: u32,
    ) -> Result<azeroth_local_data::inputs::CachePage> {
        azeroth_local_data::inputs::cache_page(&self.store()?, digest, offset, limit).map_err(error)
    }
    pub fn build_guide_facts(
        &self,
        projection: &str,
        inputs: &[String],
        preset: PerformancePreset,
    ) -> Result<azeroth_local_data::guide_facts::Summary> {
        let mut inputs = inputs.to_vec();
        inputs.sort();
        azeroth_local_data::guide_facts::build(&self.store()?, projection, &inputs, preset)
            .map_err(error)
    }
    pub fn verify_guide_facts(
        &self,
        digest: &str,
    ) -> Result<azeroth_core::local_data_guide::GuideFactsManifest> {
        azeroth_local_data::guide_facts::verify(&self.store()?, digest).map_err(error)
    }
    pub fn select(&self, path: &Path) -> Result<SelectionView> {
        lifecycle::select(
            &self.store()?,
            path,
            "wow_classic_beta",
            PerformancePreset::Max,
        )
        .map_err(error)
    }
    pub fn selections(&self) -> Result<Vec<SavedSelectionView>> {
        lifecycle::selections(&self.store()?).map_err(error)
    }
    pub fn restore_selection(&self, id: &str) -> Result<SelectionView> {
        let root = self.store()?;
        let saved = lifecycle::index(&root).map_err(error)?.selection(id)?;
        lifecycle::select(&root, &saved.installation, &saved.product, saved.preset).map_err(error)
    }
    fn selection(
        &self,
        id: &str,
        preset: PerformancePreset,
    ) -> Result<azeroth_storage::local_data::SavedSelection> {
        let mut db = lifecycle::index(&self.store()?).map_err(error)?;
        let mut selection = db.selection(id)?;
        selection.preset = preset;
        db.save_selection(&selection)?;
        Ok(selection)
    }
    pub fn preview(
        &self,
        id: &str,
        preset: PerformancePreset,
    ) -> Result<azeroth_local_data::ExtractionPlan> {
        let selected = self.selection(id, preset)?;
        azeroth_local_data::plan(
            &selected.installation,
            &selected.product,
            &self.store()?,
            preset,
        )
        .map_err(error)
    }
    pub fn start(&self, id: &str, preset: PerformancePreset) -> Result<JobView> {
        let selected = self.selection(id, preset)?;
        self.launch(ExtractionRequest {
            protocol_version: 2,
            resume: false,
            request_id: lifecycle::new_job_id().map_err(error)?,
            installation: selected.installation,
            product: selected.product,
            destination: self.store()?,
            preset,
            tables: Vec::new(),
        })
    }
    pub fn resume(&self, id: &str) -> Result<JobView> {
        self.launch(lifecycle::resume_request(&self.store()?, id).map_err(error)?)
    }
    fn launch(&self, request: ExtractionRequest) -> Result<JobView> {
        let mut jobs = self
            .jobs
            .lock()
            .map_err(|_| error("Job registry unavailable"))?;
        if !jobs.is_empty() {
            return Err(error("A local-data worker is already running"));
        }
        let mut worker = ExtractionWorker::start(&request).map_err(error)?;
        // Wait on the blocking command thread until durable checkpoint creation.
        // A caller never receives a job ID that status cannot resolve.
        match worker.next_event() {
            Ok(WorkerEvent::Progress { .. }) => {}
            _ => {
                return Err(error(
                    "Extraction could not initialize; inspect job status for retained checkpoints",
                ));
            }
        }
        let id = request.request_id;
        let view = lifecycle::index(&self.store()?)
            .map_err(error)?
            .status(&id)?;
        jobs.insert(id.clone(), worker.control());
        drop(jobs);
        let jobs = self.jobs.clone();
        std::thread::spawn(move || {
            while let Ok(WorkerEvent::Progress { .. }) = worker.next_event() {}
            // A crash releases writer locks before recovery can classify status.
            drop(worker);
            if let Ok(mut jobs) = jobs.lock() {
                jobs.remove(&id);
            }
        });
        Ok(view)
    }
    pub fn jobs(&self) -> Result<Vec<JobView>> {
        lifecycle::jobs(&self.store()?).map_err(error)
    }
    pub fn status(&self, id: &str) -> Result<JobView> {
        lifecycle::status(&self.store()?, id).map_err(error)
    }
    pub fn cancel(&self, id: &str) -> Result<JobView> {
        let status = lifecycle::cancel(&self.store()?, id).map_err(error)?;
        if status.state == JobState::Cancelled
            && let Some(control) = self
                .jobs
                .lock()
                .map_err(|_| error("Job registry unavailable"))?
                .get(id)
        {
            control.cancel().map_err(error)?;
        }
        Ok(status)
    }
    pub fn forget(&self, id: &str) -> Result<()> {
        lifecycle::index(&self.store()?).map_err(error)?.forget(id)
    }
    pub fn active(&self, revision: &str) -> Result<Activation> {
        lifecycle::active(&self.store()?, revision).map_err(error)
    }
    pub fn activate(&self, revision: &str, expected: u64, rollback: bool) -> Result<Activation> {
        lifecycle::activate(&self.store()?, revision, expected, rollback).map_err(error)
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
            for (id, control) in jobs.iter() {
                let _ = lifecycle::cancel(&self.root, id);
                let _ = control.cancel();
            }
        }
    }
}
