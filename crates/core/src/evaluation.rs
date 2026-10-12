//! Pure, bounded three-valued evaluation over explicitly supplied snapshots.
use crate::{
    entities::*,
    error::{Result, invalid},
    observation::*,
    predicate::*,
    types::*,
};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Truth {
    True,
    False,
    Unknown,
}
impl Truth {
    pub fn from_bool(value: bool) -> Self {
        if value { Self::True } else { Self::False }
    }
    pub fn negate(self) -> Self {
        match self {
            Self::True => Self::False,
            Self::False => Self::True,
            Self::Unknown => Self::Unknown,
        }
    }
    pub fn and(self, other: Self) -> Self {
        match (self, other) {
            (Self::False, _) | (_, Self::False) => Self::False,
            (Self::True, Self::True) => Self::True,
            _ => Self::Unknown,
        }
    }
    pub fn or(self, other: Self) -> Self {
        match (self, other) {
            (Self::True, _) | (_, Self::True) => Self::True,
            (Self::False, Self::False) => Self::False,
            _ => Self::Unknown,
        }
    }
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum StalePolicy {
    #[default]
    Reject,
    AllowStale,
}
pub struct EvaluationContext<'a> {
    pub snapshot: &'a CharacterSnapshot,
    pub party: Option<&'a PartySnapshot>,
    pub freshness: &'a FreshnessContext,
    pub stale_policy: StalePolicy,
    pub limits: RuleLimits,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(tag = "input", rename_all = "snake_case")]
pub enum InputPath {
    Field { field: Field },
    Equipment { slot: SlotId },
    Variant { slot: SlotId },
    Modifiers { slot: SlotId },
    Location,
    Participant,
    RuleExpiry,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(tag = "reason", rename_all = "snake_case")]
