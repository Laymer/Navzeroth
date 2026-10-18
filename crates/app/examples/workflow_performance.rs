//! Explicit synthetic workflow measurement. Never opens personal or installed game data.
use azeroth_app::snapshots::{ImportDisposition, SnapshotApplication};
use azeroth_core::types::DatasetRevision;
use serde_json::{Value, json};
use std::{
    fs,
    hint::black_box,
    path::PathBuf,
    time::{Instant, SystemTime, UNIX_EPOCH},
};
const SNAPSHOT: &str = include_str!("../../../fixtures/contracts/v1/character-snapshot.json");
const CONTENT: &str = include_str!("../../../fixtures/desktop/content/demo-revision.json");
const MANIFEST: &str = include_str!("../../../fixtures/performance/continuation-v1.json");
struct Temporary(PathBuf);
impl Temporary {
    fn new() -> Self {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("synthetic clock")
            .as_nanos();
        let path = std::env::temp_dir().join(format!(
            "nav-synthetic-performance-{}-{nonce}",
            std::process::id()
        ));
        fs::create_dir(&path).expect("new synthetic measurement directory");
        Self(path)
    }
}
impl Drop for Temporary {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}
fn rss_kib() -> Option<u64> {
    fs::read_to_string("/proc/self/status")
        .ok()?
        .lines()
        .find(|line| line.starts_with("VmRSS:"))?
        .split_whitespace()
        .nth(1)?
        .parse()
        .ok()
}
fn summarize(mut samples: Vec<f64>) -> Value {
    assert!(!samples.is_empty() && samples.iter().all(|n| n.is_finite() && *n >= 0.0));
    let raw = samples.clone();
    samples.sort_by(f64::total_cmp);
    let count = samples.len();
    let median = if count.is_multiple_of(2) {
        (samples[count / 2 - 1] + samples[count / 2]) / 2.0
    } else {
        samples[count / 2]
    };
    json!({"samples_ms":raw,"min_ms":samples[0],"median_ms":median,"p95_ms":samples[(95*count).div_ceil(100)-1],"max_ms":samples[count-1]})
}
fn measure(name: &str, warmup: usize, samples: usize, mut run: impl FnMut()) -> Value {
    for _ in 0..warmup {
        run();
    }
    let before = rss_kib();
    let mut memory = Vec::new();
    let mut times = Vec::new();
    for n in 0..samples {
        let start = Instant::now();
        run();
        times.push(start.elapsed().as_secs_f64() * 1000.0);
        if (n + 1) % 10 == 0 || n + 1 == samples {
            memory.push(json!({"after_sample":n+1,"rss_kib":rss_kib()}));
        }
    }
    let mut report = summarize(times);
    report["workload"] = json!(name);
    report["rss_before_kib"] = json!(before);
    report["rss_after_kib"] = json!(rss_kib());
    report["rss_samples"] = json!(memory);
    report
}
fn snapshot(sequence: u32) -> String {
    let mut value: Value = serde_json::from_str(SNAPSHOT).expect("original synthetic snapshot");
    value["producer"]["sequence"] = json!(sequence);
    value["payload"]["id"] = json!(format!("performance-synthetic-{sequence}"));
    value["payload"]["capture"]["sequence"] = json!(sequence);
    value["payload"]["capture"]["observed_at_utc_ms"] =
        json!(1791791990000u64 + u64::from(sequence));
    format!("ANAV1:{value}")
}
fn workloads(warmup: usize, samples: usize) -> Value {
    let manifest: Value = serde_json::from_str(MANIFEST).expect("declared measurement manifest");
    assert_eq!(manifest["format_version"], 1);
    assert!(samples > 0 && warmup + samples <= 200);
    let temporary = Temporary::new();
    let app = SnapshotApplication::open(&temporary.0.join("synthetic.sqlite3"))
        .expect("synthetic SQLite store");
    let mut results = Vec::new();
    let mut sequence = 12;
    let mut profile = String::new();
    results.push(measure("snapshot-import-fresh", warmup, samples, || {
        sequence += 1;
        // Includes serialization and bounded parsing, normalization and transaction.
        profile = black_box(
            app.import_text(&snapshot(sequence), None)
                .expect("valid synthetic import"),
        )
        .profile_id;
    }));
    let final_snapshot = snapshot(sequence);
    let retained = app
        .history(&profile, 200, None)
        .expect("synthetic retained history")
        .len();
    assert_eq!(retained, warmup + samples);
    results.push(measure(
        "snapshot-import-duplicate",
        warmup,
        samples,
        || {
            let receipt = black_box(
                app.import_text(&final_snapshot, None)
                    .expect("synthetic duplicate"),
            );
            assert!(matches!(receipt.disposition, ImportDisposition::Duplicate));
        },
    ));
    assert_eq!(
        app.history(&profile, 200, None)
            .expect("unchanged synthetic history")
            .len(),
        retained
    );
    results.push(measure("snapshot-current-lookup", warmup, samples, || {
        assert!(
            black_box(
                app.current(&profile, None)
                    .expect("synthetic current lookup")
            )
            .is_some()
        );
    }));
    assert!(
        app.stage_content(CONTENT)
            .expect("original synthetic content")
            .report
            .passes
    );
    let revision = DatasetRevision::new("demo-r1").expect("synthetic revision");
    app.activate_content(&profile, &revision)
        .expect("explicit synthetic activation");
    results.push(measure("active-content-lookup", warmup, samples, || {
        assert_eq!(
            black_box(
                app.active_content(&profile)
                    .expect("synthetic active lookup")
            )
            .expect("explicit active content")
            .key
            .revision,
            revision
        );
    }));
    results.push(measure("content-stage-repeat", warmup, samples, || {
        assert!(
            black_box(
                app.stage_content(CONTENT)
                    .expect("repeat synthetic staging")
            )
            .report
            .passes
        );
    }));
    results.push(measure("content-stage-refusal", warmup, samples, || {
        assert!(black_box(app.stage_content("{truncated synthetic content")).is_err());
        assert_eq!(
            app.active_content(&profile)
                .expect("preserved synthetic selection")
                .expect("active content remains")
                .key
                .revision,
            revision
        );
    }));
    json!({"format_version":1,"fixture":manifest["identity"],"synthetic":true,"os":std::env::consts::OS,"architecture":std::env::consts::ARCH,"build_mode":if cfg!(debug_assertions){"debug-verification"}else{"release"},"warmup":warmup,"samples":samples,"retained_snapshots":retained,"sqlite_main_bytes_after":fs::metadata(temporary.0.join("synthetic.sqlite3")).expect("synthetic database metadata").len(),"sqlite_wal_bytes_after":fs::metadata(temporary.0.join("synthetic.sqlite3-wal")).ok().map(|m|m.len()),"sqlite_shm_bytes_after":fs::metadata(temporary.0.join("synthetic.sqlite3-shm")).ok().map(|m|m.len()),"results":results,"ordinary_windows_verified":false,"gameplay_verified":false})
}
fn main() {
    let arguments = std::env::args().skip(1).collect::<Vec<_>>();
    let verify = arguments == ["--verify"];
    if !arguments.is_empty() && !verify {
        eprintln!(
            "Use no options for release measurements, or --verify for a bounded harness check"
        );
        std::process::exit(2);
    }
    if cfg!(debug_assertions) && !verify {
        eprintln!("Use --release for measurements");
        std::process::exit(2);
    }
    let manifest: Value = serde_json::from_str(MANIFEST).expect("measurement manifest");
    let (warmup, samples) = if verify {
        (1, 2)
    } else {
        (
            manifest["warmup"].as_u64().expect("declared warmup") as usize,
            manifest["samples"].as_u64().expect("declared samples") as usize,
        )
    };
    println!(
        "{}",
        serde_json::to_string(&workloads(warmup, samples)).expect("measurement JSON")
    );
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn statistics_use_nearest_rank_and_preserve_raw_samples() {
        let report = summarize(vec![4.0, 1.0, 3.0, 2.0]);
        assert_eq!(report["median_ms"], 2.5);
        assert_eq!(report["p95_ms"], 4.0);
        assert_eq!(report["samples_ms"], json!([4.0, 1.0, 3.0, 2.0]));
        assert_eq!(summarize(vec![1.0, 9.0, 3.0])["median_ms"], 3.0);
    }
    #[test]
    fn synthetic_workloads_validate_duplicate_and_failed_stage_preservation() {
        let report = workloads(1, 2);
        assert_eq!(report["results"].as_array().unwrap().len(), 6);
        assert_eq!(report["retained_snapshots"], 3);
        assert_eq!(report["gameplay_verified"], false);
    }
}
