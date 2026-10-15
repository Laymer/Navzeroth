//! Version-aware immutable content. V1 documents and hashes retain their bytes.
use crate::{
    SqliteStore,
    content::{self, ContentRevision, ContentRevisionKey, ContentScope, StageOutcome},
    error::{SqliteResult, invalid, storage},
};
use azeroth_core::{
    content_v2::{self, MAX_CONTENT_BYTES, ReferenceContent},
    contract::ReferenceDataset,
    coverage::content_report,
    error::{Result, Validate},
    sources::{ConflictKind, RecordKey, SourceConflict, SourceRecord, SourceRegistry},
    types::{DataContext, DatasetRevision},
};
use rusqlite::{Connection, TransactionBehavior, params};
use serde::{Deserialize, Serialize, Serializer};
use std::{collections::BTreeSet, sync::Arc};

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ContentRevisionV2 {
    pub content: ReferenceContent,
    pub sources: Vec<SourceRecord>,
    pub conflicts: Vec<SourceConflict>,
}
#[derive(Clone, Debug)]
pub enum ActiveContentRevision {
    V1(Arc<ContentRevision>),
    V2(Arc<ContentRevisionV2>),
}
impl Serialize for ActiveContentRevision {
    fn serialize<S: Serializer>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error> {
        match self {
            Self::V1(r) => r.as_ref().serialize(serializer),
            Self::V2(r) => r.as_ref().serialize(serializer),
        }
    }
}
impl ActiveContentRevision {
    pub fn reference(&self) -> &ReferenceDataset {
        match self {
            Self::V1(r) => &r.envelope.payload,
            Self::V2(r) => &r.content.reference.payload,
        }
    }
    pub fn context(&self) -> DataContext {
        match self {
            Self::V1(r) => r.envelope.data_context(),
            Self::V2(r) => r.content.reference.data_context(),
        }
    }
    pub fn fixture(&self) -> bool {
        match self {
            Self::V1(r) => r.envelope.fixture,
            Self::V2(r) => r.content.reference.fixture,
        }
    }
    pub fn revision(&self) -> &DatasetRevision {
        &self.reference().revision
    }
    pub fn sources(&self) -> &[SourceRecord] {
        match self {
            Self::V1(r) => &r.sources,
            Self::V2(r) => &r.sources,
        }
    }
    pub fn conflicts(&self) -> &[SourceConflict] {
        match self {
            Self::V1(r) => &r.conflicts,
            Self::V2(r) => &r.conflicts,
        }
    }
    pub fn key(&self) -> ContentRevisionKey {
        ContentRevisionKey {
            scope: ContentScope {
                context: self.context(),
                fixture: self.fixture(),
            },
            revision: self.revision().clone(),
        }
    }
    pub fn content_v2(&self) -> Option<&ReferenceContent> {
        match self {
            Self::V1(_) => None,
            Self::V2(r) => Some(&r.content),
        }
    }
    pub fn report(&self) -> azeroth_core::coverage::ContentReport {
        match self {
            Self::V1(r) => content_report(&r.envelope),
            Self::V2(r) => content_report(&r.content.reference),
        }
    }
    pub(crate) fn into_v1(self) -> Result<ContentRevision> {
        match self {
            Self::V1(r) => Ok((*r).clone()),
            Self::V2(_) => Err(invalid("Active content requires the v2 catalog API")),
        }
    }
}
impl ContentRevisionV2 {
    pub fn key(&self) -> ContentRevisionKey {
        ContentRevisionKey {
            scope: ContentScope {
                context: self.content.reference.data_context(),
                fixture: self.content.reference.fixture,
            },
            revision: self.content.reference.payload.revision.clone(),
        }
    }
}
fn encode<T: Serialize>(value: &T) -> Result<String> {
    struct Writer(Vec<u8>);
    impl std::io::Write for Writer {
        fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
            if bytes.len() > MAX_CONTENT_BYTES.saturating_sub(self.0.len()) {
                return Err(std::io::Error::other("catalog exceeds byte capacity"));
            }
            self.0.extend_from_slice(bytes);
            Ok(bytes.len())
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }
    let mut writer = Writer(Vec::new());
    serde_json::to_writer(&mut writer, value)
        .map_err(|_| invalid("Catalog exceeds byte capacity or cannot serialize"))?;
    String::from_utf8(writer.0).map_err(|_| storage("Catalog serialization failed"))
}
pub(crate) fn canonical(input: &ContentRevisionV2) -> Result<ContentRevisionV2> {
    if input.sources.len() > 1024 || input.conflicts.len() > 250_000 {
        return Err(invalid("Catalog provenance capacity exceeded"));
    }
    let encoded = encode(input)?;
    drop(encoded);
    input.content.validate()?;
    let content = content_v2::decode(encode(&input.content)?.as_bytes())?;
    catalog_content_sha256(&content)?;
    let registry = SourceRegistry::try_from(input.sources.clone())?;
    let data = &content.reference.payload;
    for evidence in &data.evidence {
        let source = registry.ensure_registered(&evidence.source)?;
        if source
            .editions
            .value()
            .is_some_and(|editions| !editions.contains(&evidence.context.build.edition))
        {
            return Err(invalid("Catalog source edition differs"));
        }
    }
    for image in &data.map_images {
        let source = registry.ensure_registered(&image.source)?;
        if source
            .editions
            .value()
            .is_some_and(|editions| !editions.contains(&image.map.context.build.edition))
        {
            return Err(invalid("Catalog image source edition differs"));
        }
    }
    let evidence: BTreeSet<_> = data.evidence.iter().map(|e| &e.id).collect();
    let items: BTreeSet<_> = data.items.iter().map(|i| &i.key).collect();
    let quests: BTreeSet<_> = data.quests.iter().map(|q| &q.id).collect();
    let context = content.reference.data_context();
    let mut conflicts = input.conflicts.clone();
    for conflict in &mut conflicts {
        if (conflict.others.is_empty() && conflict.kind != ConflictKind::CrossEditionRelabel)
            || conflict.key.context() != &context
            || conflict.others.len() > 1024
            || !evidence.contains(&conflict.preferred)
            || conflict
                .others
                .iter()
                .any(|id| !evidence.contains(id) || id == &conflict.preferred)
        {
            return Err(invalid("Catalog conflict references invalid content"));
        }
        let found = match &conflict.key {
            RecordKey::Item(key) => items.contains(key),
            RecordKey::Quest { quest, .. } => quests.contains(quest),
        };
        if !found {
            return Err(invalid("Catalog conflict identity is absent"));
        }
        conflict.others.sort();
        if conflict.others.windows(2).any(|p| p[0] == p[1]) {
            return Err(invalid("Catalog conflict repeats evidence"));
        }
    }
    conflicts.sort();
    if conflicts.windows(2).any(|p| p[0] == p[1]) {
        return Err(invalid("Catalog repeats conflict"));
    }
    Ok(ContentRevisionV2 {
        content,
        sources: Vec::from(registry),
        conflicts,
    })
}
pub(crate) fn load_checked(
    conn: &Connection,
    id: i64,
) -> std::result::Result<ContentRevisionV2, content::LoadFailure> {
    let (document, hash, coverage): (String,String,String) = conn.query_row("SELECT r.document,m.content_sha256,m.coverage_document FROM dataset_revisions r JOIN dataset_manifests m ON m.revision_id=r.id WHERE r.id=?1 AND r.content_version=2 AND length(CAST(r.document AS BLOB))<=?2 AND length(CAST(m.coverage_document AS BLOB))<=?2 AND m.manifest_version=1",params![id,MAX_CONTENT_BYTES as i64],|row| Ok((row.get(0)?,row.get(1)?,row.get(2)?))).map_err(content::load_sql_error)?;
    let input: ContentRevisionV2 =
        serde_json::from_str(&document).map_err(|_| storage("Stored v2 catalog is invalid"))?;
    let revision = canonical(&input)?;
    if content::hash(&revision)?.as_str() != hash
        || encode(&content_report(&revision.content.reference))? != coverage
    {
        return Err(storage("Stored v2 catalog hash or coverage differs").into());
    }
    for (sql, expected) in [
        (
            "SELECT document FROM source_records WHERE revision_id=?1 ORDER BY source_id LIMIT 1025",
            revision
                .sources
                .iter()
                .map(encode)
                .collect::<Result<Vec<_>>>()?,
        ),
        (
            "SELECT document FROM source_conflicts WHERE revision_id=?1 ORDER BY conflict_index LIMIT 250001",
            revision
                .conflicts
                .iter()
                .map(encode)
                .collect::<Result<Vec<_>>>()?,
        ),
    ] {
        let bounded_sql = sql.replace(
            "SELECT document",
            "SELECT CASE WHEN length(CAST(document AS BLOB))<=16777216 THEN document END",
        );
        let mut statement = conn
            .prepare(&bounded_sql)
            .map_err(content::load_sql_error)?;
        let mut rows = statement.query([id]).map_err(content::load_sql_error)?;
        let mut index = 0;
        while let Some(row) = rows.next().map_err(content::load_sql_error)? {
            let actual: String = row.get(0).map_err(content::load_sql_error)?;
            if expected.get(index) != Some(&actual) {
                return Err(storage("Stored v2 catalog provenance differs").into());
            }
            index += 1;
        }
        if index != expected.len() {
            return Err(storage("Stored v2 catalog provenance count differs").into());
        }
    }
    Ok(revision)
}
impl SqliteStore {
    pub fn stage_content_v2_revision(&mut self, input: &ContentRevisionV2) -> Result<StageOutcome> {
        let revision = canonical(input)?;
        let tx = self
            .conn
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .db()?;
        let outcome = stage_canonical(&tx, &revision)?;
        tx.commit().db()?;
        Ok(outcome)
    }
    pub fn active_content_v2_revision(
        &self,
        scope: &ContentScope,
    ) -> Result<Option<Arc<ContentRevisionV2>>> {
        Ok(match self.active_catalog_revision(scope)? {
            Some(ActiveContentRevision::V2(r)) => Some(r),
            _ => None,
        })
    }
    pub(crate) fn cached_catalog(&self, id: i64) -> Result<ActiveContentRevision> {
        let version: i64 = self
            .conn
            .pragma_query_value(None, "data_version", |r| r.get(0))
            .db()?;
        let changes = self.conn.total_changes();
        if let Some((cached_id, cached_version, cached_changes, catalog)) =
            self.catalog_cache.borrow().as_ref()
            && *cached_id == id
            && *cached_version == version
            && *cached_changes == changes
        {
            return Ok(catalog.clone());
        }
        let catalog = content::load(&self.conn, id)?;
        *self.catalog_cache.borrow_mut() = Some((id, version, changes, catalog.clone()));
        Ok(catalog)
    }
}