pub enum Reason {
    MissingRecord,
    Unavailable { status: ObservationStatus },
    Stale,
    ClockUncertain,
    ExpiredRule,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct EvaluationIssue {
    pub participant: CharacterId,
    pub input: InputPath,
    pub reason: Reason,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct EvaluationResult {
    pub truth: Truth,
    pub reasons: Vec<EvaluationIssue>,
    pub warnings: Vec<EvaluationIssue>,
    pub steps: usize,
}
impl EvaluationResult {
    pub fn should_trigger(&self) -> bool {
        self.truth == Truth::True
    }
}

struct Evaluator<'a, 'b> {
    context: &'a EvaluationContext<'b>,
    result: EvaluationResult,
}
impl Evaluator<'_, '_> {
    fn charge(&mut self) -> Result<()> {
        if self.result.steps >= self.context.limits.steps() {
            return Err(invalid("Rule evaluation step limit exceeded"));
        }
        self.result.steps += 1;
        Ok(())
    }
    fn issue(&mut self, who: &CharacterId, input: InputPath, reason: Reason) {
        self.result.reasons.push(EvaluationIssue {
            participant: who.clone(),
            input,
            reason,
        });
    }
    fn usable<'a, T>(
        &mut self,
        obs: &'a Observation<T>,
        who: &CharacterId,
        input: InputPath,
    ) -> Result<Option<&'a T>> {
        self.charge()?;
        let Some(value) = obs.value() else {
            self.issue(
                who,
                input,
                Reason::Unavailable {
                    status: obs.status(),
                },
            );
            return Ok(None);
        };
        match freshness(obs.metadata(), self.context.freshness) {
            Freshness::Fresh { .. } => Ok(Some(value)),
            Freshness::Stale { .. } if self.context.stale_policy == StalePolicy::AllowStale => {
                self.result.warnings.push(EvaluationIssue {
                    participant: who.clone(),
                    input,
                    reason: Reason::Stale,
                });
                Ok(Some(value))
            }
            Freshness::Stale { .. } => {
                self.issue(who, input, Reason::Stale);
                Ok(None)
            }
            Freshness::Uncertain { .. } => {
                self.issue(who, input, Reason::ClockUncertain);
                Ok(None)
            }
        }
    }
    fn missing(&mut self, who: &CharacterId, input: InputPath) {
        self.issue(who, input, Reason::MissingRecord);
    }
    fn field(&mut self, snapshot: &CharacterSnapshot, field: &Field) -> Result<Option<Scalar>> {
        self.charge()?;
        let who = &snapshot.character;
        let input = InputPath::Field {
            field: field.clone(),
        };
        Ok(match field {
            Field::Level => self.usable(&snapshot.level, who, input)?.map(|n| {
                Scalar::Integer(
                    SafeInteger::new(i64::from(n.get())).expect("u16 is a safe integer"),
                )
            }),
            Field::Role => self.usable(&snapshot.role, who, input)?.map(|role| {
                Scalar::Text(Text::new(role.as_str()).expect("validated ID fits Text"))
            }),
            Field::Preference { key } => match snapshot.preferences.get(key) {
                Some(obs) => self.usable(obs, who, input)?.map(|b| Scalar::Bool(*b)),
                None => {
                    self.missing(who, input);
                    None
                }
            },
            Field::Objective { objective } => match snapshot.objectives.get(objective) {
                Some(obs) => self.usable(obs, who, input)?.map(|b| Scalar::Bool(*b)),
                None => {
                    self.missing(who, input);
                    None
                }
            },
            Field::Quest { quest, state } => match snapshot.quests.get(quest) {
                Some(progress) => {
                    let obs = match state {
                        QuestState::Accepted => &progress.accepted,
                        QuestState::Completed => &progress.completed,
                        QuestState::TurnedIn => &progress.turned_in,
                    };
                    self.usable(obs, who, input)?.map(|b| Scalar::Bool(*b))
                }
                None => {
                    self.missing(who, input);
                    None
                }
            },
        })
    }
    fn compare(
        &mut self,
        snapshot: &CharacterSnapshot,
        field: &Field,
        value: &Scalar,
        gte: bool,
    ) -> Result<Truth> {
        let Some(actual) = self.field(snapshot, field)? else {
            return Ok(Truth::Unknown);
        };
        Ok(Truth::from_bool(if gte {
            match (&actual, value) {
                (Scalar::Integer(a), Scalar::Integer(b)) => a >= b,
                _ => return Err(invalid("Numeric comparison type mismatch")),
            }
        } else {
            actual == *value
        }))
    }
    fn equipment(
        &mut self,
        snapshot: &CharacterSnapshot,
        slot: &SlotId,
        item: &ItemKey,
        modifiers: &RecordMap<AttributeId, FiniteNumber>,
    ) -> Result<Truth> {
        let who = &snapshot.character;
        let path = InputPath::Equipment { slot: slot.clone() };
        self.charge()?;
        let Some(obs) = snapshot.equipment.get(slot) else {
            self.missing(who, path);
            return Ok(Truth::Unknown);
        };
        let Some(loaded) = self.usable(obs, who, path)? else {
            return Ok(Truth::Unknown);
        };
        let EquipmentSlot::Item(equipped) = loaded else {
            return Ok(Truth::False);
        };
        equipped.context.ensure_compatible(&item.context)?;
        if equipped.item_id != item.item_id {
            return Ok(Truth::False);
        }
        let mut truth = match self.usable(
            &equipped.variant,
            who,
            InputPath::Variant { slot: slot.clone() },
        )? {
            Some(v) if v != &item.variant => return Ok(Truth::False),
            Some(_) => Truth::True,
            None => Truth::Unknown,
        };
        if !modifiers.is_empty() {
            let Some(actual) = self.usable(
                &equipped.modifiers,
                who,
                InputPath::Modifiers { slot: slot.clone() },
            )?
            else {
                return Ok(Truth::Unknown);
            };
            for (key, expected) in modifiers.iter() {
                self.charge()?;
                match actual.get(key) {
                    Some(value) if value != expected => return Ok(Truth::False),
                    Some(_) => {}
                    None => {
                        self.missing(who, InputPath::Modifiers { slot: slot.clone() });
                        truth = Truth::Unknown;
                    }
                }
            }
        }
        Ok(truth)
    }
    fn node(&mut self, node: &PredicateNode, snapshot: &CharacterSnapshot) -> Result<Truth> {
        self.charge()?;
        match node {
            PredicateNode::All { predicates } => {
                let mut result = Truth::True;
                for p in predicates {
                    result = result.and(self.node(p, snapshot)?);
                    if result == Truth::False {
                        break;
                    }
                }
                Ok(result)
            }
            PredicateNode::Any { predicates } => {
                let mut result = Truth::False;
                for p in predicates {
                    result = result.or(self.node(p, snapshot)?);
                    if result == Truth::True {
                        break;
                    }
                }
                Ok(result)
            }
            PredicateNode::Not { predicate } => Ok(self.node(predicate, snapshot)?.negate()),
            PredicateNode::Known { field } => {
                Ok(Truth::from_bool(self.field(snapshot, field)?.is_some()))
            }
            PredicateNode::Equals { field, value } => self.compare(snapshot, field, value, false),
            PredicateNode::GreaterThanOrEqual { field, value } => {
                self.compare(snapshot, field, value, true)
            }
            PredicateNode::QuestState {
                quest,
                state,
                value,
            } => self.compare(
                snapshot,
                &Field::Quest {
                    quest: quest.clone(),
                    state: *state,
                },
                &Scalar::Bool(*value),
                false,
            ),
            PredicateNode::ObjectiveState {
                objective,
                completed,
            } => self.compare(
                snapshot,
                &Field::Objective {
                    objective: objective.clone(),
                },
                &Scalar::Bool(*completed),
                false,
            ),
            PredicateNode::LevelAtLeast { level } => self.compare(
                snapshot,
                &Field::Level,
                &Scalar::Integer(SafeInteger::new(i64::from(level.get()))?),
                true,
            ),
            PredicateNode::EquipmentMatches {
                slot,
                item,
                modifiers,
            } => self.equipment(snapshot, slot, item, modifiers),
            PredicateNode::RoleIs { role } => self.compare(
                snapshot,
                &Field::Role,
                &Scalar::Text(Text::new(role.as_str())?),
                false,
            ),
            PredicateNode::LocationInSupportedArea { area } => {
                match self.usable(&snapshot.location, &snapshot.character, InputPath::Location)? {
                    Some(p) => Ok(Truth::from_bool(area.contains(p)?)),
                    None => Ok(Truth::Unknown),
                }
            }
            PredicateNode::UserPreference { key, value } => self.compare(
                snapshot,
                &Field::Preference { key: key.clone() },
                &Scalar::Bool(*value),
                false,
            ),
            PredicateNode::ParticipantState {
                participant,
                predicate,
            } => {
                if participant == &snapshot.character {
                    return self.node(predicate, snapshot);
                }
                if let Some(party) = self.context.party {
                    for p in &party.participants {
                        self.charge()?;
                        if &p.character == participant {
                            return self.node(predicate, p);
                        }
                    }
                }
                self.missing(participant, InputPath::Participant);
                Ok(Truth::Unknown)
            }
        }
    }
}

