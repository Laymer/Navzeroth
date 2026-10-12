//! M8.3 session interval model: behaviour of the builder and the idempotent
//! merge, driven by the synthetic fixtures under fixtures/sessions/v1.
use azeroth_core::{
    entities::{MeasurementQuality, SessionSummary},
    error::ErrorCode,
    observation::{ClockTrust, ObservationMetadata, ObservationStatus},
    sessions::*,
    types::*,
};
use serde_json::{Value, json};
use std::{collections::BTreeMap, path::PathBuf};

/// Synthetic data context, defined locally (shared test helpers are frozen).
fn context() -> Value {
    json!({"build":{"edition":"wow_forever","build":"SYNTHETIC","interface":"SYNTHETIC"},"namespace":"demo"})
}

fn fixture_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/sessions/v1")
}
fn read(name: &str) -> Value {
    let text = std::fs::read_to_string(fixture_dir().join(name)).expect("fixture is readable");
    serde_json::from_str(&text).expect("fixture is JSON")
}
/// Expands the optional `generate` member (used where a literal fixture would be
/// needlessly large) into `count` events.
fn expand(mut doc: Value) -> Value {
    if let Some(spec) = doc.as_object_mut().and_then(|o| o.remove("generate")) {
        let count = spec["count"].as_u64().unwrap();
        let step = spec["step_ms"].as_u64().unwrap();
        let events: Vec<Value> = (0..count)
            .map(|i| json!({"session": spec["session"], "sequence": i, "elapsed_ms": i * step, "kind": spec["kind"]}))
            .collect();
        doc["events"] = Value::Array(events);
    }
    doc
}
fn batch_from(doc: Value) -> SessionBatch {
    serde_json::from_value(expand(doc)).expect("fixture decodes as a session batch")
}
fn config(batch: &Value) -> SessionBuildConfig {
    serde_json::from_value(batch["config"].clone()).unwrap()
}
/// Splits a fixture into the batch and its (separately stored) config.
fn load(name: &str) -> (SessionBatch, SessionBuildConfig) {
    let mut doc = read(name);
    let config = config(&doc);
    doc.as_object_mut().unwrap().remove("config");
    (batch_from(doc), config)
}
fn build(name: &str) -> BTreeMap<SessionId, SessionBuild> {
    let (batch, config) = load(name);
    build_sessions(&batch, &config).unwrap_or_else(|e| panic!("{name}: {e:?}"))
}
fn sid(s: &str) -> SessionId {
    SessionId::new(s).unwrap()
}
fn ms(n: u64) -> Millis {
    Millis::new(n).unwrap()
}
fn code_name(code: ErrorCode) -> String {
    serde_json::to_value(code)
        .unwrap()
        .as_str()
        .unwrap()
        .to_owned()
}
fn cfg() -> SessionBuildConfig {
    SessionBuildConfig {
        max_observed_span_ms: ms(60_000),
        clock_tolerance_ms: ms(1_000),
    }
}
/// A batch from compact event tuples: (session, sequence, elapsed_ms, kind).
fn inline(events: Vec<Value>) -> SessionBatch {
    batch_from(json!({"character":"c1","context":context(),"events":events}))
}
fn event(session: &str, sequence: u64, elapsed: u64, kind: Value) -> Value {
    json!({"session":session,"sequence":sequence,"elapsed_ms":elapsed,"kind":kind})
}
fn quest(id: &str) -> Value {
    json!({"type":"quest_turn_in","quest":id})
}
fn start() -> Value {
    json!({"type":"session_start"})
}
fn spans(build: &SessionBuild) -> Vec<(u64, u64, MeasurementQuality, String)> {
    build
        .summary
        .intervals
        .iter()
        .map(|i| {
            (
                i.start_ms.get(),
                i.end_ms.get(),
                i.quality,
                i.label.as_str().to_owned(),
            )
        })
        .collect()
}
fn gaps(build: &SessionBuild) -> Vec<(u64, u64, String)> {
    build
        .summary
        .gaps
        .iter()
        .map(|g| {
            (
                g.start_ms.get(),
                g.end_ms.get(),
                g.reason.as_str().to_owned(),
            )
        })
        .collect()
}

