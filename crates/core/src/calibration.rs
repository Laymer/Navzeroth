//! Deterministic personal estimates from retained complete, comparable observations.
//! Annotations are user declarations, never inferred progression or live telemetry.
use crate::{
    analytics::SessionAnalytics,
    entities::MeasurementQuality,
    error::{Result, invalid},
    types::{CharacterId, DataContext, SessionId},
};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CalibrationConditions {
    /// Explicit route/quest-stage identity; no automatic catalog association.
    pub route_stage: String,
    pub level: u32,
    pub role: String,
    pub party_size: u32,
}
impl CalibrationConditions {
    pub fn validate(&self) -> Result<()> {
        if self.route_stage.trim().is_empty()
            || self.route_stage.len() > 128
            || self.role.trim().is_empty()
            || self.role.len() > 64
            || !(1..=1000).contains(&self.level)
            || !(1..=40).contains(&self.party_size)
        {
            return Err(invalid(
                "Calibration conditions require bounded route/stage, role, level and party size",
            ));
        }
        Ok(())
    }
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SessionCalibrationAnnotation {
    pub conditions: CalibrationConditions,
    /// User confirms the whole recorded session covers a completed route/stage.
    pub complete: bool,
    pub completion_quest: String,
}
impl SessionCalibrationAnnotation {
    pub fn validate(&self) -> Result<()> {
        self.conditions.validate()?;
        if self.completion_quest.trim().is_empty() || self.completion_quest.len() > 128 {
            return Err(invalid(
                "Annotation requires a bounded observed terminal quest identity",
            ));
        }
        Ok(())
    }
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CalibrationRequest {
    pub conditions: CalibrationConditions,
    pub baseline_ms: u64,
    pub baseline_source: String,
}
impl CalibrationRequest {
    pub fn validate(&self) -> Result<()> {
        self.conditions.validate()?;
        if !(1..=86_400_000).contains(&self.baseline_ms)
            || self.baseline_source.trim().is_empty()
            || self.baseline_source.len() > 256
        {
            return Err(invalid(
                "Calibration requires a positive baseline up to 24 hours and its documented source",
            ));
        }
        Ok(())
    }
}
pub struct CalibrationSample {
    pub fixture: bool,
    pub analytics: SessionAnalytics,
    pub batch: crate::sessions::SessionBatch,
    pub annotation: Option<SessionCalibrationAnnotation>,
}
#[derive(Clone, Debug, Serialize)]
pub struct CalibrationComparison {
    pub session_id: SessionId,
    pub observed_ms: Option<u64>,
    pub difference_ms: Option<i64>,
    pub exclusion: Option<String>,
    pub outlier: bool,
}
#[derive(Clone, Debug, Serialize)]
pub struct CalibrationReport {
    pub methodology_version: u32,
    pub character: CharacterId,
    pub context: DataContext,
    pub fixture: bool,
    pub request: CalibrationRequest,
    pub sample_count: usize,
    pub median_ms: Option<u64>,
    pub minimum_ms: Option<u64>,
    pub maximum_ms: Option<u64>,
    pub blended_ms: u64,
    pub uncertainty: String,
    pub comparisons: Vec<CalibrationComparison>,
    pub methodology: String,
}
/// Exact context/fixture/conditions; median resists extremes. All exclusions and
/// extremes remain inspectable. A five-sample baseline prior limits sparse shifts.
pub fn calibrate(
    character: &CharacterId,
    context: &DataContext,
    fixture: bool,
    request: &CalibrationRequest,
    samples: &[CalibrationSample],
) -> Result<CalibrationReport> {
    request.validate()?;
    if samples.len() > 128 {
        return Err(invalid("Calibration accepts at most 128 retained sessions"));
    }
    let mut comparisons = Vec::new();
    let mut seen = std::collections::BTreeSet::new();
    for sample in samples {
        let summary = &sample.analytics.summary;
        if !seen.insert((
            summary.context.clone(),
            summary.character.clone(),
            summary.session.clone(),
            sample.fixture,
        )) {
            return Err(invalid("Duplicate calibration session identity"));
        }
        let reason = if summary.character != *character
            || summary.context != *context
            || sample.fixture != fixture
        {
            Some("incompatible_context_or_fixture")
        } else if sample.annotation.is_none() {
            Some("missing_annotation")
        } else {
            let annotation = sample.annotation.as_ref().expect("checked");
            annotation.validate()?;
            if annotation.conditions != request.conditions { Some("different_route_or_conditions") }
            else if !annotation.complete { Some("completion_not_declared") }
            else if !matches!(sample.batch.events.first().map(|event| &event.kind), Some(crate::sessions::SessionEventKind::SessionStart)) || !sample.batch.events.last().is_some_and(|event| matches!(&event.kind, crate::sessions::SessionEventKind::QuestTurnIn { quest } if quest.as_str() == annotation.completion_quest)) { Some("missing_terminal_completion_evidence") }
            else if !summary.gaps.is_empty() || sample.analytics.timing.uncovered_ms.get() != 0 { Some("gaps_or_uncovered_time") }
            else if summary.intervals.iter().any(|span| span.quality != MeasurementQuality::Observed || span.label.as_str() == crate::sessions::OBSERVED_SPAN_LABEL) { Some("activity_not_completely_observed") }
            else if summary.elapsed_ms.get() == 0 || sample.analytics.timing.totals.observed_ms != summary.elapsed_ms { Some("no_complete_positive_duration") }
            else { None }
        };
        let observed = reason.is_none().then_some(summary.elapsed_ms.get());
        comparisons.push(CalibrationComparison {
            session_id: summary.session.clone(),
            observed_ms: observed,
            difference_ms: observed.map(|v| v as i64 - request.baseline_ms as i64),
            exclusion: reason.map(str::to_owned),
            outlier: false,
        });
    }
    comparisons.sort_by(|a, b| a.session_id.cmp(&b.session_id));
    let mut values: Vec<u64> = comparisons.iter().filter_map(|v| v.observed_ms).collect();
    values.sort_unstable();
    let n = values.len();
    let median = if n == 0 {
        None
    } else {
        Some(((u128::from(values[(n - 1) / 2]) + u128::from(values[n / 2])) / 2) as u64)
    };
    if let Some(median) = median {
        for comparison in &mut comparisons {
            comparison.outlier = comparison.observed_ms.is_some_and(|v| {
                u128::from(v) > u128::from(median) * 3 || u128::from(v) * 3 < u128::from(median)
            });
        }
    }
    let blended_ms = median.map_or(request.baseline_ms, |median| {
        ((u128::from(request.baseline_ms) * 5 + u128::from(median) * n as u128) / (5 + n as u128))
            as u64
    });
    Ok(CalibrationReport { methodology_version: 1, character: character.clone(), context: context.clone(), fixture, request: request.clone(), sample_count: n, median_ms: median, minimum_ms: values.first().copied(), maximum_ms: values.last().copied(), blended_ms, uncertainty: if n == 0 { "no_complete_samples" } else if n < 5 { "sparse_personal_samples" } else { "descriptive_personal_samples" }.into(), comparisons, methodology: "Version 1: exact build/context/fixture and declared route/stage, level, role and party size; whole-session association is explicitly user-declared, conditional and not proof of route duration; session-start and terminal matching quest-turn-in evidence required. Only explicitly classified observed intervals without gaps or uncovered time contribute; UTC, unclassified and estimated time never become observed duration. Median includes retained extremes; outlier means outside one-third to three times median. Blend=(5*documented baseline+n*median)/(5+n), rounded down. Range is descriptive variability, not a confidence interval. Annotations are user declarations; this is not universal timing or automatic planner calibration.".into() })
}