fn validate_context(predicate: &PredicateNode, context: &EvaluationContext<'_>) -> Result<()> {
    predicate.validate_with(context.limits)?;
    if let Some(party) = context.party {
        context.snapshot.context.ensure_compatible(&party.context)?;
        if let Some(member) = party
            .participants
            .iter()
            .find(|p| p.character == context.snapshot.character)
            && member != context.snapshot
        {
            return Err(invalid(
                "Current snapshot conflicts with the party participant snapshot",
            ));
        }
    }
    let mut stack = vec![predicate];
    while let Some(node) = stack.pop() {
        match node {
            PredicateNode::All { predicates } | PredicateNode::Any { predicates } => {
                stack.extend(predicates)
            }
            PredicateNode::Not { predicate }
            | PredicateNode::ParticipantState { predicate, .. } => stack.push(predicate),
            PredicateNode::EquipmentMatches { item, .. } => {
                context.snapshot.context.ensure_compatible(&item.context)?
            }
            PredicateNode::LocationInSupportedArea { area } => context
                .snapshot
                .context
                .ensure_compatible(&area.map.context)?,
            _ => {}
        }
    }
    Ok(())
}
pub fn evaluate(
    predicate: &Predicate,
    context: &EvaluationContext<'_>,
) -> Result<EvaluationResult> {
    validate_context(predicate, context)?;
    let mut evaluator = Evaluator {
        context,
        result: EvaluationResult {
            truth: Truth::Unknown,
            reasons: vec![],
            warnings: vec![],
            steps: 0,
        },
    };
    evaluator.result.truth = evaluator.node(predicate, context.snapshot)?;
    Ok(evaluator.result)
}
/// Expiry gates the rule, without rewriting the observations' statuses.
/// Cooldown, dismissal and trigger dispatch are the caller's responsibility.
pub fn evaluate_rule(
    rule: &ContextRule,
    context: &EvaluationContext<'_>,
) -> Result<EvaluationResult> {
    rule.context.ensure_compatible(&context.snapshot.context)?;
    validate_context(&rule.predicate, context)?;
    if let Some(expiry) = rule.expires_at_utc_ms {
        let outcome = match (context.freshness.clock_trust, context.freshness.now_utc_ms) {
            (ClockTrust::Trusted, Some(now)) if now >= expiry => {
                Some((Truth::False, Reason::ExpiredRule))
            }
            (ClockTrust::Trusted, Some(_)) => None,
            _ => Some((Truth::Unknown, Reason::ClockUncertain)),
        };
        if let Some((truth, reason)) = outcome {
            return Ok(EvaluationResult {
                truth,
                reasons: vec![EvaluationIssue {
                    participant: context.snapshot.character.clone(),
                    input: InputPath::RuleExpiry,
                    reason,
                }],
                warnings: vec![],
                steps: 0,
            });
        }
    }
    evaluate(&rule.predicate, context)
}
