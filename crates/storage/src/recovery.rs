//! Read-only untrusted SQLite input; SQL comes exclusively from our migrations.
use crate::{
    SnapshotStore, SqliteStore,
    error::{SqliteResult, storage},
    migrations,
};
use azeroth_core::{contract::decode_envelope, entities::CharacterSnapshot, error::Result};
use rusqlite::{Connection, OpenFlags, params_from_iter, types::Value};
use serde::Serialize;
use std::{collections::BTreeMap, path::Path};

pub const MAX_BACKUP_BYTES: u64 = 4 * 1024 * 1024 * 1024;
const MAX_ROWS: usize = 1_000_000;
const MAX_CELL: usize = 32 * 1024 * 1024;
fn row_limit(table: &str) -> usize {
    match table {
        "dataset_revisions" | "dataset_manifests" => {
            azeroth_core::content_v2::MAX_CONTENT_BYTES + 16 * 1024
        }
        "catalog_asset_blobs" => crate::catalog_assets::MAX_ASSET_BYTES + 16 * 1024,
        _ => MAX_CELL,
    }
}
const TABLES: &[&str] = &[
    "profiles",
    "snapshots",
    "snapshot_observations",
    "profile_preferences",
    "imports",
    "dataset_revisions",
    "source_records",
    "source_conflicts",
    "dataset_manifests",
    "catalog_asset_blobs",
    "catalog_asset_layers",
    "active_datasets",
    "party_profiles",
    "party_participants",
    "snapshot_promotions",
    "sessions",
    "reminder_state",
    "session_calibration_annotations",
    "party_plan_proposals",
    "content_dispositions",
    "app_settings",
    "current_plan",
    "local_submissions",
];
#[derive(Clone, Debug, Serialize)]
pub struct RecoveryInspection {
    pub source_schema_version: u32,
    pub restored_schema_version: u32,
    pub rows: BTreeMap<String, usize>,
}
type Schema = Vec<(String, String, String, Option<String>)>;
fn schema(conn: &Connection) -> Result<Schema> {
    let mut q=conn.prepare("SELECT type,name,tbl_name,CASE WHEN length(CAST(sql AS BLOB))<=65536 THEN sql ELSE NULL END FROM sqlite_schema WHERE name NOT GLOB 'sqlite_*' ORDER BY type,name LIMIT 129").db()?;
    let rows = q
        .query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)))
        .db()?
        .collect::<rusqlite::Result<Schema>>()
        .db()?;
    if rows.len() > 128
        || rows
            .iter()
            .any(|(kind, _, _, s)| kind != "index" && s.is_none())
        || rows.iter().any(|(_, n, t, s)| {
            n.len() > 128 || t.len() > 128 || s.as_ref().is_some_and(|s| s.len() > 65536)
        })
    {
        return Err(storage("Backup schema exceeds bounds"));
    }
    Ok(rows)
}
fn canonical(version: usize) -> Result<Connection> {
    let mut conn = Connection::open_in_memory().db()?;
    migrations::migrate_with(&mut conn, &migrations::MIGRATIONS[..version])?;
    Ok(conn)
}
fn open_source(path: &Path) -> Result<Connection> {
    let info = std::fs::symlink_metadata(path).map_err(|_| storage("Backup cannot be read"))?;
    if !info.file_type().is_file() || info.len() > MAX_BACKUP_BYTES {
        return Err(storage("Backup must be a regular file within 512 MiB"));
    }
    let mut total = info.len();
    for suffix in ["-wal", "-shm"] {
        let sidecar = std::path::PathBuf::from(format!("{}{suffix}", path.to_string_lossy()));
        match std::fs::symlink_metadata(sidecar) {
            Ok(meta) => {
                if !meta.file_type().is_file() {
                    return Err(storage("Backup sidecar must be a regular file"));
                }
                total = total.saturating_add(meta.len());
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(_) => return Err(storage("Backup sidecar cannot be read")),
        }
    }
    if total > MAX_BACKUP_BYTES {
        return Err(storage("Backup and sidecars exceed 512 MiB"));
    }
    // Reject invalid file headers before SQLite can initialize a shared-memory
    // sidecar for a broken source. WAL locks/SHM are SQLite-managed for valid DBs.
    use std::io::Read;
    let mut header = [0u8; 16];
    std::fs::File::open(path)
        .and_then(|mut file| file.read_exact(&mut header))
        .map_err(|_| storage("Backup is not a readable SQLite database"))?;
    if &header != b"SQLite format 3\0" {
        return Err(storage("Backup is not a SQLite database"));
    }
    // READ_ONLY, no URI interpretation, extension loading is not enabled.
    let conn = Connection::open_with_flags(
        path,
        OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX,
    )
    .db()?;
    conn.pragma_update(None, "trusted_schema", false).db()?;
    conn.pragma_update(None, "query_only", true).db()?;
    Ok(conn)
}
/// Build a new trusted database, preserving supported historical identities and
/// derived state. Destination must not exist. No imported schema is executed.
pub fn reconstruct_backup(source: &Path, destination: &Path) -> Result<RecoveryInspection> {
    let input = open_source(source)?;
    // Hold one read transaction through schema validation and the entire copy.
    input.execute_batch("BEGIN").db()?;
    let actual = schema(&input)?;
    let mut version = None;
    for n in 1..=migrations::MIGRATIONS.len() {
        if schema(&canonical(n)?)? == actual {
            version = Some(n);
            break;
        }
    }
    let version = version.ok_or_else(|| storage("Backup schema is unsupported or modified"))?;
    // A newly registered migration must also register its recovery table copier;
    // refusing is safer than accepting its schema and silently dropping state.
    if actual.iter().any(|(kind, name, _, _)| {
        kind == "table" && name != "schema_migrations" && !TABLES.contains(&name.as_str())
    }) {
        return Err(storage(
            "Recovery copier does not support every schema table",
        ));
    }
    let mut history = input
        .prepare("SELECT version,name FROM schema_migrations ORDER BY version LIMIT ?1")
        .db()?;
    let history = history
        .query_map([i64::from(migrations::latest_version()) + 1], |r| {
            Ok((r.get::<_, u32>(0)?, r.get::<_, String>(1)?))
        })
        .db()?
        .collect::<rusqlite::Result<Vec<_>>>()
        .db()?;
    if history
        != migrations::MIGRATIONS[..version]
            .iter()
            .map(|m| (m.version, m.name.to_owned()))
            .collect::<Vec<_>>()
    {
        return Err(storage("Backup migration history is inconsistent"));
    }
    let check: String = input
        .query_row("PRAGMA integrity_check(1)", [], |r| r.get(0))
        .db()?;
    if check != "ok" {
        return Err(storage("Backup integrity check failed"));
    }
    let file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(destination)
        .map_err(|_| storage("Recovery destination must be new"))?;
    drop(file);
    let result = (|| {
        let mut output = Connection::open(destination).db()?;
        migrations::migrate_with(&mut output, &migrations::MIGRATIONS[..version])?;
        output.pragma_update(None, "foreign_keys", false).db()?;
        let tx = output.transaction().db()?;
        let trusted = canonical(version)?;
        let mut counts = BTreeMap::new();
        let mut total = 0;
        for &table in TABLES {
            if !actual
                .iter()
                .any(|(kind, name, _, _)| kind == "table" && name == table)
            {
                continue;
            }
            // Migration9 seeds its empty CAS tombstone. Replace it with the exact
            // copied source row, rather than colliding or silently keeping revision1.
            if table == "current_plan" {
                tx.execute("DELETE FROM current_plan", []).db()?;
            }
            // Names and columns below originate solely from our canonical schema.
            let mut columns = trusted
                .prepare(&format!("PRAGMA table_info({table})"))
                .db()?;
            let names = columns
                .query_map([], |r| r.get::<_, String>(1))
                .db()?
                .collect::<rusqlite::Result<Vec<_>>>()
                .db()?;
            let size = names
                .iter()
                .map(|n| format!("coalesce(length(CAST({n} AS BLOB)),0)"))
                .collect::<Vec<_>>()
                .join("+");
            let oversized: bool = input
                .query_row(
                    &format!("SELECT EXISTS(SELECT 1 FROM {table} WHERE {size}>?1)"),
                    [row_limit(table) as i64],
                    |r| r.get(0),
                )
                .db()?;
            if oversized {
                return Err(storage("Backup row exceeds bounded document size"));
            }
            let mut select = input
                .prepare(&format!(
                    "SELECT {} FROM {table} LIMIT {}",
                    names.join(","),
                    MAX_ROWS + 1
                ))
                .db()?;
            let mut rows = select.query([]).db()?;
            let insert = format!(
                "INSERT INTO {table}({}) VALUES({})",
                names.join(","),
                vec!["?"; names.len()].join(",")
            );
            let mut write = tx.prepare(&insert).db()?;
            let mut count = 0;
            while let Some(row) = rows.next().db()? {
                count += 1;
                total += 1;
                if total > MAX_ROWS {
                    return Err(storage("Backup row count exceeds capacity"));
                }
                let values = (0..names.len())
                    .map(|i| row.get::<_, Value>(i))
                    .collect::<rusqlite::Result<Vec<_>>>()
                    .db()?;
                write.execute(params_from_iter(values)).db()?;
            }
            counts.insert(table.to_owned(), count);
        }
        tx.commit().db()?;
        migrations::migrate(&mut output)?;
        output.pragma_update(None, "foreign_keys", true).db()?;
        let store = SqliteStore {
            conn: output,
            catalog_cache: Default::default(),
            catalog_index_cache: Default::default(),
        };
        validate(&store)?;
        // A backup can preserve references, but cannot authenticate a successful
        // locally guarded publication. Never promote imported advice to Current.
        let guard: Option<String> = store
            .conn
            .query_row(
                "SELECT guard_json FROM current_plan WHERE slot=1",
                [],
                |row| row.get(0),
            )
            .db()?;
        if let Some(guard) = guard {
            let recovered = crate::today::mark_recovered_plan_guard(&guard)?;
            store
                .conn
                .execute(
                    "UPDATE current_plan SET guard_json=?1 WHERE slot=1",
                    [recovered],
                )
                .db()?;
        }
        validate(&store)?;
        Ok(RecoveryInspection {
            source_schema_version: version as u32,
            restored_schema_version: migrations::latest_version(),
            rows: counts,
        })
    })();
    if result.is_err() {
        let _ = std::fs::remove_file(destination);
    }
    result
}
fn ids(conn: &Connection, sql: &str) -> Result<Vec<i64>> {
    let mut q = conn.prepare(sql).db()?;
    q.query_map([], |r| r.get(0))
        .db()?
        .collect::<rusqlite::Result<Vec<_>>>()
        .db()
}
fn reject(conn: &Connection, sql: &str) -> Result<()> {
    if conn.query_row(sql, [], |r| r.get::<_, bool>(0)).db()? {
        Err(storage(
            "Backup identity, bounds or pointer is inconsistent",
        ))
    } else {
        Ok(())
    }
}
/// Integrity plus semantic validation, including all historical rows, not only
/// the current selections displayed by the application.
pub fn validate(store: &SqliteStore) -> Result<()> {
    let conn = &store.conn;
    let trusted = canonical(migrations::MIGRATIONS.len())?;
    if schema(conn)? != schema(&trusted)? {
        return Err(storage("Database schema differs from trusted schema"));
    }
    let mut total = 0usize;
    for &table in TABLES {
        let mut columns = trusted
            .prepare(&format!("PRAGMA table_info({table})"))
            .db()?;
        let names = columns
            .query_map([], |r| r.get::<_, String>(1))
            .db()?
            .collect::<rusqlite::Result<Vec<_>>>()
            .db()?;
        let size = names
            .iter()
            .map(|name| format!("coalesce(length(CAST({name} AS BLOB)),0)"))
            .collect::<Vec<_>>()
            .join("+");
        let oversized: bool = conn
            .query_row(
                &format!("SELECT EXISTS(SELECT 1 FROM {table} WHERE {size}>?1)"),
                [row_limit(table) as i64],
                |r| r.get(0),
            )
            .db()?;
        if oversized {
            return Err(storage("Database row exceeds validation capacity"));
        }
        let count: u32 = conn
            .query_row(
                &format!(
                    "SELECT COUNT(*) FROM (SELECT 1 FROM {table} LIMIT {})",
                    MAX_ROWS + 1
                ),
                [],
                |r| r.get(0),
            )
            .db()?;
        total = total.saturating_add(count as usize);
        if total > MAX_ROWS {
            return Err(storage("Database row count exceeds validation capacity"));
        }
    }

    let integrity: String = conn
        .query_row("PRAGMA integrity_check(1)", [], |r| r.get(0))
        .db()?;
    if integrity != "ok" {
        return Err(storage("Database integrity check failed"));
    }
    let mut fk = conn.prepare("PRAGMA foreign_key_check").db()?;
    if fk.query([]).db()?.next().db()?.is_some() {
        return Err(storage("Database foreign-key check failed"));
    }
    validate_local_foundation(store)?;
    for sql in [
        "SELECT (SELECT COUNT(*) FROM profiles)>1024 OR (SELECT COUNT(*) FROM imports)>1024 OR (SELECT COUNT(*) FROM party_profiles)>128 OR (SELECT COUNT(*) FROM dataset_revisions)>1024 OR (SELECT COUNT(*) FROM snapshot_promotions)>256",
        "SELECT EXISTS(SELECT 1 FROM snapshots GROUP BY profile_id HAVING count(*)>257)",
        "SELECT EXISTS(SELECT 1 FROM snapshot_observations GROUP BY snapshot_id HAVING count(*)>65536)",
        "SELECT EXISTS(SELECT 1 FROM profile_preferences GROUP BY profile_id HAVING count(*)>1024)",
        "SELECT EXISTS(SELECT 1 FROM sessions GROUP BY profile_id HAVING count(*)>128)",
        "SELECT EXISTS(SELECT 1 FROM profiles p JOIN snapshots s ON s.id=p.current_snapshot_id WHERE s.profile_id!=p.id)",
        "SELECT EXISTS(SELECT 1 FROM party_participants GROUP BY party_id HAVING min(position)!=0 OR max(position)+1!=count(*))",
        "SELECT EXISTS(SELECT 1 FROM source_conflicts GROUP BY revision_id HAVING min(conflict_index)!=0 OR max(conflict_index)+1!=count(*))",
        "SELECT EXISTS(SELECT 1 FROM active_datasets a JOIN dataset_revisions r ON r.id=a.revision_id WHERE a.edition!=r.edition OR a.build!=r.build OR a.interface!=r.interface OR a.namespace!=r.namespace OR a.is_fixture!=r.is_fixture)",
        "SELECT EXISTS(SELECT 1 FROM active_datasets a JOIN dataset_revisions r ON r.id=a.previous_revision_id WHERE a.edition!=r.edition OR a.build!=r.build OR a.interface!=r.interface OR a.namespace!=r.namespace OR a.is_fixture!=r.is_fixture)",
    ] {
        reject(conn, sql)?;
    }
    let mut profiles = conn
        .prepare(
            "SELECT id,character_id,edition,build,interface,namespace,is_fixture FROM profiles",
        )
        .db()?;
    let keys = profiles
        .query_map([], |r| {
            Ok((r.get::<_, i64>(0)?, crate::sqlite::recovery_profile_key(r)?))
        })
        .db()?
        .collect::<rusqlite::Result<BTreeMap<_, _>>>()
        .db()?;
    let mut preferences = conn
        .prepare("SELECT preference_id,status,value,document FROM profile_preferences")
        .db()?;
    let mut entries = preferences.query([]).db()?;
    while let Some(row) = entries.next().db()? {
        azeroth_core::types::PreferenceId::new(row.get::<_, String>(0).db()?)?;
        let observation: azeroth_core::observation::Observation<bool> =
            serde_json::from_str(&row.get::<_, String>(3).db()?)
                .map_err(|_| storage("Invalid preference document"))?;
        let status = serde_json::to_value(observation.status())
            .map_err(|_| storage("Invalid preference status"))?;
        if row.get::<_, String>(1).db()? != status.as_str().unwrap_or("")
            || row.get::<_, Option<bool>>(2).db()? != observation.value().copied()
        {
            return Err(storage("Backup preference columns disagree"));
        }
    }
    let mut sessions = conn
        .prepare("SELECT document,content_sha256 FROM sessions")
        .db()?;
    for row in sessions
        .query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)))
        .db()?
    {
        use sha2::{Digest, Sha256};
        let (text, hash) = row.db()?;
        if format!("{:x}", Sha256::digest(text.as_bytes())) != hash {
            return Err(storage("Backup session hash disagrees"));
        }
    }
    let mut members = conn
        .prepare("SELECT profile_id,profile_key FROM party_participants")
        .db()?;
    for row in members
        .query_map([], |r| {
            Ok((r.get::<_, Option<i64>>(0)?, r.get::<_, String>(1)?))
        })
        .db()?
    {
        let (profile, text) = row.db()?;
        let key: crate::ProfileKey =
            serde_json::from_str(&text).map_err(|_| storage("Invalid party profile identity"))?;
        if profile.is_some_and(|id| keys.get(&id) != Some(&key)) {
            return Err(storage("Party profile pointer disagrees"));
        }
    }
    for key in keys.values() {
        store.preferences(key)?;
        store.reminder_state(key)?;
        for id in store.session_ids(key)? {
            store.session(key, &id)?;
            store.session_calibration(key, &id)?;
        }
    }
    let mut snapshots = conn
        .prepare("SELECT id,profile_id,document,snapshot_id,content_sha256 FROM snapshots")
        .db()?;
    let mut rows = snapshots.query([]).db()?;
    while let Some(row) = rows.next().db()? {
        let id: i64 = row.get(0).db()?;
        let profile: i64 = row.get(1).db()?;
        let text: String = row.get(2).db()?;
        let envelope = decode_envelope::<CharacterSnapshot>(&text)?;
        if keys.get(&profile) != Some(&crate::ProfileKey::of(&envelope))
            || row.get::<_, String>(3).db()? != envelope.payload.id.as_str()
            || row.get::<_, String>(4).db()? != crate::snapshot_content_hash(&envelope)?.as_str()
        {
            return Err(storage("Backup snapshot identity or hash disagrees"));
        }
        crate::sqlite::validate_recovery_snapshot(conn, id, &envelope)?;
    }
    let orphan_assets: bool = conn.query_row("SELECT EXISTS(SELECT 1 FROM catalog_asset_blobs b WHERE NOT EXISTS(SELECT 1 FROM catalog_asset_layers l WHERE l.sha256=b.sha256))", [], |r| r.get(0)).db()?;
    if orphan_assets {
        return Err(storage("Backup contains unreferenced catalog asset bytes"));
    }
    for id in ids(conn, "SELECT id FROM dataset_revisions")? {
        let revision = crate::content::load(conn, id)?;
        crate::catalog_assets::validate_revision_assets(conn, id, &revision)?;
        let context = revision.context();
        let columns:(String,String,String,String,bool,String)=conn.query_row("SELECT edition,build,interface,namespace,is_fixture,revision FROM dataset_revisions WHERE id=?1",[id],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?,r.get(4)?,r.get(5)?))).db()?;
        let namespace =
            serde_json::to_value(context.namespace).map_err(|_| storage("Invalid namespace"))?;
        if columns
            != (
                context.build.edition.as_str().to_owned(),
                context.build.build.as_str().to_owned(),
                context.build.interface.as_str().to_owned(),
                namespace.as_str().unwrap_or("").to_owned(),
                revision.fixture(),
                revision.reference().revision.as_str().to_owned(),
            )
        {
            return Err(storage("Backup content identity disagrees"));
        }
    }
    let mut records = conn
        .prepare("SELECT source_id,document FROM source_records")
        .db()?;
    for row in records
        .query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)))
        .db()?
    {
        let (id, text) = row.db()?;
        let value: serde_json::Value =
            serde_json::from_str(&text).map_err(|_| storage("Invalid source record"))?;
        if value.get("id").and_then(|v| v.as_str()) != Some(id.as_str()) {
            return Err(storage("Source record identity disagrees"));
        }
    }
    let mut promotions=conn.prepare("SELECT previous_snapshot_id,selected_snapshot_id,reason,operated_at_utc_ms FROM snapshot_promotions").db()?;
    for row in promotions
        .query_map([], |r| {
            Ok((
                r.get::<_, String>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, String>(2)?,
                r.get::<_, i64>(3)?,
            ))
        })
        .db()?
    {
        let (previous, selected, reason, now) = row.db()?;
        azeroth_core::types::SnapshotId::new(previous)?;
        azeroth_core::types::SnapshotId::new(selected)?;
        azeroth_core::types::UtcMillis::new(now)?;
        if reason.trim().is_empty() || reason.len() > 1024 {
            return Err(storage("Promotion audit exceeds bounds"));
        }
    }
    let mut audits = conn
        .prepare("SELECT error_code,message FROM imports")
        .db()?;
    for row in audits
        .query_map([], |r| {
            Ok((
                r.get::<_, Option<String>>(0)?,
                r.get::<_, Option<String>>(1)?,
            ))
        })
        .db()?
    {
        let (code, message) = row.db()?;
        if let Some(code) = code {
            let _: azeroth_core::error::ErrorCode =
                serde_json::from_value(serde_json::Value::String(code))
                    .map_err(|_| storage("Import audit error code is invalid"))?;
        }
        if message.is_some_and(|text| text.len() > 1024) {
            return Err(storage("Import audit message exceeds bounds"));
        }
    }
    let mut receipts=conn.prepare("SELECT received_at_utc_ms FROM snapshots WHERE received_at_utc_ms IS NOT NULL UNION ALL SELECT received_at_utc_ms FROM imports WHERE received_at_utc_ms IS NOT NULL").db()?;
    for row in receipts.query_map([], |r| r.get::<_, i64>(0)).db()? {
        azeroth_core::types::UtcMillis::new(row.db()?)?;
    }
    for party in store.party_ids()? {
        store.party(&party)?;
        store.party_proposal(&party)?;
    }
    // Proposals' internal request is application-owned; storage validates the
    // bounded typed proposal and app validates guarded joint requests on read.
    Ok(())
}

// Validate every populated feature document using its owning typed reader.
fn validate_local_foundation(store: &SqliteStore) -> Result<()> {
    crate::content_dispositions::validate_content_dispositions(&store.conn)?;
    reject(&store.conn, "SELECT (SELECT count(*) FROM current_plan)!=1")?;
    store.settings()?;
    store.current_plan()?;
    crate::local_submissions::validate_local_submissions(store)?;
    Ok(())
}

#[cfg(test)]
mod catalog_capacity_tests {
    #[test]
    fn catalog_rows_have_v2_capacity_without_loosening_snapshot_rows() {
        assert_eq!(
            super::row_limit("dataset_revisions"),
            256 * 1024 * 1024 + 16 * 1024
        );
        assert_eq!(
            super::row_limit("dataset_manifests"),
            256 * 1024 * 1024 + 16 * 1024
        );
        assert_eq!(super::row_limit("snapshots"), 32 * 1024 * 1024);
    }
}
