//! Profile-bound projections of declared quest stages. A stage is an identity,
//! not proof of objective details, a count, completion, or eligibility.
use crate::catalog::CatalogView;
use azeroth_core::{
    entities::{Objective, ObjectiveData, ObjectiveTarget},
    error::{AppError, ErrorCode, Result},
    observation::Observation,
    types::{CharacterId, ObjectiveId, QuestId},
};
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;

const MAX_PROJECTED_OBJECTIVES: usize = 131_072;

pub(crate) fn stage_identity(quest: &QuestId, stage: &ObjectiveId) -> String {
    format!(
        "quest-stage-{:x}",
        Sha256::digest(format!("{}\0{}", quest.as_str(), stage.as_str()).as_bytes())
    )
}

pub(crate) fn objectives(
    content: &dyn CatalogView,
    character: &CharacterId,
) -> Result<Vec<Objective>> {
    let data = content.reference();
    let mut result = data.objectives.clone();
    let represented: BTreeSet<_> = data
        .objectives
        .iter()
        .flat_map(|o| &o.targets)
        .filter_map(|t| {
            if let ObjectiveTarget::QuestStage { quest, stage } = t {
                Some((quest.clone(), stage.clone()))
            } else {
                None
            }
        })
        .collect();
    for quest in &data.quests {
        // Empty evidence cannot establish a source-backed projection.
        if quest.evidence.is_empty() {
            continue;
        }
        let bound = content
            .content_v2()
            .map(|v2| crate::authored_content::quest_facts(quest, v2))
            .transpose()?;
        let definition = bound.as_ref().map_or(quest, |b| &b.definition);
        let mut requirements = crate::authored_content::conjunction(&[
            &definition.eligibility,
            &definition.prerequisites,
        ])?;
        if requirements.value().is_some()
            && bound
                .as_ref()
                .and_then(|b| b.access.as_ref())
                .is_some_and(|a| a.classes.is_some() || a.races.is_some() || a.factions.is_some())
        {
            // The planner snapshot has no race/class/faction facts. Preserve the
            // unresolved restriction instead of treating it as satisfied.
            requirements = Observation::unknown();
        }
        // Not-applicable mixed with other predicates is not an affirmative.
        if requirements.status() == azeroth_core::observation::ObservationStatus::NotApplicable {
            requirements = Observation::unknown();
        }
        for stage in &quest.stages {
            if represented.contains(&(quest.id.clone(), stage.clone())) {
                continue;
            }
            if result.len() >= MAX_PROJECTED_OBJECTIVES {
                return Err(AppError::new(
                    ErrorCode::InvalidInput,
                    "Projected objectives exceed planner catalog limit",
                    false,
                ));
            }
            let id = ObjectiveId::new(stage_identity(&quest.id, stage))?;
            result.push(Objective::try_new(ObjectiveData {
                id,
                context: quest.context.clone(),
                participants: vec![character.clone()],
                targets: vec![ObjectiveTarget::QuestStage {
                    quest: quest.id.clone(),
                    stage: stage.clone(),
                }],
                requirements: requirements.clone(),
                expected_ms: Observation::unknown(),
                evidence: quest.evidence.clone(),
            })?);
        }
    }
    result.sort_by(|a, b| a.id.cmp(&b.id));
    Ok(result)
}

pub(crate) fn page(
    store: &azeroth_storage::SqliteStore,
    content: &azeroth_storage::content::ActiveContentRevision,
    character: &CharacterId,
    request: Option<&azeroth_core::catalog::PageRequest>,
) -> Result<crate::catalog_pages::RecordPage> {
    use azeroth_core::catalog::{Collection, MAX_PAGE_ROWS};
    let values = objectives(content, character)?;
    let hash = crate::catalog_pages::page(store, content, Collection::Objectives, None)?
        .page
        .catalog_sha256;
    let offset = request.map_or(0, |r| r.offset);
    if offset > values.len()
        || request.is_some_and(|r| {
            r.revision != content.reference().revision
                || r.catalog_sha256 != hash
                || r.collection != Collection::Objectives
                || r.limit != MAX_PAGE_ROWS
        })
    {
        return Err(AppError::new(
            ErrorCode::InvalidInput,
            "Projected objective page changed or offset is invalid",
            false,
        ));
    }
    let end = offset.saturating_add(MAX_PAGE_ROWS).min(values.len());
    Ok(crate::catalog_pages::RecordPage {
        page: crate::catalog_pages::PageInfo {
            revision: content.reference().revision.clone(),
            catalog_sha256: hash,
            collection: Collection::Objectives,
            offset,
            total: values.len(),
            next_offset: (end < values.len()).then_some(end),
        },
        records: values[offset..end]
            .iter()
            .map(serde_json::to_value)
            .collect::<std::result::Result<_, _>>()
            .map_err(|_| {
                AppError::new(
                    ErrorCode::InvalidInput,
                    "Projected objective cannot serialize",
                    false,
                )
            })?,
    })
}