/// Invariants every built session satisfies, whatever the input.
fn assert_sound(session: &SessionBuild, max_observed_span_ms: u64) {
    let summary = &session.summary;
    let totals = &session.totals;
    let sum = totals.observed_ms.get()
        + totals.estimated_ms.get()
        + totals.unclassified_ms.get()
        + totals.gap_ms.get();
    assert!(sum <= summary.elapsed_ms.get(), "{totals:?} fits elapsed");
    for i in &summary.intervals {
        assert!(i.start_ms <= i.end_ms && i.end_ms <= summary.elapsed_ms);
        if i.quality == MeasurementQuality::Observed {
            assert!(i.end_ms.get() - i.start_ms.get() <= max_observed_span_ms);
        }
    }
    // The summary survives the strict deserializer (same checks as try_new).
    let again: SessionSummary =
        serde_json::from_value(serde_json::to_value(summary).unwrap()).unwrap();
    assert_eq!(&again, summary);
}

#[test]
fn manifest_fixtures_have_declared_outcomes() {
    let manifest = read("manifest.json");
    let entries = manifest["fixtures"].as_array().unwrap();
    let mut listed: Vec<_> = entries
        .iter()
        .map(|e| e["file"].as_str().unwrap().to_owned())
        .collect();
    listed.sort();
    let mut on_disk: Vec<_> = std::fs::read_dir(fixture_dir())
        .unwrap()
        .map(|e| e.unwrap().file_name().into_string().unwrap())
        .filter(|n| n != "manifest.json")
        .collect();
    on_disk.sort();
    assert_eq!(listed, on_disk, "every fixture file is listed exactly once");
    for entry in entries {
        let file = entry["file"].as_str().unwrap();
        let (batch, config) = load(file);
        let result = build_sessions(&batch, &config);
        match entry["expect"].as_str().unwrap() {
            "built" => {
                let built = result.unwrap_or_else(|e| panic!("{file}: {e:?}"));
                assert!(!built.is_empty(), "{file}");
                for session in built.values() {
                    assert_sound(session, config.max_observed_span_ms.get());
                }
                assert_eq!(built, build_sessions(&batch, &config).unwrap(), "{file}");
            }
            "error" => {
                let err = result.expect_err(file);
                assert_eq!(
                    code_name(err.code),
                    entry["error_code"].as_str().unwrap(),
                    "{file}: {err}"
                );
            }
            other => panic!("{file}: unknown expectation {other}"),
        }
    }
}

#[test]
fn reload_and_reset_make_gaps_never_negative_or_summed_durations() {
    let built = build("reload-gap.json");
    assert_eq!(built.len(), 2, "one summary per session ID");
    let s1 = &built[&sid("s1")];
    // Reset 20000 -> 500 is a zero-width gap; the naive last-minus-first would
    // be 8000 and a sum across the reset would double count.
    assert_eq!(s1.summary.elapsed_ms, ms(27_500));
    assert_eq!(
        gaps(s1),
        vec![
            (20_000, 20_000, "session_reset".to_owned()),
            (25_000, 25_000, "reload".to_owned())
        ]
    );
    assert_eq!(
        spans(s1),
        vec![
            (
                0,
                20_000,
                MeasurementQuality::Observed,
                "observed_span".to_owned()
            ),
            (
                20_000,
                25_000,
                MeasurementQuality::Observed,
                "observed_span".to_owned()
            ),
            (
                25_000,
                27_500,
                MeasurementQuality::Observed,
                "observed_span".to_owned()
            ),
        ],
        "intervals are not merged across a reset or restart"
    );
    assert_eq!(s1.totals.gap_ms, ms(0));
    assert_eq!(s1.totals.observed_ms, ms(27_500));
    assert_eq!(
        s1.summary.starting_level.value(),
        Some(&Level::new(10).unwrap())
    );
    assert_eq!(
        s1.summary.ending_level.value(),
        Some(&Level::new(11).unwrap())
    );
    let quests: Vec<_> = s1
        .summary
        .observed_quests
        .iter()
        .map(|q| q.as_str())
        .collect();
    assert_eq!(quests, ["q1", "q2", "q3"]);
    // The new session ID is rebased to its own t=0 and never joined to s1.
    let s2 = &built[&sid("s2")];
    assert_eq!(s2.summary.elapsed_ms, ms(15_000));
    assert!(s2.summary.gaps.is_empty());
    assert_eq!(
        s2.summary.starting_level.status(),
        ObservationStatus::Unknown
    );
}

