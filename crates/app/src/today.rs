//! Explicit local intent and guarded saved advice. Reads never optimize or import.
use crate::{
    planning_job_service::{PublicationGuard, advanced_now},
    planning_jobs::PlanningJobs,
    snapshots::{SnapshotApplication, context, profile_id},
};
use azeroth_core::{
    error::{AppError, ErrorCode, Result},
    observation::{Freshness, freshness},
    types::{SnapshotId, UtcMillis},
};
pub use azeroth_storage::today::SettingsRecord;
use azeroth_storage::{SnapshotStore, SqliteStore, today::*};
use serde::{Deserialize, Serialize};
use std::time::Instant;
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SettingsUpdate {
    pub schema_version: u32,
    pub expected_revision: i64,
    pub settings: Settings,
}
#[derive(Clone, Debug, Serialize)]
pub struct TodayMember {
    pub profile_id: String,
    pub character: String,
    pub snapshot_id: Option<SnapshotId>,
    pub freshness: Option<Freshness>,
}
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PlanState {
    Empty,
    Historical,
    Current,
}
#[derive(Clone, Debug, Serialize)]
pub struct CurrentPlanView {
    pub revision: i64,
    pub state: PlanState,
    pub subject: Option<Subject>,
    pub fixture: Option<bool>,
    pub context: Option<azeroth_core::types::DataContext>,
    pub saved_at_utc_ms: Option<UtcMillis>,
    pub evaluated_at_utc_ms: Option<UtcMillis>,
    pub age_ms: Option<u64>,
    pub valid_from_utc_ms: Option<UtcMillis>,
    pub valid_until_utc_ms: Option<UtcMillis>,
    pub original_step_count: usize,
    pub summary_truncated: bool,
    pub reason: Option<&'static str>,
    pub summary: Option<PlanSummary>,
}
#[derive(Clone, Debug, Serialize)]
pub struct TodayView {
    pub schema_version: u32,
    pub settings_revision: i64,
    pub selected_subject: Option<Subject>,
    pub members: Vec<TodayMember>,
    pub selection_available: bool,
    pub manual_goal: Option<azeroth_core::types::SourceId>,
    pub current_plan: CurrentPlanView,
}
fn invalid(message: &'static str) -> AppError {
    AppError::new(ErrorCode::InvalidInput, message, false)
}
pub(crate) fn plan_view(
    store: &SqliteStore,
    now: impl FnMut() -> Option<UtcMillis>,
) -> Result<CurrentPlanView> {
    let record = store.current_plan()?;
    let Some(plan) = record.plan else {
        return Ok(CurrentPlanView {
            revision: record.revision,
            state: PlanState::Empty,
            subject: None,
            fixture: None,
            context: None,
            saved_at_utc_ms: None,
            evaluated_at_utc_ms: None,
            age_ms: None,
            valid_from_utc_ms: None,
            valid_until_utc_ms: None,
            original_step_count: 0,
            summary_truncated: false,
            reason: None,
            summary: None,
        });
    };
    let selection = store.settings()?.settings.selected_subject;
    let selection_matches = selection.as_ref() == Some(&plan.guard.subject);
    let mut now = now;
    let restored = PublicationGuard::restore(store, &plan.guard);
    let valid_from_utc_ms = restored
        .as_ref()
        .ok()
        .and_then(|g| g.valid_from_utc_ms())
        .and_then(|n| UtcMillis::new(n).ok());
    let valid_until_utc_ms = restored
        .as_ref()
        .ok()
        .and_then(|g| g.valid_until_utc_ms())
        .and_then(|n| UtcMillis::new(n).ok());
    let checked = restored.and_then(|guard| guard.validate(store, &mut now));
    // Infrastructure failures are not misrepresented as ordinary historical advice.
    if let Err(e) = &checked
        && e.code != ErrorCode::SnapshotOutdated
    {
        return Err(e.clone());
    }
    let sampled = now();
    let age_ms = sampled
        .and_then(|n| n.get().checked_sub(plan.guard.publication_utc_ms.get()))
        .filter(|d| *d >= 0 && *d <= azeroth_core::types::MAX_SAFE_INTEGER as i64)
        .and_then(|d| u64::try_from(d).ok());
    Ok(CurrentPlanView {
        revision: record.revision,
        state: if checked.is_ok() && selection_matches {
            PlanState::Current
        } else {
            PlanState::Historical
        },
        subject: Some(plan.guard.subject),
        fixture: Some(plan.guard.content.scope.fixture),
        context: Some(plan.guard.content.scope.context),
        saved_at_utc_ms: Some(plan.guard.publication_utc_ms),
        evaluated_at_utc_ms: Some(plan.guard.evaluation_utc_ms),
        age_ms,
        valid_from_utc_ms,
        valid_until_utc_ms,
        original_step_count: plan.summary.steps.len(),
        summary_truncated: false,
        reason: if plan.guard.provenance == PlanProvenance::RecoveredReference {
            Some("recovered_reference_requires_new_plan")
        } else if !selection_matches {
            Some("selected_subject_differs")
        } else {
            checked.err().map(|_| "dependencies_or_freshness_changed")
        },
        summary: Some(plan.summary),
    })
}
pub(crate) fn revalidate_view(
    store: &SqliteStore,
    view: &mut CurrentPlanView,
    now: impl FnMut() -> Option<UtcMillis>,
) -> Result<()> {
    if !matches!(view.state, PlanState::Current) {
        return Ok(());
    }
    let plan = store
        .current_plan()?
        .plan
        .ok_or_else(|| invalid("Saved plan slot changed"))?;
    if let Err(e) =
        PublicationGuard::restore(store, &plan.guard).and_then(|g| g.validate(store, now))
    {
        if e.code != ErrorCode::SnapshotOutdated {
            return Err(e);
        }
        view.state = PlanState::Historical;
        view.reason = Some("dependencies_or_freshness_changed");
    }
    Ok(())
}
impl SnapshotApplication {
    pub fn app_settings(&self) -> Result<SettingsRecord> {
        self.with_store_mut(|s| s.settings())
    }
    pub fn update_settings(&self, text: &str) -> Result<SettingsRecord> {
        let input: SettingsUpdate = decode(text, MAX_DOCUMENT)?;
        if input.schema_version != 1 {
            return Err(invalid("Unsupported settings update version"));
        }
        self.save_app_settings(input.expected_revision, &input.settings)
    }
    pub fn pin_current_plan(
        &self,
        jobs: &PlanningJobs,
        id: &str,
        expected: i64,
        now: Option<UtcMillis>,
    ) -> Result<i64> {
        let started = Instant::now();
        self.with_store_mut(|store| {
            store.with_intent_transaction(|store| {
                // Registry is consulted under the same lock order as successful publication.
                let descriptor = jobs.pin_descriptor(id)?;
                let guard = PublicationGuard::restore(store, &descriptor.guard)?;
                store.replace_current_plan_guarded(expected, Some(&descriptor), |store| {
                    guard.validate(store, || advanced_now(now, started.elapsed()))
                })
            })
        })
    }
    pub fn clear_current_plan(&self, expected: i64) -> Result<i64> {
        self.with_store_mut(|s| {
            s.with_intent_transaction(|s| s.replace_current_plan(expected, None))
        })
    }
    pub fn today_view(&self, now: Option<UtcMillis>) -> Result<TodayView> {
        let started = Instant::now();
        self.with_store_mut(|store| {
            store.with_immediate_read(|store| {
                let settings = store.settings()?;
                let mut members = Vec::new();
                let mut selection_available = true;
                match &settings.settings.selected_subject {
                    Some(Subject::Character { profile_id: id }) => {
                        let key = store
                            .list_profiles()?
                            .into_iter()
                            .find(|p| profile_id(&p.key).ok().as_ref() == Some(id))
                            .map(|p| p.key);
                        if let Some(key) = key {
                            let s = store.current_snapshot(&key)?;
                            members.push(TodayMember {
                                profile_id: id.clone(),
                                character: key.character.as_str().into(),
                                snapshot_id: s.as_ref().map(|s| s.payload.id.clone()),
                                freshness: s.as_ref().map(|s| {
                                    freshness(
                                        &s.payload.capture,
                                        &context(advanced_now(now, started.elapsed())),
                                    )
                                }),
                            });
                        } else {
                            selection_available = false;
                        }
                    }
                    Some(Subject::Party { party_id }) => {
                        if let Some(party) = store.party(party_id)? {
                            for key in party.participants {
                                let s = store.current_snapshot(&key)?;
                                members.push(TodayMember {
                                    profile_id: profile_id(&key)?,
                                    character: key.character.as_str().into(),
                                    snapshot_id: s.as_ref().map(|s| s.payload.id.clone()),
                                    freshness: s.as_ref().map(|s| {
                                        freshness(
                                            &s.payload.capture,
                                            &context(advanced_now(now, started.elapsed())),
                                        )
                                    }),
                                });
                            }
                        } else {
                            selection_available = false;
                        }
                    }
                    None => selection_available = false,
                }
                let mut view = TodayView {
                    schema_version: 1,
                    settings_revision: settings.revision,
                    selected_subject: settings.settings.selected_subject,
                    selection_available,
                    members,
                    manual_goal: settings.settings.manual_planning_defaults.goal_profile,
                    current_plan: plan_view(store, || advanced_now(now, started.elapsed()))?,
                };
                encode(&view, 128 * 1024 - 64)?;
                revalidate_view(store, &mut view.current_plan, || {
                    advanced_now(now, started.elapsed())
                })?;
                Ok(view)
            })
        })
    }
}
