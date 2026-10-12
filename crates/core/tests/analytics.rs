//! Synthetic replay evidence only; no live client observations.
use azeroth_core::{
    analytics::analyze_session,
    error::{ErrorCode, Validated},
    observation::ObservationStatus,
    sessions::{
        SessionBatch, SessionBuild, SessionBuildConfig, SessionUtc, build_sessions, quality_totals,
    },
    types::{Millis, SessionId},
};
use serde_json::{Value, json};
use std::{collections::BTreeMap, path::PathBuf};

fn replay(name: &str) -> BTreeMap<SessionId, SessionBuild> {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/sessions/v1")
        .join(name);
    let mut doc: Value = serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap();
    let config: SessionBuildConfig =
        serde_json::from_value(doc.as_object_mut().unwrap().remove("config").unwrap()).unwrap();
    let batch: SessionBatch = serde_json::from_value(doc).unwrap();
    build_sessions(&batch, &config).unwrap()
}
fn sid(id: &str) -> SessionId {
    SessionId::new(id).unwrap()
}
fn ms(value: u64) -> Millis {
    Millis::new(value).unwrap()
}

#[test]
fn timing_preserves_observed_estimated_unclassified_and_gap_evidence() {
    let builds = replay("unclassified-span.json");
    let build = &builds[&sid("s1")];
    let report = analyze_session(build).unwrap();
    assert_eq!(report.summary, build.summary);
    assert_eq!(report.timing.covered_ms.get(), 1_060_000);
    assert_eq!(report.timing.totals.observed_ms.get(), 50_000);
    assert_eq!(report.timing.totals.estimated_ms.get(), 30_000);
    assert_eq!(report.timing.totals.unclassified_ms.get(), 940_000);
    assert_eq!(report.timing.totals.gap_ms.get(), 40_000);
    assert_eq!(report.timing.uncovered_ms.get(), 0);
    assert_eq!(
        report.timing.categories["travel"].estimated_ms.get(),
        30_000
    );
    assert_eq!(
        report.timing.categories["grind"].unclassified_ms.get(),
        370_000
    );
    assert_eq!(
        report.timing.gap_reasons_ms["recording_disabled"].get(),
        40_000
    );
    assert_eq!(report.progression.unique_observed_quest_count, 3);
    assert_eq!(report.progression.observed_level_change, None);
    assert_eq!(report, analyze_session(build).unwrap());
}

#[test]
fn reset_replays_keep_progression_context_and_zero_width_gap_reasons() {
    let builds = replay("reload-gap.json");
    let report = analyze_session(&builds[&sid("s1")]).unwrap();
    assert_eq!(report.progression.observed_level_change, Some(1));
    assert_eq!(report.progression.unique_observed_quest_count, 3);
    assert_eq!(report.progression.equipment_change_item_count, 0);
    assert_eq!(report.timing.covered_ms.get(), 27_500);
    assert_eq!(report.timing.gap_reasons_ms["session_reset"].get(), 0);
    assert_eq!(report.timing.gap_reasons_ms["reload"].get(), 0);
    assert_eq!(report.utc, builds[&sid("s1")].utc);
    assert!(matches!(report.utc, SessionUtc::Uncertain { .. }));
    assert_eq!(report.summary.context, builds[&sid("s1")].summary.context);
    let second = analyze_session(&builds[&sid("s2")]).unwrap();
    assert_eq!(second.progression.observed_level_change, None);
    assert_eq!(
        second.summary.starting_level.status(),
        ObservationStatus::Unknown
    );
    assert_eq!(second.timing.covered_ms.get(), 15_000);
}

#[test]
fn offline_and_clock_uncertainty_are_not_converted_to_active_time() {
    for name in ["offline-between-sessions.json", "clock-change.json"] {
        for build in replay(name).values() {
            let report = analyze_session(build).unwrap();
            assert_eq!(report.timing.covered_ms, build.summary.elapsed_ms);
            assert_eq!(report.clock_flags, build.clock_flags);
            assert_eq!(report.utc, build.utc);
        }
    }
    let clock = replay("clock-change.json");
    let report = analyze_session(&clock[&sid("s1")]).unwrap();
    assert_eq!(report.clock_flags.len(), 2);
    assert_eq!(report.timing.covered_ms.get(), 40_000);
}

