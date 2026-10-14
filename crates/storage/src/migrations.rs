//! Versioned schema migrations. Each migration runs in one immediate
//! transaction together with its `schema_migrations` row, so a failure leaves
//! the previous version and its data intact. A database that records a version
//! this build does not know is refused without being modified.
//!
//! Adding version N+1: append one `Migration` to `MIGRATIONS` and add a test
//! that migrates a database populated at version N (see `tests/migrations.rs`).
//! Migrations must not rely on `PRAGMA foreign_keys` changes (a no-op inside a
//! transaction); table rebuilds must preserve rows explicitly.
use crate::error::{SqliteResult, storage};
use azeroth_core::error::Result;
use rusqlite::{Connection, OptionalExtension, TransactionBehavior};

/// One ordered schema step.
#[derive(Clone, Copy, Debug)]
pub struct Migration {
    pub version: u32,
    pub name: &'static str,
    pub sql: &'static str,
}

const CREATE_MIGRATIONS_TABLE: &str = "CREATE TABLE IF NOT EXISTS schema_migrations (
    version INTEGER PRIMARY KEY CHECK (version > 0),
    name TEXT NOT NULL
)";

/// Every migration this build knows, in ascending contiguous order from 1.
pub const MIGRATIONS: &[Migration] = &[
    Migration {
        version: 1,
        name: "character snapshot store",
        sql: include_str!("schema_v1.sql"),
    },
    Migration {
        version: 2,
        name: "immutable reference content revisions",
        sql: include_str!("schema_v2.sql"),
    },
    Migration {
        version: 3,
        name: "manual party profiles",
        sql: include_str!("schema_v3.sql"),
    },
    Migration {
        version: 4,
        name: "audited snapshot promotion",
        sql: include_str!("schema_v4.sql"),
    },
    Migration {
        version: 5,
        name: "immutable session events and summaries",
        sql: include_str!("schema_v5.sql"),
    },
    Migration {
        version: 6,
        name: "bounded reminder state",
        sql: include_str!("schema_v6.sql"),
    },
    Migration {
        version: 7,
        name: "explicit session calibration annotations",
        sql: include_str!("schema_v7.sql"),
    },
    Migration {
        version: 8,
        name: "bounded local party plan proposals",
        sql: include_str!("schema_v8.sql"),
    },
    Migration {
        version: 9,
        name: "bounded local settings plans and content governance",
        sql: include_str!("schema_v9.sql"),
    },
    Migration {
        version: 10,
        name: "registered source identity compatibility",
        sql: include_str!("schema_v10.sql"),
    },
];

/// The newest version in `MIGRATIONS`.
pub fn latest_version() -> u32 {
    MIGRATIONS.last().map_or(0, |m| m.version)
}

/// The highest recorded version, or 0 for a database without migrations.
pub fn current_version(conn: &Connection) -> Result<u32> {
    let table: Option<String> = conn
        .query_row(
            "SELECT name FROM sqlite_master WHERE type = 'table' AND name = 'schema_migrations'",
            [],
            |row| row.get(0),
        )
        .optional()
        .db()?;
    if table.is_none() {
        return Ok(0);
    }
    let (count, max): (i64, Option<i64>) = conn
        .query_row(
            "SELECT COUNT(*), MAX(version) FROM schema_migrations",
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .db()?;
    let max = max.unwrap_or(0);
    // Versions are recorded 1..=max with no gaps.
    if count != max {
        return Err(storage("Database schema history is inconsistent"));
    }
    u32::try_from(max).map_err(|_| storage("Database schema history is inconsistent"))
}

/// Brings `conn` to the newest version of this build.
pub fn migrate(conn: &mut Connection) -> Result<()> {
    migrate_with(conn, MIGRATIONS)
}

/// Applies the pending part of `list`. Exposed for tests, which inject a
/// deliberately failing migration; production code calls `migrate`.
#[doc(hidden)]
pub fn migrate_with(conn: &mut Connection, list: &[Migration]) -> Result<()> {
    let known = list.last().map_or(0, |m| m.version);
    if list
        .iter()
        .zip(1u32..)
        .any(|(migration, expected)| migration.version != expected)
    {
        return Err(storage("Migration list is not contiguous"));
    }
    let current = current_version(conn)?;
    if current > known {
        return Err(storage("Database schema is newer than this application"));
    }
    for migration in list.iter().filter(|m| m.version > current) {
        let tx = conn
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .db()?;
        tx.execute_batch(CREATE_MIGRATIONS_TABLE).db()?;
        // Re-check under the write lock: another process may have migrated.
        let recorded: i64 = tx
            .query_row(
                "SELECT COUNT(*) FROM schema_migrations WHERE version >= ?1",
                [migration.version],
                |row| row.get(0),
            )
            .db()?;
        if recorded != 0 {
            return Err(storage("Database schema changed during migration"));
        }
        tx.execute_batch(migration.sql).db()?;
        tx.execute(
            "INSERT INTO schema_migrations (version, name) VALUES (?1, ?2)",
            rusqlite::params![migration.version, migration.name],
        )
        .db()?;
        tx.commit().db()?;
    }
    Ok(())
}
