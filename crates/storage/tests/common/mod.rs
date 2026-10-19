//! Shared test helpers (local to this crate; core's test helpers are frozen).
//! Documents are derived from the committed synthetic contract fixtures by
//! mutating parsed JSON and decoding the result through `decode_envelope`.
#![allow(dead_code)]
use azeroth_core::{
    contract::{Envelope, decode_envelope},
    entities::CharacterSnapshot,
};
use rusqlite::Connection;
use serde_json::{Value, json};
use std::path::{Path, PathBuf};

const BASE: &str = include_str!("../../../../fixtures/contracts/v1/character-snapshot.json");
const REFERENCE: &str =
    include_str!("../../../../fixtures/contracts/v1/reference-namespace-fixture.json");

pub const BASE_SNAPSHOT_ID: &str = "demo-snapshot-001";
pub const BASE_CHARACTER: &str = "demo-character-paladin";

/// A mutable snapshot document. Every builder step keeps producer and capture
/// session facts in agreement, as the contract requires.
#[derive(Clone)]
pub struct Doc(pub Value);

impl Doc {
    /// The demo-namespace fixture: session `demo-session-001`, sequence 12,
    /// trusted capture clock.
    pub fn base() -> Self {
        Self(serde_json::from_str(BASE).unwrap())
    }
    /// The reference-namespace fixture (still synthetic, `fixture: true`).
    pub fn reference() -> Self {
        Self(serde_json::from_str(REFERENCE).unwrap())
    }
    pub fn snapshot_id(mut self, id: &str) -> Self {
        self.0["payload"]["id"] = json!(id);
        self
    }
    pub fn character(mut self, id: &str) -> Self {
        self.0["payload"]["character"] = json!(id);
        self
    }
    /// Sets the producer and capture session and sequence together.
    pub fn session(mut self, session: &str, sequence: u64) -> Self {
        self.0["producer"]["session_id"] = json!(session);
        self.0["producer"]["sequence"] = json!(sequence);
        self.0["payload"]["capture"]["session_id"] = json!(session);
        self.0["payload"]["capture"]["sequence"] = json!(sequence);
        self
    }
    pub fn elapsed(mut self, ms: u64) -> Self {
        self.0["payload"]["capture"]["session_elapsed_ms"] = json!(ms);
        self
    }
    pub fn observed(mut self, utc_ms: i64, trusted: bool) -> Self {
        self.0["payload"]["capture"]["observed_at_utc_ms"] = json!(utc_ms);
        self.0["payload"]["capture"]["clock_trust"] =
            json!(if trusted { "trusted" } else { "uncertain" });
        self
    }
    pub fn level(mut self, level: u64) -> Self {
        self.0["payload"]["level"]["value"] = json!(level);
        self
    }
    /// Changes only the desktop receipt time of the level observation.
    pub fn received_at(mut self, utc_ms: i64) -> Self {
        self.0["payload"]["level"]["received_at_utc_ms"] = json!(utc_ms);
        self
    }
    /// Rewrites the build and interface everywhere they appear together.
    pub fn build(mut self, build: &str) -> Self {
        fn walk(value: &mut Value, build: &str) {
            match value {
                Value::Object(map) => {
                    if map.contains_key("edition")
                        && map.contains_key("build")
                        && map.contains_key("interface")
                    {
                        map.insert("build".into(), json!(build));
                        map.insert("interface".into(), json!(build));
                    }
                    map.values_mut().for_each(|v| walk(v, build));
                }
                Value::Array(items) => items.iter_mut().for_each(|v| walk(v, build)),
                _ => {}
            }
        }
        walk(&mut self.0, build);
        self
    }
    /// Turns the document into non-fixture addon data (reference namespace only).
    pub fn real(mut self) -> Self {
        fn walk(value: &mut Value) {
            match value {
                Value::Object(map) => {
                    if map.get("source") == Some(&json!("synthetic_fixture")) {
                        map.insert("source".into(), json!("addon_snapshot"));
                    }
                    map.values_mut().for_each(walk);
                }
                Value::Array(items) => items.iter_mut().for_each(walk),
                _ => {}
            }
        }
        self.0["fixture"] = json!(false);
        self.0["producer"]["component"] = json!("addon");
        walk(&mut self.0);
        self
    }
    pub fn text(&self) -> String {
        serde_json::to_string(&self.0).unwrap()
    }
    pub fn pretty(&self) -> String {
        serde_json::to_string_pretty(&self.0).unwrap()
    }
    pub fn decode(&self) -> Envelope<CharacterSnapshot> {
        decode_envelope(&self.text()).expect("test document must satisfy the contract")
    }
}

/// A unique database file in a unique temp directory, removed on drop.
pub struct TempDb {
    dir: PathBuf,
    pub path: PathBuf,
}
impl TempDb {
    pub fn new(name: &str) -> Self {
        static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let dir = (0..1024)
            .find_map(|_| {
                let dir = std::env::temp_dir().join(format!(
                    "azeroth-storage-{name}-{}-{}",
                    std::process::id(),
                    NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
                ));
                match std::fs::create_dir(&dir) {
                    Ok(()) => Some(dir),
                    Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => None,
                    Err(error) => panic!("Cannot reserve storage fixture: {error}"),
                }
            })
            .expect("Storage fixture reservation exhausted");
        let path = dir.join("store.sqlite3");
        Self { dir, path }
    }
}
impl Drop for TempDb {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.dir);
    }
}

/// A plain second connection for inspecting (or tampering with) a store file.
pub fn raw(path: &Path) -> Connection {
    Connection::open(path).unwrap()
}
pub fn count(path: &Path, sql: &str) -> i64 {
    raw(path).query_row(sql, [], |row| row.get(0)).unwrap()
}
/// Every snapshot-related table, as text, in a deterministic order.
pub fn dump_state(path: &Path) -> String {
    let conn = raw(path);
    let mut out = String::new();
    for sql in [
        "SELECT id, character_id, edition, build, interface, namespace, is_fixture, current_snapshot_id FROM profiles ORDER BY id",
        "SELECT id, profile_id, snapshot_id, content_sha256, capture_sequence, document FROM snapshots ORDER BY id",
        "SELECT snapshot_id, field, entry_key, sub_key, status, clock_trust FROM snapshot_observations ORDER BY id",
    ] {
        let mut statement = conn.prepare(sql).unwrap();
        let columns = statement.column_count();
        let rows = statement
            .query_map([], |row| {
                (0..columns)
                    .map(|i| {
                        row.get::<_, rusqlite::types::Value>(i)
                            .map(|v| format!("{v:?}"))
                    })
                    .collect::<rusqlite::Result<Vec<_>>>()
            })
            .unwrap();
        for row in rows {
            out.push_str(&row.unwrap().join("|"));
            out.push('\n');
        }
        out.push_str("--\n");
    }
    out
}
