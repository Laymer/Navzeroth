//! Pure, bounded context-reminder engine over `ContextRule` and `evaluate_rule`.
//! It reads no clock, performs no I/O and stores nothing: callers pass the
//! evaluation context, the suppression observations and the persisted
//! [`ReminderState`], and receive a [`ReminderPlan`].
//!
//! Policy (plan section 11.1, M7.1):
//! - Three-valued gating. Every rule is evaluated through `evaluate_rule`, so
//!   expiry, clock trust and data-context compatibility are never re-implemented
//!   here. Only `Truth::True` yields advice. `Truth::False` yields nothing and
//!   re-arms the rule. `Truth::Unknown` is never coerced: by default it is
//!   omitted, and with [`UnknownPolicy::Informational`] it becomes a
//!   "cannot evaluate" item that carries the evaluation reasons and omits the
//!   rule's result template, so it is never phrased as a fact. An `Err`
//!   from `evaluate_rule` (data-context mismatch, step limit) propagates and
//!   fails the whole plan; it is never folded into `Unknown`.
//! - Ordering. Items are ordered by strength (advice before informational),
//!   then by priority, highest first (`i16`), then by `RuleId` ascending. The
//!   result does not depend on the order of the input rules. Duplicate rule IDs
//!   are rejected and at most 1024 rules are accepted.
//! - Deduplication. Among the rules that produce an item, one per
//!   `deduplication_key` survives: the best-ranked by the order above. The
//!   dismissal, cooldown and unchanged-state gates then apply to the survivor
//!   only, because the group is one piece of advice.
//! - Unchanged state. A structural [`StateFingerprint`] (rule, optionally the
//!   snapshot ID, truth and the sorted evaluation reasons) is recorded when an
//!   item is presented; an equal fingerprint is not presented again, even after
//!   the cooldown. A definite `False` clears the fingerprint so a later `True`
//!   is new. No hash is used, so persisted fingerprints are stable.
//! - Cooldown. `cooldown_ms` blocks re-presentation until it has elapsed on a
//!   trusted clock. An untrusted or missing clock, a clock that went backwards
//!   and a record made without a trusted clock all block (conservative).
//!   A zero cooldown never depends on the clock.
//! - Dismissal. `Session` applies while the session ID is unchanged,
//!   `UntilRulesetChanges` while the ruleset revision is unchanged and
//!   `Permanent` always. A dismissal stays in force unless the new session or
//!   revision is known and different. Dismissals are keyed by data context and
//!   rule, so demo and reference namespaces never share one. History is capped
//!   at [`MAX_HISTORY`]; a full history evicts the oldest non-permanent
//!   dismissal and refuses a new one when only permanent dismissals remain.
//! - Suppression. Combat, loading and cutscene state are observations. Any fresh
//!   known `true` holds every reminder back in `held` (kept, not dropped). All
//!   three fresh known `false` allows normal presentation. Anything else
//!   (unknown, unsupported, restricted, not applicable, stale) selects the
//!   non-intrusive mode: no primary slot and at most one secondary item.
//!   Nothing is inferred from other state.
//! - Presentation budget. [`ReminderPlan::select_hud`] returns at most one
//!   primary and one secondary item and lists the remaining queue; the desktop
//!   reads the full `queue`. Only what the caller reports as presented is
//!   recorded by [`ReminderState::commit`].
use crate::{
    entities::{ContextRule, DismissalPolicy, RuleTrigger, bounded, unique},
    error::{Result, invalid},
    evaluation::*,
    observation::*,
    types::*,
};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::collections::{BTreeMap, BTreeSet};

/// Maximum dismissal records, shown records and fingerprint reasons.
pub const MAX_HISTORY: usize = 1024;
const MAX_ISSUE_KEY_BYTES: usize = 4096;

/// Advice sorts before informational items.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ReminderStrength {
    Strong,
    Informational,
}

/// What to do with a rule whose truth is `Unknown`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum UnknownPolicy {
    /// Produce nothing; the rule is listed as excluded with its reasons.
    #[default]
    Omit,
    /// Produce a non-factual "cannot evaluate" item carrying the reasons.
    Informational,
}