#[test]
fn clock_changes_are_flagged_and_never_change_durations() {
    let built = build("clock-change.json");
    let s1 = &built[&sid("s1")];
    assert_eq!(s1.summary.elapsed_ms, ms(40_000));
    assert_eq!(s1.totals.observed_ms, ms(40_000), "monotonic time only");
    assert_eq!(
        s1.clock_flags,
        vec![
            SessionClockFlag::UtcWentBackward {
                at_ms: ms(20_000),
                utc_delta_ms: -5_000
            },
            SessionClockFlag::UtcDisagrees {
                at_ms: ms(30_000),
                monotonic_delta_ms: ms(10_000),
                utc_delta_ms: 100_000
            },
        ],
        "untrusted UTC on the last event is never compared"
    );
    assert_eq!(
        s1.utc,
        SessionUtc::Uncertain {
            reason: SessionUtcUncertainty::ClockChanged
        }
    );
}

#[test]
fn utc_is_mapped_only_from_trusted_consistent_clocks() {
    let trusted = |utc: i64, trust: &str, t: u64, seq: u64, kind: Value| json!({"session":"s1","sequence":seq,"elapsed_ms":t,"utc":utc,"clock_trust":trust,"kind":kind});
    let good = inline(vec![
        trusted(5_000, "trusted", 0, 1, start()),
        trusted(15_500, "trusted", 10_000, 2, quest("q1")),
    ]);
    let built = build_sessions(&good, &cfg()).unwrap();
    assert_eq!(
        built[&sid("s1")].utc,
        SessionUtc::Mapped {
            start_utc_ms: UtcMillis::new(5_000).unwrap()
        },
        "a 500 ms drift is within the 1000 ms tolerance"
    );
    let unsure = inline(vec![
        trusted(5_000, "uncertain", 0, 1, start()),
        trusted(15_000, "trusted", 10_000, 2, quest("q1")),
    ]);
    let built = build_sessions(&unsure, &cfg()).unwrap();
    assert_eq!(
        built[&sid("s1")].utc,
        SessionUtc::Uncertain {
            reason: SessionUtcUncertainty::MissingOrUntrustedTime
        }
    );
    let none = inline(vec![event("s1", 1, 0, start())]);
    assert_eq!(
        build_sessions(&none, &cfg()).unwrap()[&sid("s1")].utc,
        SessionUtc::Uncertain {
            reason: SessionUtcUncertainty::MissingOrUntrustedTime
        }
    );
}

#[test]
fn offline_time_is_never_counted() {
    let built = build("offline-between-sessions.json");
    let s1 = &built[&sid("s1")];
    let s2 = &built[&sid("s2")];
    // The hour after the logout and the day before s2 appear nowhere.
    assert_eq!(s1.summary.elapsed_ms, ms(30_000));
    assert_eq!(s2.summary.elapsed_ms, ms(12_000));
    assert_eq!(gaps(s1), vec![(30_000, 30_000, "logout".to_owned())]);
    assert_eq!(s1.totals.observed_ms, ms(30_000));
    assert_eq!(s1.totals.gap_ms, ms(0));
    let total: u64 = built.values().map(|b| b.summary.elapsed_ms.get()).sum();
    assert_eq!(total, 42_000);
    assert!(
        s1.clock_flags.is_empty(),
        "the offline span is not compared with UTC"
    );
    assert_eq!(
        s1.utc,
        SessionUtc::Uncertain {
            reason: SessionUtcUncertainty::TimelineDiscontinuity
        }
    );
    assert_eq!(
        s2.utc,
        SessionUtc::Mapped {
            start_utc_ms: UtcMillis::new(90_000_000).unwrap()
        }
    );
}

#[test]
fn long_spans_are_unclassified_and_pauses_are_gaps() {
    let built = build("unclassified-span.json");
    let s1 = &built[&sid("s1")];
    use MeasurementQuality::*;
    assert_eq!(
        spans(s1),
        vec![
            (0, 30_000, Observed, "observed_span".to_owned()),
            (
                30_000,
                600_000,
                Unclassified,
                "unclassified_span".to_owned()
            ),
            (600_000, 630_000, Estimated, "travel".to_owned()),
            (630_000, 1_000_000, Unclassified, "grind".to_owned()),
            (1_000_000, 1_010_000, Observed, "observed_span".to_owned()),
            (1_050_000, 1_060_000, Observed, "observed_span".to_owned()),
        ]
    );
    assert_eq!(
        gaps(s1),
        vec![(1_010_000, 1_050_000, "recording_disabled".to_owned())]
    );
    let t = s1.totals;
    assert_eq!(
        (t.observed_ms, t.estimated_ms, t.unclassified_ms, t.gap_ms),
        (ms(50_000), ms(30_000), ms(940_000), ms(40_000))
    );
    assert_eq!(s1.summary.elapsed_ms, ms(1_060_000));
}

