//! Snapshot recency (M2.4, CASE-009). A pure function over capture metadata;
//! it never reads a clock and never compares across data contexts (callers
//! only compare snapshots of one profile).
//!
//! The precedence mirrors `azeroth_core::observation::freshness`:
//! 1. Same producer session: compare `sequence`, then `session_elapsed_ms`.
//! 2. Otherwise compare `observed_at_utc_ms`, but only when both captures
//!    carry `ClockTrust::Trusted` and a timestamp.
//! 3. Otherwise the pair is `Incomparable`, never guessed from wall clocks.
use azeroth_core::observation::{ClockTrust, ObservationMetadata};
use std::cmp::Ordering;

/// How an incoming capture relates to the capture it would replace.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Recency {
    /// Strictly newer: the only outcome that may replace current state.
    Newer,
    Older,
    /// Equal position (same sequence/elapsed time or same trusted instant).
    Same,
    /// No rule can order the pair; neither is assumed newer.
    Incomparable,
}

/// Compares `incoming` against `current`.
pub fn compare_capture(incoming: &ObservationMetadata, current: &ObservationMetadata) -> Recency {
    if let (Some(a), Some(b)) = (&incoming.session_id, &current.session_id)
        && a == b
        && let Some(recency) = within_session(incoming, current)
    {
        return recency;
    }
    match (
        incoming.clock_trust,
        current.clock_trust,
        incoming.observed_at_utc_ms,
        current.observed_at_utc_ms,
    ) {
        (ClockTrust::Trusted, ClockTrust::Trusted, Some(a), Some(b)) => from_ordering(a.cmp(&b)),
        _ => Recency::Incomparable,
    }
}

fn from_ordering(ordering: Ordering) -> Recency {
    match ordering {
        Ordering::Greater => Recency::Newer,
        Ordering::Less => Recency::Older,
        Ordering::Equal => Recency::Same,
    }
}

/// `None` means the session carries nothing to compare (no sequence or elapsed
/// time on both sides), so the caller falls back to the trusted clock.
fn within_session(
    incoming: &ObservationMetadata,
    current: &ObservationMetadata,
) -> Option<Recency> {
    let mut equal_seen = false;
    if let (Some(a), Some(b)) = (incoming.sequence, current.sequence) {
        match a.cmp(&b) {
            Ordering::Equal => equal_seen = true,
            other => return Some(from_ordering(other)),
        }
    }
    if let (Some(a), Some(b)) = (incoming.session_elapsed_ms, current.session_elapsed_ms) {
        match a.cmp(&b) {
            Ordering::Equal => equal_seen = true,
            other => return Some(from_ordering(other)),
        }
    }
    equal_seen.then_some(Recency::Same)
}

#[cfg(test)]
mod tests {
    use super::*;
    use azeroth_core::types::{Millis, Sequence, SessionId, UtcMillis};

    fn meta(
        session: Option<&str>,
        sequence: Option<u64>,
        elapsed: Option<u64>,
        trust: ClockTrust,
        observed: Option<i64>,
    ) -> ObservationMetadata {
        ObservationMetadata {
            clock_trust: trust,
            observed_at_utc_ms: observed.map(|v| UtcMillis::new(v).unwrap()),
            session_id: session.map(|v| SessionId::new(v).unwrap()),
            session_elapsed_ms: elapsed.map(|v| Millis::new(v).unwrap()),
            sequence: sequence.map(|v| Sequence::new(v).unwrap()),
            ..ObservationMetadata::default()
        }
    }
    use ClockTrust::{Trusted, Uncertain};

    #[test]
    fn same_session_orders_by_sequence() {
        let a = meta(Some("s"), Some(13), None, Uncertain, None);
        let b = meta(Some("s"), Some(12), None, Uncertain, None);
        assert_eq!(compare_capture(&a, &b), Recency::Newer);
        assert_eq!(compare_capture(&b, &a), Recency::Older);
    }

    #[test]
    fn sequence_wins_over_clock_and_elapsed() {
        // Trusted clocks and elapsed time disagree with the sequence; sequence decides.
        let a = meta(Some("s"), Some(13), Some(1), Trusted, Some(1));
        let b = meta(Some("s"), Some(12), Some(999), Trusted, Some(999));
        assert_eq!(compare_capture(&a, &b), Recency::Newer);
    }

    #[test]
    fn equal_sequence_falls_to_elapsed_time() {
        let a = meta(Some("s"), Some(5), Some(200), Uncertain, None);
        let b = meta(Some("s"), Some(5), Some(100), Uncertain, None);
        assert_eq!(compare_capture(&a, &b), Recency::Newer);
        assert_eq!(compare_capture(&b, &a), Recency::Older);
    }

    #[test]
    fn identical_position_is_same_not_newer() {
        let a = meta(Some("s"), Some(5), Some(100), Uncertain, None);
        assert_eq!(compare_capture(&a, &a.clone()), Recency::Same);
        let only_seq = meta(Some("s"), Some(5), None, Uncertain, None);
        assert_eq!(compare_capture(&only_seq, &only_seq.clone()), Recency::Same);
    }

    #[test]
    fn different_sessions_use_trusted_clocks_only() {
        let a = meta(Some("a"), Some(1), None, Trusted, Some(2_000));
        let b = meta(Some("b"), Some(900), None, Trusted, Some(1_000));
        assert_eq!(compare_capture(&a, &b), Recency::Newer);
        assert_eq!(compare_capture(&b, &a), Recency::Older);
        let same_instant = meta(Some("c"), None, None, Trusted, Some(2_000));
        assert_eq!(compare_capture(&a, &same_instant), Recency::Same);
    }

    #[test]
    fn untrusted_or_missing_clock_is_incomparable() {
        let trusted = meta(Some("a"), Some(1), None, Trusted, Some(2_000));
        let untrusted = meta(Some("b"), Some(1), None, Uncertain, Some(1_000));
        assert_eq!(compare_capture(&trusted, &untrusted), Recency::Incomparable);
        assert_eq!(compare_capture(&untrusted, &trusted), Recency::Incomparable);
        let no_time = meta(Some("b"), Some(1), None, Trusted, None);
        assert_eq!(compare_capture(&trusted, &no_time), Recency::Incomparable);
        let no_session = meta(None, None, None, Uncertain, None);
        assert_eq!(
            compare_capture(&no_session, &no_session.clone()),
            Recency::Incomparable
        );
    }

    #[test]
    fn same_session_without_position_falls_back_to_trusted_clock() {
        let a = meta(Some("s"), None, None, Trusted, Some(2_000));
        let b = meta(Some("s"), None, None, Trusted, Some(1_000));
        assert_eq!(compare_capture(&a, &b), Recency::Newer);
        let u = meta(Some("s"), None, None, Uncertain, Some(1_000));
        assert_eq!(compare_capture(&a, &u), Recency::Incomparable);
    }

    #[test]
    fn one_sided_sequence_is_not_compared() {
        let with = meta(Some("s"), Some(9), None, Uncertain, None);
        let without = meta(Some("s"), None, None, Uncertain, None);
        assert_eq!(compare_capture(&with, &without), Recency::Incomparable);
    }
}
