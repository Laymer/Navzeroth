//! Snapshot-only reminder inspection and explicitly acknowledged presentation.
use crate::snapshots::{SnapshotApplication, context};
use azeroth_core::{
    entities::{Confidence, ContextRule, DismissalPolicy},
    error::{AppError, ErrorCode, Result},
    evaluation::{EvaluationContext, StalePolicy},
    predicate::RuleLimits,
    reminders::*,
    types::*,
};
use azeroth_storage::{ProfileKey, SnapshotStore, SqliteStore, content::ContentScope};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize)]
pub struct ReminderRuleSummary {
    pub id: RuleId,
    pub text: Text,
    pub dismissal: DismissalPolicy,
    pub delivery: RuleDelivery,
}
#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum RuleDelivery {
    Synthetic,
    Verified,
    AuthoredHeuristic,
    MissingEvidence,
    UnverifiedEvidence,
    ConflictingEvidence,
    SourceRevisionUnknown,
}
#[derive(Clone, Debug, Serialize)]
pub struct ReminderGap {
    pub category: &'static str,
    pub explanation: &'static str,
}
fn gaps() -> Vec<ReminderGap> {
    [
    ("repair", "Durability is not represented in imported snapshots. A repair warning needs supported durability observations and a verified nearby repair service."),
    ("bag_space", "Bag capacity is not represented in imported snapshots. An optional manual checklist makes no claim about available slots."),
    ("training", "Learned spells and verified training tables are absent. Inspect verified trainer reference data when available; no missing spell is inferred."),
    ("prerequisite", "Prerequisite advice needs an authored validated dependency and known quest state; inspect Party and Content when those inputs are available."),
    ("upgrade", "Upgrade ranking needs current gear, a verified acquisition source, route proximity and an explicit role model; inspect Gear for missing comparison inputs."),
].into_iter().map(|(category,explanation)|ReminderGap{category,explanation}).collect()
}
fn delivery(
    rule: &ContextRule,
    content: &dyn crate::catalog::CatalogView,
    fixture: bool,
) -> RuleDelivery {
    if fixture {
        return RuleDelivery::Synthetic;
    }
    if rule.evidence.is_empty() {
        return RuleDelivery::MissingEvidence;
    }
    let mut heuristic = 0;
    for id in &rule.evidence {
        let Some(e) = content
            .reference()
            .evidence
            .iter()
            .find(|e| e.id == *id && e.context == rule.context)
        else {
            return RuleDelivery::MissingEvidence;
        };
        if !e.conflicts.is_empty()
            || content
                .conflicts()
                .iter()
                .any(|c| c.preferred == *id || c.others.contains(id))
        {
            return RuleDelivery::ConflictingEvidence;
        }
        let Some(source) = content.sources().iter().find(|s| s.id == e.source) else {
            return RuleDelivery::MissingEvidence;
        };
        if project_authored_heuristic(e, source) {
            heuristic += 1;
        } else {
            if e.confidence != Confidence::VerifiedOnBuild {
                return RuleDelivery::UnverifiedEvidence;
            }
            if source.verified_revision.value() != Some(&content.reference().revision) {
                return RuleDelivery::SourceRevisionUnknown;
            }
        }
    }
    if heuristic == rule.evidence.len()
        && rule.result_template.as_str().len() <= 4096 - HEURISTIC_LABEL.len()
    {
        RuleDelivery::AuthoredHeuristic
    } else if heuristic == 0 {
        RuleDelivery::Verified
    } else {
        RuleDelivery::UnverifiedEvidence
    }
}
const HEURISTIC_LABEL: &str = "Authored heuristic: ";
/// This classifies declared original authoring, not target-game observations or
/// cryptographic admission. Distributed built-in bytes are separately checked
/// by bundle receipts; custom imports remain the user's supplied content.
fn project_authored_heuristic(
    evidence: &azeroth_core::entities::EvidenceRecord,
    source: &azeroth_core::sources::SourceRecord,
) -> bool {
    use azeroth_core::sources::{AccessMethod, Redistribution};
    source.id.as_str() == "navzeroth-authored-v1"
        && source.publisher.as_str() == "Navzeroth project (repository owner)"
        && source.access.value() == Some(&AccessMethod::Authored)
        && source
            .license
            .value()
            .is_some_and(|v| v.as_str() == "BSD-3-Clause (repository LICENSE)")
        && source
            .attribution
            .value()
            .is_some_and(|v| v.as_str() == "Original Navzeroth authoring, (c) 2026 Igor K")
        && source.verified_revision.value().is_some()
        && source.redistribution.value() == Some(&Redistribution::Permitted)
        && source.offline_usable.value() == Some(&true)
        && source.allowed_fields.value().is_some_and(|v| {
            v.len() == 4
                && [
                    "class_roles.models",
                    "dungeons.preparation_checklist",
                    "party.procedures",
                    "reminders.rules",
                ]
                .iter()
                .all(|field| v.iter().any(|f| f.as_str() == *field))
        })
        && source
            .transformations
            .value()
            .is_some_and(|v| v.len() == 1 && v[0].as_str() == "canonical_typed_serialization")
        && evidence.origin.is_none()
        && matches!(
            evidence.confidence,
            Confidence::PublisherDocumented | Confidence::Inferred
        )
}
#[derive(Clone, Debug, Serialize)]
pub struct ReminderView {
    pub profile_id: String,
    pub snapshot_id: Option<SnapshotId>,
    pub revision: Option<DatasetRevision>,
    pub fixture: bool,
    pub enabled: bool,
    pub plan: Option<ReminderPlan>,
    pub dismissed: Vec<RuleId>,
    pub rules: Vec<ReminderRuleSummary>,
    pub gaps: Vec<ReminderGap>,
    pub bag_checklist_enabled: bool,
    pub bag_checklist_checked: bool,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum ReminderAction {
    Acknowledge { rules: Vec<RuleId> },
    Dismiss { rule: RuleId },
    Restore { rule: RuleId },
    Preferences { enabled: bool },
    BagChecklist { enabled: bool, checked: bool },
}
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ReminderRequest {
    pub profile_id: String,
    pub snapshot_id: Option<SnapshotId>,
    pub revision: Option<DatasetRevision>,
    pub action: ReminderAction,
}
fn invalid(message: &'static str) -> AppError {
    AppError::new(ErrorCode::InvalidInput, message, false)
}
fn inputs(
    snapshot: &azeroth_core::entities::CharacterSnapshot,
    revision: Option<DatasetRevision>,
) -> ReminderInputs {
    ReminderInputs {
        scope: ReminderScope {
            session: snapshot.capture.session_id.clone(),
            ruleset: revision,
        },
        ..Default::default()
    }
}
fn evaluate(
    store: &SqliteStore,
    key: &ProfileKey,
    id: &str,
    now: Option<UtcMillis>,
) -> Result<(ReminderView, Vec<ContextRule>)> {
    let snapshot = store.current_snapshot(key)?;
    let content = store.active_catalog_revision(&ContentScope {
        context: key.context.clone(),
        fixture: key.fixture,
    })?;
    let rules = content
        .as_ref()
        .map(|c| c.reference().rules.clone())
        .unwrap_or_default();
    let deliveries = rules
        .iter()
        .map(|r| {
            (
                r.id.clone(),
                delivery(
                    r,
                    content.as_ref().expect("rules require content"),
                    key.fixture,
                ),
            )
        })
        .collect::<std::collections::BTreeMap<_, _>>();
    let eligible = rules
        .iter()
        .filter(|r| {
            matches!(
                deliveries.get(&r.id),
                Some(
                    RuleDelivery::Synthetic
                        | RuleDelivery::Verified
                        | RuleDelivery::AuthoredHeuristic
                )
            )
        })
        .map(|rule| {
            if deliveries[&rule.id] == RuleDelivery::AuthoredHeuristic {
                let mut data = rule.clone().into_inner();
                data.result_template = Text::try_from(format!(
                    "{HEURISTIC_LABEL}{}",
                    data.result_template.as_str()
                ))
                .expect("delivery bounds the authored heuristic label");
                ContextRule::try_new(data).expect("only bounded presentation text changed")
            } else {
                rule.clone()
            }
        })
        .collect::<Vec<_>>();
    let revision = content.as_ref().map(|c| c.reference().revision.clone());
    let state = store.reminder_state(key)?;
    let clock = context(now);
    let plan = if state.enabled {
        snapshot
            .as_ref()
            .map(|s| {
                ReminderEngine::default().plan(
                    &eligible,
                    &EvaluationContext {
                        snapshot: &s.payload,
                        party: None,
                        freshness: &clock,
                        stale_policy: StalePolicy::Reject,
                        limits: RuleLimits::default(),
                    },
                    &inputs(&s.payload, revision.clone()),
                    &state.history,
                )
            })
            .transpose()?
    } else {
        None
    };
    let dismissed = state
        .history
        .dismissals()
        .filter(|r| {
            snapshot.as_ref().is_none_or(|s| match r.policy {
                DismissalPolicy::Session => r
                    .session
                    .as_ref()
                    .zip(s.payload.capture.session_id.as_ref())
                    .is_none_or(|(a, b)| a == b),
                DismissalPolicy::UntilRulesetChanges => r
                    .ruleset
                    .as_ref()
                    .zip(revision.as_ref())
                    .is_none_or(|(a, b)| a == b),
                DismissalPolicy::Permanent => true,
            })
        })
        .map(|r| r.rule.clone())
        .collect();
    Ok((
        ReminderView {
            profile_id: id.to_owned(),
            snapshot_id: snapshot.map(|s| s.payload.id.clone()),
            revision,
            fixture: key.fixture,
            enabled: state.enabled,
            bag_checklist_enabled: state.bag_checklist_enabled,
            bag_checklist_checked: state.bag_checklist_checked,
            gaps: gaps(),
            plan,
            dismissed,
            rules: rules
                .iter()
                .map(|r| ReminderRuleSummary {
                    id: r.id.clone(),
                    text: r.result_template.clone(),
                    dismissal: r.dismissal,
                    delivery: deliveries[&r.id].clone(),
                })
                .collect(),
        },
        rules,
    ))
}
impl SnapshotApplication {
    pub fn reminder_view(&self, profile_id: &str, now: Option<UtcMillis>) -> Result<ReminderView> {
        self.with_profile_store_mut(profile_id, |store, key| {
            Ok(evaluate(store, key, profile_id, now)?.0)
        })
    }
    pub fn mutate_reminders(&self, text: &str, now: Option<UtcMillis>) -> Result<ReminderView> {
        if text.len() > 32768 {
            return Err(invalid("Reminder request exceeds capacity"));
        }
        let request: ReminderRequest =
            serde_json::from_str(text).map_err(|_| invalid("Invalid reminder request"))?;
        self.with_profile_store_mut(&request.profile_id, |store, key| {
            let (view, rules) = evaluate(store, key, &request.profile_id, now)?;
            if view.snapshot_id != request.snapshot_id || view.revision != request.revision {
                return Err(invalid(
                    "Reminder inputs changed; refresh before applying controls",
                ));
            }
            let mut state = store.reminder_state(key)?;
            let snapshot = store.current_snapshot(key)?;
            let scope = snapshot
                .as_ref()
                .map(|s| inputs(&s.payload, view.revision.clone()).scope)
                .unwrap_or_default();
            let clock = context(now);
            match request.action {
                ReminderAction::Acknowledge { rules } => {
                    if rules.len() > 1024 {
                        return Err(invalid("Too many reminder acknowledgments"));
                    }
                    let plan = view
                        .plan
                        .as_ref()
                        .ok_or_else(|| invalid("No reminder presentation is available"))?;
                    state.history.commit(plan, &rules, &clock)?;
                }
                ReminderAction::Dismiss { rule } => {
                    let rule = rules
                        .iter()
                        .find(|r| r.id == rule)
                        .ok_or_else(|| invalid("Reminder is not in active content"))?;
                    state.history.dismiss(rule, &scope, &clock)?;
                }
                ReminderAction::Restore { rule } => {
                    let rule = rules
                        .iter()
                        .find(|r| r.id == rule)
                        .ok_or_else(|| invalid("Reminder is not in active content"))?;
                    state.history.restore(rule);
                }
                ReminderAction::Preferences { enabled } => state.enabled = enabled,
                ReminderAction::BagChecklist { enabled, checked } => {
                    state.bag_checklist_enabled = enabled;
                    state.bag_checklist_checked = checked;
                }
            }
            store.save_reminder_state(key, &state)?;
            Ok(evaluate(store, key, &request.profile_id, now)?.0)
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use azeroth_storage::content::ContentRevision;
    use serde_json::{Value, json};
    fn namespace(value: &mut Value) {
        match value {
            Value::Object(map) => {
                for (key, value) in map {
                    if key == "source" && *value == json!("synthetic_fixture") {
                        *value = json!("reference_data");
                    } else if key == "component" && *value == json!("test_fixture") {
                        *value = json!("cli");
                    } else if key == "namespace" {
                        *value = json!("reference");
                    } else {
                        namespace(value);
                    }
                }
            }
            Value::Array(values) => {
                for value in values {
                    namespace(value);
                }
            }
            _ => {}
        }
    }
    fn bundle() -> Value {
        let mut value: Value = serde_json::from_str(include_str!(
            "../../../fixtures/desktop/content/demo-revision.json"
        ))
        .unwrap();
        namespace(&mut value);
        value["envelope"]["fixture"] = json!(false);
        value
    }
    fn check(value: &Value) -> RuleDelivery {
        let content: ContentRevision = serde_json::from_value(value.clone()).unwrap();
        delivery(&content.envelope.payload.rules[0], &content, false)
    }
    #[test]
    fn evidence_and_source_revision_gate_are_explicit() {
        let mut value = bundle();
        assert_eq!(check(&value), RuleDelivery::UnverifiedEvidence);
        value["envelope"]["payload"]["evidence"][0]["confidence"] = json!("verified_on_build");
        assert_eq!(check(&value), RuleDelivery::SourceRevisionUnknown);
        value["sources"][0]["verified_revision"] = json!({"status":"known","value":"demo-r1","source":"reference_data","clock_trust":"uncertain"});
        assert_eq!(check(&value), RuleDelivery::Verified);
        value["sources"][0]["verified_revision"]["value"] = json!("another-revision");
        assert_eq!(check(&value), RuleDelivery::SourceRevisionUnknown);
        value["envelope"]["payload"]["rules"][0]["evidence"] = json!([]);
        assert_eq!(check(&value), RuleDelivery::MissingEvidence);
    }
    #[test]
    fn conflicts_never_become_verified_advice() {
        let mut value = bundle();
        value["envelope"]["payload"]["evidence"][0]["confidence"] = json!("verified_on_build");
        let mut other = value["envelope"]["payload"]["evidence"][0].clone();
        other["id"] = json!("other");
        value["envelope"]["payload"]["evidence"]
            .as_array_mut()
            .unwrap()
            .push(other);
        value["envelope"]["payload"]["evidence"][0]["conflicts"] = json!(["other"]);
        assert_eq!(check(&value), RuleDelivery::ConflictingEvidence);
    }
    fn authored() -> Value {
        let content: Value = serde_json::from_str(include_str!(
            "../../../content/authored/slice-v1/catalog.json"
        ))
        .unwrap();
        let sources: Value = serde_json::from_str(include_str!(
            "../../../content/authored/slice-v1/sources.json"
        ))
        .unwrap();
        json!({"content":content,"sources":sources["sources"],"conflicts":[]})
    }
    fn authored_delivery(value: &Value) -> RuleDelivery {
        let content =
            azeroth_core::content_v2::decode(&serde_json::to_vec(&value["content"]).unwrap())
                .unwrap();
        let rule = content.reference.payload.rules[0].clone();
        let catalog = azeroth_storage::content::ActiveContentRevision::V2(std::sync::Arc::new(
            azeroth_storage::content::ContentRevisionV2 {
                content,
                sources: serde_json::from_value(value["sources"].clone()).unwrap(),
                conflicts: serde_json::from_value(value["conflicts"].clone()).unwrap(),
            },
        ));
        delivery(&rule, &catalog, false)
    }
    #[test]
    fn original_authoring_is_heuristic_and_exact_source_policy_is_required() {
        let original = authored();
        assert_eq!(
            authored_delivery(&original),
            RuleDelivery::AuthoredHeuristic
        );
        for (field, value) in [
            ("publisher", json!("Other author")),
            ("access", json!({"status":"unknown"})),
            ("license", json!({"status":"known","value":"Other license"})),
            (
                "attribution",
                json!({"status":"known","value":"Other attribution"}),
            ),
            ("verified_revision", json!({"status":"unknown"})),
            (
                "redistribution",
                json!({"status":"known","value":"not_permitted"}),
            ),
            ("offline_usable", json!({"status":"known","value":false})),
            (
                "allowed_fields",
                json!({"status":"known","value":["other.field"]}),
            ),
            (
                "transformations",
                json!({"status":"known","value":["other_transform"]}),
            ),
        ] {
            let mut v = original.clone();
            v["sources"][0][field] = value;
            assert_eq!(
                authored_delivery(&v),
                RuleDelivery::UnverifiedEvidence,
                "{field}"
            );
        }
        let source: azeroth_core::sources::SourceRecord =
            serde_json::from_value(original["sources"][0].clone()).unwrap();
        for (confidence, allowed) in [
            ("cross_edition_unverified", false),
            ("disputed_or_stale", false),
            ("limited_sample", false),
            ("inferred", true),
        ] {
            let mut e = original["content"]["reference"]["payload"]["evidence"][0].clone();
            e["confidence"] = json!(confidence);
            let e: azeroth_core::entities::EvidenceRecord = serde_json::from_value(e).unwrap();
            assert_eq!(project_authored_heuristic(&e, &source), allowed);
        }
        let mut v = original.clone();
        v["content"]["reference"]["payload"]["rules"][0]["result_template"] =
            json!("x".repeat(4096));
        assert_eq!(authored_delivery(&v), RuleDelivery::UnverifiedEvidence);
    }
    #[test]
    fn original_reminder_provenance_is_separate_from_dungeon_evidence() {
        let v = authored();
        assert_eq!(
            v["content"]["reference"]["payload"]["evidence"]
                .as_array()
                .unwrap()
                .iter()
                .find(|e| e["id"] == "ev-authored-reminders-r4")
                .unwrap()["observed_at_utc_ms"]["status"],
            "unknown"
        );
        let rules = v["content"]["reference"]["payload"]["rules"]
            .as_array()
            .unwrap();
        assert!(rules.iter().all(|r| {
            !r["evidence"]
                .as_array()
                .unwrap()
                .iter()
                .any(|id| id == "ev-authored-v1")
        }));
        assert!(
            v["content"]["reference"]["payload"]["evidence"]
                .as_array()
                .unwrap()
                .iter()
                .any(|e| e["id"] == "ev-authored-v1")
        );
    }
}
