//! Apply authored facts without turning unavailable fields into permissive defaults.
use azeroth_core::{
    content_v2::{ClassRoleModel, Cost, LootEntitlement, ReferenceContent, ResetRule},
    eligibility::{AccessRequirements, ExclusiveGroup, ExclusiveGroupData},
    entities::{
        AcquisitionSource, CharacterSnapshot, CompletionSemantics, ItemDefinition, QuestDefinition,
    },
    error::{AppError, ErrorCode, Result},
    evaluation::{EvaluationContext, StalePolicy, Truth, evaluate},
    observation::{
        Freshness, FreshnessContext, Observation, ObservationMetadata, ObservationStatus, freshness,
    },
    predicate::{Predicate, PredicateNode, RuleLimits},
    types::*,
};

fn invalid(message: &'static str) -> AppError {
    AppError::new(ErrorCode::InvalidInput, message, false)
}
fn unavailable<T, U>(value: &Observation<T>) -> Result<Observation<U>> {
    Observation::unavailable(value.status(), value.metadata().clone())
}
pub(crate) fn conjunction(values: &[&Observation<Predicate>]) -> Result<Observation<Predicate>> {
    // Keep the strongest unavailable state and its original metadata. False remains
    // an actual predicate; no absent restriction is silently treated as true.
    for status in [
        ObservationStatus::Restricted,
        ObservationStatus::Unsupported,
        ObservationStatus::Unknown,
        ObservationStatus::NotApplicable,
    ] {
        if let Some(value) = values.iter().find(|v| v.status() == status) {
            return unavailable(value);
        }
    }
    let predicates = values
        .iter()
        .map(|v| v.value().expect("known predicate").clone().into_inner())
        .collect();
    Observation::known(
        Predicate::try_new(PredicateNode::All { predicates })?,
        ObservationMetadata::default(),
    )
}
fn false_condition() -> Result<Observation<Predicate>> {
    Observation::known(
        Predicate::try_new(PredicateNode::Any { predicates: vec![] })?,
        ObservationMetadata::default(),
    )
}
pub(crate) fn item_facts(
    item: &ItemDefinition,
    content: &ReferenceContent,
) -> Result<(ItemDefinition, Observation<SourceId>)> {
    let mut item = item.clone().into_inner();
    let Some(facts) = content.equipment.iter().find(|f| f.item == item.key) else {
        item.requirements = Observation::unknown();
        return Ok((ItemDefinition::try_new(item)?, Observation::unknown()));
    };
    item.requirements = conjunction(&[&item.requirements, &facts.configuration])?;
    Ok((ItemDefinition::try_new(item)?, facts.category.clone()))
}
pub(crate) fn acquisition_facts(
    source: &AcquisitionSource,
    target: &ItemKey,
    content: &ReferenceContent,
) -> Result<AcquisitionSource> {
    let mut source = source.clone().into_inner();
    let Some(facts) = content
        .acquisition_facts
        .iter()
        .find(|f| f.acquisition == source.id)
    else {
        source.conditions = Observation::unknown();
        source.probability = Observation::unknown();
        source.cost = Observation::unknown();
        return AcquisitionSource::try_new(source);
    };
    source.conditions = conjunction(&[
        &source.conditions,
        &facts.availability,
        &facts.prerequisites,
    ])?;
    match facts.rewards.value() {
        Some(rewards) => {
            if !rewards.iter().any(|r| &r.item == target && r.quantity > 0) {
                source.conditions = false_condition()?;
            }
            // A selected item does not prove entitlement to a mutually exclusive reward.
            if rewards
                .iter()
                .any(|r| &r.item == target && r.choice_group.is_some())
            {
                source.probability = Observation::unknown();
            }
        }
        None => source.conditions = unavailable(&facts.rewards)?,
    }
    if !matches!(facts.entitlement.value(), Some(LootEntitlement::Individual)) {
        source.probability = if facts.entitlement.status() == ObservationStatus::Known {
            Observation::unknown()
        } else {
            unavailable(&facts.entitlement)?
        };
    }
    source.cost = match facts.costs.value() {
        Some(costs) => {
            let mut total = Some(0u64);
            for cost in costs {
                total = match (total, cost) {
                    (Some(total), Cost::Copper { amount }) => total.checked_add(amount.get()),
                    _ => None, // Currency/item balances are not present in snapshot v1.
                };
            }
            match total {
                Some(total) => {
                    Observation::known(Copper::new(total), facts.costs.metadata().clone())?
                }
                None => Observation::unknown(),
            }
        }
        None => unavailable(&facts.costs)?,
    };
    AcquisitionSource::try_new(source)
}

