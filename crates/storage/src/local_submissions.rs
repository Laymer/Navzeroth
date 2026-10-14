//! Bounded local queue. Tombstones count toward capacity and never resurrect data.
use crate::{
    SqliteStore,
    content::{ContentRevisionKey, ContentScope},
    error::{SqliteResult, invalid, storage},
};
use azeroth_core::{
    error::Result,
    submissions::{LocalObservation, LocalPayload, MAX_DOCUMENT_BYTES, decode_local_observation},
    types::{
        BuildId, BuildKey, DataContext, DatasetNamespace, DatasetRevision, EditionId, InterfaceId,
        MAX_SAFE_INTEGER, Sha256Hex, SourceId, UtcMillis,
    },
};
use rusqlite::{OptionalExtension, Transaction, TransactionBehavior, params};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
pub const MAX_SUBMISSIONS: usize = 1024;
pub const MAX_PAGE: usize = 32;
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SubmissionState {
    Pending,
    Accepted,
    Rejected,
    Withdrawn,
    Revoked,
}
impl SubmissionState {
    fn as_str(self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Accepted => "accepted",
            Self::Rejected => "rejected",
            Self::Withdrawn => "withdrawn",
            Self::Revoked => "revoked",
        }
    }
    fn decision(self) -> &'static str {
        match self {
            Self::Pending => "pending_review",
            Self::Accepted => "accepted_local_review",
            Self::Rejected => "rejected_local_review",
            Self::Withdrawn => "withdrawn_by_submitter",
            Self::Revoked => "revoked_local_review",
        }
    }
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SubmissionGuard {
    pub id: Sha256Hex,
    pub revision: u64,
}
impl SubmissionGuard {
    pub fn validate(&self) -> Result<()> {
        if self.revision == 0 || self.revision > MAX_SAFE_INTEGER {
            return Err(invalid("Invalid local submission guard"));
        }
        Ok(())
    }
}
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct SubmissionRecord {
    pub guard: SubmissionGuard,
    pub base: ContentRevisionKey,
    pub source: SourceId,
    pub state: SubmissionState,
    pub decision_code: String,
    pub document: Option<LocalObservation>,
    pub created_at_utc_ms: Option<UtcMillis>,
    pub changed_at_utc_ms: Option<UtcMillis>,
}
#[derive(Clone, Debug, Serialize)]
pub struct SubmissionPage {
    pub rows: Vec<SubmissionRecord>,
    pub next: Option<Sha256Hex>,
    pub total: u32,
}
#[derive(Clone, Debug, Serialize)]
pub struct SubmitReceipt {
    pub duplicate: bool,
    pub record: SubmissionRecord,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SubmissionAction {
    Accept,
    Reject,
    Withdraw,
    Revoke,
    EraseRevoked,
}
fn canonical(doc: &LocalObservation) -> Result<String> {
    doc.validate()?;
    let text =
        serde_json::to_string(doc).map_err(|_| storage("Cannot encode local observation"))?;
    if text.len() > MAX_DOCUMENT_BYTES {
        return Err(invalid("Local observation exceeds size limit"));
    }
    decode_local_observation(&text)?;
    Ok(text)
}
fn identity(text: &str) -> Result<Sha256Hex> {
    Sha256Hex::new(format!("{:x}", Sha256::digest(text.as_bytes())))
}
fn base(doc: &LocalObservation) -> ContentRevisionKey {
    ContentRevisionKey {
        scope: ContentScope {
            context: doc.context.clone(),
            fixture: doc.fixture,
        },
        revision: doc.base_revision.clone(),
    }
}
fn namespace(scope: &ContentScope) -> &'static str {
    match scope.context.namespace {
        DatasetNamespace::Demo => "demo",
        DatasetNamespace::Reference => "reference",
    }
}
fn validate_links(store: &SqliteStore, record: &SubmissionRecord) -> Result<()> {
    let content = store
        .content_revision(&record.base)?
        .ok_or_else(|| storage("Local submission base is unavailable"))?;
    if !content.sources.iter().any(|s| s.id == record.source) {
        return Err(storage("Local submission source is unavailable"));
    }
    if let Some(doc) = &record.document {
        if base(doc) != record.base || doc.source != record.source {
            return Err(storage("Local submission identity is inconsistent"));
        }
        let evidence = content
            .envelope
            .payload
            .evidence
            .iter()
            .find(|e| e.id == doc.evidence && e.source == doc.source && e.context == doc.context)
            .ok_or_else(|| {
                invalid("Local observation evidence is not registered for this context")
            })?;
        let present = match &doc.payload {
            LocalPayload::QuestState { quest, .. } => {
                content.envelope.payload.quests.iter().any(|q| {
                    q.id == *quest && q.context == doc.context && q.evidence.contains(&evidence.id)
                })
            }
            LocalPayload::Price { item, .. } | LocalPayload::AcquisitionOutcome { item, .. } => {
                content.envelope.payload.items.iter().any(|i| {
                    i.key.item_id == *item
                        && i.key.context == doc.context
                        && i.evidence.contains(&evidence.id)
                })
            }
        };
        if !present {
            return Err(invalid(
                "Local observation target does not cite the registered evidence",
            ));
        }
    }
    Ok(())
}
fn read(store: &SqliteStore, id: &Sha256Hex) -> Result<Option<SubmissionRecord>> {
    let mut stmt=store.conn.prepare("SELECT CASE WHEN length(CAST(s.id AS BLOB))=64 THEN s.id END, CASE WHEN length(CAST(s.fingerprint AS BLOB))=64 THEN s.fingerprint END, CASE WHEN length(CAST(s.edition AS BLOB))<=256 THEN s.edition END,CASE WHEN length(CAST(s.build AS BLOB))<=256 THEN s.build END,CASE WHEN length(CAST(s.interface AS BLOB))<=256 THEN s.interface END,CASE WHEN length(CAST(s.namespace AS BLOB))<=16 THEN s.namespace END,s.is_fixture,CASE WHEN length(CAST(d.revision AS BLOB))<=256 THEN d.revision END,CASE WHEN length(CAST(s.source_id AS BLOB))<=128 THEN s.source_id END,CASE WHEN length(CAST(s.state AS BLOB))<=16 THEN s.state END,s.state_revision,s.consent_version,CASE WHEN s.document IS NULL OR length(CAST(s.document AS BLOB))<=16384 THEN s.document ELSE 'INVALID' END,CASE WHEN length(CAST(s.decision_code AS BLOB))<=32 THEN s.decision_code END,s.created_at_utc_ms,s.changed_at_utc_ms FROM local_submissions s JOIN dataset_revisions d ON d.id=s.base_revision_id AND d.edition=s.edition AND d.build=s.build AND d.interface=s.interface AND d.namespace=s.namespace AND d.is_fixture=s.is_fixture WHERE s.id=?1").db()?;
    let raw = stmt
        .query_row([id.as_str()], |r| {
            Ok((
                r.get::<_, Option<String>>(0)?,
                r.get::<_, Option<String>>(1)?,
                r.get::<_, String>(2)?,
                r.get::<_, String>(3)?,
                r.get::<_, String>(4)?,
                r.get::<_, String>(5)?,
                r.get::<_, i64>(6)?,
                r.get::<_, String>(7)?,
                r.get::<_, String>(8)?,
                r.get::<_, String>(9)?,
                r.get::<_, i64>(10)?,
                r.get::<_, u32>(11)?,
                r.get::<_, Option<String>>(12)?,
                r.get::<_, String>(13)?,
                r.get::<_, Option<i64>>(14)?,
                r.get::<_, Option<i64>>(15)?,
            ))
        })
        .optional()
        .db()?;
    let Some((
        stored_id,
        fingerprint,
        edition,
        build,
        interface,
        ns,
        fixture,
        revision,
        source,
        state,
        version,
        consent,
        document,
        decision,
        created,
        changed,
    )) = raw
    else {
        return Ok(None);
    };
    let state: SubmissionState = serde_json::from_value(serde_json::Value::String(state))
        .map_err(|_| storage("Invalid local submission state"))?;
    let scope = ContentScope {
        context: DataContext {
            build: BuildKey {
                edition: EditionId::new(edition)?,
                build: BuildId::new(build)?,
                interface: InterfaceId::new(interface)?,
            },
            namespace: match ns.as_str() {
                "demo" => DatasetNamespace::Demo,
                "reference" => DatasetNamespace::Reference,
                _ => return Err(storage("Invalid local submission context")),
            },
        },
        fixture: match fixture {
            0 => false,
            1 => true,
            _ => return Err(storage("Invalid local submission context")),
        },
    };
    let doc = document
        .as_deref()
        .map(decode_local_observation)
        .transpose()?;
    if stored_id.as_deref() != Some(id.as_str())
        || fingerprint.as_deref() != Some(id.as_str())
        || consent != 1
        || decision != state.decision()
        || (doc.is_none()
            && !matches!(state, SubmissionState::Withdrawn | SubmissionState::Revoked))
        || (doc.is_some() && state == SubmissionState::Withdrawn)
        || doc
            .as_ref()
            .map(canonical)
            .transpose()?
            .as_ref()
            .is_some_and(|s| identity(s).ok().as_ref() != Some(id))
    {
        return Err(storage("Stored local submission is inconsistent"));
    }
    let record = SubmissionRecord {
        guard: SubmissionGuard {
            id: id.clone(),
            revision: u64::try_from(version)
                .map_err(|_| storage("Invalid local submission revision"))?,
        },
        base: ContentRevisionKey {
            scope,
            revision: DatasetRevision::new(revision)?,
        },
        source: SourceId::new(source)?,
        state,
        decision_code: decision,
        document: doc,
        created_at_utc_ms: created.map(UtcMillis::new).transpose()?,
        changed_at_utc_ms: changed.map(UtcMillis::new).transpose()?,
    };
    record.guard.validate()?;
    validate_links(store, &record)?;
    Ok(Some(record))
}
impl SqliteStore {
    pub fn local_submission(&self, id: &Sha256Hex) -> Result<Option<SubmissionRecord>> {
        read(self, id)
    }
    pub fn local_submission_page(
        &self,
        after: Option<&Sha256Hex>,
        limit: usize,
    ) -> Result<SubmissionPage> {
        if limit == 0 || limit > MAX_PAGE {
            return Err(invalid("Local submission page must contain 1..32 rows"));
        }
        let total: u32 = self
            .conn
            .query_row("SELECT count(*) FROM local_submissions", [], |r| r.get(0))
            .db()?;
        if total as usize > MAX_SUBMISSIONS {
            return Err(storage("Stored local queue exceeds capacity"));
        }
        let mut stmt=self.conn.prepare("SELECT CASE WHEN length(CAST(id AS BLOB))=64 THEN id END FROM local_submissions WHERE ?1 IS NULL OR id>?1 ORDER BY id LIMIT ?2").db()?;
        let ids = stmt
            .query_map(
                params![after.map(Sha256Hex::as_str), (limit + 1) as i64],
                |r| r.get::<_, Option<String>>(0),
            )
            .db()?
            .collect::<rusqlite::Result<Vec<_>>>()
            .db()?;
        let mut ids = ids
            .into_iter()
            .map(|v| Sha256Hex::new(v.ok_or_else(|| storage("Invalid local submission identity"))?))
            .collect::<Result<Vec<_>>>()?;
        let more = ids.len() > limit;
        ids.truncate(limit);
        let next = if more { ids.last().cloned() } else { None };
        let rows = ids
            .iter()
            .map(|id| read(self, id)?.ok_or_else(|| storage("Local submission disappeared")))
            .collect::<Result<Vec<_>>>()?;
        Ok(SubmissionPage { rows, next, total })
    }
    pub fn submit_local_observation(
        &mut self,
        doc: &LocalObservation,
        opt_in: bool,
        now: Option<UtcMillis>,
    ) -> Result<SubmitReceipt> {
        if !opt_in {
            return Err(invalid("Explicit local submission opt-in is required"));
        }
        let text = canonical(doc)?;
        let id = identity(&text)?;
        let tx = Transaction::new_unchecked(&self.conn, TransactionBehavior::Immediate).db()?;
        if let Some(record) = read(self, &id)? {
            tx.commit().db()?;
            return Ok(SubmitReceipt {
                duplicate: true,
                record,
            });
        }
        let count: i64 = tx
            .query_row("SELECT count(*) FROM local_submissions", [], |r| r.get(0))
            .db()?;
        if count >= MAX_SUBMISSIONS as i64 {
            return Err(invalid(
                "Local submission queue is full; erased tombstones remain reserved",
            ));
        }
        let record = SubmissionRecord {
            guard: SubmissionGuard {
                id: id.clone(),
                revision: 1,
            },
            base: base(doc),
            source: doc.source.clone(),
            state: SubmissionState::Pending,
            decision_code: SubmissionState::Pending.decision().to_owned(),
            document: Some(doc.clone()),
            created_at_utc_ms: now,
            changed_at_utc_ms: now,
        };
        validate_links(self, &record)?;
        let scope = &record.base.scope;
        let revision_id:i64=tx.query_row("SELECT id FROM dataset_revisions WHERE edition=?1 AND build=?2 AND interface=?3 AND namespace=?4 AND is_fixture=?5 AND revision=?6",params![scope.context.build.edition.as_str(),scope.context.build.build.as_str(),scope.context.build.interface.as_str(),namespace(scope),scope.fixture,record.base.revision.as_str()],|r|r.get(0)).db()?;
        tx.execute("INSERT INTO local_submissions(id,fingerprint,edition,build,interface,namespace,is_fixture,base_revision_id,source_id,state,state_revision,consent_version,document,decision_code,created_at_utc_ms,changed_at_utc_ms) VALUES(?1,?1,?2,?3,?4,?5,?6,?7,?8,'pending',1,1,?9,'pending_review',?10,?10)",params![id.as_str(),scope.context.build.edition.as_str(),scope.context.build.build.as_str(),scope.context.build.interface.as_str(),namespace(scope),scope.fixture,revision_id,doc.source.as_str(),text,now.map(UtcMillis::get)]).db()?;
        tx.commit().db()?;
        Ok(SubmitReceipt {
            duplicate: false,
            record,
        })
    }
    pub fn change_local_submission(
        &mut self,
        expected: &SubmissionGuard,
        action: SubmissionAction,
        now: Option<UtcMillis>,
    ) -> Result<SubmissionRecord> {
        expected.validate()?;
        let tx = Transaction::new_unchecked(&self.conn, TransactionBehavior::Immediate).db()?;
        let mut record =
            read(self, &expected.id)?.ok_or_else(|| invalid("No such local submission"))?;
        if record.guard != *expected {
            return Err(invalid("Local submission changed; refresh before deciding"));
        }
        let (state, erase) = match (record.state, action) {
            (SubmissionState::Pending, SubmissionAction::Accept) => {
                (SubmissionState::Accepted, false)
            }
            (SubmissionState::Pending, SubmissionAction::Reject) => {
                (SubmissionState::Rejected, false)
            }
            (
                SubmissionState::Pending | SubmissionState::Accepted | SubmissionState::Rejected,
                SubmissionAction::Withdraw,
            ) => (SubmissionState::Withdrawn, true),
            (
                SubmissionState::Pending | SubmissionState::Accepted | SubmissionState::Rejected,
                SubmissionAction::Revoke,
            ) => (SubmissionState::Revoked, false),
            (SubmissionState::Revoked, SubmissionAction::EraseRevoked) => {
                (SubmissionState::Revoked, true)
            }
            _ => {
                return Err(invalid(
                    "Local submission decision is not permitted from this state",
                ));
            }
        };
        let revision = record
            .guard
            .revision
            .checked_add(1)
            .filter(|v| *v <= MAX_SAFE_INTEGER)
            .ok_or_else(|| invalid("Local submission revision exhausted"))?;
        let updated=tx.execute("UPDATE local_submissions SET state=?1,state_revision=?2,decision_code=?3,document=CASE WHEN ?4 THEN NULL ELSE document END,changed_at_utc_ms=?5 WHERE id=?6 AND state_revision=?7",params![state.as_str(),revision as i64,state.decision(),erase,now.map(UtcMillis::get),expected.id.as_str(),expected.revision as i64]).db()?;
        if updated != 1 {
            return Err(invalid("Local submission changed; refresh before deciding"));
        }
        tx.commit().db()?;
        record.state = state;
        record.guard.revision = revision;
        record.decision_code = state.decision().to_owned();
        record.changed_at_utc_ms = now;
        if erase {
            record.document = None;
        }
        Ok(record)
    }
}
/// Called only after canonical schema/content validation. No unchecked row is copied.
pub fn validate_local_submissions(store: &SqliteStore) -> Result<()> {
    let mut after = None;
    let mut seen = 0;
    loop {
        let page = store.local_submission_page(after.as_ref(), MAX_PAGE)?;
        seen += page.rows.len();
        if seen > MAX_SUBMISSIONS {
            return Err(storage("Stored local queue exceeds capacity"));
        }
        match page.next {
            Some(next) => after = Some(next),
            None => {
                if seen != page.total as usize {
                    return Err(storage("Stored local queue identity is inconsistent"));
                }
                return Ok(());
            }
        }
    }
}
