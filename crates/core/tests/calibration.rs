use azeroth_core::{analytics::analyze_session, calibration::*, sessions::*, types::*};
use serde_json::json;
fn conditions() -> CalibrationConditions {
    CalibrationConditions {
        route_stage: "route-a/turn-in".into(),
        level: 12,
        role: "healer".into(),
        party_size: 2,
    }
}
fn request() -> CalibrationRequest {
    CalibrationRequest {
        conditions: conditions(),
        baseline_ms: 100_000,
        baseline_source: "declared planning assumption".into(),
    }
}
fn sample(id: &str, duration: u64) -> CalibrationSample {
    let batch: SessionBatch = serde_json::from_value(json!({"character":"example","context":{"build":{"edition":"wow_forever","build":"SYNTHETIC","interface":"SYNTHETIC"},"namespace":"demo"},"events":[{"session":id,"sequence":1,"elapsed_ms":0,"kind":{"type":"session_start"}},{"session":id,"sequence":2,"elapsed_ms":duration,"kind":{"type":"activity","label":"declared_stage","quality":"observed"}},{"session":id,"sequence":3,"elapsed_ms":duration,"kind":{"type":"quest_turn_in","quest":"quest-a"}}]})).unwrap();
    let build = build_sessions(
        &batch,
        &SessionBuildConfig {
            max_observed_span_ms: Millis::new(86_400_000).unwrap(),
            clock_tolerance_ms: Millis::new(5000).unwrap(),
        },
    )
    .unwrap()
    .into_values()
    .next()
    .unwrap();
    CalibrationSample {
        fixture: true,
        analytics: analyze_session(&build).unwrap(),
        batch,
        annotation: Some(SessionCalibrationAnnotation {
            conditions: conditions(),
            complete: true,
            completion_quest: "quest-a".into(),
        }),
    }
}
#[test]
fn sparse_data_blends_baseline_and_preserves_extremes_reproducibly() {
    let samples = vec![
        sample("b", 200_000),
        sample("a", 200_000),
        sample("c", 2_000_000),
    ];
    let report = calibrate(
        &CharacterId::new("example").unwrap(),
        &samples[0].batch.context,
        true,
        &request(),
        &samples,
    )
    .unwrap();
    assert_eq!(report.sample_count, 3);
    assert_eq!(report.median_ms, Some(200_000));
    assert_eq!(report.blended_ms, 137_500);
    assert_eq!(report.maximum_ms, Some(2_000_000));
    assert!(report.comparisons[2].outlier);
    let reversed = samples.into_iter().rev().collect::<Vec<_>>();
    assert_eq!(
        serde_json::to_value(&report).unwrap(),
        serde_json::to_value(
            calibrate(
                &CharacterId::new("example").unwrap(),
                &reversed[0].batch.context,
                true,
                &request(),
                &reversed
            )
            .unwrap()
        )
        .unwrap()
    );
}
#[test]
fn ambiguous_incomplete_and_unclassified_samples_do_not_calibrate() {
    let mut a = sample("a", 200_000);
    a.annotation = None;
    let mut b = sample("b", 200_000);
    b.annotation.as_mut().unwrap().complete = false;
    let mut c = sample("c", 200_000);
    c.analytics.summary = {
        let mut data = (*c.analytics.summary).clone();
        data.intervals[0].quality = azeroth_core::entities::MeasurementQuality::Unclassified;
        azeroth_core::error::Validated::try_new(data).unwrap()
    };
    let mut d = sample("d", 200_000);
    d.annotation.as_mut().unwrap().completion_quest = "other".into();
    let samples = vec![a, b, c, d];
    let report = calibrate(
        &CharacterId::new("example").unwrap(),
        &samples[0].batch.context,
        true,
        &request(),
        &samples,
    )
    .unwrap();
    assert_eq!(report.sample_count, 0);
    assert_eq!(report.blended_ms, 100_000);
    assert!(report.comparisons.iter().all(|r| r.exclusion.is_some()));
}
#[test]
fn conditions_context_fixture_and_duplicates_are_guarded() {
    let mut a = sample("a", 200_000);
    a.annotation.as_mut().unwrap().conditions.party_size = 1;
    let mut b = sample("b", 200_000);
    b.fixture = false;
    let context = a.batch.context.clone();
    let report = calibrate(
        &CharacterId::new("example").unwrap(),
        &context,
        true,
        &request(),
        &[a, b],
    )
    .unwrap();
    assert_eq!(report.sample_count, 0);
    let a = sample("a", 200_000);
    let b = sample("a", 200_000);
    assert!(
        calibrate(
            &CharacterId::new("example").unwrap(),
            &context,
            true,
            &request(),
            &[a, b]
        )
        .is_err()
    );
    let mut bad = request();
    bad.baseline_ms = 0;
    assert!(
        calibrate(
            &CharacterId::new("example").unwrap(),
            &context,
            true,
            &bad,
            &[]
        )
        .is_err()
    );
}

#[test]
fn unknown_activity_zero_width_resets_and_other_identity_remain_excluded() {
    let mut a = sample("plain", 120_000);
    let mut data = (*a.analytics.summary).clone();
    data.intervals[0].label = Text::new(OBSERVED_SPAN_LABEL).unwrap();
    a.analytics.summary = azeroth_core::error::Validated::try_new(data).unwrap();
    let mut b = sample("reset", 120_000);
    let mut data = (*b.analytics.summary).clone();
    data.gaps.push(azeroth_core::entities::SessionGap {
        start_ms: Millis::new(0).unwrap(),
        end_ms: Millis::new(0).unwrap(),
        reason: Text::new("session_reset").unwrap(),
    });
    b.analytics.summary = azeroth_core::error::Validated::try_new(data).unwrap();
    let mut c = sample("other-build", 120_000);
    let mut data = (*c.analytics.summary).clone();
    data.context = serde_json::from_value(json!({"build":{"edition":"wow_forever","build":"OTHER","interface":"SYNTHETIC"},"namespace":"demo"})).unwrap();
    c.analytics.summary = azeroth_core::error::Validated::try_new(data).unwrap();
    let mut d = sample("other-character", 120_000);
    let mut data = (*d.analytics.summary).clone();
    data.character = CharacterId::new("other").unwrap();
    d.analytics.summary = azeroth_core::error::Validated::try_new(data).unwrap();
    let context = a.batch.context.clone();
    let report = calibrate(
        &CharacterId::new("example").unwrap(),
        &context,
        true,
        &request(),
        &[a, b, c, d],
    )
    .unwrap();
    assert_eq!(report.sample_count, 0);
    assert!(report.comparisons.iter().all(|v| v.observed_ms.is_none()));
}
