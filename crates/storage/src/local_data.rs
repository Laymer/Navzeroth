//! Separate, bounded private lifecycle index. Canonical catalogs remain immutable files.
use crate::error::{SqliteResult, invalid, storage};
use azeroth_core::{error::Result, local_data::PerformancePreset, local_data_lifecycle::*};
use rusqlite::{Connection, OptionalExtension, TransactionBehavior, params};
use serde::{Deserialize, Serialize};
use std::{
    path::{Path, PathBuf},
    time::Duration,
};

const APPLICATION_ID: i64 = 0x4e4c4431;
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SavedSelection {
    pub selection_id: String,
    pub installation: PathBuf,
    pub product: String,
    pub preset: PerformancePreset,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SelectedMaterial {
    pub id: String,
    pub kind: azeroth_core::local_data_workflow::MaterialKind,
    pub path: PathBuf,
}
pub struct LocalDataIndex {
    conn: Connection,
}
fn encode<T: Serialize>(value: &T) -> Result<String> {
    let text =
        serde_json::to_string(value).map_err(|_| invalid("Cannot encode lifecycle record"))?;
    if text.len() > MAX_CHECKPOINT_BYTES {
        return Err(invalid("Lifecycle record exceeds capacity"));
    }
    Ok(text)
}
fn decode<T: serde::de::DeserializeOwned>(text: &str) -> Result<T> {
    let value = azeroth_core::strict_json::value(text.as_bytes(), MAX_CHECKPOINT_BYTES)
        .map_err(|_| invalid("Invalid lifecycle record"))?;
    serde_json::from_value(value).map_err(|_| invalid("Unsupported lifecycle record"))
}
fn status_record(text: &str, id: &str) -> Result<JobStatus> {
    let status: JobStatus = decode(text)?;
    if status.format_version != VERSION
        || status.job_id != id
        || !valid_id(id)
        || status.completed > status.total
        || status.total > 32768
        || status.error.as_ref().is_some_and(|s| s.len() > 512)
        || (status.state == JobState::Complete) != status.summary.is_some()
    {
        return Err(invalid("Invalid or unsupported job status"));
    }
    Ok(status)
}
impl LocalDataIndex {
    pub fn open(path: &Path) -> Result<Self> {
        let conn = Connection::open(path).db()?;
        conn.busy_timeout(Duration::from_secs(5)).db()?;
        let application: i64 = conn
            .pragma_query_value(None, "application_id", |r| r.get(0))
            .db()?;
        let version: i64 = conn
            .pragma_query_value(None, "user_version", |r| r.get(0))
            .db()?;
        let tables: i64 = conn
            .query_row("SELECT count(*) FROM sqlite_master", [], |r| r.get(0))
            .db()?;
        if !((application == 0 && version == 0 && tables == 0)
            || (application == APPLICATION_ID && matches!(version, 1..=3)))
        {
            return Err(invalid("Unsupported private lifecycle database"));
        }
        let page_size: i64 = conn
            .pragma_query_value(None, "page_size", |r| r.get(0))
            .db()?;
        let journal: String = conn
            .pragma_query_value(None, "journal_mode", |r| r.get(0))
            .db()?;
        if page_size != 4096 || journal != "delete" {
            return Err(invalid("Unsupported lifecycle page size or journal mode"));
        }
        conn.pragma_update(None, "foreign_keys", true).db()?;
        conn.pragma_update(None, "max_page_count", 16384).db()?;
        conn.pragma_update(None, "synchronous", "FULL").db()?;
        if version == 0 {
            conn.execute_batch("BEGIN IMMEDIATE;
                CREATE TABLE selections(id TEXT PRIMARY KEY, document TEXT NOT NULL);
                CREATE TABLE jobs(id TEXT PRIMARY KEY, request TEXT NOT NULL, checkpoint TEXT NOT NULL, status TEXT NOT NULL);
                CREATE TABLE activations(scope TEXT NOT NULL, generation INTEGER NOT NULL, catalog TEXT NOT NULL, PRIMARY KEY(scope,generation));
                PRAGMA application_id=1313621041; PRAGMA user_version=1; COMMIT;").db()?;
        }
        if version < 2 {
            conn.execute_batch("BEGIN IMMEDIATE;
                CREATE TABLE delivery_activations(scope TEXT NOT NULL, generation INTEGER NOT NULL, delivery TEXT NOT NULL, PRIMARY KEY(scope,generation));
                PRAGMA user_version=2; COMMIT;").db()?;
        }
        if version < 3 {
            conn.execute_batch("BEGIN IMMEDIATE;
                CREATE TABLE workflow_preferences(id INTEGER PRIMARY KEY CHECK(id=1), revision INTEGER NOT NULL, document TEXT NOT NULL);
                CREATE TABLE selected_materials(id TEXT PRIMARY KEY, document TEXT NOT NULL);
                CREATE TABLE change_candidates(selection TEXT PRIMARY KEY, digest TEXT NOT NULL);
                PRAGMA user_version=3; COMMIT;").db()?;
        }
        Ok(Self { conn })
    }
    pub fn workflow(&self) -> Result<azeroth_core::local_data_workflow::PreferencesRecord> {
        let row: Option<(u32, String)> = self
            .conn
            .query_row(
                "SELECT revision,document FROM workflow_preferences WHERE id=1",
                [],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .optional()
            .db()?;
        let (revision, preferences) = if let Some((revision, text)) = row {
            if revision == 0 || revision > 1_000_000 || text.len() > 64 * 1024 {
                return Err(invalid("Invalid workflow preferences record"));
            }
            let value: azeroth_core::local_data_workflow::Preferences = decode(&text)?;
            value.validate().map_err(invalid)?;
            (revision, value)
        } else {
            (0, azeroth_core::local_data_workflow::Preferences::default())
        };
        Ok(azeroth_core::local_data_workflow::PreferencesRecord {
            format_version: 1,
            revision: u64::from(revision),
            preferences,
        })
    }
    pub fn replace_workflow(
        &mut self,
        expected: u64,
        preferences: &azeroth_core::local_data_workflow::Preferences,
    ) -> Result<azeroth_core::local_data_workflow::PreferencesRecord> {
        preferences.validate().map_err(invalid)?;
        let text = encode(preferences)?;
        if text.len() > 64 * 1024 {
            return Err(invalid("Workflow preferences capacity exceeded"));
        }
        let tx = self
            .conn
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .db()?;
        let current: u32 = tx
            .query_row(
                "SELECT coalesce((SELECT revision FROM workflow_preferences WHERE id=1),0)",
                [],
                |r| r.get(0),
            )
            .db()?;
        if expected != u64::from(current) || current >= 1_000_000 {
            return Err(invalid(
                "Workflow choices changed or revision capacity reached",
            ));
        }
        for (id, choices) in &preferences.selections {
            let record: String = tx
                .query_row("SELECT document FROM selections WHERE id=?1", [id], |r| {
                    r.get(0)
                })
                .db()?;
            let mut selected: SavedSelection = decode(&record)?;
            selected.preset = choices.preset;
            tx.execute(
                "UPDATE selections SET document=?2 WHERE id=?1",
                params![id, encode(&selected)?],
            )
            .db()?;
        }
        tx.execute("INSERT INTO workflow_preferences VALUES(1,?1,?2) ON CONFLICT(id) DO UPDATE SET revision=excluded.revision,document=excluded.document",params![current+1,text]).db()?;
        tx.commit().db()?;
        Ok(azeroth_core::local_data_workflow::PreferencesRecord {
            format_version: 1,
            revision: u64::from(current) + 1,
            preferences: preferences.clone(),
        })
    }
    pub fn save_material(&mut self, selection: &SelectedMaterial) -> Result<()> {
        if !azeroth_core::local_data::is_sha256(&selection.id) || !selection.path.is_absolute() {
            return Err(invalid("Invalid selected material"));
        }
        let text = encode(selection)?;
        if text.len() > 8192 {
            return Err(invalid("Selected material path capacity exceeded"));
        }
        let tx = self
            .conn
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .db()?;
        let count: u32 = tx
            .query_row(
                "SELECT count(*) FROM selected_materials WHERE id<>?1",
                [&selection.id],
                |r| r.get(0),
            )
            .db()?;
        if count >= 32 {
            return Err(invalid("Selected material capacity reached"));
        }
        tx.execute("INSERT INTO selected_materials VALUES(?1,?2) ON CONFLICT(id) DO UPDATE SET document=excluded.document",params![selection.id,text]).db()?;
        tx.commit().db()
    }
    /// Forget only the selected path; immutable imported artifacts are retained.
    pub fn forget_material(&mut self, id: &str) -> Result<()> {
        if !azeroth_core::local_data::is_sha256(id) {
            return Err(invalid("Invalid material selection identity"));
        }
        self.conn
            .execute("DELETE FROM selected_materials WHERE id=?1", [id])
            .db()?;
        Ok(())
    }
    pub fn material(&self, id: &str) -> Result<SelectedMaterial> {
        if !azeroth_core::local_data::is_sha256(id) {
            return Err(invalid("Invalid material selection identity"));
        }
        let text: String = self
            .conn
            .query_row(
                "SELECT document FROM selected_materials WHERE id=?1",
                [id],
                |r| r.get(0),
            )
            .db()?;
        let value: SelectedMaterial = decode(&text)?;
        if value.id != id || !value.path.is_absolute() {
            return Err(invalid("Invalid selected material record"));
        }
        Ok(value)
    }
    pub fn materials(&self) -> Result<Vec<SelectedMaterial>> {
        let mut query = self
            .conn
            .prepare("SELECT id FROM selected_materials ORDER BY id LIMIT 33")
            .db()?;
        let ids = query
            .query_map([], |r| r.get::<_, String>(0))
            .db()?
            .collect::<std::result::Result<Vec<_>, _>>()
            .db()?;
        if ids.len() > 32 {
            return Err(invalid("Selected material capacity exceeded"));
        }
        ids.iter().map(|id| self.material(id)).collect()
    }
    pub fn set_change_candidate(&mut self, selection: &str, digest: &str) -> Result<()> {
        if !azeroth_core::local_data::is_sha256(digest) {
            return Err(invalid("Invalid change candidate identity"));
        }
        self.selection(selection)?;
        self.conn.execute("INSERT INTO change_candidates VALUES(?1,?2) ON CONFLICT(selection) DO UPDATE SET digest=excluded.digest",params![selection,digest]).db()?;
        Ok(())
    }
    pub fn change_candidate(&self, selection: &str) -> Result<Option<String>> {
        self.selection(selection)?;
        let digest: Option<String> = self
            .conn
            .query_row(
                "SELECT digest FROM change_candidates WHERE selection=?1",
                [selection],
                |r| r.get(0),
            )
            .optional()
            .db()?;
        if digest
            .as_ref()
            .is_some_and(|d| !azeroth_core::local_data::is_sha256(d))
        {
            return Err(invalid("Invalid staged change record"));
        }
        Ok(digest)
    }
    pub fn save_selection(&mut self, selection: &SavedSelection) -> Result<()> {
        if !valid_id(&selection.selection_id)
            || selection.product != "wow_classic_beta"
            || !selection.installation.is_absolute()
        {
            return Err(invalid("Invalid saved installation selection"));
        }
        let document = encode(selection)?;
        if document.len() > 8192 {
            return Err(invalid("Installation selection exceeds capacity"));
        }
        let tx = self
            .conn
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .db()?;
        let count: i64 = tx
            .query_row(
                "SELECT count(*) FROM selections WHERE id<>?1",
                [&selection.selection_id],
                |r| r.get(0),
            )
            .db()?;
        if count >= MAX_SELECTIONS as i64 {
            return Err(invalid("Saved installation capacity reached"));
        }
        tx.execute("INSERT INTO selections VALUES(?1,?2) ON CONFLICT(id) DO UPDATE SET document=excluded.document", params![selection.selection_id,document]).db()?;
        tx.commit().db()
    }
    pub fn selections(&self) -> Result<Vec<SavedSelection>> {
        let mut query = self
            .conn
            .prepare("SELECT document FROM selections ORDER BY id LIMIT 17")
            .db()?;
        let rows = query.query_map([], |r| r.get::<_, String>(0)).db()?;
        let mut out = Vec::new();
        for row in rows {
            out.push(decode(&row.db()?)?);
        }
        if out.len() > MAX_SELECTIONS {
            return Err(storage("Saved selection capacity exceeded"));
        }
        Ok(out)
    }
    pub fn selection(&self, id: &str) -> Result<SavedSelection> {
        self.selections()?
            .into_iter()
            .find(|s| s.selection_id == id)
            .ok_or_else(|| invalid("Unknown installation selection"))
    }
    /// Caller owns the destination writer lock. The checkpoint is unbound until
    /// fingerprinting succeeds; cancellation is available throughout preparation.
    pub fn prepare(&mut self, id: &str, request: &str, resume: bool) -> Result<JobStatus> {
        if !valid_id(id) || request.len() > 65536 {
            return Err(invalid("Invalid lifecycle job"));
        }
        let tx = self
            .conn
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .db()?;
        let old: Option<String> = tx
            .query_row("SELECT status FROM jobs WHERE id=?1", [id], |r| r.get(0))
            .optional()
            .db()?;
        let mut status = match (old, resume) {
            (Some(text), true) => {
                let status = status_record(&text, id)?;
                if status.state == JobState::Complete {
                    return Err(invalid("Completed extraction cannot resume"));
                }
                status
            }
            (None, false) => {
                let count: i64 = tx
                    .query_row("SELECT count(*) FROM jobs", [], |r| r.get(0))
                    .db()?;
                if count >= MAX_JOBS as i64 {
                    return Err(invalid("Job capacity reached; forget a terminal job"));
                }
                JobStatus {
                    format_version: VERSION,
                    job_id: id.into(),
                    state: JobState::Preparing,
                    completed: 0,
                    total: 0,
                    summary: None,
                    error: None,
                }
            }
            _ => return Err(invalid("Job already exists or resume request is missing")),
        };
        status.state = JobState::Preparing;
        status.summary = None;
        status.error = None;
        tx.execute("INSERT INTO jobs VALUES(?1,?2,'null',?3) ON CONFLICT(id) DO UPDATE SET status=excluded.status",params![id,request,encode(&status)?]).db()?;
        tx.commit().db()?;
        Ok(status)
    }
    pub fn begin(
        &mut self,
        id: &str,
        request: &str,
        checkpoint: &Checkpoint,
        resume: bool,
    ) -> Result<Checkpoint> {
        if !valid_id(id) || request.len() > 65536 {
            return Err(invalid("Invalid lifecycle job"));
        }
        checkpoint.validate().map_err(invalid)?;
        let tx = self
            .conn
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .db()?;
        let existing: Option<String> = tx
            .query_row("SELECT checkpoint FROM jobs WHERE id=?1", [id], |r| {
                r.get(0)
            })
            .optional()
            .db()?;
        if existing.is_some() {
            let text: String = tx
                .query_row("SELECT status FROM jobs WHERE id=?1", [id], |r| r.get(0))
                .db()?;
            let status = status_record(&text, id)?;
            if existing.as_deref() == Some("null")
                && (status.state != JobState::Preparing
                    || status.completed != 0
                    || status.total != 0)
            {
                return Err(invalid("Invalid unbound preparation checkpoint"));
            }
            if !matches!(status.state, JobState::Preparing | JobState::Running) {
                return Err(invalid("Job cancelled or not prepared for resume"));
            }
        }
        let checkpoint = match (existing, resume) {
            (Some(text), _) if text == "null" => checkpoint.clone(),
            (Some(text), true) => {
                let old: Checkpoint = decode(&text)?;
                old.validate().map_err(invalid)?;
                if old.binding != checkpoint.binding {
                    return Err(invalid(
                        "Source, selection or extractor changed; resume refused",
                    ));
                }
                let status: String = tx
                    .query_row("SELECT status FROM jobs WHERE id=?1", [id], |r| r.get(0))
                    .db()?;
                let status: JobStatus = decode(&status)?;
                if status.state == JobState::Complete {
                    return Err(invalid("Completed extraction cannot resume"));
                }
                old
            }
            (None, false) => {
                let count: i64 = tx
                    .query_row("SELECT count(*) FROM jobs", [], |r| r.get(0))
                    .db()?;
                if count >= MAX_JOBS as i64 {
                    return Err(invalid("Job capacity reached; forget a terminal job"));
                }
                checkpoint.clone()
            }
            _ => {
                return Err(invalid(
                    "Job already exists or resume checkpoint is missing",
                ));
            }
        };
        let status = JobStatus {
            format_version: VERSION,
            job_id: id.into(),
            state: JobState::Running,
            completed: checkpoint.tables.len() as u32,
            total: checkpoint.binding.total,
            summary: None,
            error: None,
        };
        tx.execute("INSERT INTO jobs VALUES(?1,?2,?3,?4) ON CONFLICT(id) DO UPDATE SET checkpoint=excluded.checkpoint,status=excluded.status", params![id,request,encode(&checkpoint)?,encode(&status)?]).db()?;
        tx.commit().db()?;
        Ok(checkpoint)
    }
    pub fn status(&self, id: &str) -> Result<JobStatus> {
        let text: Option<String> = self
            .conn
            .query_row("SELECT status FROM jobs WHERE id=?1", [id], |r| r.get(0))
            .optional()
            .db()?;
        let status = status_record(&text.ok_or_else(|| invalid("Unknown local-data job"))?, id)?;
        if status.format_version != VERSION
            || status.job_id != id
            || status.completed > status.total
        {
            return Err(invalid("Invalid job status"));
        }
        Ok(status)
    }
    pub fn request(&self, id: &str) -> Result<String> {
        self.conn
            .query_row("SELECT request FROM jobs WHERE id=?1", [id], |r| r.get(0))
            .optional()
            .db()?
            .ok_or_else(|| invalid("Unknown local-data job"))
    }
    pub fn jobs(&self) -> Result<Vec<JobStatus>> {
        let mut query = self
            .conn
            .prepare("SELECT id FROM jobs ORDER BY id LIMIT 9")
            .db()?;
        let ids = query
            .query_map([], |r| r.get::<_, String>(0))
            .db()?
            .collect::<rusqlite::Result<Vec<_>>>()
            .db()?;
        if ids.len() > MAX_JOBS {
            return Err(invalid("Job capacity exceeded"));
        }
        ids.iter().map(|id| self.status(id)).collect()
    }
    pub fn checkpoint(&mut self, id: &str, checkpoint: &Checkpoint) -> Result<()> {
        checkpoint.validate().map_err(invalid)?;
        let tx = self
            .conn
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .db()?;
        let old: String = tx
            .query_row("SELECT checkpoint FROM jobs WHERE id=?1", [id], |r| {
                r.get(0)
            })
            .db()?;
        let old: Checkpoint = decode(&old)?;
        if old.binding != checkpoint.binding
            || old.tables.len() > checkpoint.tables.len()
            || serde_json::to_value(&old.tables).map_err(|_| invalid("Invalid checkpoint"))?
                != serde_json::to_value(&checkpoint.tables[..old.tables.len()])
                    .map_err(|_| invalid("Invalid checkpoint"))?
        {
            return Err(invalid("Checkpoint binding or completed prefix changed"));
        }
        let text: String = tx
            .query_row("SELECT status FROM jobs WHERE id=?1", [id], |r| r.get(0))
            .db()?;
        let mut status = status_record(&text, id)?;
        if status.state != JobState::Running {
            return Err(invalid("Extraction cancelled or interrupted"));
        }
        if checkpoint.tables.len() < status.completed as usize
            || checkpoint.binding.total != status.total
        {
            return Err(invalid("Checkpoint progress regressed"));
        }
        status.completed = checkpoint.tables.len() as u32;
        tx.execute(
            "UPDATE jobs SET checkpoint=?2,status=?3 WHERE id=?1",
            params![id, encode(checkpoint)?, encode(&status)?],
        )
        .db()?;
        tx.commit().db()
    }
    pub fn finish(
        &mut self,
        id: &str,
        state: JobState,
        summary: Option<azeroth_core::local_data_catalog::ExtractionSummary>,
    ) -> Result<JobStatus> {
        let tx = self
            .conn
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .db()?;
        let text: String = tx
            .query_row("SELECT status FROM jobs WHERE id=?1", [id], |r| r.get(0))
            .db()?;
        let mut status = status_record(&text, id)?;
        if !matches!(status.state, JobState::Preparing | JobState::Running) {
            return Ok(status);
        }
        if matches!(state, JobState::Preparing | JobState::Running) {
            return Err(invalid("Expected terminal job state"));
        }
        if state == JobState::Complete
            && (status.state != JobState::Running
                || status.completed != status.total
                || summary.as_ref().is_none_or(|s| {
                    s.tables_processed != status.total
                        || !azeroth_core::local_data::is_sha256(&s.catalog_sha256)
                }))
        {
            return Err(invalid("Incomplete extraction cannot finish"));
        }
        status.state = state;
        status.summary = summary;
        status.error = match state {
            JobState::Failed => Some("Extraction failed; checkpoint retained".into()),
            JobState::Interrupted => Some("Worker interrupted; checkpoint retained".into()),
            _ => None,
        };
        tx.execute(
            "UPDATE jobs SET status=?2 WHERE id=?1",
            params![id, encode(&status)?],
        )
        .db()?;
        tx.commit().db()?;
        Ok(status)
    }
    pub fn forget(&mut self, id: &str) -> Result<()> {
        if matches!(
            self.status(id)?.state,
            JobState::Preparing | JobState::Running
        ) {
            return Err(invalid("Cannot forget a running job"));
        }
        self.conn
            .execute("DELETE FROM jobs WHERE id=?1", [id])
            .db()?;
        Ok(())
    }
    pub fn active(&self, scope: &str) -> Result<Activation> {
        self.active_in(scope, false)
    }
    pub fn delivery_active(
        &self,
        scope: &str,
    ) -> Result<azeroth_core::local_data_delivery::Activation> {
        let active = self.active_in(scope, true)?;
        Ok(azeroth_core::local_data_delivery::Activation {
            format_version: 1,
            scope: active.scope,
            generation: active.generation,
            delivery_sha256: active.catalog_sha256,
            verified_for_planning: false,
        })
    }
    fn active_in(&self, scope: &str, delivery: bool) -> Result<Activation> {
        let (table, column) = if delivery {
            ("delivery_activations", "delivery")
        } else {
            ("activations", "catalog")
        };
        if !azeroth_core::local_data::is_sha256(scope) {
            return Err(invalid("Invalid activation scope"));
        }
        let row: Option<(u32,String)> = self.conn.query_row(&format!("SELECT generation,{column} FROM {table} WHERE scope=?1 ORDER BY generation DESC LIMIT 1"), [scope], |r| Ok((r.get(0)?,r.get(1)?))).optional().db()?;
        if row.as_ref().is_some_and(|(generation, digest)| {
            *generation == 0 || *generation > 1024 || !azeroth_core::local_data::is_sha256(digest)
        }) {
            return Err(invalid("Invalid research activation history"));
        }
        Ok(Activation {
            format_version: VERSION,
            scope: scope.into(),
            generation: row.as_ref().map_or(0, |r| u64::from(r.0)),
            catalog_sha256: row.map(|r| r.1),
            verified_for_planning: false,
        })
    }
    /// Caller holds the private-store writer lock and verifies the full catalog closure.
    pub fn activate(
        &mut self,
        scope: &str,
        digest: &str,
        expected: u64,
        rollback: bool,
    ) -> Result<Activation> {
        self.activate_in(scope, digest, expected, rollback, false)
    }
    pub fn activate_delivery(
        &mut self,
        scope: &str,
        digest: &str,
        expected: u64,
        rollback: bool,
    ) -> Result<azeroth_core::local_data_delivery::Activation> {
        let active = self.activate_in(scope, digest, expected, rollback, true)?;
        Ok(azeroth_core::local_data_delivery::Activation {
            format_version: 1,
            scope: active.scope,
            generation: active.generation,
            delivery_sha256: active.catalog_sha256,
            verified_for_planning: false,
        })
    }
    fn activate_in(
        &mut self,
        scope: &str,
        digest: &str,
        expected: u64,
        rollback: bool,
        delivery: bool,
    ) -> Result<Activation> {
        let (table, column) = if delivery {
            ("delivery_activations", "delivery")
        } else {
            ("activations", "catalog")
        };
        if !azeroth_core::local_data::is_sha256(scope)
            || !azeroth_core::local_data::is_sha256(digest)
        {
            return Err(invalid("Invalid catalog identity"));
        }
        let tx = self
            .conn
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .db()?;
        let current: u32 = tx
            .query_row(
                &format!("SELECT coalesce(max(generation),0) FROM {table} WHERE scope=?1"),
                [scope],
                |r| r.get(0),
            )
            .db()?;
        if u64::from(current) != expected || current >= 1024 {
            return Err(invalid("Activation changed or history capacity reached"));
        }
        if rollback {
            let exists: bool = tx
                .query_row(
                    &format!("SELECT EXISTS(SELECT 1 FROM {table} WHERE scope=?1 AND {column}=?2)"),
                    params![scope, digest],
                    |r| r.get(0),
                )
                .db()?;
            if !exists {
                return Err(invalid("Rollback target was never active in this scope"));
            }
        }
        tx.execute(
            &format!("INSERT INTO {table} VALUES(?1,?2,?3)"),
            params![scope, current + 1, digest],
        )
        .db()?;
        tx.commit().db()?;
        // No fallible re-read after commit: an unrelated writer must not turn a
        // successful durable activation into a reported failure.
        Ok(Activation {
            format_version: VERSION,
            scope: scope.into(),
            generation: u64::from(current) + 1,
            catalog_sha256: Some(digest.into()),
            verified_for_planning: false,
        })
    }
}

#[cfg(test)]
mod quota_tests {
    use super::*;
    #[test]
    fn sqlite_full_preserves_the_previous_checkpoint() {
        let path = std::env::temp_dir().join(format!(
            "nav-lifecycle-quota-{}.sqlite3",
            std::process::id()
        ));
        let mut db = LocalDataIndex::open(&path).unwrap();
        let cp = Checkpoint {
            format_version: 1,
            binding: CheckpointBinding {
                format_version: 1,
                source_sha256: "a".repeat(64),
                extractor_sha256: "b".repeat(64),
                selection_sha256: "c".repeat(64),
                preset: PerformancePreset::Small,
                total: 1,
            },
            tables: vec![],
            gaps: vec![],
        };
        db.begin("job", "private-request", &cp, false).unwrap();
        let pages: i64 = db
            .conn
            .pragma_query_value(None, "page_count", |r| r.get(0))
            .unwrap();
        db.conn
            .pragma_update(None, "max_page_count", pages)
            .unwrap();
        let mut large = cp.clone();
        large.gaps = (0..10000).map(|_| "x".repeat(512)).collect();
        assert!(db.checkpoint("job", &large).is_err());
        assert_eq!(db.status("job").unwrap().completed, 0);
        assert!(
            db.begin("job", "private-request", &cp, true)
                .unwrap()
                .gaps
                .is_empty()
        );
        drop(db);
        std::fs::remove_file(path).unwrap();
    }
}
