//! M5.2/M5.3: deterministic bounded enumeration of equipment configurations,
//! theoretical and obtainable rankings, baseline resolution and an adapter to
//! the existing `GearRecommendation` entity.
//!
//! Enumeration is exhaustive over the supplied candidates (no pruning), driven
//! by a caller-supplied `SlotLayout`. Candidates are sorted by full `ItemKey`
//! and slots by `SlotId`, so results never depend on input order. The search
//! space is bounded up front by `MAX_CONFIGURATIONS`; a larger scenario is a
//! safe `InvalidInput` error rather than an unbounded search.
//!
//! Classification of a slot assignment:
//! - known violations (a known two-handed item with an off-hand item, a known
//!   two-handed item outside the main hand, two items with the same known
//!   unique group) make it `Rejected`, with reasons;
//! - an unknown `two_handed` on an item placed in either hand slot (the main hand
//!   or the off hand) of a configuration with two or more items is `Uncertain`
//!   and is kept out of the known-valid results, never assumed one-handed;
//! - an unknown `unique_group` in a configuration of two or more items keeps it
//!   valid but lowers the certainty of its score.
//!
//! Assignments with the same item set are one configuration: a valid
//! arrangement wins over an uncertain one, which wins over a rejected one.
use super::{
    GEAR_MODEL_VERSION, MAX_CONFIGURATIONS, RANKING_LABEL,
    eligibility::{GearEligibility, GearEligibilityContext, assess_candidate},
    inputs::{GearCandidate, HandSlots, SlotLayout},
    profile::RoleProfile,
    score::{Certainty, CertaintyReason, ScoreBreakdown, UnmodeledEffect, score_candidates},
};
use crate::{
    entities::{
        ConfigurationItem, EquipmentSlot, GearRecommendationData, Ranking, SnapshotRef, bounded,
        unique,
    },
    error::{Result, invalid},
    evaluation::{EvaluationContext, StalePolicy},
    observation::{Freshness, Observation, ObservationMetadata, ObservationStatus, freshness},
    types::*,
};
use serde::Serialize;
use std::collections::BTreeMap;

