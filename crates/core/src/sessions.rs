//! Session interval model (M8.3, pure core slice). Turns a bounded list of
//! recorder events into one `SessionSummary` per `SessionId`, and merges
//! summaries idempotently. This module never reads a clock and does no I/O.
//!
//! Rules that hold for every output:
//! - Durations come only from `elapsed_ms`, the session-relative monotonic time
//!   of one `SessionId`. Values from different session IDs are never subtracted
//!   and UTC is never used for a duration. UTC is mapped separately, only when
//!   both sides are `ClockTrust::Trusted`, and clock changes are flagged.
//! - The first event of a session (lowest sequence) is t=0 and `elapsed_ms` is
//!   the covered monotonic time. If the monotonic value decreases while the
//!   sequence increases inside one session ID (a reset), the timeline continues
//!   from the same position through a zero-width gap and the unobserved time
//!   contributes 0 ms. So does the span after a logout event. `elapsed_ms` is
//!   therefore measured monotonic coverage, not wall-clock time, and is never
//!   negative or inflated.
//! - Time between sessions (offline) is never part of any summary.
//! - Spans longer than the caller's `max_observed_span_ms`, spans after a
//!   logout, while recording is paused, or across a recorder restart are never
//!   `Observed` (paused and restart spans are gaps of their monotonic width).
//!   A missing event is not evidence that nothing happened.
//! - Plan terms map to the existing entity: measured = `Observed`,
//!   inferred = `Estimated`, unclassified = `Unclassified`.
//!
//! The input shape is provisional: it is a local model, not a contract kind,
//! and stays unverified until the M2/M2.1 addon export exists. How the client
//! behaves across /reload and logout is unverified (M0.3-M0.6); the producer
//! session ID and monotonic values are treated as opaque.
use crate::{
    entities::{
        MeasurementQuality, SessionGap, SessionInterval, SessionSummary, SessionSummaryData,
    },
    error::{AppError, ErrorCode, Result, Validated, invalid},
    observation::{ClockTrust, Observation, ObservationMetadata, ObservationStatus},
    types::*,
};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

/// Events accepted per session ID before deduplication (the 1024 convention).
pub const MAX_SESSION_EVENTS: usize = 1024;
/// Session IDs accepted per batch.
pub const MAX_SESSIONS_PER_BATCH: usize = 64;
/// Label of an observed span that has no activity marker.
pub const OBSERVED_SPAN_LABEL: &str = "observed_span";
/// Label of a span that exceeded `max_observed_span_ms` and has no activity marker.
pub const UNCLASSIFIED_SPAN_LABEL: &str = "unclassified_span";

