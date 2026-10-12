//! Pure summaries over validated session builds. Covered monotonic time is not
//! wall-clock duration or active play. No clock reads, combat metrics, XP
//! extrapolation, cross-context aggregation, or personalized calibration.
use crate::{
    entities::{MeasurementQuality, SessionSummary},
    error::{Result, invalid},
    sessions::{SessionBuild, SessionClockFlag, SessionQualityTotals, SessionUtc, quality_totals},
    types::Millis,
};
use serde::Serialize;
use std::collections::BTreeMap;

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct ProgressionSummary {
    /// Difference between the two retained known observations, not proof of
    /// total progression during the session. Missing endpoints remain None.
    pub observed_level_change: Option<i32>,
    /// Unique quest IDs observed turning in; not a count of all quest actions.
    pub unique_observed_quest_count: usize,
    /// Items seen in equipment-change events; not proof of acquisition.
    pub equipment_change_item_count: usize,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct TimingSummary {
    /// Measured monotonic coverage, excluding offline time between sessions.
    pub covered_ms: Millis,
    pub totals: SessionQualityTotals,
    /// Time with neither interval nor gap evidence in a validated summary.
    /// Distinct from explicitly unclassified intervals.
    pub uncovered_ms: Millis,
    /// Sorted labels, preserving each measurement quality separately.
    pub categories: BTreeMap<String, SessionQualityTotals>,
    /// Includes zero-width discontinuity reasons with a zero total.
    pub gap_reasons_ms: BTreeMap<String, Millis>,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct SessionAnalytics {
    /// Retains identity, complete data context, observations and capture
    /// metadata, evidence intervals/gaps, quests, equipment, and methodology.
    pub summary: SessionSummary,
    pub progression: ProgressionSummary,
    pub timing: TimingSummary,
    pub clock_flags: Vec<SessionClockFlag>,
    pub utc: SessionUtc,
}

fn zero_totals() -> Result<SessionQualityTotals> {
    Ok(SessionQualityTotals {
        observed_ms: Millis::new(0)?,
        estimated_ms: Millis::new(0)?,
        unclassified_ms: Millis::new(0)?,
        gap_ms: Millis::new(0)?,
    })
}
fn add(slot: &mut Millis, amount: u64) -> Result<()> {
    *slot = Millis::new(
        slot.get()
            .checked_add(amount)
            .ok_or_else(|| invalid("Analytics duration overflows"))?,
    )?;
    Ok(())
}

/// Analyze one validated session build without changing its evidence. Public
/// build fields are checked against the validated summary before consumption.
/// Repeated calls are equal; no session or context is combined with another.
pub fn analyze_session(build: &SessionBuild) -> Result<SessionAnalytics> {
    let summary = &build.summary;
    let totals = quality_totals(summary)?;
    if totals != build.totals {
        return Err(invalid("Session build totals disagree with its summary"));
    }
    for flag in &build.clock_flags {
        let at_ms = match flag {
            SessionClockFlag::UtcWentBackward { at_ms, .. }
            | SessionClockFlag::UtcDisagrees { at_ms, .. } => at_ms,
        };
        if *at_ms > summary.elapsed_ms {
            return Err(invalid("Session clock flag exceeds covered time"));
        }
    }
    if !build.clock_flags.is_empty() && matches!(build.utc, SessionUtc::Mapped { .. }) {
        return Err(invalid("Clock changes cannot have a certain UTC mapping"));
    }
    let classified = [
        totals.observed_ms,
        totals.estimated_ms,
        totals.unclassified_ms,
        totals.gap_ms,
    ]
    .into_iter()
    .try_fold(0u64, |sum, ms| {
        sum.checked_add(ms.get())
            .ok_or_else(|| invalid("Analytics duration overflows"))
    })?;
    let uncovered_ms = Millis::new(
        summary
            .elapsed_ms
            .get()
            .checked_sub(classified)
            .ok_or_else(|| invalid("Session totals exceed covered time"))?,
    )?;
    let mut categories = BTreeMap::new();
    for interval in &summary.intervals {
        let totals = categories
            .entry(interval.label.as_str().to_owned())
            .or_insert(zero_totals()?);
        let slot = match interval.quality {
            MeasurementQuality::Observed => &mut totals.observed_ms,
            MeasurementQuality::Estimated => &mut totals.estimated_ms,
            MeasurementQuality::Unclassified => &mut totals.unclassified_ms,
        };
        add(slot, interval.end_ms.get() - interval.start_ms.get())?;
    }
    let mut gap_reasons_ms = BTreeMap::new();
    for gap in &summary.gaps {
        let slot = gap_reasons_ms
            .entry(gap.reason.as_str().to_owned())
            .or_insert(Millis::new(0)?);
        add(slot, gap.end_ms.get() - gap.start_ms.get())?;
    }
    let observed_level_change = summary
        .starting_level
        .value()
        .zip(summary.ending_level.value())
        .map(|(start, end)| i32::from(end.get()) - i32::from(start.get()));
    Ok(SessionAnalytics {
        summary: summary.clone(),
        progression: ProgressionSummary {
            observed_level_change,
            unique_observed_quest_count: summary.observed_quests.len(),
            equipment_change_item_count: summary.changed_equipment.len(),
        },
        timing: TimingSummary {
            covered_ms: summary.elapsed_ms,
            totals,
            uncovered_ms,
            categories,
            gap_reasons_ms,
        },
        clock_flags: build.clock_flags.clone(),
        utc: build.utc,
    })
}