/// Everything a ranking needs. Contexts are compared as full `DataContext`
/// values, never by bare item id.
pub struct GearScenario<'a> {
    pub context: &'a DataContext,
    pub profile: &'a RoleProfile,
    pub layout: &'a SlotLayout,
    pub candidates: &'a [GearCandidate],
}
impl GearScenario<'_> {
    /// Candidate keys must be unique and in the scenario context, and every slot
    /// a candidate names must belong to the layout.
    pub fn validate(&self) -> Result<()> {
        bounded(self.candidates.len())?;
        unique(self.candidates.iter().map(GearCandidate::key))?;
        for candidate in self.candidates {
            self.context.ensure_compatible(&candidate.key().context)?;
            if candidate
                .item
                .slots
                .iter()
                .any(|slot| !self.layout.slots.contains(slot))
            {
                return Err(invalid("Candidate names a slot outside the layout"));
            }
        }
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(tag = "code", rename_all = "snake_case")]
pub enum ConfigurationRejection {
    TwoHandedWithOffHand {
        main_hand_item: ItemKey,
        off_hand_item: ItemKey,
    },
    TwoHandedOutsideMainHand {
        item: ItemKey,
    },
    UniqueGroupConflict {
        first: ItemKey,
        second: ItemKey,
    },
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ConfigurationValidity {
    Valid,
    Uncertain,
}
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct GearConfiguration {
    pub validity: ConfigurationValidity,
    /// Slot to placed item; `occupied_slots` includes the off-hand slot for a
    /// known two-handed main-hand item.
    pub items: BTreeMap<SlotId, ConfigurationItem>,
    pub uncertainties: Vec<CertaintyReason>,
}
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct RankedConfiguration {
    /// 1-based position in the deterministic order (score, then certainty,
    /// then item keys). Equal scores are not distinguished by rank semantics.
    pub rank: usize,
    pub configuration: GearConfiguration,
    pub score: ScoreBreakdown,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct RejectedConfiguration {
    pub items: BTreeMap<SlotId, ItemKey>,
    pub reasons: Vec<ConfigurationRejection>,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct ExcludedCandidate {
    pub item: ItemKey,
    pub eligibility: GearEligibility,
}
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct ConfigurationRanking {
    /// `Theoretical` or `Obtainable`; `Practical` is never produced here.
    pub ranking: Ranking,
    /// Always "Best among verified candidates".
    pub label: &'static str,
    pub scope_note: &'static str,
    pub model_version: ScoreModelVersion,
    pub role: RoleId,
    /// Known-valid configurations, best first.
    pub valid: Vec<RankedConfiguration>,
    /// Configurations whose validity is unknown, never mixed into `valid`.
    pub uncertain: Vec<RankedConfiguration>,
    pub rejected: Vec<RejectedConfiguration>,
    /// Obtainable only: candidates left out, with their eligibility.
    pub excluded: Vec<ExcludedCandidate>,
    /// True only when the top result is complete and every other configuration is
    /// complete with a strictly lower total.
    pub top_is_definitive: bool,
}

const THEORETICAL_NOTE: &str = "Eligibility is ignored; configuration legality is enforced.";
const OBTAINABLE_NOTE: &str =
    "Equip eligibility only; acquisition, cost and probability are not evaluated.";

/// Unknown, unsupported or restricted. `NotApplicable` is a definite answer: the
/// item has no such property (no unique restriction, not a hand-held weapon).
fn unavailable<T>(observation: &Observation<T>) -> bool {
    observation.value().is_none() && observation.status() != ObservationStatus::NotApplicable
}
fn is_two_handed(candidate: &GearCandidate) -> bool {
    candidate.item.two_handed.value() == Some(&true)
}
fn placed_item(
    candidate: &GearCandidate,
    slot: &SlotId,
    hands: Option<&HandSlots>,
) -> ConfigurationItem {
    let mut occupied_slots = vec![slot.clone()];
    if let Some(hands) = hands
        && is_two_handed(candidate)
        && *slot == hands.main_hand
    {
        occupied_slots.push(hands.off_hand.clone());
    }
    occupied_slots.sort();
    ConfigurationItem {
        item: candidate.key().clone(),
        occupied_slots,
        unique_group: candidate.item.unique_group.clone(),
    }
}

enum Classified {
    Valid(GearConfiguration),
    Uncertain(GearConfiguration),
    Rejected(RejectedConfiguration),
}
impl Classified {
    fn tier(&self) -> u8 {
        match self {
            Self::Valid(_) => 0,
            Self::Uncertain(_) => 1,
            Self::Rejected(_) => 2,
        }
    }
}
fn classify(placed: &BTreeMap<&SlotId, &GearCandidate>, hands: Option<&HandSlots>) -> Classified {
    let mut rejections = Vec::new();
    if let Some(hands) = hands {
        for (slot, candidate) in placed {
            if is_two_handed(candidate) && **slot != hands.main_hand {
                rejections.push(ConfigurationRejection::TwoHandedOutsideMainHand {
                    item: candidate.key().clone(),
                });
            }
        }
        if let (Some(main), Some(off)) = (placed.get(&hands.main_hand), placed.get(&hands.off_hand))
            && is_two_handed(main)
        {
            rejections.push(ConfigurationRejection::TwoHandedWithOffHand {
                main_hand_item: main.key().clone(),
                off_hand_item: off.key().clone(),
            });
        }
    }
    let mut groups: BTreeMap<&str, &ItemKey> = BTreeMap::new();
    for candidate in placed.values() {
        if let Some(group) = candidate.item.unique_group.value() {
            match groups.get(group.as_str()) {
                Some(first) => rejections.push(ConfigurationRejection::UniqueGroupConflict {
                    first: (*first).clone(),
                    second: candidate.key().clone(),
                }),
                None => {
                    groups.insert(group.as_str(), candidate.key());
                }
            }
        }
    }
    if !rejections.is_empty() {
        return Classified::Rejected(RejectedConfiguration {
            items: placed
                .iter()
                .map(|(slot, c)| ((*slot).clone(), c.key().clone()))
                .collect(),
            reasons: rejections,
        });
    }
    let mut uncertainties = Vec::new();
    let mut validity = ConfigurationValidity::Valid;
    if let Some(hands) = hands {
        for (slot, candidate) in placed {
            if (**slot == hands.main_hand || **slot == hands.off_hand)
                && unavailable(&candidate.item.two_handed)
            {
                validity = ConfigurationValidity::Uncertain;
                uncertainties.push(CertaintyReason::UnknownTwoHanded {
                    item: candidate.key().clone(),
                });
            }
        }
    }
    if placed.len() >= 2 {
        for candidate in placed.values() {
            if unavailable(&candidate.item.unique_group) {
                uncertainties.push(CertaintyReason::UnknownUniqueGroup {
                    item: candidate.key().clone(),
                });
            }
        }
    }
    let configuration = GearConfiguration {
        validity,
        items: placed
            .iter()
            .map(|(slot, c)| ((*slot).clone(), placed_item(c, slot, hands)))
            .collect(),
        uncertainties,
    };
    match validity {
        ConfigurationValidity::Valid => Classified::Valid(configuration),
        ConfigurationValidity::Uncertain => Classified::Uncertain(configuration),
    }
}

struct Search<'a> {
    slots: Vec<(&'a SlotId, Vec<usize>)>,
    leaves: Vec<Vec<(usize, usize)>>,
}
impl Search<'_> {
    fn walk(&mut self, depth: usize, used: &mut [bool], current: &mut Vec<(usize, usize)>) {
        if depth == self.slots.len() {
            if !current.is_empty() {
                self.leaves.push(current.clone());
            }
            return;
        }
        self.walk(depth + 1, used, current);
        for option in 0..self.slots[depth].1.len() {
            let candidate = self.slots[depth].1[option];
            if used[candidate] {
                continue;
            }
            used[candidate] = true;
            current.push((depth, candidate));
            self.walk(depth + 1, used, current);
            current.pop();
            used[candidate] = false;
        }
    }
}

fn rank(
    scenario: &GearScenario<'_>,
    ranking: Ranking,
    scope_note: &'static str,
    included: Vec<&GearCandidate>,
    excluded: Vec<ExcludedCandidate>,
) -> Result<ConfigurationRanking> {
    let layout = scenario.layout;
    let mut sorted = included;
    sorted.sort_by(|a, b| a.key().cmp(b.key()));
    let mut slot_ids: Vec<&SlotId> = layout.slots.iter().collect();
    slot_ids.sort();
    let slots: Vec<(&SlotId, Vec<usize>)> = slot_ids
        .into_iter()
        .map(|slot| {
            let options = (0..sorted.len())
                .filter(|&i| sorted[i].item.slots.contains(slot))
                .collect::<Vec<_>>();
            (slot, options)
        })
        .filter(|(_, options)| !options.is_empty())
        .collect();
    let mut space = 1_usize;
    for (_, options) in &slots {
        space = space.saturating_mul(options.len() + 1);
        if space > MAX_CONFIGURATIONS {
            return Err(invalid(
                "Configuration search space exceeds the enumeration limit",
            ));
        }
    }
    let mut search = Search {
        slots,
        leaves: Vec::new(),
    };
    search.walk(0, &mut vec![false; sorted.len()], &mut Vec::new());

    let hands = layout.hands.as_ref();
    let mut by_items: BTreeMap<Vec<ItemKey>, Classified> = BTreeMap::new();
    for leaf in &search.leaves {
        let placed: BTreeMap<&SlotId, &GearCandidate> = leaf
            .iter()
            .map(|&(depth, candidate)| (search.slots[depth].0, sorted[candidate]))
            .collect();
        let mut identity: Vec<ItemKey> = placed.values().map(|c| c.key().clone()).collect();
        identity.sort();
        let classified = classify(&placed, hands);
        match by_items.get(&identity) {
            Some(existing) if existing.tier() <= classified.tier() => {}
            _ => {
                by_items.insert(identity, classified);
            }
        }
    }

    let mut valid = Vec::new();
    let mut uncertain = Vec::new();
    let mut rejected = Vec::new();
    for (identity, classified) in by_items {
        match classified {
            Classified::Rejected(r) => rejected.push(r),
            Classified::Valid(c) => valid.push((identity, c)),
            Classified::Uncertain(c) => uncertain.push((identity, c)),
        }
    }
    let by_key: BTreeMap<&ItemKey, &GearCandidate> = sorted.iter().map(|c| (c.key(), *c)).collect();
    let finish =
        |entries: Vec<(Vec<ItemKey>, GearConfiguration)>| -> Result<Vec<RankedConfiguration>> {
            let mut scored = Vec::with_capacity(entries.len());
            for (identity, configuration) in entries {
                let members: Vec<&GearCandidate> = configuration
                    .items
                    .values()
                    .map(|p| by_key[&p.item])
                    .collect();
                let score = score_candidates(
                    scenario.profile,
                    &members,
                    configuration.uncertainties.clone(),
                )?;
                scored.push((identity, configuration, score));
            }
            scored.sort_by(|a, b| {
                b.2.total
                    .cmp(&a.2.total)
                    .then(a.2.certainty.cmp(&b.2.certainty))
                    .then(a.0.cmp(&b.0))
            });
            Ok(scored
                .into_iter()
                .enumerate()
                .map(|(i, (_, configuration, score))| RankedConfiguration {
                    rank: i + 1,
                    configuration,
                    score,
                })
                .collect())
        };
    let valid = finish(valid)?;
    let uncertain = finish(uncertain)?;
    let top_is_definitive = valid.first().is_some_and(|top| {
        top.score.certainty == Certainty::Complete
            && valid
                .iter()
                .skip(1)
                .chain(uncertain.iter())
                .all(|other| top.score.definitively_exceeds(&other.score))
    });
    Ok(ConfigurationRanking {
        ranking,
        label: RANKING_LABEL,
        scope_note,
        model_version: scenario.profile.version.clone(),
        role: scenario.profile.role.clone(),
        valid,
        uncertain,
        rejected,
        excluded,
        top_is_definitive,
    })
}

/// Theoretical comparison: every candidate in the scenario context, ignoring
/// eligibility but enforcing configuration legality.
pub fn rank_theoretical(scenario: &GearScenario<'_>) -> Result<ConfigurationRanking> {
    scenario.validate()?;
    rank(
        scenario,
        Ranking::Theoretical,
        THEORETICAL_NOTE,
        scenario.candidates.iter().collect(),
        Vec::new(),
    )
}

/// Obtainable comparison: only `GearEligibility::Eligible` candidates are
/// ranked. Everything else is listed in `excluded` with its eligibility, so
/// uncertain candidates are visible rather than hidden.
pub fn rank_obtainable(
    scenario: &GearScenario<'_>,
    eligibility: &GearEligibilityContext<'_>,
) -> Result<ConfigurationRanking> {
    scenario.validate()?;
    scenario
        .context
        .ensure_compatible(&eligibility.evaluation.snapshot.context)?;
    let mut included = Vec::new();
    let mut excluded = Vec::new();
    for candidate in scenario.candidates {
        let assessed = assess_candidate(candidate, eligibility)?;
        if assessed.is_eligible() {
            included.push(candidate);
        } else {
            excluded.push(ExcludedCandidate {
                item: candidate.key().clone(),
                eligibility: assessed,
            });
        }
    }
    excluded.sort_by(|a, b| a.item.cmp(&b.item));
    rank(
        scenario,
        Ranking::Obtainable,
        OBTAINABLE_NOTE,
        included,
        excluded,
    )
}

/// Why a layout slot of the snapshot could not be resolved to a catalog item.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(tag = "code", rename_all = "snake_case")]
pub enum BaselineGap {
    SlotNotRecorded,
    SlotUnavailable {
        status: ObservationStatus,
    },
    SlotStale,
    SlotClockUncertain,
    VariantUnavailable {
        item_id: ItemId,
        status: ObservationStatus,
    },
    VariantStale {
        item_id: ItemId,
    },
    VariantClockUncertain {
        item_id: ItemId,
    },
    NotInCatalog {
        item_id: ItemId,
        variant: VariantId,
    },
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(tag = "state", rename_all = "snake_case")]
pub enum BaselineSlot {
    Empty,
    Matched { item: ItemKey },
    Unresolved { gap: BaselineGap },
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct BaselineResolution {
    pub slots: BTreeMap<SlotId, BaselineSlot>,
}
impl BaselineResolution {
    pub fn matched(&self) -> BTreeMap<&SlotId, &ItemKey> {
        self.slots
            .iter()
            .filter_map(|(slot, state)| match state {
                BaselineSlot::Matched { item } => Some((slot, item)),
                _ => None,
            })
            .collect()
    }
    pub fn is_complete(&self) -> bool {
        self.slots
            .values()
            .all(|s| !matches!(s, BaselineSlot::Unresolved { .. }))
    }
}

enum Unusable {
    Unavailable(ObservationStatus),
    Stale,
    ClockUncertain,
}
/// Same freshness rules as the evaluator: stale needs `AllowStale`, an
/// unverifiable clock is never fresh.
fn usable<'a, T>(
    observation: &'a Observation<T>,
    ctx: &EvaluationContext<'_>,
) -> std::result::Result<&'a T, Unusable> {
    let Some(value) = observation.value() else {
        return Err(Unusable::Unavailable(observation.status()));
    };
    match freshness(observation.metadata(), ctx.freshness) {
        Freshness::Fresh { .. } => Ok(value),
        Freshness::Stale { .. } if ctx.stale_policy == StalePolicy::AllowStale => Ok(value),
        Freshness::Stale { .. } => Err(Unusable::Stale),
        Freshness::Uncertain { .. } => Err(Unusable::ClockUncertain),
    }
}
fn resolve_slot(
    equipment: &Observation<EquipmentSlot>,
    catalog: &BTreeMap<&ItemKey, &GearCandidate>,
    ctx: &EvaluationContext<'_>,
) -> BaselineSlot {
    let equipped = match usable(equipment, ctx) {
        Ok(EquipmentSlot::Empty) => return BaselineSlot::Empty,
        Ok(EquipmentSlot::Item(equipped)) => equipped,
        Err(Unusable::Unavailable(status)) => {
            return BaselineSlot::Unresolved {
                gap: BaselineGap::SlotUnavailable { status },
            };
        }
        Err(Unusable::Stale) => {
            return BaselineSlot::Unresolved {
                gap: BaselineGap::SlotStale,
            };
        }
        Err(Unusable::ClockUncertain) => {
            return BaselineSlot::Unresolved {
                gap: BaselineGap::SlotClockUncertain,
            };
        }
    };
    let item_id = equipped.item_id.clone();
    let variant = match usable(&equipped.variant, ctx) {
        Ok(variant) => variant.clone(),
        Err(Unusable::Unavailable(status)) => {
            return BaselineSlot::Unresolved {
                gap: BaselineGap::VariantUnavailable { item_id, status },
            };
        }
        Err(Unusable::Stale) => {
            return BaselineSlot::Unresolved {
                gap: BaselineGap::VariantStale { item_id },
            };
        }
        Err(Unusable::ClockUncertain) => {
            return BaselineSlot::Unresolved {
                gap: BaselineGap::VariantClockUncertain { item_id },
            };
        }
    };
    let key = ItemKey {
        context: equipped.context.clone(),
        item_id: item_id.clone(),
        variant: variant.clone(),
    };
    if catalog.contains_key(&key) {
        BaselineSlot::Matched { item: key }
    } else {
        BaselineSlot::Unresolved {
            gap: BaselineGap::NotInCatalog { item_id, variant },
        }
    }
}

fn catalog_of<'a>(scenario: &'a GearScenario<'_>) -> BTreeMap<&'a ItemKey, &'a GearCandidate> {
    scenario.candidates.iter().map(|c| (c.key(), c)).collect()
}

