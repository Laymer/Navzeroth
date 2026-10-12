//! Helpers local to the ingest tests.
#![allow(dead_code)]
use azeroth_core::{
    observation::{ClockTrust, FreshnessContext},
    types::{Millis, UtcMillis},
};
use azeroth_ingest::import::{ExportLocation, ImportOptions};
use std::path::PathBuf;

pub fn fixture_dir(area: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures")
        .join(area)
}
pub fn transport(name: &str) -> String {
    std::fs::read_to_string(fixture_dir("transport").join(name)).expect("fixture is readable")
}
pub fn contract(name: &str) -> String {
    std::fs::read_to_string(fixture_dir("contracts/v1").join(name)).expect("fixture is readable")
}
pub fn location() -> ExportLocation {
    ExportLocation {
        global: "AzerothNavigatorDB".to_owned(),
        key: "export".to_owned(),
    }
}
pub fn options(now: Option<i64>, trust: ClockTrust, max_age_ms: u64) -> ImportOptions {
    ImportOptions::snapshots(FreshnessContext {
        now_utc_ms: now.map(|n| UtcMillis::new(n).unwrap()),
        clock_trust: trust,
        session: None,
        max_age_ms: Millis::new(max_age_ms).unwrap(),
    })
}
/// No clock at all: freshness is `Uncertain`.
pub fn no_clock() -> ImportOptions {
    options(None, ClockTrust::Uncertain, 60_000)
}
/// A unique scratch directory under the system temp dir, removed on drop.
pub struct Scratch(pub PathBuf);
impl Scratch {
    pub fn new(name: &str) -> Self {
        let path =
            std::env::temp_dir().join(format!("azeroth-ingest-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&path);
        std::fs::create_dir_all(&path).unwrap();
        Self(path)
    }
    pub fn write(&self, name: &str, content: impl AsRef<[u8]>) -> PathBuf {
        let path = self.0.join(name);
        std::fs::write(&path, content).unwrap();
        path
    }
}
impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}