/// Exact content-pack canonical bytes: sorted JSON object keys, pretty, newline.
/// The v1 revision hash function and historical manifest bytes are unchanged.
pub fn catalog_content_sha256(
    content: &ReferenceContent,
) -> Result<azeroth_core::types::Sha256Hex> {
    use sha2::{Digest, Sha256};
    let value = serde_json::to_value(content).map_err(|_| storage("Catalog cannot serialize"))?;
    let mut bytes =
        serde_json::to_vec_pretty(&value).map_err(|_| storage("Catalog cannot serialize"))?;
    bytes.push(b'\n');
    if bytes.len() > MAX_CONTENT_BYTES {
        return Err(invalid("Canonical catalog exceeds byte capacity"));
    }
    azeroth_core::types::Sha256Hex::new(format!("{:x}", Sha256::digest(bytes)))
}
impl SqliteStore {
    pub fn catalog_first_page(
        &self,
        scope: &ContentScope,
        collection: azeroth_core::catalog::Collection,
        limit: usize,
    ) -> Result<azeroth_core::catalog::CatalogPage> {
        self.with_catalog_index(scope, |index| index.first_page(collection, limit))
    }
    pub fn catalog_page(
        &self,
        scope: &ContentScope,
        request: &azeroth_core::catalog::PageRequest,
    ) -> Result<azeroth_core::catalog::CatalogPage> {
        self.with_catalog_index(scope, |index| index.page(request))
    }
    pub(crate) fn with_catalog_index<T>(
        &self,
        scope: &ContentScope,
        read: impl Fn(&azeroth_core::catalog::CatalogIndex) -> Result<T>,
    ) -> Result<T> {
        let get = |store: &Self| {
            let active = store
                .active_catalog_revision(scope)?
                .ok_or_else(|| invalid("No active catalog"))?;
            let content = active
                .content_v2()
                .ok_or_else(|| invalid("Catalog pages require v2 content"))?;
            let id = content::find(&store.conn, &active.key())?
                .ok_or_else(|| storage("Active catalog identity missing"))?;
            let version: i64 = store
                .conn
                .pragma_query_value(None, "data_version", |r| r.get(0))
                .db()?;
            let changes = store.conn.total_changes();
            if let Some((cached_id, cached_version, cached_changes, index)) =
                store.catalog_index_cache.borrow().as_ref()
                && *cached_id == id
                && *cached_version == version
                && *cached_changes == changes
            {
                return read(index);
            }
            let index = Arc::new(azeroth_core::catalog::CatalogIndex::new(
                content,
                catalog_content_sha256(content)?,
            )?);
            let page = read(&index)?;
            *store.catalog_index_cache.borrow_mut() = Some((id, version, changes, index));
            Ok(page)
        };
        if self.conn.is_autocommit() {
            self.with_immediate_read(get)
        } else {
            get(self)
        }
    }
}

