//! SQLite persistence for Azeroth Navigator character snapshots (M1.7) with
//! the M2.4 idempotency and ordering rules. Core stays free of I/O; this crate
//! is the only place that touches the database.
//!
//! The API is synchronous. Callers (`crates/app`, wired in wave 2) must run it
//! on a blocking worker, never on an async or UI thread (plan section 8.2).
//! WAL mode and online backup are out of scope until packaging is verified.
//!
//! Rules this crate enforces:
//! - A snapshot is identified by a content hash (`snapshot_content_hash`) scoped
//!   to its profile; importing identical content again is a no-op (CASE-007).
//! - A snapshot replaces the current one only when `ordering::compare_capture`
//!   proves it strictly newer. Older, equal and incomparable snapshots are kept
//!   in history and reported, never silently promoted (CASE-009).
//! - Profiles are keyed by character, edition, build, interface, dataset
//!   namespace and fixture flag; contexts never share state (CASE-015).
//! - Imports run in one transaction; a failure leaves earlier state untouched,
//!   and the audit row keeps only an error code and a static message (CASE-008).
//! - Stored documents are re-validated through `decode_envelope` on every read.
mod error;
pub mod hash;
#[doc(hidden)]
pub mod migrations;
mod model;
pub mod ordering;
mod sqlite;

pub use hash::snapshot_content_hash;
pub use model::{
    HistoryReason, ImportOutcome, ImportSource, MAX_HISTORY_LIMIT, MAX_IMPORT_ROWS,
    MAX_OBSERVATION_ROWS, MAX_PREFERENCES_PER_PROFILE, MAX_PROFILES, MAX_SNAPSHOTS_PER_PROFILE,
    ProfileKey, ProfileSummary, SnapshotStore, SnapshotSummary,
};
pub use sqlite::SqliteStore;
