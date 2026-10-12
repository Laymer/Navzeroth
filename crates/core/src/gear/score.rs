//! M5.3: fixed-point model score breakdowns.
//!
//! Arithmetic (documented for later Lua parity, M5.6/M5.7):
//! - an attribute value (`FiniteNumber`) becomes integer thousandths by
//!   `round_half_away_from_zero(value * 1000)` computed as one IEEE-754 double
//!   multiplication; magnitudes beyond the safe integer range are an error;
//! - a contribution is `round_half_away_from_zero(value_milli * weight_milli / 1000)`
//!   computed exactly in 128-bit integers, per item and per attribute;
//! - a configuration sums its items' contributions per attribute, and the total
//!   is the checked sum of all contributions, so contributions always add up
//!   to the total exactly. Any result beyond the safe integer range is an error.
//!
//! The total is a *model score* over the modelled attributes only. It is not a
//! prediction of damage or survivability. Unknown attributes are never counted
//! as zero in a result marked complete.
use super::{
    FIXED_POINT_SCALE, GearCandidate, MODEL_SCORE_LABEL,
    profile::{RoleProfile, WeightSource},
};
use crate::{
    entities::bounded,
    error::{Result, invalid},
    observation::ObservationStatus,
    types::*,
};
use serde::Serialize;
use std::collections::BTreeMap;

/// Convert a value to integer thousandths, rounding half away from zero.
pub fn to_fixed_point(value: FiniteNumber) -> Result<i64> {
    let scaled = (value.get() * FIXED_POINT_SCALE as f64).round();
    if !scaled.is_finite() || scaled.abs() > MAX_SAFE_INTEGER as f64 {
        return Err(invalid("Value is outside the fixed-point range"));
    }
    Ok(scaled as i64)
}
fn round_div_half_away(numerator: i128, denominator: i128) -> i128 {
    let quotient = numerator / denominator;
    let remainder = numerator % denominator;
    if remainder.abs() * 2 >= denominator {
        quotient + numerator.signum()
    } else {
        quotient
    }
}
fn check_safe(value: i128) -> Result<i64> {
    i64::try_from(value)
        .ok()
        .filter(|n| n.unsigned_abs() <= MAX_SAFE_INTEGER)
        .ok_or_else(|| invalid("Score arithmetic left the safe integer range"))
}
/// `a * b / 1000` over thousandths, rounded half away from zero, checked.
pub fn fixed_mul(a_milli: i64, b_milli: i64) -> Result<i64> {
    check_safe(round_div_half_away(
        i128::from(a_milli) * i128::from(b_milli),
        i128::from(FIXED_POINT_SCALE),
    ))
}
fn checked_sum(a: i64, b: i64) -> Result<i64> {
    a.checked_add(b)
        .ok_or_else(|| invalid("Score arithmetic overflow"))
        .and_then(|n| check_safe(i128::from(n)))
}

/// Why a score is less than fully certain. Codes carry item identifiers,
/// never free text from imported data.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(tag = "code", rename_all = "snake_case")]
pub enum CertaintyReason {
    AttributesUnavailable {
        item: ItemKey,
        status: ObservationStatus,
    },
    UnweightedAttribute {
        item: ItemKey,
        attribute: AttributeId,
    },
    EffectsUnavailable {
        item: ItemKey,
        status: ObservationStatus,
    },
    UnmodeledEffectsListed {
        item: ItemKey,
        count: usize,
    },
    UnknownUniqueGroup {
        item: ItemKey,
    },
    UnknownTwoHanded {
        item: ItemKey,
    },
    BaselineSlotUnresolved {
        slot: SlotId,
    },
}

/// Ordered from most to least certain. `Complete` means no item-level gap was
/// found relative to the model; it does not mean the model itself is complete
/// (see `profile_unmodeled`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Certainty {
    Complete,
    Reduced,
    Low,
}

/// An effect the weighted attribute sum does not cover.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(tag = "code", rename_all = "snake_case")]
pub enum UnmodeledEffect {
    Listed {
        item: ItemKey,
        text: Text,
    },
    Unavailable {
        item: ItemKey,
        status: ObservationStatus,
    },
}