pub(crate) type Cache<T> = std::cell::RefCell<Option<(i64, i64, u64, T)>>;

pub(crate) fn stage_canonical(
    conn: &Connection,
    revision: &ContentRevisionV2,
) -> Result<StageOutcome> {
    let key = revision.key();
    let hash = content::hash(&revision)?;
    let document = encode(&revision)?;
    let report = encode(&content_report(&revision.content.reference))?;
    if let Some(id) = content::find(conn, &key)? {
        let old = content::load(conn, id)?;
        if !matches!(old, ActiveContentRevision::V2(_)) || content::hash(&old)? != hash {
            return Err(invalid("Immutable catalog revision already differs"));
        }
        return Ok(StageOutcome::Duplicate);
    }
    let count: i64 = conn
        .query_row("SELECT count(*) FROM dataset_revisions", [], |r| r.get(0))
        .db()?;
    if count >= content::MAX_CONTENT_REVISIONS as i64 {
        return Err(invalid("Catalog revision capacity exceeded"));
    }
    conn.execute("INSERT INTO dataset_revisions(edition,build,interface,namespace,is_fixture,revision,document,content_version) VALUES(?1,?2,?3,?4,?5,?6,?7,2)",params![key.scope.context.build.edition.as_str(),key.scope.context.build.build.as_str(),key.scope.context.build.interface.as_str(),content::namespace(&key.scope),key.scope.fixture,key.revision.as_str(),document]).db()?;
    let id = conn.last_insert_rowid();
    conn.execute("INSERT INTO content_dispositions(revision_id,state,state_revision,reason_code) VALUES(?1,'eligible',1,'initial_eligibility')",[id]).db()?;
    for source in &revision.sources {
        conn.execute(
            "INSERT INTO source_records(revision_id,source_id,document) VALUES(?1,?2,?3)",
            params![id, source.id.as_str(), encode(source)?],
        )
        .db()?;
    }
    for (index, conflict) in revision.conflicts.iter().enumerate() {
        conn.execute(
            "INSERT INTO source_conflicts(revision_id,conflict_index,document) VALUES(?1,?2,?3)",
            params![id, index as i64, encode(conflict)?],
        )
        .db()?;
    }
    conn.execute("INSERT INTO dataset_manifests(revision_id,manifest_version,content_sha256,coverage_document) VALUES(?1,1,?2,?3)",params![id,hash.as_str(),report]).db()?;
    Ok(StageOutcome::Inserted)
}