pub(crate) struct QuestBinding {
    pub definition: QuestDefinition,
    pub access: Option<AccessRequirements>,
    pub exclusive_groups: Vec<ExclusiveGroup>,
}
pub(crate) fn quest_facts(
    definition: &QuestDefinition,
    content: &ReferenceContent,
) -> Result<QuestBinding> {
    let mut definition = definition.clone().into_inner();
    let mut access = None;
    let mut groups = Vec::new();
    if let Some(facts) = content
        .quest_facts
        .iter()
        .find(|q| q.quest == definition.id)
    {
        definition.eligibility = conjunction(&[&definition.eligibility, &facts.access])?;
        if let Some(value) = facts.access_requirements.value() {
            access = Some(value.clone());
        } else {
            definition.eligibility = conjunction(&[
                &definition.eligibility,
                &unavailable(&facts.access_requirements)?,
            ])?;
        }
        if let Some(values) = facts.exclusive_groups.value() {
            for group in values {
                groups.push(ExclusiveGroup::try_new(ExclusiveGroupData {
                    quests: group.quests.iter().cloned().collect(),
                    taken_when: group.taken_when,
                })?);
            }
        } else {
            definition.eligibility = conjunction(&[
                &definition.eligibility,
                &unavailable(&facts.exclusive_groups)?,
            ])?;
        }
        definition.completion = match facts.reset.value() {
            Some(value) => Observation::known(
                match value {
                    ResetRule::Never => CompletionSemantics::Unrepeatable,
                    ResetRule::Repeatable => CompletionSemantics::Repeatable,
                    ResetRule::Timed { .. } | ResetRule::ServerReset { .. } => {
                        CompletionSemantics::ResetBound
                    }
                },
                facts.reset.metadata().clone(),
            )?,
            None => unavailable(&facts.reset)?,
        };
    } else {
        definition.eligibility = Observation::unknown();
        definition.completion = Observation::unknown();
    }
    Ok(QuestBinding {
        definition: QuestDefinition::try_new(definition)?,
        access,
        exclusive_groups: groups,
    })
}