/// Match the snapshot's equipment against the catalog by full `ItemKey`
/// (context, item id and variant). An unknown variant never matches; an item
/// of the same id but another variant or context is not in the catalog.
pub fn resolve_baseline(
    scenario: &GearScenario<'_>,
    evaluation: &EvaluationContext<'_>,
) -> Result<BaselineResolution> {
    scenario.validate()?;
    scenario
        .context
        .ensure_compatible(&evaluation.snapshot.context)?;
    let catalog = catalog_of(scenario);
    let slots = scenario
        .layout
        .slots
        .iter()
        .map(|slot| {
            let state = match evaluation.snapshot.equipment.get(slot) {
                None => BaselineSlot::Unresolved {
                    gap: BaselineGap::SlotNotRecorded,
                },
                Some(observation) => resolve_slot(observation, &catalog, evaluation),
            };
            (slot.clone(), state)
        })
        .collect();
    Ok(BaselineResolution { slots })
}

/// Model score of the matched baseline items. Unresolved slots lower certainty
/// and contribute nothing; an empty slot is a known, scoreless slot.
pub fn score_baseline(
    scenario: &GearScenario<'_>,
    baseline: &BaselineResolution,
) -> Result<ScoreBreakdown> {
    scenario.validate()?;
    let catalog = catalog_of(scenario);
    let mut members = Vec::new();
    let mut reasons = Vec::new();
    for (slot, state) in &baseline.slots {
        match state {
            BaselineSlot::Empty => {}
            BaselineSlot::Matched { item } => match catalog.get(item) {
                Some(candidate) => members.push(*candidate),
                None => return Err(invalid("Baseline item is not in the catalog")),
            },
            BaselineSlot::Unresolved { .. } => {
                reasons.push(CertaintyReason::BaselineSlotUnresolved { slot: slot.clone() })
            }
        }
    }
    score_candidates(scenario.profile, &members, reasons)
}

