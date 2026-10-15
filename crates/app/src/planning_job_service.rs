//! Owned input calculation and atomic current-state publication for planning.
use crate::catalog::CatalogView;
use crate::{
    joint_planning::JointPlanningReport,
    planning::PlanningReport,
    planning_jobs::PlanningKind,
    snapshots::{SnapshotApplication, context},
};
use azeroth_core::{
    contract::{Envelope, MAX_ENVELOPE_BYTES},
    entities::{CharacterSnapshot, EquipmentSlot},
    error::{AppError, ErrorCode, Result},
    observation::{
        ClockTrust, ClockUncertainty, Freshness, FreshnessContext, Observation,
        ObservationMetadata, ObservationStatus, freshness,
    },
    search_control::{SearchControl, SearchPhase},
    types::{MAX_SAFE_INTEGER, PartyId, Sha256Hex, UtcMillis},
};
use azeroth_storage::{ProfileKey, SnapshotStore, SqliteStore, content::ContentRevisionKey};
use serde::Serialize;
use std::time::Duration;
#[derive(Serialize)]
#[serde(untagged)]
pub enum PlanningJobReport {
    Solo(PlanningReport),
    Joint(JointPlanningReport),
}
#[derive(Clone)]
pub struct PublicationGuard {
    party: Option<PartyId>,
    profiles: Vec<(ProfileKey, Sha256Hex, ObservationSignature)>,
    content: ContentRevisionKey,
    content_hash: String,
    disposition: Option<azeroth_storage::content_dispositions::DispositionGuard>,
    evaluation_utc_ms: Option<UtcMillis>,
    max_age_ms: azeroth_core::types::Millis,
}
// Snapshot maps are bounded by the core, and imported snapshots by bytes. This
// additional count cap bounds signature allocation even for constructed values.
const MAX_TYPED_OBSERVATIONS: usize = MAX_ENVELOPE_BYTES;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum FreshnessClass {
    Fresh,
    Stale,
    Uncertain(ClockUncertainty),
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ObservationClass {
    Unavailable(ObservationStatus),
    Known(FreshnessClass),
}
#[derive(Default, Clone)]
struct StableUtcRange {
    earliest: Option<i64>,
    latest: Option<i64>,
}
impl StableUtcRange {
    // Actual callers use fixed trusted UTC with no session context. Infer no
    // deadline from missing/untrusted time or unavailable observation values.
    fn include(&mut self, metadata: &ObservationMetadata, clock: &FreshnessContext) {
        if clock.session.is_some()
            || clock.clock_trust != ClockTrust::Trusted
            || metadata.clock_trust != ClockTrust::Trusted
        {
            return;
        }
        let Some(observed) = metadata.observed_at_utc_ms.map(|v| v.get()) else {
            return;
        };
        // UtcMillis and Millis bounds keep these sums within i64.
        let max_age = i64::try_from(clock.max_age_ms.get()).expect("bounded milliseconds");
        let (earliest, latest) = match freshness(metadata, clock) {
            Freshness::Fresh { .. } => (Some(observed), Some(observed + max_age)),
            Freshness::Stale { .. } => (
                Some(observed + max_age + 1),
                Some(observed + MAX_SAFE_INTEGER as i64),
            ),
            Freshness::Uncertain {
                reason: ClockUncertainty::ClockReversed,
            } => (None, Some(observed - 1)),
            Freshness::Uncertain {
                reason: ClockUncertainty::AgeOutOfRange,
            } => (Some(observed + MAX_SAFE_INTEGER as i64 + 1), None),
            Freshness::Uncertain {
                reason: ClockUncertainty::MissingOrUntrustedTime,
            } => (None, None),
        };
        if let Some(v) = earliest {
            self.earliest = Some(self.earliest.map_or(v, |n| n.max(v)));
        }
        if let Some(v) = latest {
            self.latest = Some(self.latest.map_or(v, |n| n.min(v)));
        }
    }
    fn contains(&self, now: Option<UtcMillis>) -> bool {
        if self.earliest.is_none() && self.latest.is_none() {
            return true;
        }
        now.is_some_and(|v| {
            self.earliest.is_none_or(|n| v.get() >= n) && self.latest.is_none_or(|n| v.get() <= n)
        })
    }
}
#[derive(Clone)]
struct ObservationSignature {
    classes: Vec<ObservationClass>,
    stable_utc: StableUtcRange,
}
struct ObservationClasses<'a> {
    clock: &'a FreshnessContext,
    classes: Vec<ObservationClass>,
    stable_utc: StableUtcRange,
}
impl ObservationClasses<'_> {
    fn record<T>(&mut self, observation: &Observation<T>) -> Result<()> {
        if self.classes.len() >= MAX_TYPED_OBSERVATIONS {
            return Err(AppError::new(
                ErrorCode::ImportTooLarge,
                "Planning observation count exceeds guard limit",
                false,
            ));
        }
        let class = if observation.status() != ObservationStatus::Known {
            ObservationClass::Unavailable(observation.status())
        } else {
            ObservationClass::Known(match freshness(observation.metadata(), self.clock) {
                Freshness::Fresh { .. } => FreshnessClass::Fresh,
                Freshness::Stale { .. } => FreshnessClass::Stale,
                Freshness::Uncertain { reason } => FreshnessClass::Uncertain(reason),
            })
        };
        if observation.status() == ObservationStatus::Known {
            self.stable_utc.include(observation.metadata(), self.clock);
        }
        self.classes.push(class);
        Ok(())
    }
}
/// Traverse only actual typed observations, never serialized keys/extensions.
/// Conservatively cover every observation (including unused ones), retaining
/// availability and freshness classes rather than continually changing ages.
fn observation_classes(
    snapshot: &CharacterSnapshot,
    clock: &FreshnessContext,
) -> Result<ObservationSignature> {
    let mut signature = ObservationClasses {
        clock,
        classes: Vec::new(),
        stable_utc: StableUtcRange::default(),
    };
    signature.stable_utc.include(&snapshot.capture, clock);
    signature.record(&snapshot.level)?;
    signature.record(&snapshot.role)?;
    signature.record(&snapshot.location)?;
    for value in snapshot.preferences.values() {
        signature.record(value)?;
    }
    for slot in snapshot.equipment.values() {
        signature.record(slot)?;
        if let Some(EquipmentSlot::Item(item)) = slot.value() {
            signature.record(&item.variant)?;
            signature.record(&item.instance)?;
            signature.record(&item.modifiers)?;
        }
    }
    for quest in snapshot.quests.values() {
        signature.record(&quest.accepted)?;
        signature.record(&quest.completed)?;
        signature.record(&quest.turned_in)?;
        for value in quest.objectives.values() {
            signature.record(value)?;
        }
    }
    for value in snapshot.objectives.values() {
        signature.record(value)?;
    }
    Ok(ObservationSignature {
        classes: signature.classes,
        stable_utc: signature.stable_utc,
    })
}
fn outdated() -> AppError {
    AppError::new(
        ErrorCode::SnapshotOutdated,
        "Planning inputs changed or became stale; refresh before calculating",
        false,
    )
}
fn content_hash(content: &dyn CatalogView) -> Result<String> {
    content.content_hash()
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
        content: &dyn CatalogView,
        clock: &FreshnessContext,
    ) -> Result<Self> {
        Ok(Self {
            party,
            profiles: snapshots
                .iter()
                .map(|s| {
                    Ok((
                        ProfileKey::of(s),
                        azeroth_storage::snapshot_content_hash(s)?,
                        observation_classes(&s.payload, clock)?,
                    ))
                })
                .collect::<Result<_>>()?,
            content: content.key(),
            content_hash: content_hash(content)?,
            disposition: None,
            evaluation_utc_ms: clock.now_utc_ms,
            max_age_ms: clock.max_age_ms,
        })
    }
    pub(crate) fn bind_disposition(
        mut self,
        guard: azeroth_storage::content_dispositions::DispositionGuard,
    ) -> Result<Self> {
        if guard.key != self.content {
            return Err(outdated());
        }
        self.disposition = Some(guard);
        Ok(self)
    }
    pub(crate) fn validate(
        &self,
        store: &SqliteStore,
        mut publication_now: impl FnMut() -> Option<UtcMillis>,
    ) -> Result<()> {
        let disposition = self.disposition.as_ref().ok_or_else(outdated)?;
        if let Err(e) = store.validate_usable_catalog_guard(disposition) {
            if e.code == ErrorCode::StorageError {
                return Err(e);
            }
            return Err(outdated());
        }
        if let Some(id) = &self.party {
            let party = store.party(id)?.ok_or_else(outdated)?;
            if party.participants
                != self
                    .profiles
                    .iter()
                    .map(|(key, _, _)| key.clone())
                    .collect::<Vec<_>>()
            {
                return Err(outdated());
            }
        }
        let mut snapshots = Vec::with_capacity(self.profiles.len());
        for (key, hash, _) in &self.profiles {
            let snapshot = store.current_snapshot(key)?.ok_or_else(outdated)?;
            if azeroth_storage::snapshot_content_hash(&snapshot)? != *hash {
                return Err(outdated());
            }
            snapshots.push(snapshot);
        }
        let content = store
            .active_catalog_revision(&self.content.scope)?
            .ok_or_else(outdated)?;
        if content.key() != self.content || content_hash(&content)? != self.content_hash {
            return Err(outdated());
        }
        // Sample after lock acquisition and identity hashing, immediately before
        // publication. Waiting for the mutex must not freeze snapshot age.
        let clock = context(publication_now());
        for (snapshot, (_, _, original_classes)) in snapshots.iter().zip(&self.profiles) {
            if !matches!(
                freshness(&snapshot.payload.capture, &clock),
                Freshness::Fresh { .. }
            ) || observation_classes(&snapshot.payload, &clock)?.classes
                != original_classes.classes
            {
                return Err(outdated());
            }
        }
        // Signature traversal is bounded but can itself cross a deadline. Check
        // its stable interval after all comparisons without another large walk.
        let final_now = publication_now();
        if self
            .profiles
            .iter()
            .any(|(_, _, signature)| !signature.stable_utc.contains(final_now))
        {
            return Err(outdated());
        }
        Ok(())
    }
    /// Compact server-owned intent; no observations or report serialization retained.
    pub fn pin_descriptor(
        &self,
        report: &PlanningJobReport,
        publication_now: Option<UtcMillis>,
    ) -> Result<azeroth_storage::today::PlanDescriptor> {
        use azeroth_storage::today::*;
        let (kind, request_json, subject, snapshot_ids, plan) = match report {
            PlanningJobReport::Solo(r) => (
                PlanKind::Solo,
                encode(&r.request, MAX_REQUEST)?,
                Subject::Character {
                    profile_id: r.request.profile_id.clone(),
                },
                std::collections::BTreeMap::from([(
                    r.request.profile_id.clone(),
                    r.request.snapshot_id.clone(),
                )]),
                &r.result,
            ),
            PlanningJobReport::Joint(r) => (
                PlanKind::Joint,
                encode(&r.request, MAX_REQUEST)?,
                Subject::Party {
                    party_id: r.request.party_id.clone(),
                },
                r.request
                    .participants
                    .iter()
                    .map(|p| (p.profile_id.clone(), p.snapshot_id.clone()))
                    .collect(),
                &r.result.plan,
            ),
        };
        if snapshot_ids.len() != self.profiles.len() {
            return Err(outdated());
        }
        let descriptor = PlanDescriptor {
            kind,
            request_json,
            guard: GuardDescriptor {
                schema_version: 1,
                provenance: PlanProvenance::LocalGuardedSuccess,
                subject,
                profiles: self
                    .profiles
                    .iter()
                    .map(|(key, hash, _)| {
                        let profile_id = crate::snapshots::profile_id(key)?;
                        let snapshot_id = snapshot_ids
                            .get(&profile_id)
                            .cloned()
                            .ok_or_else(outdated)?;
                        Ok(SavedProfile {
                            profile_id,
                            key: key.clone(),
                            snapshot_id,
                            snapshot_hash: hash.clone(),
                        })
                    })
                    .collect::<Result<_>>()?,
                content: self.content.clone(),
                content_hash: azeroth_core::types::Sha256Hex::new(self.content_hash.clone())?,
                content_disposition: self.disposition.clone().ok_or_else(outdated)?,
                evaluation_utc_ms: self.evaluation_utc_ms.ok_or_else(outdated)?,
                publication_utc_ms: publication_now.ok_or_else(outdated)?,
                max_age_ms: self.max_age_ms,
                ruleset_version: crate::plan_export::RULESET_VERSION.into(),
                model_version: crate::plan_export::MODEL_VERSION.into(),
            },
            summary: PlanSummary {
                schema_version: 1,
                goal_profile: plan.goal_profile.clone(),
                steps: plan
                    .selected
                    .iter()
                    .map(|c| {
                        // Original request is the authority for declared objective node.
                        let node = match report {
                            PlanningJobReport::Solo(r) => r
                                .request
                                .candidates
                                .iter()
                                .find(|e| e.objective == c.candidate)
                                .map(|e| e.node.clone()),
                            PlanningJobReport::Joint(r) => r
                                .request
                                .candidates
                                .iter()
                                .find(|e| e.objective == c.candidate)
                                .map(|e| e.node.clone()),
                        }
                        .ok_or_else(outdated)?;
                        Ok(SavedStep {
                            objective: c.candidate.clone(),
                            node,
                            completion_ms: c.completion_ms,
                            service_ms: c.service_ms,
                            expected_session_ms: c.expected_session_ms,
                        })
                    })
                    .collect::<Result<_>>()?,
                expected_total_ms: plan.expected_total_ms,
                termination: match plan.termination {
                    azeroth_core::planner::Termination::NoFeasibleCandidate => {
                        SavedTermination::NoFeasibleCandidate
                    }
                    azeroth_core::planner::Termination::StepLimit => SavedTermination::StepLimit,
                },
            },
        };
        descriptor.validate()?;
        Ok(descriptor)
    }
    /// Reconstruct the original typed classes without retaining raw snapshots.
    pub(crate) fn restore(
        store: &SqliteStore,
        descriptor: &azeroth_storage::today::GuardDescriptor,
    ) -> Result<Self> {
        if descriptor.provenance != azeroth_storage::today::PlanProvenance::LocalGuardedSuccess
            || descriptor.ruleset_version != crate::plan_export::RULESET_VERSION
            || descriptor.model_version != crate::plan_export::MODEL_VERSION
            || descriptor.max_age_ms != context(Some(descriptor.evaluation_utc_ms)).max_age_ms
        {
            return Err(outdated());
        }
        let content = store
            .active_catalog_revision(&descriptor.content.scope)?
            .ok_or_else(outdated)?;
        if content.key() != descriptor.content
            || content_hash(&content)? != descriptor.content_hash.as_str()
        {
            return Err(outdated());
        }
        let snapshots = descriptor
            .profiles
            .iter()
            .map(|p| {
                if crate::snapshots::profile_id(&p.key)? != p.profile_id {
                    return Err(outdated());
                }
                let s = store.current_snapshot(&p.key)?.ok_or_else(outdated)?;
                if s.payload.id != p.snapshot_id
                    || azeroth_storage::snapshot_content_hash(&s)? != p.snapshot_hash
                {
                    return Err(outdated());
                }
                Ok(s)
            })
            .collect::<Result<Vec<_>>>()?;
        let party = match &descriptor.subject {
            azeroth_storage::today::Subject::Party { party_id } => Some(party_id.clone()),
            _ => None,
        };
        Self::new(
            party,
            &snapshots,
            &content,
            &context(Some(descriptor.evaluation_utc_ms)),
        )?
        .bind_disposition(descriptor.content_disposition.clone())
    }

    pub(crate) fn valid_from_utc_ms(&self) -> Option<i64> {
        self.profiles
            .iter()
            .filter_map(|(_, _, s)| s.stable_utc.earliest)
            .max()
    }
    /// Earliest finite inclusive UTC upper bound retained by the typed guard.
    /// None means no finite bound was inferred, not unlimited usable freshness.
    /// Exporters must also require fresh captures and bound their own lifetime.
    pub fn valid_until_utc_ms(&self) -> Option<i64> {
        self.profiles
            .iter()
            .filter_map(|(_, _, signature)| signature.stable_utc.latest)
            .min()
    }

    /// Only validation and the constant-time result handoff run under store lock;
    /// calculation and bounded serialization have already completed outside it.
    pub fn publish(
        &self,
        app: &SnapshotApplication,
        now: Option<UtcMillis>,
        publish: &mut dyn FnMut() -> Result<()>,
    ) -> Result<()> {
        let started = std::time::Instant::now();
        app.with_store_mut(|store| {
            store.with_immediate_read(|store| {
                self.validate(store, || advanced_now(now, started.elapsed()))?;
                publish()
            })
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
                self.with_planning_guarded(text, now, |request, snapshot, content, clock, guard| {
                    control.checkpoint(SearchPhase::ValidateInputs)?;
                    let report = crate::planning::calculate_controlled(
                        request, snapshot, content, clock, control,
                    )?;
                    Ok((PlanningJobReport::Solo(report), guard.clone()))
                })
            }
            PlanningKind::Joint => self.with_joint_planning_guarded(
                text,
                now,
                control,
                |request, catalog, snapshots, content, control, guard| {
                    control.checkpoint(SearchPhase::ParticipantReadiness)?;
                    let report = crate::joint_planning::calculate_controlled(
                        request, catalog, snapshots, content, now, control,
                    )?;
                    Ok((PlanningJobReport::Joint(report), guard.clone()))
                },
            ),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::demo_snapshot;
    use azeroth_core::{search_control::NoopSearchControl, types::DatasetRevision};
    use azeroth_storage::content::ContentRevision;
    use serde_json::{Value, json};

    #[test]
    fn final_clock_after_class_walk_rejects_expiry_and_newly_usable_fields() {
        const NOW: i64 = 1791792000000;
        for observed in [NOW - 1_790_000, NOW + 10000] {
            let app = SnapshotApplication::in_memory().unwrap();
            let now = UtcMillis::new(NOW).unwrap();
            let mut snapshot: Value = serde_json::from_str(include_str!(
                "../../../fixtures/contracts/v1/character-snapshot.json"
            ))
            .unwrap();
            snapshot["payload"]["capture"]["observed_at_utc_ms"] = json!(NOW - 100);
            snapshot["payload"]["level"]["clock_trust"] = json!("trusted");
            snapshot["payload"]["level"]["observed_at_utc_ms"] = json!(observed);
            let profile = app
                .import_text(&format!("ANAV1:{snapshot}"), Some(now))
                .unwrap()
                .profile_id;
            app.stage_content(include_str!(
                "../../../fixtures/desktop/planning/synthetic-revision.json"
            ))
            .unwrap();
            app.activate_content(
                &profile,
                &DatasetRevision::new("synthetic-planning-v1").unwrap(),
            )
            .unwrap();
            let catalog = app.planning_catalog(&profile).unwrap();
            let request = json!({"profile_id":profile,"snapshot_id":catalog.snapshot_id,"revision":catalog.revision,"goal_profile":"manual-v1","origin":"demo-node-1","stopping_node":"demo-node-1","budget_ms":100000,"stopping_service_ms":0,"candidates":[],"excluded":[]});
            let (_, guard) = app
                .calculate_planning_job(
                    PlanningKind::Solo,
                    &request.to_string(),
                    Some(now),
                    &mut NoopSearchControl,
                )
                .unwrap();
            let mut samples = [Some(now), Some(UtcMillis::new(NOW + 20000).unwrap())].into_iter();
            let error = app
                .with_store_mut(|store| guard.validate(store, || samples.next().unwrap()))
                .unwrap_err();
            assert_eq!(error.code, ErrorCode::SnapshotOutdated);
            assert!(
                samples.next().is_none(),
                "must check again after the class walk"
            );
        }
    }

    #[test]
    fn publication_reserves_the_validated_content_until_handoff() {
        let path = std::env::temp_dir().join(format!(
            "anav-publication-{}-{}.sqlite",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let app = SnapshotApplication::open(&path).unwrap();
        let snapshot = demo_snapshot().unwrap();
        let now = snapshot.payload.capture.observed_at_utc_ms.unwrap();
        let profile = app
            .import_text(
                &format!("ANAV1:{}", serde_json::to_string(&snapshot).unwrap()),
                Some(now),
            )
            .unwrap()
            .profile_id;
        let content_text =
            include_str!("../../../fixtures/desktop/planning/synthetic-revision.json");
        app.stage_content(content_text).unwrap();
        app.activate_content(
            &profile,
            &DatasetRevision::new("synthetic-planning-v1").unwrap(),
        )
        .unwrap();
        let mut alternate: Value = serde_json::from_str(content_text).unwrap();
        alternate["envelope"]["payload"]["revision"] = json!("alternate-v1");
        app.stage_content(&alternate.to_string()).unwrap();
        let alternate: ContentRevision = serde_json::from_value(alternate).unwrap();
        let mut writer = SqliteStore::open(&path).unwrap();
        let catalog = app.planning_catalog(&profile).unwrap();
        let request = json!({"profile_id":profile,"snapshot_id":catalog.snapshot_id,"revision":catalog.revision,"goal_profile":"manual-v1","origin":"demo-node-1","stopping_node":"demo-node-1","budget_ms":100000,"stopping_service_ms":0,"candidates":[],"excluded":[]});
        let (_, guard) = app
            .calculate_planning_job(
                PlanningKind::Solo,
                &request.to_string(),
                Some(now),
                &mut NoopSearchControl,
            )
            .unwrap();
        guard
            .publish(&app, Some(now), &mut || {
                let error = writer
                    .activate_content_revision(&alternate.key())
                    .unwrap_err();
                assert_eq!(
                    error.code,
                    ErrorCode::StorageError,
                    "independent writer must be excluded through result handoff"
                );
                Ok(())
            })
            .unwrap();
        writer.activate_content_revision(&alternate.key()).unwrap();
        assert_eq!(
            writer
                .active_content_revision(&alternate.key().scope)
                .unwrap()
                .unwrap()
                .key(),
            alternate.key()
        );
        drop(writer);
        drop(app);
        std::fs::remove_file(path).unwrap();
    }

    #[test]
    fn export_deadline_is_earliest_finite_member_bound() {
        let mut guard = PublicationGuard {
            party: None,
            profiles: Vec::new(),
            content: ContentRevisionKey {
                scope: azeroth_storage::content::ContentScope {
                    context: demo_snapshot().unwrap().payload.context.clone(),
                    fixture: true,
                },
                revision: DatasetRevision::new("test-v1").unwrap(),
            },
            content_hash: String::new(),
            disposition: None,
            evaluation_utc_ms: None,
            max_age_ms: azeroth_core::types::Millis::new(1_800_000).unwrap(),
        };
        assert_eq!(guard.valid_until_utc_ms(), None);
        let snapshot = demo_snapshot().unwrap();
        let key = ProfileKey::of(&snapshot);
        let hash = azeroth_storage::snapshot_content_hash(&snapshot).unwrap();
        for bound in [None, Some(500), Some(100), Some(700)] {
            guard.profiles.push((
                key.clone(),
                hash.clone(),
                ObservationSignature {
                    classes: Vec::new(),
                    stable_utc: StableUtcRange {
                        earliest: None,
                        latest: bound,
                    },
                },
            ));
        }
        assert_eq!(guard.valid_until_utc_ms(), Some(100));
    }

    #[test]
    fn stable_interval_preserves_inclusive_bounds_and_missing_time() {
        let observed = UtcMillis::new(100).unwrap();
        let metadata = ObservationMetadata {
            clock_trust: ClockTrust::Trusted,
            observed_at_utc_ms: Some(observed),
            ..Default::default()
        };
        let clock = context(Some(observed));
        let mut range = StableUtcRange::default();
        range.include(&metadata, &clock);
        assert!(range.contains(Some(observed)));
        assert!(range.contains(Some(UtcMillis::new(1_800_100).unwrap())));
        assert!(!range.contains(Some(UtcMillis::new(1_800_101).unwrap())));
        assert!(!range.contains(Some(UtcMillis::new(99).unwrap())));
        assert!(!range.contains(None));
        let mut missing = StableUtcRange::default();
        missing.include(&ObservationMetadata::default(), &clock);
        assert!(missing.contains(None));
    }
}