#[test]
fn a_recorder_restart_with_running_monotonic_time_is_a_reload_gap_of_that_width() {
    let batch = inline(vec![
        event("s1", 1, 0, start()),
        event("s1", 2, 5_000, quest("q1")),
        event("s1", 3, 9_000, start()),
        event("s1", 4, 12_000, quest("q2")),
    ]);
    let s1 = &build_sessions(&batch, &cfg()).unwrap()[&sid("s1")];
    assert_eq!(gaps(s1), vec![(5_000, 9_000, "reload".to_owned())]);
    assert_eq!(s1.totals.observed_ms, ms(8_000));
    assert_eq!(s1.totals.gap_ms, ms(4_000));
    assert_eq!(s1.summary.elapsed_ms, ms(12_000));
}

#[test]
fn an_observed_activity_over_the_limit_is_demoted_and_estimates_stay_estimates() {
    let act =
        |label: &str, quality: &str| json!({"type":"activity","label":label,"quality":quality});
    let batch = inline(vec![
        event("s1", 1, 0, start()),
        event("s1", 2, 100_000, act("claimed", "observed")),
        event("s1", 3, 200_000, act("guess", "estimated")),
    ]);
    let s1 = &build_sessions(&batch, &cfg()).unwrap()[&sid("s1")];
    assert_eq!(s1.totals.observed_ms, ms(0));
    assert_eq!(s1.totals.unclassified_ms, ms(100_000));
    assert_eq!(s1.totals.estimated_ms, ms(100_000));
    let batch = inline(vec![
        event("s1", 1, 0, start()),
        event(
            "s1",
            2,
            10,
            json!({"type":"activity","label":"  ","quality":"observed"}),
        ),
    ]);
    assert_eq!(
        build_sessions(&batch, &cfg()).unwrap_err().code,
        ErrorCode::InvalidInput
    );
}

#[test]
fn reimporting_the_same_observations_is_idempotent() {
    // CASE-007: every event is listed twice and in reverse order.
    let (batch, config) = load("case-007-duplicate-summary.json");
    assert_eq!(batch.events.len(), 10);
    let first = build_sessions(&batch, &config).unwrap();
    let s1 = first[&sid("s1")].clone();
    let mut single = batch.clone();
    single.events.sort_by_key(|e| e.sequence);
    single.events.dedup();
    assert_eq!(single.events.len(), 5);
    assert_eq!(
        build_sessions(&single, &config).unwrap()[&sid("s1")].summary,
        s1.summary,
        "duplicates and order do not change the summary"
    );
    let mut store = BTreeMap::new();
    assert_eq!(classify_merge(&store, &s1.summary), MergeOutcome::Inserted);
    assert_eq!(store.len(), 0, "classify does not insert");
    assert_eq!(
        merge_summary(&mut store, s1.summary.clone()),
        MergeOutcome::Inserted
    );
    // Import again, from the events and from the summary's own JSON.
    let again = build_sessions(&batch, &config).unwrap()[&sid("s1")]
        .summary
        .clone();
    assert_eq!(merge_summary(&mut store, again), MergeOutcome::Unchanged);
    let decoded: SessionSummary =
        serde_json::from_value(serde_json::to_value(&s1.summary).unwrap()).unwrap();
    assert_eq!(merge_summary(&mut store, decoded), MergeOutcome::Unchanged);
    assert_eq!(store.len(), 1, "the session count does not increase");
    // A different summary under the same key is a conflict and is not stored.
    let mut changed = batch.clone();
    changed
        .events
        .retain(|e| e.sequence != Sequence::new(3).unwrap());
    let other = build_sessions(&changed, &config).unwrap()[&sid("s1")]
        .summary
        .clone();
    assert_ne!(other, s1.summary);
    assert_eq!(merge_summary(&mut store, other), MergeOutcome::Conflict);
    let key = summary_key(&s1.summary);
    assert_eq!(store[&key], s1.summary, "never last-writer-wins");
    assert_eq!(store.len(), 1);
}

#[test]
fn summaries_are_keyed_by_character_and_session() {
    let (mut batch, config) = load("case-007-duplicate-summary.json");
    let s1 = build_sessions(&batch, &config).unwrap()[&sid("s1")]
        .summary
        .clone();
    batch.character = CharacterId::new("c2").unwrap();
    let c2 = build_sessions(&batch, &config).unwrap()[&sid("s1")]
        .summary
        .clone();
    let mut store = BTreeMap::new();
    assert_eq!(merge_summary(&mut store, s1), MergeOutcome::Inserted);
    assert_eq!(merge_summary(&mut store, c2), MergeOutcome::Inserted);
    assert_eq!(store.len(), 2);
}