/// Identity and revision data for the adapter; none of it is derived here.
pub struct RecommendationInputs<'a> {
    pub snapshot: SnapshotRef,
    pub dataset_revision: DatasetRevision,
    pub evidence: Vec<EvidenceId>,
    pub baseline: Option<(&'a BaselineResolution, &'a ScoreBreakdown)>,
}

/// Build the existing `GearRecommendationData` from the top known-valid
/// configuration of a ranking. The caller still runs `Validated::try_new`,
/// which re-checks slot overlap and unique groups. The proposed score is the
/// model score in thousandths divided by 1000; the breakdown stays canonical.
pub fn recommendation_data(
    scenario: &GearScenario<'_>,
    ranking: &ConfigurationRanking,
    inputs: RecommendationInputs<'_>,
) -> Result<GearRecommendationData> {
    let Some(top) = ranking.valid.first() else {
        return Err(invalid("No known-valid configuration to recommend"));
    };
    if ranking.ranking == Ranking::Practical {
        return Err(invalid(
            "Practical ranking is not produced by the gear core",
        ));
    }
    scenario.validate()?;
    let hands = scenario.layout.hands.as_ref();
    let catalog = catalog_of(scenario);
    let mut baseline_items = BTreeMap::new();
    let mut baseline_score = Observation::unknown();
    if let Some((resolution, score)) = inputs.baseline {
        for (slot, item) in resolution.matched() {
            let candidate = catalog
                .get(item)
                .ok_or_else(|| invalid("Baseline item is not in the catalog"))?;
            baseline_items.insert(slot.clone(), placed_item(candidate, slot, hands));
        }
        if resolution.is_complete() && score.certainty == Certainty::Complete {
            baseline_score = Observation::known(
                FiniteNumber::new(score.total as f64 / 1000.0)?,
                ObservationMetadata::default(),
            )?;
        }
    }
    let eligibility = match ranking.ranking {
        Ranking::Obtainable => Observation::known(true, ObservationMetadata::default())?,
        _ => Observation::unknown(),
    };
    let mut assumptions = vec![
        Text::new(ranking.scope_note)?,
        Text::new(format!(
            "Scores are {} values under {GEAR_MODEL_VERSION}, not measured damage or survivability.",
            top.score.label
        ))?,
    ];
    if !ranking.top_is_definitive {
        assumptions.push(Text::new(
            "The top result is not definitive: incomplete item data or other configurations could change the order.",
        )?);
    }
    let mut unmodeled_effects = Vec::new();
    for effect in &top.score.unmodeled_effects {
        unmodeled_effects.push(match effect {
            UnmodeledEffect::Listed { text, .. } => text.clone(),
            UnmodeledEffect::Unavailable { item, status } => Text::new(format!(
                "Effects unavailable ({status:?}) for item {}",
                item.item_id.as_str()
            ))?,
        });
    }
    Ok(GearRecommendationData {
        context: scenario.context.clone(),
        snapshot: inputs.snapshot,
        dataset_revision: inputs.dataset_revision,
        score_model_version: ranking.model_version.clone(),
        scenario: ranking.role.clone(),
        ranking: ranking.ranking,
        baseline: RecordMap::new(baseline_items)?,
        proposed: RecordMap::new(top.configuration.items.clone())?,
        eligibility,
        baseline_score,
        proposed_score: if top.score.certainty == Certainty::Complete {
            Observation::known(
                FiniteNumber::new(top.score.total as f64 / 1000.0)?,
                ObservationMetadata::default(),
            )?
        } else {
            Observation::unknown()
        },
        acquisitions: Vec::new(),
        assumptions,
        unmodeled_effects,
        explanation: Text::new(format!(
            "{RANKING_LABEL} by model score; certainty {:?}.",
            top.score.certainty
        ))?,
        evidence: inputs.evidence,
    })
}
