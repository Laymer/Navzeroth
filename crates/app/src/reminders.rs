//! Snapshot-only reminder inspection and explicitly acknowledged presentation.
use crate::snapshots::{SnapshotApplication, context};
use azeroth_core::{
    entities::{ContextRule, DismissalPolicy},
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
}
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum ReminderAction {
    Acknowledge { rules: Vec<RuleId> },
    Dismiss { rule: RuleId },
    Restore { rule: RuleId },
    Preferences { enabled: bool },
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
    let content = store.active_content_revision(&ContentScope {
        context: key.context.clone(),
        fixture: key.fixture,
    })?;
    let rules = content
        .as_ref()
        .map(|c| c.envelope.payload.rules.clone())
        .unwrap_or_default();
    let revision = content.map(|c| c.envelope.payload.revision.clone());
    let state = store.reminder_state(key)?;
    let clock = context(now);
    let plan = if state.enabled {
        snapshot
            .as_ref()
            .map(|s| {
                ReminderEngine::default().plan(
                    &rules,
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
            plan,
            dismissed,
            rules: rules
                .iter()
                .map(|r| ReminderRuleSummary {
                    id: r.id.clone(),
                    text: r.result_template.clone(),
                    dismissal: r.dismissal,
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
            }
            store.save_reminder_state(key, &state)?;
            Ok(evaluate(store, key, &request.profile_id, now)?.0)
        })
    }
}
