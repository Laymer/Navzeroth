//! Durable local eligibility. Operator decisions never rewrite immutable content
//! or establish confidence, authenticity or redistribution permission.
use crate::{content::ContentRevisionKey, error::invalid};
use azeroth_core::{
    error::Result,
    types::{MAX_SAFE_INTEGER, Sha256Hex, UtcMillis},
};
use serde::{Deserialize, Serialize};
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DispositionState {
    Eligible,
    Quarantined,
    Revoked,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DispositionReason {
    InitialEligibility,
    OperatorReview,
    InvalidContent,
    SourcePermission,
    ContextMismatch,
    WithdrawnContribution,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ContentDisposition {
    pub key: ContentRevisionKey,
    pub content_sha256: Sha256Hex,
    pub state: DispositionState,
    pub state_revision: u64,
    pub reason_code: DispositionReason,
    pub changed_at_utc_ms: Option<UtcMillis>,
    pub last_activated_order: Option<u64>,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DispositionGuard {
    pub key: ContentRevisionKey,
    pub content_sha256: Sha256Hex,
    pub state: DispositionState,
    pub state_revision: u64,
}
impl ContentDisposition {
    pub fn guard(&self) -> DispositionGuard {
        DispositionGuard {
            key: self.key.clone(),
            content_sha256: self.content_sha256.clone(),
            state: self.state,
            state_revision: self.state_revision,
        }
    }
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DispositionMutation {
    pub guard: DispositionGuard,
    pub state: DispositionState,
    pub reason_code: DispositionReason,
}
#[derive(Clone, Debug, Serialize)]
pub struct DispositionOutcome {
    pub disposition: ContentDisposition,
    pub active: Option<ContentRevisionKey>,
    pub fallback_applied: bool,
}
fn validate(d: &ContentDisposition) -> Result<()> {
    if d.state_revision == 0
        || d.state_revision > MAX_SAFE_INTEGER
        || d.last_activated_order
            .is_some_and(|n| n == 0 || n > MAX_SAFE_INTEGER)
    {
        return Err(invalid("Content disposition counter is invalid"));
    }
    Ok(())
}
fn transition(current: &ContentDisposition, m: &DispositionMutation) -> Result<u64> {
    validate(current)?;
    if current.guard() != m.guard {
        return Err(invalid(
            "Content disposition changed; refresh before deciding",
        ));
    }
    if current.state == DispositionState::Revoked {
        return Err(invalid("Revoked content cannot be released or changed"));
    }
    if current.state == m.state {
        return Err(invalid(
            "Content disposition is already in the requested state",
        ));
    }
    if m.reason_code == DispositionReason::InitialEligibility
        || (m.state == DispositionState::Eligible
            && m.reason_code != DispositionReason::OperatorReview)
    {
        return Err(invalid(
            "Content disposition reason is invalid for this action",
        ));
    }
    current
        .state_revision
        .checked_add(1)
        .filter(|n| *n <= MAX_SAFE_INTEGER)
        .ok_or_else(|| invalid("Content disposition counter is exhausted"))
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn closed_policy_and_safe_counter_boundaries() {
        let key=serde_json::from_value(serde_json::json!({"scope":{"context":{"build":{"edition":"wow_forever","build":"UNVERIFIED_DEMO","interface":"UNVERIFIED_DEMO"},"namespace":"demo"},"fixture":true},"revision":"demo-r1"})).unwrap();
        let mut d = ContentDisposition {
            key,
            content_sha256: Sha256Hex::new("a".repeat(64)).unwrap(),
            state: DispositionState::Eligible,
            state_revision: 1,
            reason_code: DispositionReason::InitialEligibility,
            changed_at_utc_ms: None,
            last_activated_order: None,
        };
        let m = DispositionMutation {
            guard: d.guard(),
            state: DispositionState::Quarantined,
            reason_code: DispositionReason::InvalidContent,
        };
        assert_eq!(transition(&d, &m).unwrap(), 2);
        d.state_revision = MAX_SAFE_INTEGER;
        let m = DispositionMutation {
            guard: d.guard(),
            ..m
        };
        assert!(transition(&d, &m).is_err());
        d.state = DispositionState::Revoked;
        d.state_revision = 2;
        let m = DispositionMutation {
            guard: d.guard(),
            state: DispositionState::Eligible,
            reason_code: DispositionReason::OperatorReview,
        };
        assert!(transition(&d, &m).is_err());
        assert!(serde_json::from_str::<DispositionState>("\"future\"").is_err());
        assert!(serde_json::from_str::<DispositionReason>("\"free_text\"").is_err());
    }
}
