//! Per-participant quest eligibility over four separate phases (acceptance,
//! progress, completion, turn-in) and party aggregation. Pure: no I/O, no clock.
//! Only `Truth::True` is a definite affirmative; unknown history is never read as
//! false, incomplete or absent. Reasons are built while evaluating, from the
//! evaluation trace (`EvaluationIssue`) and from the predicate leaves that
//! evaluated to false, never written afterwards. Errors from `evaluate()` (step
//! limit, build mismatch) propagate and are never folded into `Unknown`.
//! Synthetic engineering semantics: nothing here is verified against a live client.
use crate::{
    entities::{bounded, *},
    error::{Result, Validate, Validated, invalid},
    evaluation::*,
    observation::*,
    predicate::*,
    types::*,
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

/// Class, race and faction allow-lists. `None` means "no restriction". The
/// contract cannot express these in `Predicate`, so this is a local input type.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct AccessRequirementsData {
    #[serde(default)]
    pub classes: Option<BTreeSet<SourceId>>,
    #[serde(default)]
    pub races: Option<BTreeSet<SourceId>>,
    #[serde(default)]
    pub factions: Option<BTreeSet<SourceId>>,
}
pub type AccessRequirements = Validated<AccessRequirementsData>;
impl Validate for AccessRequirementsData {
    fn validate(&self) -> Result<()> {
        for set in [&self.classes, &self.races, &self.factions]
            .into_iter()
            .flatten()
        {
            bounded(set.len())?;
            if set.is_empty() {
                return Err(invalid("An allowed set must not be empty"));
            }
        }
        Ok(())
    }
}

/// Class and race come from `CharacterProfile`; the faction is caller-supplied.
#[derive(Clone, Debug, PartialEq)]
pub struct ParticipantFacts {
    pub context: DataContext,
    pub character: CharacterId,
    pub class: Observation<SourceId>,
    pub race: Observation<SourceId>,
    pub faction: Observation<SourceId>,
}
impl ParticipantFacts {
    pub fn from_profile(profile: &CharacterProfile, faction: Observation<SourceId>) -> Self {
        Self {
            context: profile.context.clone(),
            character: profile.id.clone(),
            class: profile.class.clone(),
            race: profile.race.clone(),
            faction,
        }
    }
}