pub(crate) fn apply_model(
    request: &mut crate::gear::GearRequest,
    model: &ClassRoleModel,
    snapshot: &CharacterSnapshot,
    clock: &FreshnessContext,
) -> Result<()> {
    // Class is a declared assumption in the existing request, not a captured fact.
    if request.class.value() != Some(&model.class) {
        return Err(invalid(
            "Authored model requires an explicit matching class assumption",
        ));
    }
    if !matches!(
        freshness(snapshot.level.metadata(), clock),
        Freshness::Fresh { .. }
    ) {
        return Err(invalid("Authored model requires a fresh observed level"));
    }
    let level = snapshot
        .level
        .value()
        .ok_or_else(|| invalid("Authored model level is unavailable"))?;
    if level < &model.minimum_level || level > &model.maximum_level {
        return Err(invalid("Authored model does not cover this level"));
    }
    let evaluation = EvaluationContext {
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
    if evaluate(applicable, &evaluation)?.truth != Truth::True {
        return Err(invalid("Authored model applicability is not established"));
    }
    let exclusions = model
        .exclusions
        .value()
        .ok_or_else(|| invalid("Authored model exclusions are unavailable"))?;
    for predicate in exclusions {
        if evaluate(predicate, &evaluation)?.truth != Truth::False {
            return Err(invalid("Authored model exclusion is true or unknown"));
        }
    }
    request.model = model.profile.clone().into_inner();
    request.layout = model
        .layout
        .value()
        .ok_or_else(|| invalid("Authored slot layout is unavailable"))?
        .clone()
        .into_inner();
    request.proficiency = model
        .proficiency
        .value()
        .ok_or_else(|| invalid("Authored proficiency is unavailable"))?
        .clone()
        .into_inner();
    Ok(())
}

use crate::{dungeon_preparation::PreparationSubject, snapshots::SnapshotApplication};
use azeroth_storage::{
    SqliteStore,
    content::{ActiveContentRevision, ContentScope},
};
use serde::{Deserialize, Serialize};
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct AuthoredSelection {
    pub revision: DatasetRevision,
    pub id: SourceId,
}
#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CatalogKind {
    Models,
    Dungeons,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct CatalogRequest {
    subject: PreparationSubject,
    kind: CatalogKind,
    revision: Option<DatasetRevision>,
    catalog_sha256: Option<azeroth_core::types::Sha256Hex>,
    offset: usize,
}
#[derive(Serialize)]
pub struct AuthoredChoice {
    pub id: SourceId,
    pub label: String,
    pub evidence: Vec<EvidenceId>,
}
#[derive(Serialize)]
pub struct AuthoredCatalog {
    pub revision: Option<DatasetRevision>,
    pub catalog_sha256: Option<azeroth_core::types::Sha256Hex>,
    pub records: Vec<AuthoredChoice>,
    pub total: usize,
    pub next_offset: Option<usize>,
}
pub(crate) fn selected_catalog(
    store: &SqliteStore,
    subject: &PreparationSubject,
) -> Result<Option<ActiveContentRevision>> {
    let scope = match subject {
        PreparationSubject::Character { profile_id } => {
            let key = crate::snapshots::resolve(store, profile_id)?;
            ContentScope {
                context: key.context,
                fixture: key.fixture,
            }
        }
        PreparationSubject::Party { party_id } => {
            let party = store
                .party(party_id)?
                .ok_or_else(|| invalid("No such party"))?;
            let key = party
                .participants
                .first()
                .ok_or_else(|| invalid("Party has no participants"))?;
            ContentScope {
                context: key.context.clone(),
                fixture: key.fixture,
            }
        }
    };
    store.active_catalog_revision(&scope)
}
impl SnapshotApplication {
    pub fn authored_catalog(&self, text: &str) -> Result<AuthoredCatalog> {
        if text.len() > 4096 {
            return Err(invalid("Authored catalog request exceeds bounds"));
        }
        let request: CatalogRequest =
            serde_json::from_str(text).map_err(|_| invalid("Invalid authored catalog request"))?;
        use azeroth_core::catalog::{Collection, MAX_PAGE_ROWS, PageRequest};
        if request.offset > 0 && (request.revision.is_none() || request.catalog_sha256.is_none()) {
            return Err(invalid(
                "Subsequent pages require the exact revision and hash",
            ));
        }
        self.with_store_mut(|store| {
            let content = selected_catalog(store, &request.subject)?;
            let Some(content) = content else {
                if request.offset != 0
                    || request.revision.is_some()
                    || request.catalog_sha256.is_some()
                {
                    return Err(invalid("Authored catalog changed; refresh selection"));
                }
                return Ok(AuthoredCatalog {
                    revision: None,
                    catalog_sha256: None,
                    records: vec![],
                    total: 0,
                    next_offset: None,
                });
            };
            let collection = match request.kind {
                CatalogKind::Models => Collection::Models,
                CatalogKind::Dungeons => Collection::Dungeons,
            };
            let binding = match (request.revision, request.catalog_sha256) {
                (Some(revision), Some(catalog_sha256)) => Some(PageRequest {
                    revision,
                    catalog_sha256,
                    collection,
                    offset: request.offset,
                    limit: MAX_PAGE_ROWS,
                }),
                (None, None) => None,
                _ => return Err(invalid("Catalog binding requires both revision and hash")),
            };
            let page = crate::catalog_pages::page(store, &content, collection, binding.as_ref())?;
            let records = page
                .records
                .into_iter()
                .map(|record| match request.kind {
                    CatalogKind::Models => {
                        let m: ClassRoleModel = serde_json::from_value(record)
                            .map_err(|_| invalid("Invalid model page"))?;
                        Ok(AuthoredChoice {
                            id: m.id,
                            label: format!(
                                "{} / {} / levels {}–{}",
                                m.class.as_str(),
                                m.profile.role.as_str(),
                                m.minimum_level.get(),
                                m.maximum_level.get()
                            ),
                            evidence: m.evidence,
                        })
                    }
                    CatalogKind::Dungeons => {
                        let d: azeroth_core::content_v2::Dungeon =
                            serde_json::from_value(record)
                                .map_err(|_| invalid("Invalid dungeon page"))?;
                        Ok(AuthoredChoice {
                            id: d.id,
                            label: d.title.as_str().into(),
                            evidence: d.evidence,
                        })
                    }
                })
                .collect::<Result<Vec<_>>>()?;
            Ok(AuthoredCatalog {
                revision: Some(page.page.revision),
                catalog_sha256: Some(page.page.catalog_sha256),
                records,
                total: page.page.total,
                next_offset: page.page.next_offset,
            })
        })
    }
}

pub(crate) fn dungeon_package(
    selection: &AuthoredSelection,
    content: &ActiveContentRevision,
) -> Result<crate::dungeon_preparation::LocalPreparationPackage> {
    use crate::dungeon_preparation::{
        CheckCategory, Declared, LocalPreparationPackage, PreparationCheck, RewardReference,
    };
    if &selection.revision != content.revision() {
        return Err(invalid(
            "Authored dungeon revision changed; refresh selection",
        ));
    }
    let authored = content
        .content_v2()
        .ok_or_else(|| invalid("No authored dungeon catalog is active"))?;
    let dungeon = authored
        .dungeons
        .iter()
        .find(|d| d.id == selection.id)
        .ok_or_else(|| invalid("Selected dungeon is absent from this revision"))?;
    fn declared<T>(observation: Observation<T>, evidence: &[EvidenceId]) -> Declared<T> {
        Declared {
            observation,
            evidence: evidence.to_vec(),
        }
    }
    let rewards = if let Some(ids) = dungeon.rewards.value() {
        let mut rewards = Vec::new();
        for id in ids {
            let source = authored
                .reference
                .payload
                .acquisitions
                .iter()
                .find(|a| &a.id == id)
                .ok_or_else(|| invalid("Dungeon reward does not resolve"))?;
            for item in &source.items {
                if rewards.len() == crate::dungeon_preparation::MAX_REFERENCE_RECORDS {
                    return Err(invalid(
                        "Dungeon reward references exceed preparation bounds",
                    ));
                }
                rewards.push(RewardReference {
                    item: item.clone(),
                    source: id.clone(),
                    declared_role: Observation::unknown(),
                });
            }
        }
        Observation::known(rewards, dungeon.rewards.metadata().clone())?
    } else {
        unavailable(&dungeon.rewards)?
    };
    let checks = if let Some(checks) = dungeon.checklist.value() {
        Observation::known(
            checks
                .iter()
                .map(|c| PreparationCheck {
                    id: c.id.clone(),
                    label: c.label.clone(),
                    category: CheckCategory::BeforeEntry,
                    requirement: declared(c.condition.clone(), &c.evidence),
                })
                .collect(),
            dungeon.checklist.metadata().clone(),
        )?
    } else {
        unavailable(&dungeon.checklist)?
    };
    Ok(LocalPreparationPackage {
        schema_version: 2,
        id: dungeon.id.clone(),
        title: dungeon.title.clone(),
        context: content.context(),
        fixture: content.fixture(),
        revision: content.revision().clone(),
        entrance: declared(dungeon.entrance.clone(), &dungeon.evidence),
        access: declared(dungeon.access.clone(), &dungeon.evidence),
        quests: declared(dungeon.preparation_quests.clone(), &dungeon.evidence),
        rewards: declared(rewards, &dungeon.evidence),
        checks: declared(checks, &dungeon.evidence),
        formation_ms: declared(Observation::unknown(), &[]),
        run_ms: declared(Observation::unknown(), &[]),
        before_departure: declared(Observation::unknown(), &[]),
        before_entry: declared(Observation::unknown(), &[]),
        encounter_notes: declared(Observation::unknown(), &[]),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    fn content() -> ReferenceContent {
        azeroth_core::content_v2::decode(include_bytes!(
            "../../../fixtures/contracts/v2/reference-content.json"
        ))
        .unwrap()
    }
    #[test]
    fn missing_item_facts_and_unobservable_costs_remain_unknown() {
        let mut catalog = content();
        let item = catalog.reference.payload.items[0].clone();
        catalog.equipment.clear();
        let (item, category) = item_facts(&item, &catalog).unwrap();
        assert_eq!(item.requirements.status(), ObservationStatus::Unknown);
        assert_eq!(category.status(), ObservationStatus::Unknown);
        let source = catalog.reference.payload.acquisitions[0].clone();
        let target = source.items[0].clone();
        catalog.acquisition_facts[0].costs = Observation::known(
            vec![Cost::Item {
                item: target.clone(),
                quantity: 1,
            }],
            ObservationMetadata::default(),
        )
        .unwrap();
        catalog.acquisition_facts[0].entitlement =
            Observation::known(LootEntitlement::SharedRoll, ObservationMetadata::default())
                .unwrap();
        let effective = acquisition_facts(&source, &target, &catalog).unwrap();
        assert_eq!(effective.cost.status(), ObservationStatus::Unknown);
        assert_eq!(effective.probability.status(), ObservationStatus::Unknown);
        catalog.acquisition_facts[0].costs = Observation::known(
            vec![
                Cost::Copper {
                    amount: Copper::new(u64::MAX),
                },
                Cost::Copper {
                    amount: Copper::new(1),
                },
            ],
            ObservationMetadata::default(),
        )
        .unwrap();
        assert_eq!(
            acquisition_facts(&source, &target, &catalog)
                .unwrap()
                .cost
                .status(),
            ObservationStatus::Unknown
        );
    }
    #[test]
    fn missing_quest_restrictions_never_become_unrestricted() {
        let mut catalog = content();
        let quest = catalog.reference.payload.quests[0].clone();
        catalog.quest_facts.clear();
        let binding = quest_facts(&quest, &catalog).unwrap();
        assert_eq!(
            binding.definition.eligibility.status(),
            ObservationStatus::Unknown
        );
        assert_eq!(
            binding.definition.completion.status(),
            ObservationStatus::Unknown
        );
        assert!(binding.access.is_none());
    }
    #[test]
    fn zero_rewards_and_unavailable_entitlement_remain_distinct() {
        let mut catalog = content();
        let source = catalog.reference.payload.acquisitions[0].clone();
        let target = source.items[0].clone();
        catalog.acquisition_facts[0].rewards = serde_json::from_value(
            json!({"status":"known","value":[{"item":target,"quantity":0,"choice_group":null}]}),
        )
        .unwrap();
        catalog.acquisition_facts[0].costs =
            Observation::known(vec![], ObservationMetadata::default()).unwrap();
        catalog.acquisition_facts[0].entitlement = Observation::unavailable(
            ObservationStatus::Restricted,
            ObservationMetadata::default(),
        )
        .unwrap();
        let actual = acquisition_facts(&source, &target, &catalog).unwrap();
        assert_eq!(
            actual.conditions.value().unwrap(),
            false_condition().unwrap().value().unwrap()
        );
        assert_eq!(actual.cost.value().unwrap().get(), 0);
        assert_eq!(actual.probability.status(), ObservationStatus::Restricted);
        catalog.acquisition_facts.clear();
        let missing = acquisition_facts(&source, &target, &catalog).unwrap();
        assert_eq!(missing.cost.status(), ObservationStatus::Unknown);
        assert_eq!(missing.conditions.status(), ObservationStatus::Unknown);
    }
}