/// One modelled attribute, all values in thousandths. For a configuration,
/// `value` is the sum over items and `contribution` the sum of the items'
/// individually rounded contributions.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct AttributeContribution {
    pub attribute: AttributeId,
    pub value: i64,
    pub weight: i64,
    pub contribution: i64,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct ScoreBreakdown {
    /// Always "model score".
    pub label: &'static str,
    pub model_version: ScoreModelVersion,
    pub role: RoleId,
    pub weight_source: WeightSource,
    /// One entry per modelled attribute, sorted by attribute.
    pub contributions: Vec<AttributeContribution>,
    /// Exact sum of `contributions`, in thousandths; modelled attributes only.
    pub total: i64,
    pub unmodeled_effects: Vec<UnmodeledEffect>,
    /// Effects the profile knowingly does not model, for every item.
    pub profile_unmodeled: Vec<Text>,
    pub certainty: Certainty,
    pub certainty_reasons: Vec<CertaintyReason>,
}
impl ScoreBreakdown {
    /// Definitely better only when both scores are complete, come from the same
    /// model version and role, and this total is strictly higher. A reduced or
    /// low-certainty score is never definitively better or worse.
    pub fn definitively_exceeds(&self, other: &Self) -> bool {
        self.certainty == Certainty::Complete
            && other.certainty == Certainty::Complete
            && self.model_version == other.model_version
            && self.role == other.role
            && self.total > other.total
    }
}

/// Score candidates (one item, or one whole configuration) under a profile.
/// Candidates are processed in `ItemKey` order, so the result does not depend
/// on input order. `extra_reasons` carry configuration-level uncertainty.
pub fn score_candidates(
    profile: &RoleProfile,
    candidates: &[&GearCandidate],
    extra_reasons: Vec<CertaintyReason>,
) -> Result<ScoreBreakdown> {
    bounded(candidates.len())?;
    let mut sorted = candidates.to_vec();
    sorted.sort_by(|a, b| a.key().cmp(b.key()));
    let mut rows: BTreeMap<AttributeId, (i64, i64)> = profile
        .weights
        .keys()
        .map(|attribute| (attribute.clone(), (0, 0)))
        .collect();
    let mut reasons = extra_reasons;
    let mut unmodeled = Vec::new();
    for candidate in sorted {
        let key = candidate.key();
        match candidate.item.attributes.value() {
            None => reasons.push(CertaintyReason::AttributesUnavailable {
                item: key.clone(),
                status: candidate.item.attributes.status(),
            }),
            Some(attributes) => {
                for (attribute, value) in attributes.iter() {
                    let Some(weight) = profile.weights.get(attribute) else {
                        reasons.push(CertaintyReason::UnweightedAttribute {
                            item: key.clone(),
                            attribute: attribute.clone(),
                        });
                        continue;
                    };
                    let value = to_fixed_point(*value)?;
                    let contribution = fixed_mul(value, weight.get())?;
                    let row = rows.entry(attribute.clone()).or_insert((0, 0));
                    row.0 = checked_sum(row.0, value)?;
                    row.1 = checked_sum(row.1, contribution)?;
                }
            }
        }
        match candidate.item.effects.value() {
            None => {
                reasons.push(CertaintyReason::EffectsUnavailable {
                    item: key.clone(),
                    status: candidate.item.effects.status(),
                });
                unmodeled.push(UnmodeledEffect::Unavailable {
                    item: key.clone(),
                    status: candidate.item.effects.status(),
                });
            }
            Some(effects) if !effects.is_empty() => {
                reasons.push(CertaintyReason::UnmodeledEffectsListed {
                    item: key.clone(),
                    count: effects.len(),
                });
                unmodeled.extend(effects.iter().map(|text| UnmodeledEffect::Listed {
                    item: key.clone(),
                    text: text.clone(),
                }));
            }
            Some(_) => {}
        }
    }
    let mut total = 0_i64;
    let mut contributions = Vec::with_capacity(rows.len());
    for (attribute, (value, contribution)) in rows {
        total = checked_sum(total, contribution)?;
        let weight = profile
            .weights
            .get(&attribute)
            .map_or(0, |weight| weight.get());
        contributions.push(AttributeContribution {
            attribute,
            value,
            weight,
            contribution,
        });
    }
    let certainty = if reasons
        .iter()
        .any(|r| matches!(r, CertaintyReason::AttributesUnavailable { .. }))
    {
        Certainty::Low
    } else if reasons.is_empty() {
        Certainty::Complete
    } else {
        Certainty::Reduced
    };
    Ok(ScoreBreakdown {
        label: MODEL_SCORE_LABEL,
        model_version: profile.version.clone(),
        role: profile.role.clone(),
        weight_source: profile.weight_source.clone(),
        contributions,
        total,
        unmodeled_effects: unmodeled,
        profile_unmodeled: profile.known_unmodeled.clone(),
        certainty,
        certainty_reasons: reasons,
    })
}
/// Score a single candidate.
pub fn score_item(profile: &RoleProfile, candidate: &GearCandidate) -> Result<ScoreBreakdown> {
    score_candidates(profile, &[candidate], Vec::new())
}
