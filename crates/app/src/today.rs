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
            reason: None,
            summary: None,
        });
    };
    let mut now = now;
    let checked = PublicationGuard::restore(store, &plan.guard)
        .and_then(|guard| guard.validate(store, &mut now));
    // Infrastructure failures are not misrepresented as ordinary historical advice.
    if let Err(e) = &checked {
        if e.code != ErrorCode::SnapshotOutdated {
            return Err(e.clone());
        }
    }
    let sampled = now();
    let age_ms = sampled
        .and_then(|n| n.get().checked_sub(plan.guard.publication_utc_ms.get()))
        .filter(|d| *d >= 0)
        .and_then(|d| u64::try_from(d).ok());
    Ok(CurrentPlanView {
        revision: record.revision,
        state: if checked.is_ok() {
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
        reason: checked.err().map(|_| "dependencies_or_freshness_changed"),
        summary: Some(plan.summary),
    })
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
                guard.validate(store, || advanced_now(now, started.elapsed()))?;
                store.replace_current_plan(expected, Some(&descriptor))
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
                let view = TodayView {
                    schema_version: 1,
                    settings_revision: settings.revision,
                    selected_subject: settings.settings.selected_subject,
                    selection_available,
                    members,
                    manual_goal: settings.settings.manual_planning_defaults.goal_profile,
                    current_plan: plan_view(store, || advanced_now(now, started.elapsed()))?,
                };
                encode(&view, MAX_DOCUMENT)?;
                Ok(view)
            })
        })
    }
}
