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
    pub imported_role: Observation<RoleId>,
    pub role_assumption: Option<RoleId>,
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
    if let Some(role) = &request.score_role_assumption {
        if role != &model.profile.role {
            return Err(invalid(
                "Declared score role does not match the selected model role",
            ));
        }
        crate::authored_content::validate_model_identity_and_level(
            &request.class,
            model,
            &snapshot.payload,
            clock,
        )?;
        validate_declared_role(model, &snapshot.payload, clock, role)?;
    } else {
        crate::authored_content::validate_model_selection(
            &request.class,
            model,
            &snapshot.payload,
            clock,
        )?;
    }
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
        role_assumption: request.score_role_assumption.clone(),
        request,
        fixture: snapshot.fixture,
        context: snapshot.data_context(),
        imported_role: snapshot.payload.role.clone(),
        source_layout: model.layout.clone(),
        source_proficiency: model.proficiency.clone(),
        equipment_eligibility: Observation::unknown(),
        acquisition_availability: Observation::unknown(),
        candidates,
        assumptions: vec![
            "Individual scores use documented authored heuristics, not measured damage, healing or survivability.",
            "Any declared role is a separate explicit assumption for these scores only. The imported role observation is retained unchanged; neither role selection nor a score establishes equipment eligibility.",
            "Equipment layout or proficiency is unavailable. Equipment eligibility, configurations, baseline gains and practical acquisition are not evaluated.",
            "Manual layout, proficiency, category and acquisition inputs cannot replace the unavailable source facts in this mode.",
            "Items remain in identity order, not recommendation order. Inspect score certainty and unmodelled effects before comparing subtotals.",
        ],
    }))
}

/// Replace only the closed RoleIs leaves in a separate arithmetic context.
/// No snapshot is changed and no observation is manufactured or clock-stamped.
fn declared_predicate(
    node: &azeroth_core::predicate::PredicateNode,
    role: &RoleId,
) -> Result<azeroth_core::predicate::PredicateNode> {
    use azeroth_core::predicate::{Field, PredicateNode as P};
    Ok(match node {
        P::RoleIs { role: expected } => {
            if expected == role {
                P::All { predicates: vec![] }
            } else {
                P::Any { predicates: vec![] }
            }
        }
        P::All { predicates } => P::All {
            predicates: predicates
                .iter()
                .map(|p| declared_predicate(p, role))
                .collect::<Result<_>>()?,
        },
        P::Any { predicates } => P::Any {
            predicates: predicates
                .iter()
                .map(|p| declared_predicate(p, role))
                .collect::<Result<_>>()?,
        },
        P::Not { predicate } => P::Not {
            predicate: Box::new(declared_predicate(predicate, role)?),
        },
        P::Known { field: Field::Role }
        | P::Equals {
            field: Field::Role, ..
        }
        | P::GreaterThanOrEqual {
            field: Field::Role, ..
        } => {
            return Err(invalid(
                "Declared score roles cannot replace general captured-role checks",
            ));
        }
        other => other.clone(),
    })
}
fn validate_declared_role(
    model: &azeroth_core::content_v2::ClassRoleModel,
    snapshot: &CharacterSnapshot,
    clock: &FreshnessContext,
    role: &RoleId,
) -> Result<()> {
    use azeroth_core::{
        evaluation::{EvaluationContext, StalePolicy, Truth, evaluate},
        predicate::{Predicate, RuleLimits},
    };
    let context = EvaluationContext {
        snapshot,
        party: None,
        freshness: clock,
        stale_policy: StalePolicy::Reject,
        limits: RuleLimits::default(),
    };
    let applicable = model
        .applicability
        .value()
        .ok_or_else(|| invalid("Authored model applicability is unavailable"))?;
    if evaluate(
        &Predicate::try_new(declared_predicate(applicable, role)?)?,
        &context,
    )?
    .truth
        != Truth::True
    {
        return Err(invalid(
            "Authored model applicability is not established for the declared role",
        ));
    }
    let exclusions = model
        .exclusions
        .value()
        .ok_or_else(|| invalid("Authored model exclusions are unavailable"))?;
    for predicate in exclusions {
        if evaluate(
            &Predicate::try_new(declared_predicate(predicate, role)?)?,
            &context,
        )?
        .truth
            != Truth::False
        {
            return Err(invalid(
                "Authored model exclusion is true or unknown for the declared role",
            ));
        }
    }
    Ok(())
}