/// When another group member counts as "choice already made".
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ChoiceTaken {
    #[default]
    TurnedIn,
    /// Accepted or turned in.
    Accepted,
}
/// Mutually exclusive quests. There is no such field in `QuestDefinition`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct ExclusiveGroupData {
    pub quests: BTreeSet<QuestId>,
    #[serde(default)]
    pub taken_when: ChoiceTaken,
}
pub type ExclusiveGroup = Validated<ExclusiveGroupData>;
impl Validate for ExclusiveGroupData {
    fn validate(&self) -> Result<()> {
        bounded(self.quests.len())?;
        if self.quests.len() < 2 {
            return Err(invalid("An exclusive group needs at least two quests"));
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Phase {
    Acceptance,
    Progress,
    Completion,
    TurnIn,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum DefinitionField {
    Eligibility,
    Prerequisites,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum AccessFact {
    Class,
    Race,
    Faction,
}
/// The observation a stale or clock-uncertain reason refers to.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ObservedInput {
    Evaluated { path: InputPath },
    Access { fact: AccessFact },
    Stage { objective: ObjectiveId },
}
/// One unmet leaf. Only quest and level requirements can be caught up on.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Requirement {
    Quest {
        quest: QuestId,
        state: QuestState,
    },
    Level {
        level: Level,
    },
    /// Negations, roles, equipment, locations, preferences, other participants.
    Other,
}
/// The false part of a predicate: what would have to change.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Unmet {
    Single { requirement: Requirement },
    AllOf { items: Vec<Unmet> },
    OneOf { alternatives: Vec<Unmet> },
}
impl Unmet {
    /// True when the unmet part consists only of things a participant can still do.
    pub fn can_catch_up(&self) -> bool {
        match self {
            Self::Single { requirement } => !matches!(requirement, Requirement::Other),
            Self::AllOf { items } => items.iter().all(Self::can_catch_up),
            Self::OneOf { alternatives } => alternatives.iter().any(Self::can_catch_up),
        }
    }
}

/// Closed reason codes. Details never carry free text from imported data.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(tag = "code", rename_all = "snake_case")]
pub enum QuestEligibilityCode {
    RequirementDataUnknown {
        field: DefinitionField,
        status: ObservationStatus,
    },
    QuestHistoryUnknown {
        history_quest: QuestId,
        state: QuestState,
        status: Option<ObservationStatus>,
    },
    LevelUnknown {
        status: Option<ObservationStatus>,
    },
    ConditionInputUnknown {
        input: InputPath,
        status: Option<ObservationStatus>,
    },
    StageUnknown {
        objective: ObjectiveId,
        status: Option<ObservationStatus>,
    },
    PrerequisiteUnmet {
        requirement: Requirement,
    },
    AlternativePrerequisiteUnmet {
        requirement: Requirement,
    },
    LevelBelowMinimum {
        level: Level,
    },
    AccessClassMismatch,
    AccessRaceMismatch,
    AccessFactionMismatch,
    AccessFactUnknown {
        fact: AccessFact,
        status: Option<ObservationStatus>,
    },
    ExclusiveChoiceTaken {
        other_quest: QuestId,
    },
    ExclusiveChoiceUnknown {
        other_quest: QuestId,
        status: Option<ObservationStatus>,
    },
    AlreadyTurnedIn,
    ResetStateUnknown,
    RepeatabilityUnknown {
        status: ObservationStatus,
    },
    ObservationStale {
        input: ObservedInput,
    },
    ClockUncertain {
        input: ObservedInput,
    },
    /// Defensive: `evaluate()` itself never reports an expired rule.
    RuleExpired {
        input: ObservedInput,
    },
    CatchUpRequired {
        unmet: Unmet,
    },
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct QuestEligibilityReason {
    pub participant: CharacterId,
    pub quest: QuestId,
    pub phase: Phase,
    #[serde(flatten)]
    pub code: QuestEligibilityCode,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum AcceptanceOutcome {
    Eligible,
    /// Definitely not yet, but only through things the participant can still do.
    Behind,
    /// Definitely not, and no catch-up changes that.
    Blocked,
    Unknown,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ProgressOutcome {
    NotStarted,
    InProgress,
    AllStagesDone,
    Unknown,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CompletionOutcome {
    Complete,
    Incomplete,
    Unknown,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum TurnInOutcome {
    Ready,
    NotReady,
    AlreadyTurnedIn,
    Unknown,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct ParticipantEligibility {
    pub participant: CharacterId,
    pub quest: QuestId,
    pub acceptance: AcceptanceOutcome,
    pub progress: ProgressOutcome,
    pub completion: CompletionOutcome,
    pub turn_in: TurnInOutcome,
    /// Present exactly when acceptance is `Behind`.
    pub catch_up: Option<Unmet>,
    /// Every reason, in evaluation order.
    pub reasons: Vec<QuestEligibilityReason>,
    /// The reasons that make acceptance `Blocked`; empty otherwise.
    pub blockers: Vec<QuestEligibilityReason>,
    /// Stale observations used under `StalePolicy::AllowStale`.
    pub warnings: Vec<QuestEligibilityReason>,
}

/// Everything except the participant. Facts missing for a participant are unknown.
pub struct EligibilityInputs<'a> {
    pub definition: &'a QuestDefinition,
    pub access: Option<&'a AccessRequirements>,
    pub exclusive_groups: &'a [ExclusiveGroup],
    pub facts: &'a BTreeMap<CharacterId, ParticipantFacts>,
    pub freshness: &'a FreshnessContext,
    pub stale_policy: StalePolicy,
    pub limits: RuleLimits,
}

/// Acceptance outcome for a party. `JointlyEligible` means each participant is
/// independently eligible; it says nothing about shared progress or shareability.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(tag = "outcome", rename_all = "snake_case")]
pub enum PartyOutcome {
    JointlyEligible,
    EligibleForSome {
        eligible: Vec<CharacterId>,
        behind: Vec<CharacterId>,
        catch_up: Vec<QuestEligibilityReason>,
    },
    /// Nobody can accept yet, but nobody is blocked or unknown either.
    AllBehind {
        catch_up: Vec<QuestEligibilityReason>,
    },
    Blocked {
        reasons: Vec<QuestEligibilityReason>,
    },
    Unknown {
        missing: Vec<QuestEligibilityReason>,
    },
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct PartyEligibility {
    pub quest: QuestId,
    pub participants: Vec<ParticipantEligibility>,
    pub outcome: PartyOutcome,
}

fn node_predicate(node: &PredicateNode) -> Result<Predicate> {
    Predicate::try_new(node.clone())
}
fn quest_node(quest: &QuestId, state: QuestState) -> PredicateNode {
    PredicateNode::QuestState {
        quest: quest.clone(),
        state,
        value: true,
    }
}
fn issue_code(issue: &EvaluationIssue) -> QuestEligibilityCode {
    let input = || ObservedInput::Evaluated {
        path: issue.input.clone(),
    };
    let status = match &issue.reason {
        Reason::Unavailable { status } => Some(*status),
        _ => None,
    };
    match (&issue.reason, &issue.input) {
        (Reason::Stale, _) => QuestEligibilityCode::ObservationStale { input: input() },
        (Reason::ClockUncertain, _) => QuestEligibilityCode::ClockUncertain { input: input() },
        (Reason::ExpiredRule, _) => QuestEligibilityCode::RuleExpired { input: input() },
        (
            _,
            InputPath::Field {
                field: Field::Quest { quest, state },
            },
        ) => QuestEligibilityCode::QuestHistoryUnknown {
            history_quest: quest.clone(),
            state: *state,
            status,
        },
        (
            _,
            InputPath::Field {
                field: Field::Level,
            },
        ) => QuestEligibilityCode::LevelUnknown { status },
        _ => QuestEligibilityCode::ConditionInputUnknown {
            input: issue.input.clone(),
            status,
        },
    }
}
fn unmet_codes(unmet: &Unmet, alternative: bool, out: &mut Vec<QuestEligibilityCode>) {
    match unmet {
        Unmet::Single { requirement } => out.push(match requirement {
            Requirement::Level { level } if !alternative => {
                QuestEligibilityCode::LevelBelowMinimum { level: *level }
            }
            requirement if alternative => QuestEligibilityCode::AlternativePrerequisiteUnmet {
                requirement: requirement.clone(),
            },
            requirement => QuestEligibilityCode::PrerequisiteUnmet {
                requirement: requirement.clone(),
            },
        }),
        Unmet::AllOf { items } => items.iter().for_each(|u| unmet_codes(u, alternative, out)),
        Unmet::OneOf { alternatives } => {
            alternatives.iter().for_each(|u| unmet_codes(u, true, out))
        }
    }
}
fn all_of(mut items: Vec<Unmet>) -> Unmet {
    if items.len() == 1 {
        items.remove(0)
    } else {
        Unmet::AllOf { items }
    }
}

struct Component {
    truth: Truth,
}

struct Run<'a> {
    inputs: &'a EligibilityInputs<'a>,
    snapshot: &'a CharacterSnapshot,
    party: Option<&'a PartySnapshot>,
    reasons: Vec<QuestEligibilityReason>,
    blockers: Vec<QuestEligibilityReason>,
    warnings: Vec<QuestEligibilityReason>,
    catch_ups: Vec<Unmet>,
}
impl<'a> Run<'a> {
    fn quest(&self) -> &'a QuestId {
        &self.inputs.definition.id
    }
    fn reason(
        &self,
        phase: Phase,
        participant: &CharacterId,
        code: QuestEligibilityCode,
    ) -> QuestEligibilityReason {
        QuestEligibilityReason {
            participant: participant.clone(),
            quest: self.quest().clone(),
            phase,
            code,
        }
    }
    fn note(&mut self, phase: Phase, code: QuestEligibilityCode) {
        let who = self.snapshot.character.clone();
        self.note_for(phase, &who, code);
    }
    fn note_for(&mut self, phase: Phase, who: &CharacterId, code: QuestEligibilityCode) {
        let reason = self.reason(phase, who, code);
        if !self.reasons.contains(&reason) {
            self.reasons.push(reason);
        }
    }
    fn block(&mut self, phase: Phase, code: QuestEligibilityCode) {
        let reason = self.reason(phase, &self.snapshot.character, code.clone());
        self.note(phase, code);
        if !self.blockers.contains(&reason) {
            self.blockers.push(reason);
        }
    }
    fn evaluate(&self, predicate: &Predicate) -> Result<EvaluationResult> {
        evaluate(
            predicate,
            &EvaluationContext {
                snapshot: self.snapshot,
                party: self.party,
                freshness: self.inputs.freshness,
                stale_policy: self.inputs.stale_policy,
                limits: self.inputs.limits,
            },
        )
    }
    fn truth_of(&self, node: &PredicateNode) -> Result<Truth> {
        Ok(self.evaluate(&node_predicate(node)?)?.truth)
    }
    /// Maps the evaluation trace into reasons and warnings for one phase.
    fn absorb(&mut self, phase: Phase, result: &EvaluationResult) {
        for issue in &result.reasons {
            self.note_for(phase, &issue.participant, issue_code(issue));
        }
        for issue in &result.warnings {
            let reason = self.reason(phase, &issue.participant, issue_code(issue));
            if !self.warnings.contains(&reason) {
                self.warnings.push(reason);
            }
        }
    }
    /// Usable value of a snapshot-side observation, with the evaluator's staleness rules.
    fn observe<'o, T>(
        &mut self,
        obs: &'o Observation<T>,
        phase: Phase,
        input: ObservedInput,
        unknown: impl FnOnce(Option<ObservationStatus>) -> QuestEligibilityCode,
    ) -> Option<&'o T> {
        let Some(value) = obs.value() else {
            self.note(phase, unknown(Some(obs.status())));
            return None;
        };
        match freshness(obs.metadata(), self.inputs.freshness) {
            Freshness::Fresh { .. } => Some(value),
            Freshness::Stale { .. } if self.inputs.stale_policy == StalePolicy::AllowStale => {
                let reason = self.reason(
                    phase,
                    &self.snapshot.character,
                    QuestEligibilityCode::ObservationStale { input },
                );
                if !self.warnings.contains(&reason) {
                    self.warnings.push(reason);
                }
                Some(value)
            }
            Freshness::Stale { .. } => {
                self.note(phase, QuestEligibilityCode::ObservationStale { input });
                None
            }
            Freshness::Uncertain { .. } => {
                self.note(phase, QuestEligibilityCode::ClockUncertain { input });
                None
            }
        }
    }
    fn has_unknown(&self, node: &PredicateNode) -> Result<bool> {
        if self.truth_of(node)? == Truth::Unknown {
            return Ok(true);
        }
        match node {
            PredicateNode::All { predicates } | PredicateNode::Any { predicates } => {
                for child in predicates {
                    if self.has_unknown(child)? {
                        return Ok(true);
                    }
                }
                Ok(false)
            }
            PredicateNode::Not { predicate } => self.has_unknown(predicate),
            _ => Ok(false),
        }
    }
    /// The unmet part of a predicate that evaluated to false.
    fn unmet(&self, node: &PredicateNode) -> Result<Unmet> {
        Ok(match node {
            PredicateNode::All { predicates } => {
                let mut items = vec![];
                for p in predicates {
                    if self.truth_of(p)? == Truth::False {
                        items.push(self.unmet(p)?);
                    }
                }
                all_of(items)
            }
            PredicateNode::Any { predicates } => Unmet::OneOf {
                alternatives: predicates
                    .iter()
                    .map(|p| self.unmet(p))
                    .collect::<Result<_>>()?,
            },
            PredicateNode::QuestState {
                quest,
                state,
                value: true,
            } => Unmet::Single {
                requirement: Requirement::Quest {
                    quest: quest.clone(),
                    state: *state,
                },
            },
            PredicateNode::LevelAtLeast { level } => Unmet::Single {
                requirement: Requirement::Level { level: *level },
            },
            _ => Unmet::Single {
                requirement: Requirement::Other,
            },
        })
    }
    /// A definition predicate: unknown requirement data is never satisfied.
    fn requirement(
        &mut self,
        field: DefinitionField,
        obs: &Observation<Predicate>,
    ) -> Result<Component> {
        let Some(predicate) = obs.value() else {
            self.note(
                Phase::Acceptance,
                QuestEligibilityCode::RequirementDataUnknown {
                    field,
                    status: obs.status(),
                },
            );
            return Ok(Component {
                truth: Truth::Unknown,
            });
        };
        let result = self.evaluate(predicate)?;
        self.absorb(Phase::Acceptance, &result);
        if result.truth == Truth::False {
            let unmet = self.unmet(predicate)?;
            let mut codes = vec![];
            unmet_codes(&unmet, false, &mut codes);
            let catchable = unmet.can_catch_up();
            for code in codes {
                if catchable {
                    self.note(Phase::Acceptance, code);
                } else {
                    self.block(Phase::Acceptance, code);
                }
            }
            if catchable {
                self.catch_ups.push(unmet);
            }
        }
        Ok(Component {
            truth: if result.truth == Truth::False && self.has_unknown(predicate)? {
                Truth::Unknown
            } else {
                result.truth
            },
        })
    }
    fn access(&mut self) -> Component {
        let Some(access) = self.inputs.access else {
            return Component { truth: Truth::True };
        };
        let facts = self.inputs.facts.get(&self.snapshot.character);
        let mut truth = Truth::True;
        let checks = [
            (
                AccessFact::Class,
                &access.classes,
                QuestEligibilityCode::AccessClassMismatch,
            ),
            (
                AccessFact::Race,
                &access.races,
                QuestEligibilityCode::AccessRaceMismatch,
            ),
            (
                AccessFact::Faction,
                &access.factions,
                QuestEligibilityCode::AccessFactionMismatch,
            ),
        ];
        for (fact, allowed, mismatch) in checks {
            let Some(allowed) = allowed else { continue };
            let observed = facts.map(|f| match fact {
                AccessFact::Class => &f.class,
                AccessFact::Race => &f.race,
                AccessFact::Faction => &f.faction,
            });
            let unknown = move |status| QuestEligibilityCode::AccessFactUnknown { fact, status };
            let value = match observed {
                Some(obs) => self.observe(
                    obs,
                    Phase::Acceptance,
                    ObservedInput::Access { fact },
                    unknown,
                ),
                None => {
                    self.note(Phase::Acceptance, unknown(None));
                    None
                }
            };
            truth = truth.and(match value {
                None => Truth::Unknown,
                Some(v) if allowed.contains(v) => Truth::True,
                Some(_) => {
                    self.block(Phase::Acceptance, mismatch);
                    Truth::False
                }
            });
        }
        Component { truth }
    }
    fn exclusive(&mut self) -> Result<Component> {
        let mut taken = Truth::False;
        let (groups, me) = (self.inputs.exclusive_groups, self.quest());
        for group in groups.iter().filter(|g| g.quests.contains(me)) {
            for other in group.quests.iter().filter(|q| *q != me) {
                let turned_in = quest_node(other, QuestState::TurnedIn);
                let node = match group.taken_when {
                    ChoiceTaken::TurnedIn => turned_in,
                    ChoiceTaken::Accepted => PredicateNode::Any {
                        predicates: vec![quest_node(other, QuestState::Accepted), turned_in],
                    },
                };
                let result = self.evaluate(&node_predicate(&node)?)?;
                self.absorb_exclusive(other, &result);
                if result.truth == Truth::True {
                    self.block(
                        Phase::Acceptance,
                        QuestEligibilityCode::ExclusiveChoiceTaken {
                            other_quest: other.clone(),
                        },
                    );
                }
                taken = taken.or(result.truth);
            }
        }
        Ok(Component {
            truth: taken.negate(),
        })
    }
    fn absorb_exclusive(&mut self, other: &QuestId, result: &EvaluationResult) {
        let is_other = |issue: &EvaluationIssue| matches!(&issue.input, InputPath::Field { field: Field::Quest { quest, .. } } if quest == other);
        for issue in &result.reasons {
            let code = match (issue_code(issue), is_other(issue)) {
                (QuestEligibilityCode::QuestHistoryUnknown { status, .. }, true) => {
                    QuestEligibilityCode::ExclusiveChoiceUnknown {
                        other_quest: other.clone(),
                        status,
                    }
                }
                (code, _) => code,
            };
            self.note_for(Phase::Acceptance, &issue.participant, code);
        }
        for issue in &result.warnings {
            let reason = self.reason(Phase::Acceptance, &issue.participant, issue_code(issue));
            if !self.warnings.contains(&reason) {
                self.warnings.push(reason);
            }
        }
    }
    fn repeatability(&mut self, turned_in: &EvaluationResult) -> Component {
        let completion = &self.inputs.definition.completion;
        let Some(semantics) = completion.value() else {
            self.note(
                Phase::Acceptance,
                QuestEligibilityCode::RepeatabilityUnknown {
                    status: completion.status(),
                },
            );
            return Component {
                truth: Truth::Unknown,
            };
        };
        if *semantics == CompletionSemantics::ResetBound && turned_in.truth != Truth::False {
            self.absorb(Phase::Acceptance, turned_in);
            self.note(Phase::Acceptance, QuestEligibilityCode::ResetStateUnknown);
            return Component {
                truth: Truth::Unknown,
            };
        }
        if *semantics != CompletionSemantics::Unrepeatable {
            return Component { truth: Truth::True };
        }
        self.absorb(Phase::Acceptance, turned_in);
        if turned_in.truth == Truth::True {
            self.block(Phase::Acceptance, QuestEligibilityCode::AlreadyTurnedIn);
        }
        Component {
            truth: turned_in.truth.negate(),
        }
    }
    fn stages(&mut self) -> Truth {
        let definition = self.inputs.definition;
        let Some(record) = self.snapshot.quests.get(&definition.id) else {
            return Truth::Unknown;
        };
        let mut truth = Truth::True;
        for stage in &definition.stages {
            let unknown = |status| QuestEligibilityCode::StageUnknown {
                objective: stage.clone(),
                status,
            };
            let value = match record.objectives.get(stage) {
                Some(obs) => self.observe(
                    obs,
                    Phase::Progress,
                    ObservedInput::Stage {
                        objective: stage.clone(),
                    },
                    unknown,
                ),
                None => {
                    self.note(Phase::Progress, unknown(None));
                    None
                }
            };
            truth = truth.and(value.map_or(Truth::Unknown, |done| Truth::from_bool(*done)));
        }
        truth
    }
}

fn run_participant(
    inputs: &EligibilityInputs<'_>,
    snapshot: &CharacterSnapshot,
    party: Option<&PartySnapshot>,
) -> Result<ParticipantEligibility> {
    bounded(inputs.exclusive_groups.len())?;
    bounded(inputs.facts.len())?;
    let definition = inputs.definition;
    definition.context.ensure_compatible(&snapshot.context)?;
    if let Some(party) = party {
        definition.context.ensure_compatible(&party.context)?;
    }
    if let Some(facts) = inputs.facts.get(&snapshot.character) {
        facts.context.ensure_compatible(&snapshot.context)?;
        if facts.character != snapshot.character {
            return Err(invalid(
                "Participant facts identity does not match snapshot",
            ));
        }
    }
    let mut run = Run {
        inputs,
        snapshot,
        party,
        reasons: vec![],
        blockers: vec![],
        warnings: vec![],
        catch_ups: vec![],
    };
    let quest = &definition.id;
    let accepted = run.evaluate(&node_predicate(&quest_node(quest, QuestState::Accepted))?)?;
    let completed = run.evaluate(&node_predicate(&quest_node(quest, QuestState::Completed))?)?;
    let turned_in = run.evaluate(&node_predicate(&quest_node(quest, QuestState::TurnedIn))?)?;
    // Acceptance: every component is evaluated so that all reasons are reported.
    let components = [
        run.requirement(DefinitionField::Eligibility, &definition.eligibility)?
            .truth,
        run.requirement(DefinitionField::Prerequisites, &definition.prerequisites)?
            .truth,
        run.access().truth,
        run.exclusive()?.truth,
        run.repeatability(&turned_in).truth,
    ];
    let unresolved = components.contains(&Truth::Unknown)
        || run.reasons.iter().any(|r| {
            r.phase == Phase::Acceptance
                && matches!(
                    r.code,
                    QuestEligibilityCode::QuestHistoryUnknown { .. }
                        | QuestEligibilityCode::ObservationStale { .. }
                        | QuestEligibilityCode::ClockUncertain { .. }
                )
        });
    let truth = components.into_iter().fold(Truth::True, Truth::and);
    let acceptance = match truth {
        Truth::True => AcceptanceOutcome::Eligible,
        Truth::Unknown => AcceptanceOutcome::Unknown,
        Truth::False if run.blockers.is_empty() && unresolved => AcceptanceOutcome::Unknown,
        Truth::False if run.blockers.is_empty() => AcceptanceOutcome::Behind,
        Truth::False => AcceptanceOutcome::Blocked,
    };
    let catch_up = (acceptance == AcceptanceOutcome::Behind)
        .then(|| all_of(std::mem::take(&mut run.catch_ups)));
    let progress = match (completed.truth, turned_in.truth, accepted.truth) {
        (Truth::True, _, _) | (_, Truth::True, _) => ProgressOutcome::AllStagesDone,
        (Truth::False, Truth::False, Truth::False) => ProgressOutcome::NotStarted,
        (_, _, Truth::Unknown | Truth::False) => {
            run.absorb(Phase::Progress, &completed);
            run.absorb(Phase::Progress, &turned_in);
            run.absorb(Phase::Progress, &accepted);
            ProgressOutcome::Unknown
        }
        (_, _, Truth::True) => match run.stages() {
            Truth::True => ProgressOutcome::AllStagesDone,
            Truth::False => ProgressOutcome::InProgress,
            Truth::Unknown => ProgressOutcome::Unknown,
        },
    };
    run.absorb(Phase::Completion, &completed);
    let completion = match completed.truth {
        Truth::True => CompletionOutcome::Complete,
        Truth::False => CompletionOutcome::Incomplete,
        Truth::Unknown => CompletionOutcome::Unknown,
    };
    run.absorb(Phase::TurnIn, &completed);
    run.absorb(Phase::TurnIn, &turned_in);
    let turn_in = match (completed.truth, turned_in.truth) {
        (_, Truth::True) => TurnInOutcome::AlreadyTurnedIn,
        (Truth::True, Truth::False) => TurnInOutcome::Ready,
        (Truth::False, Truth::False) => TurnInOutcome::NotReady,
        _ => TurnInOutcome::Unknown,
    };
    Ok(ParticipantEligibility {
        participant: snapshot.character.clone(),
        quest: quest.clone(),
        acceptance,
        progress,
        completion,
        turn_in,
        catch_up,
        reasons: run.reasons,
        blockers: if acceptance == AcceptanceOutcome::Blocked {
            run.blockers
        } else {
            vec![]
        },
        warnings: run.warnings,
    })
}

/// One participant, optionally inside a party (for `participant_state` predicates).
/// Errors from rule evaluation and data-context mismatches are returned as `Err`.
pub fn evaluate_participant(
    inputs: &EligibilityInputs<'_>,
    snapshot: &CharacterSnapshot,
    party: Option<&PartySnapshot>,
) -> Result<ParticipantEligibility> {
    run_participant(inputs, snapshot, party)
}

/// Evaluates each participant separately, then aggregates acceptance. Blocked
/// outranks unknown, and unknown outranks every affirmative or catch-up outcome.
pub fn evaluate_party(
    inputs: &EligibilityInputs<'_>,
    party: &PartySnapshot,
) -> Result<PartyEligibility> {
    let participants = party
        .participants
        .iter()
        .map(|p| run_participant(inputs, p, Some(party)))
        .collect::<Result<Vec<_>>>()?;
    let of = |outcome| participants.iter().filter(move |p| p.acceptance == outcome);
    let ids = |outcome| {
        of(outcome)
            .map(|p| p.participant.clone())
            .collect::<Vec<_>>()
    };
    let outcome = if of(AcceptanceOutcome::Blocked).next().is_some() {
        PartyOutcome::Blocked {
            reasons: of(AcceptanceOutcome::Blocked)
                .flat_map(|p| p.blockers.clone())
                .collect(),
        }
    } else if of(AcceptanceOutcome::Unknown).next().is_some() {
        PartyOutcome::Unknown {
            missing: of(AcceptanceOutcome::Unknown)
                .flat_map(|p| p.reasons.clone())
                .collect(),
        }
    } else if of(AcceptanceOutcome::Behind).next().is_none() {
        PartyOutcome::JointlyEligible
    } else {
        let catch_up = of(AcceptanceOutcome::Behind)
            .filter_map(|p| {
                let unmet = p.catch_up.clone()?;
                Some(QuestEligibilityReason {
                    participant: p.participant.clone(),
                    quest: p.quest.clone(),
                    phase: Phase::Acceptance,
                    code: QuestEligibilityCode::CatchUpRequired { unmet },
                })
            })
            .collect();
        if of(AcceptanceOutcome::Eligible).next().is_some() {
            PartyOutcome::EligibleForSome {
                eligible: ids(AcceptanceOutcome::Eligible),
                behind: ids(AcceptanceOutcome::Behind),
                catch_up,
            }
        } else {
            PartyOutcome::AllBehind { catch_up }
        }
    };
    Ok(PartyEligibility {
        quest: inputs.definition.id.clone(),
        participants,
        outcome,
    })
}