#[test]
fn conflicting_duplicates_are_rejected_and_identical_ones_are_not() {
    let (batch, config) = load("duplicate-sequence-conflict.json");
    let err = build_sessions(&batch, &config).unwrap_err();
    assert_eq!(err.code, ErrorCode::InvalidInput);
    // The same sequence in different sessions is not a duplicate.
    let ok = inline(vec![
        event("s1", 1, 0, start()),
        event("s2", 1, 0, quest("q1")),
        event("s2", 1, 0, quest("q1")),
    ]);
    assert_eq!(build_sessions(&ok, &cfg()).unwrap().len(), 2);
}

#[test]
fn input_bounds_reject_instead_of_truncating() {
    let many = |n: u64| inline((0..n).map(|i| event("s1", i, i, quest("q1"))).collect());
    let ok = build_sessions(&many(1024), &cfg()).unwrap();
    assert_eq!(ok[&sid("s1")].summary.elapsed_ms, ms(1023));
    assert_eq!(
        build_sessions(&many(1025), &cfg()).unwrap_err().code,
        ErrorCode::ImportTooLarge
    );
    // Duplicates count toward the bound: the input, not the output, is bounded.
    let dup = inline((0..1025).map(|_| event("s1", 1, 0, quest("q1"))).collect());
    assert_eq!(
        build_sessions(&dup, &cfg()).unwrap_err().code,
        ErrorCode::ImportTooLarge
    );
    let sessions = |n: usize| {
        inline(
            (0..n)
                .map(|i| event(&format!("s{i}"), 1, 0, start()))
                .collect(),
        )
    };
    assert_eq!(
        build_sessions(&sessions(MAX_SESSIONS_PER_BATCH), &cfg())
            .unwrap()
            .len(),
        64
    );
    assert_eq!(
        build_sessions(&sessions(MAX_SESSIONS_PER_BATCH + 1), &cfg())
            .unwrap_err()
            .code,
        ErrorCode::ImportTooLarge
    );
    // Worst case output: alternating restarts give one gap per event, still bounded.
    let busy = inline(
        (0..1024)
            .map(|i| event("s1", i, i * 100, start()))
            .collect(),
    );
    let built = build_sessions(&busy, &cfg()).unwrap();
    let summary = &built[&sid("s1")].summary;
    assert!(summary.intervals.len() + summary.gaps.len() <= 1024);
}

#[test]
fn hostile_arithmetic_is_an_error_not_a_panic() {
    let max = MAX_SAFE_INTEGER;
    let batch = inline(vec![
        event("s1", 1, 0, start()),
        event("s1", 2, max, quest("q1")),
        event("s1", 3, 0, quest("q2")),
        event("s1", 4, max, quest("q3")),
    ]);
    // Two full-range segments would exceed the safe integer range.
    assert_eq!(
        build_sessions(&batch, &cfg()).unwrap_err().code,
        ErrorCode::InvalidInput
    );
    let single = inline(vec![
        event("s1", 1, 0, start()),
        event("s1", 2, max, quest("q1")),
    ]);
    let built = build_sessions(&single, &cfg()).unwrap();
    assert_eq!(built[&sid("s1")].summary.elapsed_ms, ms(max));
    assert_eq!(built[&sid("s1")].totals.unclassified_ms, ms(max));
}

#[test]
fn contexts_and_session_ownership_are_checked() {
    let mut other_context = context();
    other_context["namespace"] = json!("reference");
    let item = json!({"type":"equipment_changed","item":{"context":other_context,"item_id":"i1","variant":"base"}});
    let batch = inline(vec![event("s1", 1, 0, start()), event("s1", 2, 5, item)]);
    assert_eq!(
        build_sessions(&batch, &cfg()).unwrap_err().code,
        ErrorCode::BuildMismatch,
        "demo and reference namespaces never merge"
    );
    let foreign = json!({"type":"level","level":{"status":"known","value":5,"session_id":"other"}});
    let batch = inline(vec![event("s1", 1, 0, foreign)]);
    assert_eq!(
        build_sessions(&batch, &cfg()).unwrap_err().code,
        ErrorCode::InvalidInput
    );
}

