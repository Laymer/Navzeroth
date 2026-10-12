//! Closed declarative rule language. Root validation bounds every tree.
use crate::{
    error::{Result, Validate, Validated, invalid},
    types::*,
};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", content = "value", rename_all = "snake_case")]
pub enum Scalar {
    Bool(bool),
    Integer(SafeInteger),
    Number(FiniteNumber),
    Copper(Copper),
    Text(Text),
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum QuestState {
    Accepted,
    Completed,
    TurnedIn,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "field", rename_all = "snake_case")]
pub enum Field {
    Level,
    Role,
    Preference { key: PreferenceId },
    Quest { quest: QuestId, state: QuestState },
    Objective { objective: ObjectiveId },
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "op", rename_all = "snake_case")]
pub enum PredicateNode {
    All {
        predicates: Vec<Self>,
    },
    Any {
        predicates: Vec<Self>,
    },
    Not {
        predicate: Box<Self>,
    },
    Known {
        field: Field,
    },
    Equals {
        field: Field,
        value: Scalar,
    },
    GreaterThanOrEqual {
        field: Field,
        value: Scalar,
    },
    QuestState {
        quest: QuestId,
        state: QuestState,
        value: bool,
    },
    ObjectiveState {
        objective: ObjectiveId,
        completed: bool,
    },
    LevelAtLeast {
        level: Level,
    },
    EquipmentMatches {
        slot: SlotId,
        item: ItemKey,
        modifiers: RecordMap<AttributeId, FiniteNumber>,
    },
    RoleIs {
        role: RoleId,
    },
    LocationInSupportedArea {
        area: SupportedArea,
    },
    UserPreference {
        key: PreferenceId,
        value: bool,
    },
    ParticipantState {
        participant: CharacterId,
        predicate: Box<Self>,
    },
}
pub type Predicate = Validated<PredicateNode>;

#[derive(Clone, Copy, Debug)]
pub struct RuleLimits {
    depth: usize,
    nodes: usize,
    steps: usize,
}
impl Default for RuleLimits {
    fn default() -> Self {
        Self {
            depth: 32,
            nodes: 1024,
            steps: 1024,
        }
    }
}
impl RuleLimits {
    pub fn new(depth: usize, nodes: usize, steps: usize) -> Result<Self> {
        if !(1..=32).contains(&depth)
            || !(1..=1024).contains(&nodes)
            || !(1..=1024).contains(&steps)
        {
            return Err(invalid(
                "Rule limits must be positive and no larger than defaults",
            ));
        }
        Ok(Self {
            depth,
            nodes,
            steps,
        })
    }
    pub fn steps(self) -> usize {
        self.steps
    }
}
impl Validate for PredicateNode {
    fn validate(&self) -> Result<()> {
        self.validate_with(RuleLimits::default())
    }
}
impl PredicateNode {
    pub fn validate_with(&self, limits: RuleLimits) -> Result<()> {
        let mut stack = vec![(self, 1)];
        let mut count = 0;
        while let Some((node, depth)) = stack.pop() {
            count += 1;
            if depth > limits.depth || count > limits.nodes {
                return Err(invalid("Rule depth or node limit exceeded"));
            }
            match node {
                Self::All { predicates } | Self::Any { predicates } => {
                    if count + stack.len() + predicates.len() > limits.nodes {
                        return Err(invalid("Rule node limit exceeded"));
                    }
                    stack.extend(predicates.iter().map(|p| (p, depth + 1)));
                }
                Self::Not { predicate } | Self::ParticipantState { predicate, .. } => {
                    stack.push((predicate, depth + 1))
                }
                Self::Equals { field, value } | Self::GreaterThanOrEqual { field, value } => {
                    let valid = matches!(
                        (field, value),
                        (Field::Level, Scalar::Integer(_))
                            | (Field::Role, Scalar::Text(_))
                            | (
                                Field::Preference { .. }
                                    | Field::Quest { .. }
                                    | Field::Objective { .. },
                                Scalar::Bool(_)
                            )
                    );
                    if !valid
                        || (matches!(node, Self::GreaterThanOrEqual { .. })
                            && !matches!(field, Field::Level))
                    {
                        return Err(invalid("Predicate operand type does not match field"));
                    }
                }
                Self::EquipmentMatches { modifiers, .. } if modifiers.len() > 1024 => {
                    return Err(invalid("Too many modifier constraints"));
                }
                _ => {}
            }
        }
        Ok(())
    }
}
