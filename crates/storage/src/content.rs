//! Immutable local reference revisions. Coverage errors prevent staging and
//! activation; coverage gaps are retained. Source policies and declared conflict
//! classifications are preserved, not interpreted as permission to redistribute.
use crate::{
    SqliteStore,
    error::{SqliteResult, invalid, storage},
    hash::write_canonical_to,
};
use azeroth_core::{
    contract::{Envelope, ReferenceDataset, decode_envelope},
    coverage::content_report,
    error::{AppError, ErrorCode, Result},
    sources::{ConflictKind, RecordKey, SourceConflict, SourceRecord, SourceRegistry},
    types::{DataContext, DatasetRevision, Sha256Hex},
};
use rusqlite::{Connection, OptionalExtension, TransactionBehavior, params};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;

pub use crate::content_catalog::{
    ActiveContentRevision, ContentRevisionV2, catalog_content_sha256,
};

pub const MAX_CONTENT_REVISIONS: usize = 1024;
pub const MAX_CONTENT_RECORDS: usize = 1024;
/// Envelope plus bounded source-policy/conflict records; no archive or Lua input.
pub const MAX_CONTENT_REVISION_BYTES: usize = 32 * 1024 * 1024;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ContentScope {
    pub context: DataContext,
    pub fixture: bool,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ContentRevisionKey {
    pub scope: ContentScope,
    pub revision: DatasetRevision,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ContentRevision {
    pub envelope: Envelope<ReferenceDataset>,
    pub sources: Vec<SourceRecord>,
    /// Classified source disagreements from the normalization pipeline. Full
    /// references are checked here; competing payloads are not reclassified.
    pub conflicts: Vec<SourceConflict>,
}
impl ContentRevision {
    pub fn key(&self) -> ContentRevisionKey {
        ContentRevisionKey {
            scope: ContentScope {
                context: self.envelope.data_context(),
                fixture: self.envelope.fixture,
            },
            revision: self.envelope.payload.revision.clone(),
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum StageOutcome {
    Inserted,
    Duplicate,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct ActivationOutcome {
    pub active: ContentRevisionKey,
    pub previous: Option<DatasetRevision>,
    pub changed: bool,
}
fn too_large() -> AppError {
    AppError::new(
        ErrorCode::ImportTooLarge,
        "Content revision limit exceeded",
        false,
    )
}
fn encode<T: Serialize>(value: &T) -> Result<String> {
    struct BoundedJson {
        bytes: Vec<u8>,
        exceeded: bool,
    }
    impl std::io::Write for BoundedJson {
        fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
            if bytes.len() > MAX_CONTENT_REVISION_BYTES - self.bytes.len() {
                self.exceeded = true;
                return Err(std::io::Error::other("content bound"));
            }
            self.bytes.extend_from_slice(bytes);
            Ok(bytes.len())
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }
    let mut output = BoundedJson {
        bytes: Vec::new(),
        exceeded: false,
    };
    if serde_json::to_writer(&mut output, value).is_err() {
        return Err(if output.exceeded {
            too_large()
        } else {
            storage("Content cannot be serialized")
        });
    }
    String::from_utf8(output.bytes).map_err(|_| storage("Content cannot be serialized"))
}
fn canonical_revision(input: &ContentRevision) -> Result<ContentRevision> {
    if input.sources.len() > MAX_CONTENT_RECORDS
        || input.conflicts.len() > MAX_CONTENT_RECORDS
        || input
            .conflicts
            .iter()
            .any(|c| c.others.len() > MAX_CONTENT_RECORDS)
    {
        return Err(too_large());
    }
    // Check the combined size before cloning policy/conflict records.
    encode(input)?;
    // Re-decode the typed envelope through bounded exchange validation.
    let envelope = decode_envelope::<ReferenceDataset>(&encode(&input.envelope)?)?;
    if !content_report(&envelope).passes {
        return Err(AppError::new(
            ErrorCode::DatasetIncomplete,
            "Content validation failed",
            false,
        ));
    }
    let registry = SourceRegistry::try_from(input.sources.clone())?;
    for evidence in &envelope.payload.evidence {
        let source = registry.ensure_registered(&evidence.source)?;
        if source
            .editions
            .value()
            .is_some_and(|editions| !editions.contains(&evidence.context.build.edition))
        {
            return Err(AppError::new(
                ErrorCode::BuildMismatch,
                "Source does not cover content edition",
                false,
            ));
        }
    }
    for image in &envelope.payload.map_images {
        let source = registry.ensure_registered(&image.source)?;
        if source
            .editions
            .value()
            .is_some_and(|editions| !editions.contains(&image.map.context.build.edition))
        {
            return Err(AppError::new(
                ErrorCode::BuildMismatch,
                "Image source does not cover content edition",
                false,
            ));
        }
    }
    let evidence_ids: BTreeSet<_> = envelope.payload.evidence.iter().map(|e| &e.id).collect();
    let context = envelope.data_context();
    let mut conflicts = input.conflicts.clone();
    for conflict in &mut conflicts {
        if conflict.key.context() != &context {
            return Err(AppError::new(
                ErrorCode::BuildMismatch,
                "Conflict belongs to another context",
                false,
            ));
        }
        let found = match &conflict.key {
            RecordKey::Item(key) => envelope.payload.items.iter().any(|item| &item.key == key),
            RecordKey::Quest { quest, .. } => {
                envelope.payload.quests.iter().any(|q| &q.id == quest)
            }
            RecordKey::Dungeon { .. } => false, // v1 has no dungeon records.
        };
        if !found
            || !evidence_ids.contains(&conflict.preferred)
            || conflict.others.iter().any(|id| !evidence_ids.contains(id))
        {
            return Err(invalid("Conflict references missing content"));
        }
        if conflict.others.len() > MAX_CONTENT_RECORDS {
            return Err(too_large());
        }
        if conflict.others.contains(&conflict.preferred)
            || (conflict.others.is_empty() && conflict.kind != ConflictKind::CrossEditionRelabel)
        {
            return Err(invalid(
                "Conflict must identify distinct competing evidence",
            ));
        }
        let unique: BTreeSet<_> = conflict.others.iter().collect();
        if unique.len() != conflict.others.len() {
            return Err(invalid("Conflict repeats evidence"));
        }
        conflict.others.sort();
    }
    conflicts.sort();
    if conflicts.windows(2).any(|pair| pair[0] == pair[1]) {
        return Err(invalid("Content repeats a conflict"));
    }
    let revision = ContentRevision {
        envelope,
        sources: Vec::from(registry),
        conflicts,
    };
    if encode(&revision)?.len() > MAX_CONTENT_REVISION_BYTES {
        return Err(too_large());
    }
    Ok(revision)
}
pub(crate) fn hash<T: Serialize>(revision: &T) -> Result<Sha256Hex> {
    let value =
        serde_json::to_value(revision).map_err(|_| storage("Content cannot be serialized"))?;
    struct DigestWriter(Sha256);
    impl std::io::Write for DigestWriter {
        fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
            self.0.update(bytes);
            Ok(bytes.len())
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }
    let mut writer = DigestWriter(Sha256::new());
    write_canonical_to(&value, &mut writer)?;
    let digest = writer.0.finalize();
    Sha256Hex::new(
        digest
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect::<String>(),
    )
}
pub(crate) fn namespace(scope: &ContentScope) -> &'static str {
    match scope.context.namespace {
        azeroth_core::types::DatasetNamespace::Demo => "demo",
        azeroth_core::types::DatasetNamespace::Reference => "reference",
    }
}
pub(crate) fn find(conn: &Connection, key: &ContentRevisionKey) -> Result<Option<i64>> {
    conn.query_row("SELECT id FROM dataset_revisions WHERE edition=?1 AND build=?2 AND interface=?3 AND namespace=?4 AND is_fixture=?5 AND revision=?6",
        params![key.scope.context.build.edition.as_str(), key.scope.context.build.build.as_str(), key.scope.context.build.interface.as_str(), namespace(&key.scope), key.scope.fixture, key.revision.as_str()],
        |row| row.get(0)).optional().db()
}
/// Malformed retained data may be skipped during restriction fallback; genuine
/// SQLite failures must abort the whole restriction transaction.
pub(crate) enum LoadFailure {
    Invalid(AppError),
    Database(AppError),
}
impl From<AppError> for LoadFailure {
    fn from(e: AppError) -> Self {
        Self::Invalid(e)
    }
}
impl LoadFailure {
    pub(crate) fn error(self) -> AppError {
        match self {
            Self::Invalid(e) | Self::Database(e) => e,
        }
    }
}
pub(crate) fn load_sql_error(e: rusqlite::Error) -> LoadFailure {
    match e {
        rusqlite::Error::InvalidColumnType(..)
        | rusqlite::Error::IntegralValueOutOfRange(..)
        | rusqlite::Error::FromSqlConversionFailure(..)
        | rusqlite::Error::QueryReturnedNoRows => {
            LoadFailure::Invalid(storage("Stored content metadata is invalid"))
        }
        _ => LoadFailure::Database(crate::error::from_sqlite(e)),
    }
}
#[cfg(test)]
thread_local! {
    pub(crate) static FULL_LOADS: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
}
pub(crate) fn load(conn: &Connection, id: i64) -> Result<ActiveContentRevision> {
    #[cfg(test)]
    FULL_LOADS.with(|count| count.set(count.get() + 1));
    load_checked(conn, id).map_err(LoadFailure::error)
}
pub(crate) fn load_checked(
    conn: &Connection,
    id: i64,
) -> std::result::Result<ActiveContentRevision, LoadFailure> {
    let version: i64 = conn
        .query_row(
            "SELECT content_version FROM dataset_revisions WHERE id=?1",
            [id],
            |r| r.get(0),
        )
        .map_err(load_sql_error)?;
    match version {
        1 => load_v1_checked(conn, id).map(|r| ActiveContentRevision::V1(std::sync::Arc::new(r))),
        2 => crate::content_catalog::load_checked(conn, id)
            .map(|r| ActiveContentRevision::V2(std::sync::Arc::new(r))),
        _ => Err(storage("Unsupported stored content version").into()),
    }
}
fn load_v1_checked(
    conn: &Connection,
    id: i64,
) -> std::result::Result<ContentRevision, LoadFailure> {
    let (document, digest, coverage): (String, String, String) = conn.query_row(
        "SELECT r.document, m.content_sha256, m.coverage_document FROM dataset_revisions r JOIN dataset_manifests m ON m.revision_id=r.id WHERE r.id=?1 AND length(CAST(r.document AS BLOB)) <= ?2 AND length(CAST(m.coverage_document AS BLOB)) <= ?2 AND m.manifest_version=1",
        params![id,MAX_CONTENT_REVISION_BYTES as i64], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?))).optional().map_err(load_sql_error)?.ok_or_else(|| LoadFailure::Invalid(storage("Stored content metadata is missing or exceeds bounds")))?;
    if document.len() > MAX_CONTENT_REVISION_BYTES {
        return Err(storage("Stored content exceeds bounds").into());
    }
    let revision: ContentRevision =
        serde_json::from_str(&document).map_err(|_| storage("Stored content is invalid"))?;
    let revision = canonical_revision(&revision)?;
    if hash(&revision)?.as_str() != digest {
        return Err(storage("Stored content hash disagrees").into());
    }
    if coverage != encode(&content_report(&revision.envelope))? {
        return Err(storage("Stored content coverage disagrees").into());
    }
    for (sql, expected) in [
        (
            "SELECT CASE WHEN length(CAST(document AS BLOB))<=?2 THEN document END FROM source_records WHERE revision_id=?1 ORDER BY source_id LIMIT 1025",
            revision
                .sources
                .iter()
                .map(encode)
                .collect::<Result<Vec<_>>>()?,
        ),
        (
            "SELECT CASE WHEN length(CAST(document AS BLOB))<=?2 THEN document END FROM source_conflicts WHERE revision_id=?1 ORDER BY conflict_index LIMIT 1025",
            revision
                .conflicts
                .iter()
                .map(encode)
                .collect::<Result<Vec<_>>>()?,
        ),
    ] {
        let mut statement = conn.prepare(sql).map_err(load_sql_error)?;
        let mut rows = statement
            .query(params![id, MAX_CONTENT_REVISION_BYTES as i64])
            .map_err(load_sql_error)?;
        let mut index = 0;
        while let Some(row) = rows.next().map_err(load_sql_error)? {
            let document: String = row.get(0).map_err(load_sql_error)?;
            if expected.get(index) != Some(&document) {
                return Err(storage("Stored content records disagree").into());
            }
            index += 1;
        }
        if index != expected.len() {
            return Err(storage("Stored content records disagree").into());
        }
    }
    Ok(revision)
}
pub(crate) fn pointers(
    conn: &Connection,
    scope: &ContentScope,
) -> Result<Option<(i64, Option<i64>)>> {
    conn.query_row("SELECT revision_id, previous_revision_id FROM active_datasets WHERE edition=?1 AND build=?2 AND interface=?3 AND namespace=?4 AND is_fixture=?5",
        params![scope.context.build.edition.as_str(), scope.context.build.build.as_str(), scope.context.build.interface.as_str(), namespace(scope), scope.fixture],
        |row| Ok((row.get(0)?, row.get(1)?))).optional().db()
}
pub(crate) fn eligible(conn: &Connection, id: i64) -> Result<bool> {
    conn.query_row("SELECT EXISTS(SELECT 1 FROM content_dispositions WHERE revision_id=?1 AND state='eligible')",[id],|row|row.get(0)).db()
}

pub(crate) fn activate(
    conn: &Connection,
    id: i64,
    key: ContentRevisionKey,
) -> Result<ActivationOutcome> {
    crate::content_dispositions::ensure_eligible(conn, id)?;
    if !eligible(conn, id)? {
        return Err(invalid("Content revision is unavailable for activation"));
    }
    let previous_id = pointers(conn, &key.scope)?.map(|(id, _)| id);
    let previous = previous_id
        .map(|id| crate::content_dispositions::read(conn, id).map(|d| d.key.revision))
        .transpose()?;
    if previous_id == Some(id) {
        return Ok(ActivationOutcome {
            active: key,
            previous,
            changed: false,
        });
    }
    let ordinal: i64 = conn.query_row(
        "SELECT coalesce(max(d.last_activated_order),0) FROM content_dispositions d JOIN dataset_revisions r ON r.id=d.revision_id WHERE r.edition=?1 AND r.build=?2 AND r.interface=?3 AND r.namespace=?4 AND r.is_fixture=?5",
        params![key.scope.context.build.edition.as_str(),key.scope.context.build.build.as_str(),key.scope.context.build.interface.as_str(),namespace(&key.scope),key.scope.fixture], |row| row.get(0)
    ).db()?;
    if ordinal >= azeroth_core::types::MAX_SAFE_INTEGER as i64 {
        return Err(invalid("Content activation order is exhausted"));
    }
    if conn.execute("UPDATE content_dispositions SET last_activated_order=?1 WHERE revision_id=?2 AND state='eligible'", params![ordinal+1,id]).db()? != 1 {
        return Err(storage("Content disposition is unavailable"));
    }
    conn.execute("INSERT INTO active_datasets (edition, build, interface, namespace, is_fixture, revision_id, previous_revision_id) VALUES (?1,?2,?3,?4,?5,?6,?7) ON CONFLICT (edition,build,interface,namespace,is_fixture) DO UPDATE SET previous_revision_id=active_datasets.revision_id, revision_id=excluded.revision_id",
        params![key.scope.context.build.edition.as_str(),key.scope.context.build.build.as_str(),key.scope.context.build.interface.as_str(),namespace(&key.scope),key.scope.fixture,id,previous_id]).db()?;
    Ok(ActivationOutcome {
        active: key,
        previous,
        changed: true,
    })
}
impl SqliteStore {
    /// Store a validated immutable candidate without activating it. A reused
    /// full-context revision key must have identical canonical content.
    pub fn stage_content_revision(&mut self, input: &ContentRevision) -> Result<StageOutcome> {
        let revision = canonical_revision(input)?;
        let key = revision.key();
        let digest = hash(&revision)?;
        let document = encode(&revision)?;
        let report = encode(&content_report(&revision.envelope))?;
        let tx = self
            .conn
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .db()?;
        if let Some(id) = find(&tx, &key)? {
            if hash(&load(&tx, id)?)? != digest {
                return Err(invalid(
                    "Immutable content revision already has different content",
                ));
            }
            return Ok(StageOutcome::Duplicate);
        }
        let count: i64 = tx
            .query_row("SELECT COUNT(*) FROM dataset_revisions", [], |row| {
                row.get(0)
            })
            .db()?;
        if count >= MAX_CONTENT_REVISIONS as i64 {
            return Err(too_large());
        }
        tx.execute("INSERT INTO dataset_revisions (edition,build,interface,namespace,is_fixture,revision,document) VALUES (?1,?2,?3,?4,?5,?6,?7)",
            params![key.scope.context.build.edition.as_str(),key.scope.context.build.build.as_str(),key.scope.context.build.interface.as_str(),namespace(&key.scope),key.scope.fixture,key.revision.as_str(),document]).db()?;
        let id = tx.last_insert_rowid();
        tx.execute("INSERT INTO content_dispositions (revision_id,state,state_revision,reason_code) VALUES (?1,'eligible',1,'initial_eligibility')",[id]).db()?;
        for source in &revision.sources {
            tx.execute(
                "INSERT INTO source_records (revision_id,source_id,document) VALUES (?1,?2,?3)",
                params![id, source.id.as_str(), encode(source)?],
            )
            .db()?;
        }
        for (index, conflict) in revision.conflicts.iter().enumerate() {
            tx.execute("INSERT INTO source_conflicts (revision_id,conflict_index,document) VALUES (?1,?2,?3)", params![id,index as i64,encode(conflict)?]).db()?;
        }
        tx.execute("INSERT INTO dataset_manifests (revision_id,manifest_version,content_sha256,coverage_document) VALUES (?1,1,?2,?3)", params![id,digest.as_str(),report]).db()?;
        tx.commit().db()?;
        Ok(StageOutcome::Inserted)
    }
    /// Read an immutable staged or active revision without changing its disposition.
    pub fn catalog_revision(
        &self,
        key: &ContentRevisionKey,
    ) -> Result<Option<ActiveContentRevision>> {
        let get = |store: &Self| {
            let Some(id) = find(&store.conn, key)? else {
                return Ok(None);
            };
            let revision = store.cached_catalog(id)?;
            if revision.key() != *key {
                return Err(storage("Stored content identity disagrees"));
            }
            Ok(Some(revision))
        };
        if self.conn.is_autocommit() {
            self.with_immediate_read(get)
        } else {
            get(self)
        }
    }

    pub fn content_revision(&self, key: &ContentRevisionKey) -> Result<Option<ContentRevision>> {
        let Some(id) = find(&self.conn, key)? else {
            return Ok(None);
        };
        let revision = load(&self.conn, id)?;
        if revision.key() != *key {
            return Err(storage("Stored content identity disagrees"));
        }
        Ok(Some(revision.into_v1()?))
    }
    /// Revalidate the stored candidate under an immediate transaction before
    /// moving the full-context pointer. Failure preserves the previous pointer.
    pub fn activate_content_revision(
        &mut self,
        key: &ContentRevisionKey,
    ) -> Result<ActivationOutcome> {
        let tx = self
            .conn
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .db()?;
        let id = find(&tx, key)?.ok_or_else(|| invalid("Content revision is not stored"))?;
        let revision = load(&tx, id)?;
        if revision.key() != *key {
            return Err(storage("Stored content identity disagrees"));
        }
        let outcome = activate(&tx, id, key.clone())?;
        tx.commit().db()?;
        Ok(outcome)
    }
    pub fn active_content_revision(&self, scope: &ContentScope) -> Result<Option<ContentRevision>> {
        self.active_catalog_revision(scope)?
            .map(ActiveContentRevision::into_v1)
            .transpose()
    }
    pub fn active_catalog_revision(
        &self,
        scope: &ContentScope,
    ) -> Result<Option<ActiveContentRevision>> {
        let get = |s: &Self| {
            let Some((id, _)) = pointers(&s.conn, scope)? else {
                return Ok(None);
            };
            if !eligible(&s.conn, id)? {
                return Ok(None);
            };
            let disposition = crate::content_dispositions::ensure_eligible(&s.conn, id)?;
            let revision = s.cached_catalog(id)?;
            if disposition.key.scope != *scope || revision.key() != disposition.key {
                return Err(storage("Stored content identity disagrees"));
            }
            Ok(Some(revision))
        };
        if self.conn.is_autocommit() {
            self.with_immediate_read(get)
        } else {
            get(self)
        }
    }
    /// Restore the previously active immutable revision after revalidation.
    /// Rollback itself is an activation: a subsequent rollback toggles back.
    /// Explicit activation can select any retained revision.
    pub fn rollback_content_revision(
        &mut self,
        scope: &ContentScope,
    ) -> Result<Option<ActivationOutcome>> {
        let tx = self
            .conn
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .db()?;
        let Some((_, Some(previous))) = pointers(&tx, scope)? else {
            return Ok(None);
        };
        let revision = load(&tx, previous)?;
        if revision.key().scope != *scope {
            return Err(storage("Stored content identity disagrees"));
        }
        let outcome = activate(&tx, previous, revision.key())?;
        tx.commit().db()?;
        Ok(Some(outcome))
    }
}