#[test]
fn level_observations_keep_their_status() {
    let level = |status: &str, value: Option<u16>| {
        let mut v = json!({"type":"level","level":{"status":status}});
        if let Some(value) = value {
            v["level"]["value"] = json!(value);
        }
        v
    };
    let batch = inline(vec![
        event("s1", 1, 0, level("restricted", None)),
        event("s1", 2, 1, level("known", Some(7))),
        event("s1", 3, 2, level("unknown", None)),
        event("s2", 1, 0, level("restricted", None)),
        event("s3", 1, 0, quest("q1")),
    ]);
    let built = build_sessions(&batch, &cfg()).unwrap();
    let s1 = &built[&sid("s1")].summary;
    assert_eq!(s1.starting_level.value(), Some(&Level::new(7).unwrap()));
    assert_eq!(s1.ending_level.value(), Some(&Level::new(7).unwrap()));
    let s2 = &built[&sid("s2")].summary;
    assert_eq!(s2.starting_level.status(), ObservationStatus::Restricted);
    assert_eq!(s2.ending_level.status(), ObservationStatus::Restricted);
    let s3 = &built[&sid("s3")].summary;
    assert_eq!(s3.starting_level.status(), ObservationStatus::Unknown);
}

#[test]
fn events_from_metadata_never_default_missing_timing() {
    let mut metadata = ObservationMetadata {
        session_id: Some(sid("s1")),
        session_elapsed_ms: Some(ms(5)),
        sequence: Some(Sequence::new(3).unwrap()),
        clock_trust: ClockTrust::Trusted,
        observed_at_utc_ms: Some(UtcMillis::new(9).unwrap()),
        ..ObservationMetadata::default()
    };
    let event = SessionEvent::from_metadata(&metadata, SessionEventKind::Logout).unwrap();
    assert_eq!(
        (event.elapsed_ms, event.utc, event.clock_trust),
        (ms(5), Some(UtcMillis::new(9).unwrap()), ClockTrust::Trusted)
    );
    metadata.session_elapsed_ms = None;
    let err = SessionEvent::from_metadata(&metadata, SessionEventKind::Logout).unwrap_err();
    assert_eq!(err.code, ErrorCode::MissingRequiredObservation);
}

#[test]
fn gap_reasons_have_stable_codes_and_the_entity_still_rejects_overlaps() {
    for (reason, code) in [
        (GapReason::Reload, "reload"),
        (GapReason::SessionReset, "session_reset"),
        (GapReason::Logout, "logout"),
        (GapReason::RecordingDisabled, "recording_disabled"),
    ] {
        assert_eq!(reason.as_str(), code);
        assert_eq!(serde_json::to_value(reason).unwrap(), json!({"code": code}));
    }
    let built = build("unclassified-span.json");
    let mut json = serde_json::to_value(&built[&sid("s1")].summary).unwrap();
    json["gaps"][0]["start_ms"] = json!(1_000_000);
    assert!(serde_json::from_value::<SessionSummary>(json).is_err());
}

fn captured_level(sequence: Option<u64>, elapsed: Option<u64>) -> Value {
    let mut level = json!({"status":"known","value":7,"session_id":"s1"});
    if let Some(sequence) = sequence {
        level["sequence"] = json!(sequence);
    }
    if let Some(elapsed) = elapsed {
        level["session_elapsed_ms"] = json!(elapsed);
    }
    json!({"type":"level","level":level})
}

#[test]
fn level_capture_rejects_future_sequence_and_conflicting_same_capture_time() {
    for kind in [
        captured_level(Some(3), Some(20)),
        captured_level(Some(2), Some(9)),
    ] {
        let batch = inline(vec![event("s1", 2, 10, kind)]);
        assert_eq!(
            build_sessions(&batch, &cfg()).unwrap_err().code,
            ErrorCode::InvalidInput
        );
    }
}

#[test]
fn level_capture_preserves_earlier_missing_and_same_event_metadata() {
    for kind in [
        captured_level(Some(1), Some(5)),
        // Earlier captures can have larger elapsed values across a reset.
        captured_level(Some(1), Some(100)),
        captured_level(Some(2), Some(10)),
        captured_level(None, Some(100)),
        captured_level(Some(1), None),
    ] {
        let batch = inline(vec![event("s1", 2, 10, kind)]);
        let captured = match &batch.events[0].kind {
            SessionEventKind::Level { level } => level,
            _ => unreachable!(),
        };
        let built = build_sessions(&batch, &cfg()).unwrap();
        assert_eq!(&built[&sid("s1")].summary.starting_level, captured);
        assert_eq!(&built[&sid("s1")].summary.ending_level, captured);
    }
}
