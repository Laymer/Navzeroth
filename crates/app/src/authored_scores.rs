//! Individual heuristic arithmetic when authored equipment facts are unavailable.
//! Deliberately contains no configuration ranking, baseline or acquisition advice.
use crate::{
    catalog::CatalogView,
    gear::{GearReport, GearRequest},
};
use azeroth_core::{
    contract::Envelope,
    entities::CharacterSnapshot,
    error::{AppError, ErrorCode, Result},
    gear::{GearCandidate, ProficiencyTable, ScoreBreakdown, SlotLayout, score_item},
    observation::{FreshnessContext, Observation},
    types::*,
};
use serde::Serialize;
use std::collections::BTreeSet;

#[derive(Clone, Debug, Serialize)]
#[serde(untagged)]
pub enum GearComparison {
    Full(Box<GearReport>),
    ScoreOnly(Box<AuthoredScores>),
}
#[derive(Clone, Debug, Serialize)]
pub struct ItemScore {
    pub item: ItemKey,
    pub score: ScoreBreakdown,
}
#[derive(Clone, Debug, Serialize)]
pub struct AuthoredScores {
    pub mode: &'static str,
    pub request: GearRequest,
    pub fixture: bool,
    pub context: DataContext,
    pub source_layout: Observation<SlotLayout>,
    pub source_proficiency: Observation<ProficiencyTable>,
    pub equipment_eligibility: Observation<bool>,
    pub acquisition_availability: Observation<bool>,
    pub candidates: Vec<ItemScore>,
    pub assumptions: Vec<&'static str>,
}
fn invalid(message: &'static str) -> AppError {
    AppError::new(ErrorCode::InvalidInput, message, false)
}
pub(crate) fn score_only(
    request: &GearRequest,
    snapshot: &Envelope<CharacterSnapshot>,
    content: &dyn CatalogView,
    clock: &FreshnessContext,
) -> Result<Option<AuthoredScores>> {
    let Some(id) = &request.authored_model else {
        return Ok(None);
    };
    let catalog = content
        .content_v2()
        .ok_or_else(|| invalid("No authored model catalog is active"))?;
    let model = catalog
        .models
        .iter()
        .find(|m| &m.id == id)
        .ok_or_else(|| invalid("Selected authored model is absent from this revision"))?;
    if model.layout.value().is_some() && model.proficiency.value().is_some() {
        return Ok(None);
    }
    if request.candidates.len() > crate::gear::MAX_CANDIDATES {
        return Err(invalid("Gear input exceeds 128 candidates"));
    }
    snapshot
        .data_context()
        .ensure_compatible(&content.context())?;
    if snapshot.fixture != content.fixture()
        || request.snapshot_id != snapshot.payload.id
        || request.revision != content.reference().revision
    {
        return Err(invalid("Gear inputs changed"));
    }
    crate::authored_content::validate_model_selection(
        &request.class,
        model,
        &snapshot.payload,
        clock,
    )?;
    let mut candidates = Vec::new();
    let mut seen = BTreeSet::new();
    for input in &request.candidates {
        if !seen.insert(&input.item) {
            return Err(invalid("Duplicate gear candidate"));
        }
        let item = content
            .reference()
            .items
            .iter()
            .find(|i| i.key == input.item)
            .ok_or_else(|| invalid("Gear candidate is absent from active content"))?;
        let candidate = GearCandidate {
            item: item.clone(),
            category: Observation::unknown(),
        };
        candidates.push(ItemScore {
            item: input.item.clone(),
            score: score_item(&model.profile, &candidate)?,
        });
    }
    candidates.sort_by(|a, b| a.item.cmp(&b.item));
    let mut request = request.clone();
    request.model = model.profile.clone().into_inner();
    Ok(Some(AuthoredScores {
        mode: "authored_score_only_v1",
        request,
        fixture: snapshot.fixture,
        context: snapshot.data_context(),
        source_layout: model.layout.clone(),
        source_proficiency: model.proficiency.clone(),
        equipment_eligibility: Observation::unknown(),
        acquisition_availability: Observation::unknown(),
        candidates,
        assumptions: vec![
            "Individual scores use documented authored heuristics, not measured damage, healing or survivability.",
            "Equipment layout or proficiency is unavailable. Equipment eligibility, configurations, baseline gains and practical acquisition are not evaluated.",
            "Manual layout, proficiency, category and acquisition inputs cannot replace the unavailable source facts in this mode.",
            "Items remain in identity order, not recommendation order. Inspect score certainty and unmodelled effects before comparing subtotals.",
        ],
    }))
}
