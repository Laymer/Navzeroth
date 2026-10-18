//! Durable private extraction orchestration. UI receives opaque identities only.
pub use crate::local_changes::View as ChangeDetectorView;
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
#[derive(Serialize)]
pub struct MaterialView {
    pub id: String,
    pub kind: azeroth_core::local_data_workflow::MaterialKind,
}
#[derive(Serialize)]
pub struct MediaDisplayAsset {
    pub id: String,
    pub width: u32,
    pub height: u32,
    pub url: String,
}
#[derive(Serialize)]
pub struct MediaDisplayPage {
    pub revision: String,
    pub total: u32,
    pub offset: u32,
    pub assets: Vec<MediaDisplayAsset>,
    pub next_offset: Option<u32>,
}
pub struct LocalDataService {
    root: PathBuf,
    jobs: Arc<Mutex<BTreeMap<String, WorkerControl>>>,
    detectors: Mutex<BTreeMap<String, crate::local_changes::Detector>>,
}
impl LocalDataService {
    fn sync_detectors(&self) -> Result<()> {
        let root = self.store()?;
        let db = lifecycle::index(&root).map_err(error)?;
        let mut workers = self
            .detectors
            .lock()
            .map_err(|_| error("Change detector registry unavailable"))?;
        // Read after taking the reconciliation lock so a delayed caller cannot
        // restart a worker using a preferences snapshot from before opt-out.
        let preferences = db.workflow()?.preferences;
        workers.retain(|id, worker| {
            preferences.selections.get(id).is_some_and(|c| {
                c.change_detection
                    && c.preset == worker.preset
                    && c.baseline_sha256.as_deref() == Some(worker.baseline.as_str())
            })
        });
        for (id, choices) in &preferences.selections {
            if choices.change_detection && !workers.contains_key(id) {
                let mut selected = db.selection(id)?;
                selected.preset = choices.preset;
                workers.insert(
                    id.clone(),
                    crate::local_changes::Detector::start(
                        root.clone(),
                        selected,
                        choices
                            .baseline_sha256
                            .clone()
                            .ok_or_else(|| error("Change baseline missing"))?,
                    )
                    .map_err(error)?,
                );
            }
        }
        Ok(())
    }
    pub fn workflow_preferences(
        &self,
    ) -> Result<azeroth_core::local_data_workflow::PreferencesRecord> {
        let preferences = lifecycle::index(&self.store()?)
            .map_err(error)?
            .workflow()?;
        self.sync_detectors()?;
        Ok(preferences)
    }
    pub fn finish_startup(
        &self,
        expected: u64,
        mode: azeroth_core::local_data_workflow::StartupMode,
        completed: bool,
    ) -> Result<azeroth_core::local_data_workflow::PreferencesRecord> {
        let mut db = lifecycle::index(&self.store()?).map_err(error)?;
        let mut record = db.workflow()?;
        record.preferences.startup_mode = mode;
        record.preferences.first_run_completed = completed;
        let result = db.replace_workflow(expected, &record.preferences)?;
        self.sync_detectors()?;
        Ok(result)
    }
    pub fn save_workflow_choices(
        &self,
        id: &str,
        expected: u64,
        locales: Vec<String>,
        preset: PerformancePreset,
        detect: bool,
    ) -> Result<azeroth_core::local_data_workflow::PreferencesRecord> {
        let mut db = lifecycle::index(&self.store()?).map_err(error)?;
        let selected = db.selection(id)?;
        let inspected = azeroth_local_data::inspect(&selected.installation, &selected.product)
            .map_err(error)?;
        azeroth_core::local_data_workflow::select_locales(
            &inspected.metadata.declared_locales,
            &locales,
        )
        .map_err(error)?;
        let mut record = db.workflow()?;
        let baseline = if detect {
            record
                .preferences
                .selections
                .get(id)
                .filter(|c| c.change_detection)
                .and_then(|c| c.baseline_sha256.clone())
                .or(Some(inspected.build_info_sha256))
        } else {
            None
        };
        record.preferences.selections.insert(
            id.into(),
            azeroth_core::local_data_workflow::SelectionChoices {
                locales,
                preset,
                change_detection: detect,
                baseline_sha256: baseline,
            },
        );
        let result = db.replace_workflow(expected, &record.preferences)?;
        self.sync_detectors()?;
        Ok(result)
    }
    pub fn change_detectors(&self) -> Result<Vec<ChangeDetectorView>> {
        let _ = self.workflow_preferences()?;
        Ok(self
            .detectors
            .lock()
            .map_err(|_| error("Change detector registry unavailable"))?
            .values()
            .map(crate::local_changes::Detector::view)
            .collect())
    }
    pub fn acknowledge_build_change(
        &self,
        id: &str,
        expected: u64,
        candidate: &str,
    ) -> Result<azeroth_core::local_data_workflow::PreferencesRecord> {
        let root = self.store()?;
        let staged = azeroth_local_data::changes::verify(&root, candidate).map_err(error)?;
        let mut db = lifecycle::index(&root).map_err(error)?;
        let selected = db.selection(id)?;
        if staged.selection_id != id
            || azeroth_local_data::changes::read_build_identity(&selected.installation)
                .map_err(error)?
                != staged.current_sha256
        {
            return Err(error("Changed build moved again; review the new candidate"));
        }
        let mut record = db.workflow()?;
        let choice = record
            .preferences
            .selections
            .get_mut(id)
            .filter(|c| c.change_detection)
            .ok_or_else(|| error("Change detection is not enabled"))?;
        choice.baseline_sha256 = Some(staged.current_sha256);
        let saved = db.replace_workflow(expected, &record.preferences)?;
        self.sync_detectors()?;
        Ok(saved)
    }
    pub fn material_selections(&self) -> Result<Vec<MaterialView>> {
        Ok(lifecycle::index(&self.store()?)
            .map_err(error)?
            .materials()?
            .into_iter()
            .map(|m| MaterialView {
                id: m.id,
                kind: m.kind,
            })
            .collect())
    }
    pub fn forget_material(&self, id: &str) -> Result<()> {
        lifecycle::index(&self.store()?)
            .map_err(error)?
            .forget_material(id)
    }
    pub fn select_material(
        &self,
        path: &Path,
        kind: azeroth_core::local_data_workflow::MaterialKind,
    ) -> Result<MaterialView> {
        let path = azeroth_local_data::checked_path(path).map_err(error)?;
        if (kind == azeroth_core::local_data_workflow::MaterialKind::AuthoredPack && !path.is_dir())
            || (kind != azeroth_core::local_data_workflow::MaterialKind::AuthoredPack
                && !path.is_file())
        {
            return Err(error("Selected material has unsupported type"));
        }
        let id = azeroth_local_data::lifecycle::selection_id(&path, &format!("{kind:?}"));
        lifecycle::index(&self.store()?)
            .map_err(error)?
            .save_material(&azeroth_storage::local_data::SelectedMaterial {
                id: id.clone(),
                kind,
                path,
            })?;
        Ok(MaterialView { id, kind })
    }
    pub fn stage_selected_input(
        &self,
        catalog: &str,
        id: &str,
        locale: &str,
        preset: PerformancePreset,
    ) -> Result<azeroth_local_data::inputs::StagedInput> {
        let selected = lifecycle::index(&self.store()?)
            .map_err(error)?
            .material(id)?;
        let kind = selected
            .kind
            .input_kind()
            .ok_or_else(|| error("Selected material is not an observation or cache"))?;
        self.stage_input(catalog, &selected.path, kind, locale, preset)
    }
    fn material_path(
        &self,
        id: &str,
        kind: azeroth_core::local_data_workflow::MaterialKind,
    ) -> Result<PathBuf> {
        let selected = lifecycle::index(&self.store()?)
            .map_err(error)?
            .material(id)?;
        if selected.kind != kind {
            return Err(error("Selected material kind differs"));
        }
        Ok(selected.path)
    }
    pub fn inventory_selected(
        &self,
        delivery: &str,
        authored: Option<&str>,
        preset: PerformancePreset,
    ) -> Result<String> {
        let path = authored
            .map(|id| {
                self.material_path(
                    id,
                    azeroth_core::local_data_workflow::MaterialKind::AuthoredPack,
                )
            })
            .transpose()?;
        self.build_research_inventory(delivery, path.as_deref(), preset)
    }
    pub fn inventory_page_selected(
        &self,
        revision: &str,
        domain: &str,
        offset: u32,
        limit: u32,
        authored: Option<&str>,
    ) -> Result<azeroth_local_data::inventory::Page> {
        let path = authored
            .map(|id| {
                self.material_path(
                    id,
                    azeroth_core::local_data_workflow::MaterialKind::AuthoredPack,
                )
            })
            .transpose()?;
        self.research_inventory_page(revision, domain, offset, limit, path.as_deref())
    }
    pub fn review_selected_guide(
        &self,
        delivery: &str,
        guide: &str,
        character: azeroth_ingest::rxp::Character,
        mode: azeroth_core::local_data_inventory::Mode,
    ) -> Result<azeroth_local_data::guide_review::Review> {
        let path = self.material_path(
            guide,
            azeroth_core::local_data_workflow::MaterialKind::Guide,
        )?;
        self.review_bound_guide(delivery, &path, character, mode)
    }
    pub fn artifact_page(
        &self,
        namespace: &str,
        offset: u32,
        limit: u32,
    ) -> Result<azeroth_local_data::changes::ArtifactPage> {
        azeroth_local_data::changes::artifacts(&self.store()?, namespace, offset, limit)
            .map_err(error)
    }
    pub fn start_selected(
        &self,
        id: &str,
        preset: PerformancePreset,
        locales: Vec<String>,
    ) -> Result<JobView> {
        let selected = self.selection(id, preset)?;
        self.launch(ExtractionRequest {
            protocol_version: 3,
            resume: false,
            request_id: lifecycle::new_job_id().map_err(error)?,
            installation: selected.installation,
            product: selected.product,
            destination: self.store()?,
            preset,
            tables: Vec::new(),
            locales,
        })
    }
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
    pub fn media_display_page(
        &self,
        revision: &str,
        offset: u32,
        limit: u32,
    ) -> Result<MediaDisplayPage> {
        let page = self.media_page(revision, offset, limit)?;
        let origin = if cfg!(any(target_os = "windows", target_os = "android")) {
            "http://atlas.localhost"
        } else {
            "atlas://localhost"
        };
        Ok(MediaDisplayPage {
            revision: page.revision,
            total: page.total,
            offset: page.offset,
            next_offset: page.next_offset,
            assets: page
                .assets
                .into_iter()
                .map(|asset| MediaDisplayAsset {
                    url: format!("{origin}/research/{revision}/{}", asset.id),
                    id: asset.id,
                    width: asset.width,
                    height: asset.height,
                })
                .collect(),
        })
    }
    pub fn media_asset(&self, revision: &str, id: &str) -> Result<Vec<u8>> {
        azeroth_local_data::media::asset(&self.store()?, revision, id).map_err(error)
    }
    /// Serve only exact opaque revision/asset IDs; never interpret a file path.
    pub fn research_asset(&self, method: &str, path: &str, has_query: bool) -> Option<Vec<u8>> {
        if method != "GET" || has_query || path.len() != 139 {
            return None;
        }
        let selected = path.strip_prefix("/research/")?;
        let (revision, id) = selected.split_once('/')?;
        if !azeroth_core::local_data::is_sha256(revision)
            || !azeroth_core::local_data::is_sha256(id)
        {
            return None;
        }
        self.media_asset(revision, id).ok()
    }
    pub fn media_manifest(
        &self,
        revision: &str,
    ) -> Result<azeroth_core::local_data_media::MediaManifest> {
        azeroth_local_data::media::verify(&self.store()?, revision).map_err(error)
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
            detectors: Mutex::default(),
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
            locales: Vec::new(),
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
