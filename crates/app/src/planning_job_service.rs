//! Owned input calculation and atomic current-state publication for planning.
use crate::{
    joint_planning::JointPlanningReport,
    planning::PlanningReport,
    planning_jobs::PlanningKind,
    snapshots::{SnapshotApplication, context},
};
use azeroth_core::{
    contract::Envelope,
    entities::CharacterSnapshot,
    error::{AppError, ErrorCode, Result},
    observation::{Freshness, freshness},
    search_control::{SearchControl, SearchPhase},
    types::{PartyId, Sha256Hex, UtcMillis},
};
use azeroth_storage::{
    ProfileKey, SnapshotStore, SqliteStore,
    content::{ContentRevision, ContentRevisionKey},
};
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::time::Duration;
#[derive(Serialize)]
#[serde(untagged)]
pub enum PlanningJobReport {
    Solo(PlanningReport),
    Joint(JointPlanningReport),
}
pub struct PublicationGuard {
    party: Option<PartyId>,
    profiles: Vec<(ProfileKey, Sha256Hex)>,
    content: ContentRevisionKey,
    content_hash: String,
}
fn outdated() -> AppError {
    AppError::new(
        ErrorCode::SnapshotOutdated,
        "Planning inputs changed or became stale; refresh before calculating",
        false,
    )
}
fn content_hash(content: &ContentRevision) -> Result<String> {
    let bytes = serde_json::to_vec(content).map_err(|_| {
        AppError::new(
            ErrorCode::InvalidInput,
            "Planning input cannot be hashed",
            false,
        )
    })?;
    Ok(format!("{:x}", Sha256::digest(bytes)))
}
pub(crate) fn advanced_now(now: Option<UtcMillis>, elapsed: Duration) -> Option<UtcMillis> {
    now.and_then(|n| {
        n.get()
            .checked_add(i64::try_from(elapsed.as_millis()).ok()?)
    })
    .and_then(|n| UtcMillis::new(n).ok())
}
impl PublicationGuard {
    pub(crate) fn new(
        party: Option<PartyId>,
        snapshots: &[Envelope<CharacterSnapshot>],
        content: &ContentRevision,
    ) -> Result<Self> {
        Ok(Self {
            party,
            profiles: snapshots
                .iter()
                .map(|s| {
                    Ok((
                        ProfileKey::of(s),
                        azeroth_storage::snapshot_content_hash(s)?,
                    ))
                })
                .collect::<Result<_>>()?,
            content: content.key(),
            content_hash: content_hash(content)?,
        })
    }
    fn validate(&self, store: &SqliteStore, now: Option<UtcMillis>) -> Result<()> {
        if let Some(id) = &self.party {
            let party = store.party(id)?.ok_or_else(outdated)?;
            if party.participants
                != self
                    .profiles
                    .iter()
                    .map(|(key, _)| key.clone())
                    .collect::<Vec<_>>()
            {
                return Err(outdated());
            }
        }
        let clock = context(now);
        for (key, hash) in &self.profiles {
            let snapshot = store.current_snapshot(key)?.ok_or_else(outdated)?;
            if azeroth_storage::snapshot_content_hash(&snapshot)? != *hash
                || !matches!(
                    freshness(&snapshot.payload.capture, &clock),
                    Freshness::Fresh { .. }
                )
            {
                return Err(outdated());
            }
        }
        let content = store
            .active_content_revision(&self.content.scope)?
            .ok_or_else(outdated)?;
        if content.key() != self.content || content_hash(&content)? != self.content_hash {
            return Err(outdated());
        }
        Ok(())
    }
    /// Only validation and the constant-time result handoff run under store lock;
    /// calculation and bounded serialization have already completed outside it.
    pub fn publish(
        &self,
        app: &SnapshotApplication,
        now: Option<UtcMillis>,
        publish: &mut dyn FnMut() -> Result<()>,
    ) -> Result<()> {
        app.with_store_mut(|store| {
            self.validate(store, now)?;
            publish()
        })
    }
}
impl SnapshotApplication {
    pub fn calculate_planning_job(
        &self,
        kind: PlanningKind,
        text: &str,
        now: Option<UtcMillis>,
        control: &mut dyn SearchControl,
    ) -> Result<(PlanningJobReport, PublicationGuard)> {
        control.checkpoint(SearchPhase::ValidateInputs)?;
        match kind {
            PlanningKind::Solo => {
                self.with_planning(text, now, |request, snapshot, content, clock| {
                    control.checkpoint(SearchPhase::ValidateInputs)?;
                    let guard =
                        PublicationGuard::new(None, std::slice::from_ref(snapshot), content)?;
                    let report = crate::planning::calculate_controlled(
                        request, snapshot, content, clock, control,
                    )?;
                    Ok((PlanningJobReport::Solo(report), guard))
                })
            }
            PlanningKind::Joint => self.with_joint_planning_controlled(
                text,
                now,
                control,
                |request, catalog, snapshots, content, control| {
                    control.checkpoint(SearchPhase::ParticipantReadiness)?;
                    let guard =
                        PublicationGuard::new(Some(request.party_id.clone()), &snapshots, content)?;
                    let report = crate::joint_planning::calculate_controlled(
                        request, catalog, snapshots, content, now, control,
                    )?;
                    Ok((PlanningJobReport::Joint(report), guard))
                },
            ),
        }
    }
}