/// Why a stretch of a session timeline is a gap. Rendered as `as_str()` into
/// `SessionGap.reason`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(tag = "code", rename_all = "snake_case")]
pub enum GapReason {
    /// The recorder restarted (a session-start event after the first event).
    Reload,
    /// The monotonic value decreased inside one session ID.
    SessionReset,
    /// The span follows a logout event.
    Logout,
    /// The span lies between a pause and the next resume.
    RecordingDisabled,
}
impl GapReason {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Reload => "reload",
            Self::SessionReset => "session_reset",
            Self::Logout => "logout",
            Self::RecordingDisabled => "recording_disabled",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum SessionEventKind {
    SessionStart,
    /// A level observation. Its status and original capture metadata are preserved.
    /// Earlier captures may be carried forward. Capture sequence cannot exceed
    /// the enclosing event's sequence; equal sequences identify the same capture
    /// and require equal monotonic times when present. Earlier elapsed times are
    /// not compared because a session may have a monotonic reset. UTC is not an
    /// ordering constraint because clocks can change. Missing metadata stays missing.
    Level {
        level: Observation<Level>,
    },
    QuestTurnIn {
        quest: QuestId,
    },
    EquipmentChanged {
        item: ItemKey,
    },
    /// Classifies the span that ends at this event.
    Activity {
        label: Text,
        quality: MeasurementQuality,
    },
    RecordingPaused,
    RecordingResumed,
    Logout,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SessionEvent {
    pub session: SessionId,
    pub sequence: Sequence,
    /// Session-relative monotonic time; the only source of durations.
    pub elapsed_ms: Millis,
    #[serde(default)]
    pub utc: Option<UtcMillis>,
    #[serde(default)]
    pub clock_trust: ClockTrust,
    pub kind: SessionEventKind,
}
impl SessionEvent {
    /// Builds an event from observation metadata. Session ID, elapsed time and
    /// sequence are required; nothing is defaulted to zero.
    pub fn from_metadata(metadata: &ObservationMetadata, kind: SessionEventKind) -> Result<Self> {
        let (Some(session), Some(elapsed_ms), Some(sequence)) = (
            metadata.session_id.clone(),
            metadata.session_elapsed_ms,
            metadata.sequence,
        ) else {
            return Err(AppError::new(
                ErrorCode::MissingRequiredObservation,
                "Session events require a session ID, elapsed time and sequence",
                false,
            ));
        };
        Ok(Self {
            session,
            sequence,
            elapsed_ms,
            utc: metadata.observed_at_utc_ms,
            clock_trust: metadata.clock_trust,
            kind,
        })
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SessionBatch {
    pub character: CharacterId,
    pub context: DataContext,
    pub events: Vec<SessionEvent>,
}

/// Local recorder transport; deliberately separate from exchange schemas.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SessionDocument {
    pub format: String,
    pub version: u32,
    pub fixture: bool,
    pub batch: SessionBatch,
}

/// Caller-chosen thresholds; there is deliberately no default.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SessionBuildConfig {
    /// A span between consecutive events longer than this is never `Observed`.
    pub max_observed_span_ms: Millis,
    /// Allowed difference between the UTC delta and the monotonic delta.
    pub clock_tolerance_ms: Millis,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(tag = "code", rename_all = "snake_case")]
pub enum SessionClockFlag {
    /// Trusted UTC moved backward between two events.
    UtcWentBackward { at_ms: Millis, utc_delta_ms: i64 },
    /// Trusted UTC and monotonic deltas differ by more than the tolerance.
    UtcDisagrees {
        at_ms: Millis,
        monotonic_delta_ms: Millis,
        utc_delta_ms: i64,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(tag = "code", rename_all = "snake_case")]
pub enum SessionUtcUncertainty {
    MissingOrUntrustedTime,
    ClockChanged,
    TimelineDiscontinuity,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum SessionUtc {
    /// UTC of timeline t=0, reported only without clock flags or resets.
    Mapped {
        start_utc_ms: UtcMillis,
    },
    Uncertain {
        reason: SessionUtcUncertainty,
    },
}

/// Milliseconds per measurement quality, reported separately. Together with
/// `gap_ms` they never exceed the summary's `elapsed_ms`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub struct SessionQualityTotals {
    /// Plan term: measured.
    pub observed_ms: Millis,
    /// Plan term: inferred.
    pub estimated_ms: Millis,
    pub unclassified_ms: Millis,
    pub gap_ms: Millis,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct SessionBuild {
    pub summary: SessionSummary,
    pub totals: SessionQualityTotals,
    pub clock_flags: Vec<SessionClockFlag>,
    pub utc: SessionUtc,
}

fn too_large(message: &'static str) -> AppError {
    AppError::new(ErrorCode::ImportTooLarge, message, false)
}
fn add(a: Millis, b: u64) -> Result<Millis> {
    let sum = a
        .get()
        .checked_add(b)
        .ok_or_else(|| invalid("Session time overflows"))?;
    Millis::new(sum)
}

/// Sums interval and gap widths per quality. Fails only on arithmetic overflow
/// or an inverted range, which a validated summary does not contain.
pub fn quality_totals(summary: &SessionSummaryData) -> Result<SessionQualityTotals> {
    let width = |start: Millis, end: Millis| {
        end.get()
            .checked_sub(start.get())
            .ok_or_else(|| invalid("Session range is inverted"))
    };
    let (mut observed, mut estimated, mut unclassified, mut gaps) = (0u64, 0u64, 0u64, 0u64);
    for interval in &summary.intervals {
        let w = width(interval.start_ms, interval.end_ms)?;
        let slot = match interval.quality {
            MeasurementQuality::Observed => &mut observed,
            MeasurementQuality::Estimated => &mut estimated,
            MeasurementQuality::Unclassified => &mut unclassified,
        };
        *slot = slot
            .checked_add(w)
            .ok_or_else(|| invalid("Session time overflows"))?;
    }
    for gap in &summary.gaps {
        gaps = gaps
            .checked_add(width(gap.start_ms, gap.end_ms)?)
            .ok_or_else(|| invalid("Session time overflows"))?;
    }
    Ok(SessionQualityTotals {
        observed_ms: Millis::new(observed)?,
        estimated_ms: Millis::new(estimated)?,
        unclassified_ms: Millis::new(unclassified)?,
        gap_ms: Millis::new(gaps)?,
    })
}

fn validate_event(event: &SessionEvent, context: &DataContext) -> Result<()> {
    match &event.kind {
        SessionEventKind::Level { level } => {
            if level
                .metadata()
                .session_id
                .as_ref()
                .is_some_and(|id| *id != event.session)
            {
                return Err(invalid("Level observation belongs to another session"));
            }
            if let Some(sequence) = level.metadata().sequence {
                if sequence > event.sequence {
                    return Err(invalid("Level observation capture follows its event"));
                }
                if sequence == event.sequence
                    && level
                        .metadata()
                        .session_elapsed_ms
                        .is_some_and(|elapsed| elapsed != event.elapsed_ms)
                {
                    return Err(invalid(
                        "Level observation and event disagree on capture time",
                    ));
                }
            }
        }
        SessionEventKind::EquipmentChanged { item } => context.ensure_compatible(&item.context)?,
        SessionEventKind::Activity { label, .. } if label.as_str().trim().is_empty() => {
            return Err(invalid("Activity label must not be empty"));
        }
        _ => {}
    }
    Ok(())
}

/// Builds one summary per session ID. The same events (in any order, with
/// identical duplicates) always give equal output. Errors: more than
/// `MAX_SESSION_EVENTS` events per session or `MAX_SESSIONS_PER_BATCH`
/// sessions (`ImportTooLarge`); a second, different event under one
/// (session, sequence) (`InvalidInput`); an item from another data context
/// (`BuildMismatch`). Nothing is truncated.
pub fn build_sessions(
    batch: &SessionBatch,
    config: &SessionBuildConfig,
) -> Result<BTreeMap<SessionId, SessionBuild>> {
    type Grouped<'a> = BTreeMap<&'a SessionId, (usize, BTreeMap<Sequence, &'a SessionEvent>)>;
    let mut grouped: Grouped = BTreeMap::new();
    for event in &batch.events {
        if !grouped.contains_key(&event.session) && grouped.len() >= MAX_SESSIONS_PER_BATCH {
            return Err(too_large("Too many sessions in one batch"));
        }
        let (count, by_sequence) = grouped.entry(&event.session).or_default();
        *count += 1;
        if *count > MAX_SESSION_EVENTS {
            return Err(too_large("Too many events in one session"));
        }
        validate_event(event, &batch.context)?;
        match by_sequence.get(&event.sequence) {
            None => {
                by_sequence.insert(event.sequence, event);
            }
            Some(existing) if **existing == *event => {}
            Some(_) => {
                return Err(invalid("Conflicting events share one session and sequence"));
            }
        }
    }
    let mut out = BTreeMap::new();
    for (session, (_, by_sequence)) in grouped {
        let events: Vec<&SessionEvent> = by_sequence.into_values().collect();
        out.insert(session.clone(), build_one(session, &events, batch, config)?);
    }
    Ok(out)
}

struct Timeline {
    intervals: Vec<SessionInterval>,
    gaps: Vec<SessionGap>,
    /// False once a gap was pushed after the last interval, so zero-width
    /// reset gaps are never merged across.
    extendable: bool,
}
impl Timeline {
    fn gap(&mut self, start: Millis, end: Millis, reason: GapReason) -> Result<()> {
        self.gaps.push(SessionGap {
            start_ms: start,
            end_ms: end,
            reason: Text::new(reason.as_str())?,
        });
        self.extendable = false;
        Ok(())
    }
    fn interval(&mut self, start: Millis, end: Millis, quality: MeasurementQuality, label: Text) {
        if self.extendable
            && let Some(last) = self.intervals.last_mut()
            && last.end_ms == start
            && last.quality == quality
            && last.label == label
        {
            last.end_ms = end;
            return;
        }
        self.intervals.push(SessionInterval {
            start_ms: start,
            end_ms: end,
            quality,
            label,
        });
        self.extendable = true;
    }
}

fn build_one(
    session: &SessionId,
    events: &[&SessionEvent],
    batch: &SessionBatch,
    config: &SessionBuildConfig,
) -> Result<SessionBuild> {
    let (first, rest) = events
        .split_first()
        .ok_or_else(|| invalid("Session has no events"))?;
    let mut timeline = Timeline {
        intervals: Vec::new(),
        gaps: Vec::new(),
        extendable: false,
    };
    let mut quests = BTreeSet::new();
    let mut equipment = BTreeSet::new();
    let mut levels: Vec<&Observation<Level>> = Vec::new();
    let mut flags = Vec::new();
    let mut discontinuous = false;
    let mut pos = Millis::new(0)?;

    for event in events {
        match &event.kind {
            SessionEventKind::Level { level } => levels.push(level),
            SessionEventKind::QuestTurnIn { quest } => {
                quests.insert(quest.clone());
            }
            SessionEventKind::EquipmentChanged { item } => {
                equipment.insert(item.clone());
            }
            _ => {}
        }
    }
    let mut paused = matches!(first.kind, SessionEventKind::RecordingPaused);

    let mut prev = *first;
    for &cur in rest {
        let after_logout = matches!(prev.kind, SessionEventKind::Logout);
        let restarted = matches!(cur.kind, SessionEventKind::SessionStart);
        let gap_reason = |reset: bool| {
            if after_logout {
                Some(GapReason::Logout)
            } else if paused {
                Some(GapReason::RecordingDisabled)
            } else if restarted {
                Some(GapReason::Reload)
            } else if reset {
                Some(GapReason::SessionReset)
            } else {
                None
            }
        };
        let monotonic = cur.elapsed_ms.get().checked_sub(prev.elapsed_ms.get());
        // Only a span that is a real, recorded stretch of this session has a
        // monotonic delta that may be compared with UTC.
        let mut measured = None;
        match monotonic {
            None => {
                discontinuous = true;
                let reason = gap_reason(true).unwrap_or(GapReason::SessionReset);
                timeline.gap(pos, pos, reason)?;
            }
            Some(_) if after_logout => {
                discontinuous = true;
                timeline.gap(pos, pos, GapReason::Logout)?;
            }
            Some(delta) => {
                measured = Some(delta);
                let end = add(pos, delta)?;
                if let Some(reason) = gap_reason(false) {
                    timeline.gap(pos, end, reason)?;
                } else if delta > 0 {
                    let (quality, label) = match &cur.kind {
                        SessionEventKind::Activity { label, quality } => {
                            let demote = *quality == MeasurementQuality::Observed
                                && delta > config.max_observed_span_ms.get();
                            let quality = if demote {
                                MeasurementQuality::Unclassified
                            } else {
                                *quality
                            };
                            (quality, label.clone())
                        }
                        _ if delta > config.max_observed_span_ms.get() => (
                            MeasurementQuality::Unclassified,
                            Text::new(UNCLASSIFIED_SPAN_LABEL)?,
                        ),
                        _ => (
                            MeasurementQuality::Observed,
                            Text::new(OBSERVED_SPAN_LABEL)?,
                        ),
                    };
                    timeline.interval(pos, end, quality, label);
                }
                pos = end;
            }
        }
        if let (ClockTrust::Trusted, ClockTrust::Trusted, Some(before), Some(after)) =
            (prev.clock_trust, cur.clock_trust, prev.utc, cur.utc)
        {
            let utc_delta = after
                .get()
                .checked_sub(before.get())
                .ok_or_else(|| invalid("UTC delta overflows"))?;
            if utc_delta < 0 {
                flags.push(SessionClockFlag::UtcWentBackward {
                    at_ms: pos,
                    utc_delta_ms: utc_delta,
                });
            } else if let Some(delta) = measured {
                let difference = (i128::from(utc_delta) - i128::from(delta)).unsigned_abs();
                if difference > u128::from(config.clock_tolerance_ms.get()) {
                    flags.push(SessionClockFlag::UtcDisagrees {
                        at_ms: pos,
                        monotonic_delta_ms: Millis::new(delta)?,
                        utc_delta_ms: utc_delta,
                    });
                }
            }
        }
        match cur.kind {
            SessionEventKind::RecordingPaused => paused = true,
            SessionEventKind::RecordingResumed => paused = false,
            _ => {}
        }
        prev = cur;
    }

    let utc = if !flags.is_empty() {
        SessionUtc::Uncertain {
            reason: SessionUtcUncertainty::ClockChanged,
        }
    } else if discontinuous {
        SessionUtc::Uncertain {
            reason: SessionUtcUncertainty::TimelineDiscontinuity,
        }
    } else if events
        .iter()
        .any(|event| event.clock_trust != ClockTrust::Trusted || event.utc.is_none())
    {
        SessionUtc::Uncertain {
            reason: SessionUtcUncertainty::MissingOrUntrustedTime,
        }
    } else {
        match (first.clock_trust, first.utc) {
            (ClockTrust::Trusted, Some(start_utc_ms)) => SessionUtc::Mapped { start_utc_ms },
            _ => SessionUtc::Uncertain {
                reason: SessionUtcUncertainty::MissingOrUntrustedTime,
            },
        }
    };
    let is_known = |level: &&&Observation<Level>| level.status() == ObservationStatus::Known;
    let starting_level = levels
        .iter()
        .find(is_known)
        .or(levels.first())
        .map_or_else(Observation::unknown, |level| (**level).clone());
    let ending_level = levels
        .iter()
        .rev()
        .find(is_known)
        .or(levels.last())
        .map_or_else(Observation::unknown, |level| (**level).clone());
    let methodology = Text::new(format!(
        "Intervals use session-relative monotonic time of one session ID only, rebased so \
         the first event is t=0 ms; UTC is never used for durations. Spans between consecutive \
         events longer than {} ms are unclassified, never observed. Spans after logout, \
         while recording is paused or across a recorder restart are gaps. A decrease of the \
         monotonic value inside the session is a zero-width session_reset gap that adds 0 ms. \
         Quality mapping: observed=measured, estimated=inferred, unclassified=unclassified.",
        config.max_observed_span_ms.get()
    ))?;
    let summary = Validated::try_new(SessionSummaryData {
        session: session.clone(),
        character: batch.character.clone(),
        context: batch.context.clone(),
        elapsed_ms: pos,
        starting_level,
        ending_level,
        observed_quests: quests.into_iter().collect(),
        changed_equipment: equipment.into_iter().collect(),
        intervals: timeline.intervals,
        gaps: timeline.gaps,
        methodology,
    })?;
    let totals = quality_totals(&summary)?;
    Ok(SessionBuild {
        summary,
        totals,
        clock_flags: flags,
        utc,
    })
}

/// Identity of a stored summary.
pub type SessionKey = (DataContext, CharacterId, SessionId);

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum MergeOutcome {
    Inserted,
    /// An equal summary is already stored; importing again changes nothing.
    Unchanged,
    /// A different summary is stored under the same key; it is not replaced.
    Conflict,
}

pub fn summary_key(summary: &SessionSummary) -> SessionKey {
    (
        summary.context.clone(),
        summary.character.clone(),
        summary.session.clone(),
    )
}

/// What `merge_summary` would do, without changing the store.
pub fn classify_merge(
    store: &BTreeMap<SessionKey, SessionSummary>,
    incoming: &SessionSummary,
) -> MergeOutcome {
    match store.get(&summary_key(incoming)) {
        None => MergeOutcome::Inserted,
        Some(existing) if existing == incoming => MergeOutcome::Unchanged,
        Some(_) => MergeOutcome::Conflict,
    }
}

/// Idempotent merge: inserts only when the key is new. Never last-writer-wins.
pub fn merge_summary(
    store: &mut BTreeMap<SessionKey, SessionSummary>,
    incoming: SessionSummary,
) -> MergeOutcome {
    let outcome = classify_merge(store, &incoming);
    if outcome == MergeOutcome::Inserted {
        store.insert(summary_key(&incoming), incoming);
    }
    outcome
}