#[test]
fn metadata_statuses_and_zero_changes_survive_without_imputation() {
    let config = SessionBuildConfig {
        max_observed_span_ms: ms(100),
        clock_tolerance_ms: ms(0),
    };
    for status in [
        "unknown",
        "unsupported",
        "restricted",
        "not_applicable",
        "known",
    ] {
        let mut observation = json!({"status":status, "source":"synthetic_fixture", "clock_trust":"uncertain", "session_id":"s1", "sequence":1, "session_elapsed_ms":50});
        if status == "known" {
            observation["value"] = json!(10);
        }
        let batch: SessionBatch = serde_json::from_value(json!({
            "character":"fixture-character",
            "context":{"build":{"edition":"wow_forever","build":"SYNTHETIC","interface":"SYNTHETIC"},"namespace":"demo"},
            "events":[{"session":"s1","sequence":2,"elapsed_ms":10,"kind":{"type":"level","level":observation}}]
        })).unwrap();
        let builds = build_sessions(&batch, &config).unwrap();
        let report = analyze_session(&builds[&sid("s1")]).unwrap();
        let serialized = serde_json::to_value(&report).unwrap();
        assert_eq!(serialized["summary"]["starting_level"], observation);
        assert_eq!(
            report.progression.observed_level_change,
            if status == "known" { Some(0) } else { None }
        );
    }
}

#[test]
fn uncovered_summary_time_remains_distinct_from_unclassified_time() {
    let mut build = replay("unclassified-span.json").remove(&sid("s1")).unwrap();
    let mut summary = build.summary.into_inner();
    summary.intervals.remove(0);
    build.summary = Validated::try_new(summary).unwrap();
    build.totals = quality_totals(&build.summary).unwrap();
    let report = analyze_session(&build).unwrap();
    assert_eq!(report.timing.uncovered_ms.get(), 30_000);
    assert_eq!(report.timing.totals.unclassified_ms.get(), 940_000);
}

#[test]
fn altered_public_build_fields_are_rejected() {
    let original = replay("clock-change.json").remove(&sid("s1")).unwrap();
    let mut build = original.clone();
    build.totals.observed_ms = ms(0);
    assert_eq!(
        analyze_session(&build).unwrap_err().code,
        ErrorCode::InvalidInput
    );
    let mut build = original.clone();
    build.utc = SessionUtc::Mapped {
        start_utc_ms: azeroth_core::types::UtcMillis::new(0).unwrap(),
    };
    assert_eq!(
        analyze_session(&build).unwrap_err().code,
        ErrorCode::InvalidInput
    );
    let mut build = original;
    build
        .clock_flags
        .push(azeroth_core::sessions::SessionClockFlag::UtcWentBackward {
            at_ms: ms(40_001),
            utc_delta_ms: -1,
        });
    assert_eq!(
        analyze_session(&build).unwrap_err().code,
        ErrorCode::InvalidInput
    );
}

#[test]
fn same_labels_keep_separate_quality_totals_and_contexts_stay_separate() {
    let mut first = replay("unclassified-span.json").remove(&sid("s1")).unwrap();
    let mut summary = first.summary.into_inner();
    for interval in &mut summary.intervals {
        interval.label = azeroth_core::types::Text::new("shared").unwrap();
    }
    first.summary = Validated::try_new(summary).unwrap();
    let report = analyze_session(&first).unwrap();
    assert_eq!(report.timing.categories.len(), 1);
    assert_eq!(report.timing.categories["shared"].observed_ms.get(), 50_000);
    assert_eq!(
        report.timing.categories["shared"].estimated_ms.get(),
        30_000
    );
    assert_eq!(
        report.timing.categories["shared"].unclassified_ms.get(),
        940_000
    );
    let mut second = first.clone();
    let mut summary = second.summary.into_inner();
    summary.context.build.build = azeroth_core::types::BuildId::new("SYNTHETIC-OTHER").unwrap();
    second.summary = Validated::try_new(summary).unwrap();
    let other = analyze_session(&second).unwrap();
    assert_ne!(report.summary.context, other.summary.context);
    assert_eq!(report.timing, other.timing);
}
