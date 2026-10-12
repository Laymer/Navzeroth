//! Availability and freshness are independent. This module never reads a clock.
use crate::{
    error::{Result, Validate, invalid},
    types::*,
};
use serde::{Deserialize, Deserializer, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ObservationStatus {
    Known,
    Unknown,
    Unsupported,
    Restricted,
    NotApplicable,
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StateSource {
    #[default]
    Unrecorded,
    AddonSnapshot,
    ManualEntry,
    OfficialApi,
    ReferenceData,
    SyntheticFixture,
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ClockTrust {
    Trusted,
    #[default]
    Uncertain,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ObservationMetadata {
    #[serde(default)]
    pub source: StateSource,
    #[serde(default)]
    pub clock_trust: ClockTrust,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub observed_at_utc_ms: Option<UtcMillis>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub received_at_utc_ms: Option<UtcMillis>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub session_id: Option<SessionId>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub session_elapsed_ms: Option<Millis>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sequence: Option<Sequence>,
}
impl Validate for ObservationMetadata {
    fn validate(&self) -> Result<()> {
        if (self.session_elapsed_ms.is_some() || self.sequence.is_some())
            && self.session_id.is_none()
        {
            return Err(invalid(
                "Session timing and sequence require a producer session ID",
            ));
        }
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Observation<T> {
    status: ObservationStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    value: Option<T>,
    #[serde(flatten)]
    metadata: ObservationMetadata,
}
impl<T> Observation<T> {
    pub fn status(&self) -> ObservationStatus {
        self.status
    }
    pub fn value(&self) -> Option<&T> {
        self.value.as_ref()
    }
    pub fn metadata(&self) -> &ObservationMetadata {
        &self.metadata
    }
    pub fn unavailable(status: ObservationStatus, metadata: ObservationMetadata) -> Result<Self> {
        metadata.validate()?;
        if status == ObservationStatus::Known {
            return Err(invalid("Known observations require a value"));
        }
        Ok(Self {
            status,
            value: None,
            metadata,
        })
    }
    pub fn unknown() -> Self {
        Self {
            status: ObservationStatus::Unknown,
            value: None,
            metadata: ObservationMetadata::default(),
        }
    }
}
impl<T: Serialize> Observation<T> {
    pub fn known(value: T, metadata: ObservationMetadata) -> Result<Self> {
        metadata.validate()?;
        if serde_json::to_value(&value)
            .map_err(|_| invalid("Observation value cannot be serialized"))?
            .is_null()
        {
            return Err(invalid("Known observations require a non-null value"));
        }
        Ok(Self {
            status: ObservationStatus::Known,
            value: Some(value),
            metadata,
        })
    }
}

// A defaulted Option alone would silently conflate an absent value with explicit null.
#[derive(Default)]
enum Presence<T> {
    #[default]
    Missing,
    Null,
    Value(T),
}
impl<'de, T: Deserialize<'de>> Deserialize<'de> for Presence<T> {
    fn deserialize<D: Deserializer<'de>>(d: D) -> std::result::Result<Self, D::Error> {
        Ok(match Option::<T>::deserialize(d)? {
            Some(v) => Self::Value(v),
            None => Self::Null,
        })
    }
}
#[derive(Deserialize)]
#[serde(bound(deserialize = "T: Deserialize<'de>"))]
struct RawObservation<T> {
    status: ObservationStatus,
    #[serde(default)]
    value: Presence<T>,
    #[serde(flatten)]
    metadata: ObservationMetadata,
}
impl<'de, T: Deserialize<'de>> Deserialize<'de> for Observation<T> {
    fn deserialize<D: Deserializer<'de>>(d: D) -> std::result::Result<Self, D::Error> {
        let raw = RawObservation::<T>::deserialize(d)?;
        raw.metadata.validate().map_err(serde::de::Error::custom)?;
        let value = match (raw.status, raw.value) {
            (ObservationStatus::Known, Presence::Value(v)) => Some(v),
            (ObservationStatus::Known, _) => {
                return Err(serde::de::Error::custom(
                    "Known observation requires a non-null value",
                ));
            }
            (_, Presence::Missing) => None,
            _ => {
                return Err(serde::de::Error::custom(
                    "Unavailable observation must omit value",
                ));
            }
        };
        Ok(Self {
            status: raw.status,
            value,
            metadata: raw.metadata,
        })
    }
}

#[derive(Clone, Debug)]
pub struct FreshnessContext {
    pub now_utc_ms: Option<UtcMillis>,
    pub clock_trust: ClockTrust,
    pub session: Option<(SessionId, Millis)>,
    pub max_age_ms: Millis,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ClockUncertainty {
    MissingOrUntrustedTime,
    ClockReversed,
    AgeOutOfRange,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum Freshness {
    Fresh { age_ms: Millis },
    Stale { age_ms: Millis },
    Uncertain { reason: ClockUncertainty },
}
pub fn freshness(metadata: &ObservationMetadata, context: &FreshnessContext) -> Freshness {
    let elapsed = match (
        &context.session,
        &metadata.session_id,
        metadata.session_elapsed_ms,
    ) {
        (Some((current, now)), Some(producer), Some(observed)) if current == producer => {
            Some(now.get().checked_sub(observed.get()))
        }
        _ => None,
    };
    let delta = match elapsed {
        Some(delta) => delta,
        None => match (
            context.clock_trust,
            metadata.clock_trust,
            context.now_utc_ms,
            metadata.observed_at_utc_ms,
        ) {
            (ClockTrust::Trusted, ClockTrust::Trusted, Some(now), Some(observed)) => {
                u64::try_from(now.get() - observed.get()).ok()
            }
            _ => {
                return Freshness::Uncertain {
                    reason: ClockUncertainty::MissingOrUntrustedTime,
                };
            }
        },
    };
    let Some(delta) = delta else {
        return Freshness::Uncertain {
            reason: ClockUncertainty::ClockReversed,
        };
    };
    let Ok(age_ms) = Millis::new(delta) else {
        return Freshness::Uncertain {
            reason: ClockUncertainty::AgeOutOfRange,
        };
    };
    if age_ms > context.max_age_ms {
        Freshness::Stale { age_ms }
    } else {
        Freshness::Fresh { age_ms }
    }
}