/// What a state fingerprint covers.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum FingerprintBasis {
    /// Includes the snapshot ID, so any new snapshot is new state.
    Snapshot,
    /// Only truth and reasons: the same outcome on a newer snapshot is unchanged.
    #[default]
    OutcomeOnly,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ReminderPolicy {
    pub unknown: UnknownPolicy,
    pub fingerprint: FingerprintBasis,
}

/// The session and ruleset a dismissal is judged against. `None` means unknown.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ReminderScope {
    pub session: Option<SessionId>,
    pub ruleset: Option<DatasetRevision>,
}

/// Caller-supplied disruptive-state observations. Each defaults to unknown and
/// is never inferred from anything else.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SuppressionState {
    pub combat: Observation<bool>,
    pub loading: Observation<bool>,
    pub cutscene: Observation<bool>,
}
impl Default for SuppressionState {
    fn default() -> Self {
        Self {
            combat: Observation::unknown(),
            loading: Observation::unknown(),
            cutscene: Observation::unknown(),
        }
    }
}
impl SuppressionState {
    pub fn assess(&self, freshness_context: &FreshnessContext) -> PresentationMode {
        let mut confirmed_clear = true;
        for observation in [&self.combat, &self.loading, &self.cutscene] {
            let fresh = matches!(
                freshness(observation.metadata(), freshness_context),
                Freshness::Fresh { .. }
            );
            match observation.value() {
                Some(true) if fresh => return PresentationMode::Suppressed,
                Some(false) if fresh => {}
                _ => confirmed_clear = false,
            }
        }
        if confirmed_clear {
            PresentationMode::Normal
        } else {
            PresentationMode::NonIntrusive
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PresentationMode {
    Normal,
    /// Suppression state is not confirmed clear: no primary, at most one secondary.
    NonIntrusive,
    /// Disruptive state is known true: everything is held back.
    Suppressed,
}

#[derive(Clone, Debug, Default)]
pub struct ReminderInputs {
    pub suppression: SuppressionState,
    pub scope: ReminderScope,
    /// `None` evaluates every rule; otherwise only rules whose trigger is listed.
    pub fired_triggers: Option<Vec<RuleTrigger>>,
}

/// Stable structural identity of a rule's evaluated state. Reasons are the
/// canonical JSON of each `EvaluationIssue`, sorted and de-duplicated.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StateFingerprint {
    pub rule: RuleId,
    pub snapshot: Option<SnapshotId>,
    pub truth: Truth,
    pub issues: Vec<String>,
}
impl StateFingerprint {
    fn new(
        rule: &RuleId,
        snapshot: Option<&SnapshotId>,
        truth: Truth,
        reasons: &[EvaluationIssue],
    ) -> Result<Self> {
        let mut issues = reasons
            .iter()
            .map(|issue| {
                serde_json::to_string(issue).map_err(|_| invalid("Reason cannot be serialized"))
            })
            .collect::<Result<Vec<_>>>()?;
        issues.sort();
        issues.dedup();
        let fingerprint = Self {
            rule: rule.clone(),
            snapshot: snapshot.cloned(),
            truth,
            issues,
        };
        fingerprint.validate()?;
        Ok(fingerprint)
    }
    fn validate(&self) -> Result<()> {
        bounded(self.issues.len())?;
        if self.issues.iter().any(|i| i.len() > MAX_ISSUE_KEY_BYTES) {
            return Err(invalid("Fingerprint reason exceeds its size limit"));
        }
        if !self.issues.windows(2).all(|pair| pair[0] < pair[1]) {
            return Err(invalid("Fingerprint reasons must be sorted and unique"));
        }
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ReminderKind {
    /// `Truth::True`: the rule's result template.
    Advice { text: Text },
    /// `Truth::Unknown`: no template and no claim, only what could not be read.
    CannotEvaluate { reasons: Vec<EvaluationIssue> },
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct ReminderItem {
    pub rule: RuleId,
    pub deduplication_key: RuleId,
    /// Higher first.
    pub priority: i16,
    pub kind: ReminderKind,
    pub warnings: Vec<EvaluationIssue>,
    pub evidence: Vec<EvidenceId>,
    pub dismissal: DismissalPolicy,
    pub cooldown_ms: Millis,
    pub fingerprint: StateFingerprint,
}
impl ReminderItem {
    pub fn strength(&self) -> ReminderStrength {
        match self.kind {
            ReminderKind::Advice { .. } => ReminderStrength::Strong,
            ReminderKind::CannotEvaluate { .. } => ReminderStrength::Informational,
        }
    }
}

/// Why a rule has no queued item. Closed, carries IDs and evaluation reasons only.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(tag = "code", rename_all = "snake_case")]
pub enum ReminderReason {
    TriggerNotFired,
    ConditionFalse,
    RuleExpired,
    InputsUnknown {
        reasons: Vec<EvaluationIssue>,
    },
    Deduplicated {
        kept: RuleId,
    },
    Dismissed {
        policy: DismissalPolicy,
    },
    StateUnchanged,
    CooldownActive,
    /// The cooldown cannot be judged: untrusted, missing or reversed clock.
    CooldownUnverifiable,
    /// Held back because disruptive state is known true; still queued.
    Suppressed,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct ReminderPlan {
    pub context: DataContext,
    pub mode: PresentationMode,
    /// Presentable now, in deterministic order.
    pub queue: Vec<ReminderItem>,
    /// Held back by `Suppressed` mode, in the same order; not dropped.
    pub held: Vec<ReminderItem>,
    pub excluded: BTreeMap<RuleId, ReminderReason>,
    rearm: BTreeSet<RuleId>,
    scope_session: Option<SessionId>,
}
impl ReminderPlan {
    /// Rules that evaluated to a definite `False`.
    pub fn rearm(&self) -> &BTreeSet<RuleId> {
        &self.rearm
    }
    /// Section 4.1 budget: at most one primary and one secondary item.
    pub fn select_hud(&self) -> HudSelection {
        let mut queue = self.queue.iter();
        let (primary, secondary) = match self.mode {
            PresentationMode::Suppressed => (None, None),
            PresentationMode::NonIntrusive => (None, queue.next()),
            PresentationMode::Normal => {
                let primary = queue.next();
                (primary, queue.next())
            }
        };
        let remaining = queue.map(|item| item.rule.clone()).collect();
        HudSelection {
            mode: self.mode,
            primary: primary.cloned(),
            secondary: secondary.cloned(),
            remaining,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct HudSelection {
    pub mode: PresentationMode,
    pub primary: Option<ReminderItem>,
    pub secondary: Option<ReminderItem>,
    /// Queued but not presented, in queue order.
    pub remaining: Vec<RuleId>,
}
impl HudSelection {
    pub fn presented(&self) -> Vec<RuleId> {
        [&self.primary, &self.secondary]
            .into_iter()
            .flatten()
            .map(|item| item.rule.clone())
            .collect()
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DismissalRecord {
    pub context: DataContext,
    pub rule: RuleId,
    pub policy: DismissalPolicy,
    /// Present exactly for `Session`.
    pub session: Option<SessionId>,
    /// Present exactly for `UntilRulesetChanges`.
    pub ruleset: Option<DatasetRevision>,
    /// Only recorded from a trusted clock.
    pub dismissed_at_utc_ms: Option<UtcMillis>,
    /// Monotonic order within this state; the oldest is evicted first.
    pub serial: u64,
}
impl DismissalRecord {
    fn applies(&self, scope: &ReminderScope) -> bool {
        match self.policy {
            DismissalPolicy::Permanent => true,
            DismissalPolicy::Session => same_or_unknown(&self.session, &scope.session),
            DismissalPolicy::UntilRulesetChanges => same_or_unknown(&self.ruleset, &scope.ruleset),
        }
    }
}
fn same_or_unknown<T: PartialEq>(recorded: &Option<T>, current: &Option<T>) -> bool {
    match (recorded, current) {
        (Some(a), Some(b)) => a == b,
        _ => true,
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ShownRecord {
    pub context: DataContext,
    pub rule: RuleId,
    /// `None` after the rule evaluated to a definite `False`.
    pub fingerprint: Option<StateFingerprint>,
    /// Only recorded from a trusted clock.
    pub shown_at_utc_ms: Option<UtcMillis>,
    #[serde(default)]
    pub shown_at_session: Option<(SessionId, Millis)>,
    /// Session identity can be known without an elapsed-time observation.
    #[serde(default)]
    pub shown_session: Option<SessionId>,
    pub serial: u64,
}

type StateKey = (DataContext, RuleId);

/// Persistable dismissal and last-presented history. Plain serde records for a
/// later storage layer; no SQL and not part of the exchange contract.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ReminderState {
    next_serial: u64,
    dismissals: BTreeMap<StateKey, DismissalRecord>,
    shown: BTreeMap<StateKey, ShownRecord>,
}

#[derive(Serialize)]
struct StateOut<'a> {
    next_serial: u64,
    dismissals: Vec<&'a DismissalRecord>,
    shown: Vec<&'a ShownRecord>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct StateIn {
    next_serial: u64,
    dismissals: Vec<DismissalRecord>,
    shown: Vec<ShownRecord>,
}
impl Serialize for ReminderState {
    fn serialize<S: Serializer>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error> {
        StateOut {
            next_serial: self.next_serial,
            dismissals: self.dismissals.values().collect(),
            shown: self.shown.values().collect(),
        }
        .serialize(serializer)
    }
}
impl<'de> Deserialize<'de> for ReminderState {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> std::result::Result<Self, D::Error> {
        Self::try_from(StateIn::deserialize(deserializer)?).map_err(serde::de::Error::custom)
    }
}
impl TryFrom<StateIn> for ReminderState {
    type Error = crate::error::AppError;
    fn try_from(input: StateIn) -> Result<Self> {
        if input.next_serial > MAX_SAFE_INTEGER {
            return Err(invalid("Reminder serial is outside the safe range"));
        }
        bounded(input.dismissals.len())?;
        bounded(input.shown.len())?;
        let mut state = Self {
            next_serial: input.next_serial,
            ..Self::default()
        };
        for record in input.dismissals {
            let well_formed = match record.policy {
                DismissalPolicy::Session => record.session.is_some() && record.ruleset.is_none(),
                DismissalPolicy::UntilRulesetChanges => {
                    record.session.is_none() && record.ruleset.is_some()
                }
                DismissalPolicy::Permanent => record.session.is_none() && record.ruleset.is_none(),
            };
            if !well_formed || record.serial >= input.next_serial {
                return Err(invalid("Dismissal record is inconsistent"));
            }
            let key = (record.context.clone(), record.rule.clone());
            if state.dismissals.insert(key, record).is_some() {
                return Err(invalid("Duplicate dismissal record"));
            }
        }
        for record in input.shown {
            if record.serial >= input.next_serial {
                return Err(invalid("Shown record is inconsistent"));
            }
            if let Some(fingerprint) = &record.fingerprint {
                fingerprint.validate()?;
                if fingerprint.rule != record.rule {
                    return Err(invalid("Shown record fingerprint names another rule"));
                }
            }
            let key = (record.context.clone(), record.rule.clone());
            if state.shown.insert(key, record).is_some() {
                return Err(invalid("Duplicate shown record"));
            }
        }
        Ok(state)
    }
}

fn trusted_now(context: &FreshnessContext) -> Option<UtcMillis> {
    match (context.clock_trust, context.now_utc_ms) {
        (ClockTrust::Trusted, Some(now)) => Some(now),
        _ => None,
    }
}

impl ReminderState {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn dismissals(&self) -> impl Iterator<Item = &DismissalRecord> {
        self.dismissals.values()
    }
    pub fn shown(&self) -> impl Iterator<Item = &ShownRecord> {
        self.shown.values()
    }
    pub fn dismissal_count(&self) -> usize {
        self.dismissals.len()
    }
    pub fn shown_count(&self) -> usize {
        self.shown.len()
    }
    fn take_serials(&mut self, count: usize) -> Result<u64> {
        let first = self.next_serial;
        let count = u64::try_from(count).map_err(|_| invalid("Too many reminder records"))?;
        match first.checked_add(count) {
            Some(end) if end <= MAX_SAFE_INTEGER => {
                self.next_serial = end;
                Ok(first)
            }
            _ => Err(invalid("Reminder serial space is exhausted")),
        }
    }
    /// Drops dismissals that are known not to apply any more.
    pub fn prune_inactive(&mut self, scope: &ReminderScope) {
        self.dismissals.retain(|_, record| record.applies(scope));
    }
    /// Records the user's "hide this reminder" under the rule's own policy.
    /// `Session` needs a session ID and `UntilRulesetChanges` a ruleset
    /// revision in `scope`.
    pub fn dismiss(
        &mut self,
        rule: &ContextRule,
        scope: &ReminderScope,
        now: &FreshnessContext,
    ) -> Result<()> {
        let (session, ruleset) = match rule.dismissal {
            DismissalPolicy::Session => match &scope.session {
                Some(session) => (Some(session.clone()), None),
                None => return Err(invalid("Session dismissal requires a session ID")),
            },
            DismissalPolicy::UntilRulesetChanges => match &scope.ruleset {
                Some(ruleset) => (None, Some(ruleset.clone())),
                None => return Err(invalid("Ruleset dismissal requires a ruleset revision")),
            },
            DismissalPolicy::Permanent => (None, None),
        };
        self.prune_inactive(scope);
        let key = (rule.context.clone(), rule.id.clone());
        let victim = if self.dismissals.contains_key(&key) || self.dismissals.len() < MAX_HISTORY {
            None
        } else {
            let oldest = self
                .dismissals
                .iter()
                .filter(|(_, record)| record.policy != DismissalPolicy::Permanent)
                .min_by_key(|(_, record)| record.serial)
                .map(|(key, _)| key.clone());
            Some(oldest.ok_or_else(|| invalid("Dismissal history is full of permanent entries"))?)
        };
        let serial = self.take_serials(1)?;
        if let Some(victim) = victim {
            self.dismissals.remove(&victim);
        }
        self.dismissals.insert(
            key,
            DismissalRecord {
                context: rule.context.clone(),
                rule: rule.id.clone(),
                policy: rule.dismissal,
                session,
                ruleset,
                dismissed_at_utc_ms: trusted_now(now),
                serial,
            },
        );
        Ok(())
    }
    /// Removes a dismissal (the preference control). True when one existed.
    pub fn restore(&mut self, rule: &ContextRule) -> bool {
        self.dismissals
            .remove(&(rule.context.clone(), rule.id.clone()))
            .is_some()
    }
    /// Records what the caller actually presented and re-arms rules that
    /// evaluated `False`. Every presented rule must be in the plan's queue.
    /// Nothing changes when this returns an error.
    pub fn commit(
        &mut self,
        plan: &ReminderPlan,
        presented: &[RuleId],
        now: &FreshnessContext,
    ) -> Result<()> {
        unique(presented.iter())?;
        let items = presented
            .iter()
            .map(|id| {
                plan.queue
                    .iter()
                    .find(|item| &item.rule == id)
                    .ok_or_else(|| invalid("Presented reminder is not in the plan queue"))
            })
            .collect::<Result<Vec<_>>>()?;
        let first = self.take_serials(items.len())?;
        let at = trusted_now(now);
        for rule in plan.rearm() {
            if let Some(record) = self.shown.get_mut(&(plan.context.clone(), rule.clone())) {
                record.fingerprint = None;
            }
        }
        for (offset, item) in (0u64..).zip(items) {
            let key = (plan.context.clone(), item.rule.clone());
            if !self.shown.contains_key(&key) && self.shown.len() >= MAX_HISTORY {
                // Evicting the oldest record can at worst allow one extra
                // presentation; it never hides advice.
                let oldest = self
                    .shown
                    .iter()
                    .min_by_key(|(_, record)| record.serial)
                    .map(|(key, _)| key.clone());
                if let Some(oldest) = oldest {
                    self.shown.remove(&oldest);
                }
            }
            self.shown.insert(
                key,
                ShownRecord {
                    context: plan.context.clone(),
                    rule: item.rule.clone(),
                    fingerprint: Some(item.fingerprint.clone()),
                    shown_at_utc_ms: at,
                    shown_at_session: now.session.clone(),
                    shown_session: plan.scope_session.clone(),
                    serial: first + offset,
                },
            );
        }
        Ok(())
    }
}

fn cooldown_gate(
    cooldown: Millis,
    record: &ShownRecord,
    now: &FreshnessContext,
    scope: &ReminderScope,
) -> Option<ReminderReason> {
    if cooldown.get() == 0 {
        return None;
    }
    let utc_elapsed = trusted_now(now)
        .zip(record.shown_at_utc_ms)
        .and_then(|(now, then)| now.get().checked_sub(then.get()))
        .and_then(|delta| u64::try_from(delta).ok());
    let session_elapsed = now
        .session
        .as_ref()
        .zip(record.shown_at_session.as_ref())
        .filter(|((a, _), (b, _))| a == b)
        .and_then(|((_, now), (_, then))| now.get().checked_sub(then.get()));
    match utc_elapsed.or(session_elapsed) {
        Some(elapsed) if elapsed >= cooldown.get() => None,
        Some(_) => Some(ReminderReason::CooldownActive),
        None if record.fingerprint.is_none()
            || scope
                .session
                .as_ref()
                .zip(record.shown_session.as_ref())
                .is_some_and(|(a, b)| a != b)
            || now
                .session
                .as_ref()
                .zip(record.shown_at_session.as_ref())
                .is_some_and(|((a, _), (b, _))| a != b) =>
        {
            None
        }
        None => Some(ReminderReason::CooldownUnverifiable),
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct ReminderEngine {
    pub policy: ReminderPolicy,
}
impl ReminderEngine {
    pub fn new(policy: ReminderPolicy) -> Self {
        Self { policy }
    }
    /// Evaluates `rules` against the snapshot in `context` and builds the plan.
    /// Pure: `state` is only read; call [`ReminderState::commit`] afterwards.
    pub fn plan(
        &self,
        rules: &[ContextRule],
        context: &EvaluationContext<'_>,
        inputs: &ReminderInputs,
        state: &ReminderState,
    ) -> Result<ReminderPlan> {
        unique(rules.iter().map(|rule| &rule.id))?;
        if let Some(triggers) = &inputs.fired_triggers {
            bounded(triggers.len())?;
        }
        let snapshot = context.snapshot;
        let basis = match self.policy.fingerprint {
            FingerprintBasis::Snapshot => Some(&snapshot.id),
            FingerprintBasis::OutcomeOnly => None,
        };
        let mut excluded = BTreeMap::new();
        let mut rearm = BTreeSet::new();
        let mut candidates = Vec::new();
        for rule in rules {
            if let Some(fired) = &inputs.fired_triggers
                && !fired.contains(&rule.trigger)
            {
                excluded.insert(rule.id.clone(), ReminderReason::TriggerNotFired);
                continue;
            }
            let result = evaluate_rule(rule, context)?;
            let kind = match result.truth {
                Truth::True => ReminderKind::Advice {
                    text: rule.result_template.clone(),
                },
                Truth::False => {
                    let expired = result
                        .reasons
                        .iter()
                        .any(|issue| issue.reason == Reason::ExpiredRule);
                    let reason = if expired {
                        ReminderReason::RuleExpired
                    } else {
                        ReminderReason::ConditionFalse
                    };
                    rearm.insert(rule.id.clone());
                    excluded.insert(rule.id.clone(), reason);
                    continue;
                }
                Truth::Unknown => match self.policy.unknown {
                    UnknownPolicy::Omit => {
                        excluded.insert(
                            rule.id.clone(),
                            ReminderReason::InputsUnknown {
                                reasons: result.reasons,
                            },
                        );
                        continue;
                    }
                    UnknownPolicy::Informational => ReminderKind::CannotEvaluate {
                        reasons: result.reasons.clone(),
                    },
                },
            };
            candidates.push(ReminderItem {
                rule: rule.id.clone(),
                deduplication_key: rule.deduplication_key.clone(),
                priority: rule.priority,
                kind,
                warnings: result.warnings,
                evidence: rule.evidence.clone(),
                dismissal: rule.dismissal,
                cooldown_ms: rule.cooldown_ms,
                fingerprint: StateFingerprint::new(&rule.id, basis, result.truth, &result.reasons)?,
            });
        }
        candidates.sort_by(|a, b| {
            a.strength()
                .cmp(&b.strength())
                .then(b.priority.cmp(&a.priority))
                .then_with(|| a.rule.cmp(&b.rule))
        });

        let mode = inputs.suppression.assess(context.freshness);
        let mut survivors: BTreeMap<RuleId, RuleId> = BTreeMap::new();
        let mut queue = Vec::new();
        let mut held = Vec::new();
        for item in candidates {
            if let Some(kept) = survivors.get(&item.deduplication_key) {
                excluded.insert(
                    item.rule.clone(),
                    ReminderReason::Deduplicated { kept: kept.clone() },
                );
                continue;
            }
            survivors.insert(item.deduplication_key.clone(), item.rule.clone());
            let key = (snapshot.context.clone(), item.rule.clone());
            let blocked = if let Some(dismissal) = state
                .dismissals
                .get(&key)
                .filter(|record| record.applies(&inputs.scope))
            {
                Some(ReminderReason::Dismissed {
                    policy: dismissal.policy,
                })
            } else if let Some(record) = state.shown.get(&key) {
                if record.fingerprint.as_ref() == Some(&item.fingerprint) {
                    Some(ReminderReason::StateUnchanged)
                } else {
                    cooldown_gate(item.cooldown_ms, record, context.freshness, &inputs.scope)
                }
            } else {
                None
            };
            if let Some(reason) = blocked {
                excluded.insert(item.rule.clone(), reason);
            } else if mode == PresentationMode::Suppressed {
                excluded.insert(item.rule.clone(), ReminderReason::Suppressed);
                held.push(item);
            } else {
                queue.push(item);
            }
        }
        Ok(ReminderPlan {
            scope_session: inputs.scope.session.clone(),
            context: snapshot.context.clone(),
            mode,
            queue,
            held,
            excluded,
            rearm,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn issue_keys_are_sorted_unique_and_stable() {
        let who = CharacterId::new("c1").unwrap();
        let issue = |reason| EvaluationIssue {
            participant: who.clone(),
            input: InputPath::Location,
            reason,
        };
        let reasons = [
            issue(Reason::Stale),
            issue(Reason::ClockUncertain),
            issue(Reason::Stale),
        ];
        let rule = RuleId::new("r1").unwrap();
        let fingerprint = StateFingerprint::new(&rule, None, Truth::Unknown, &reasons).unwrap();
        assert_eq!(fingerprint.issues.len(), 2);
        assert!(fingerprint.issues[0] < fingerprint.issues[1]);
        assert_eq!(
            fingerprint.issues[0],
            r#"{"participant":"c1","input":{"input":"location"},"reason":{"reason":"clock_uncertain"}}"#
        );
    }

    #[test]
    fn cooldown_is_conservative_without_a_trusted_forward_clock() {
        let record = |at: Option<i64>| ShownRecord {
            context: DataContext {
                build: BuildKey {
                    edition: EditionId::new("e").unwrap(),
                    build: BuildId::new("b").unwrap(),
                    interface: InterfaceId::new("i").unwrap(),
                },
                namespace: DatasetNamespace::Demo,
            },
            rule: RuleId::new("r1").unwrap(),
            fingerprint: Some(
                StateFingerprint::new(&RuleId::new("r1").unwrap(), None, Truth::True, &[]).unwrap(),
            ),
            shown_at_session: None,
            shown_session: None,
            shown_at_utc_ms: at.map(|n| UtcMillis::new(n).unwrap()),
            serial: 0,
        };
        let ms = |n| Millis::new(n).unwrap();
        let clock = |now| FreshnessContext {
            now_utc_ms: now,
            clock_trust: ClockTrust::Trusted,
            session: None,
            max_age_ms: ms(1000),
        };
        let utc = |n| Some(UtcMillis::new(n).unwrap());
        assert_eq!(
            cooldown_gate(
                ms(0),
                &record(None),
                &clock(None),
                &ReminderScope::default()
            ),
            None
        );
        assert_eq!(
            cooldown_gate(
                ms(10),
                &record(Some(100)),
                &clock(utc(110)),
                &ReminderScope::default()
            ),
            None
        );
        assert_eq!(
            cooldown_gate(
                ms(10),
                &record(Some(100)),
                &clock(utc(109)),
                &ReminderScope::default()
            ),
            Some(ReminderReason::CooldownActive)
        );
        for (then, now) in [(Some(100), utc(99)), (None, utc(500)), (Some(100), None)] {
            assert_eq!(
                cooldown_gate(
                    ms(10),
                    &record(then),
                    &clock(now),
                    &ReminderScope::default()
                ),
                Some(ReminderReason::CooldownUnverifiable)
            );
        }
    }
}
