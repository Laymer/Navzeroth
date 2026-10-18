//! Application eligibility views retain independent observations and do not
//! promote incomplete/stale captures into definite joint recommendations.
use azeroth_core::{
    eligibility::{
        AcceptanceOutcome, CompletionOutcome, ParticipantEligibility, Phase, ProgressOutcome,
        QuestEligibilityCode, Requirement, TurnInOutcome, Unmet,
    },
    observation::Freshness,
    types::{CharacterId, DataContext, DatasetRevision, QuestId, SnapshotId},
};
use serde::Serialize;

#[derive(Clone, Debug, Serialize)]
pub struct PartyQuestCatalog {
    pub pages: crate::catalog_pages::Pages,
    pub context: DataContext,
    pub fixture: bool,
    pub revision: Option<DatasetRevision>,
    pub quests: Vec<QuestId>,
}
#[derive(Clone, Debug, Serialize)]
pub struct EligibilityReason {
    pub phase: Phase,
    pub message: String,
}
#[derive(Clone, Debug, Serialize)]
pub struct QuestAssessment {
    pub acceptance: AcceptanceOutcome,
    pub progress: ProgressOutcome,
    pub completion: CompletionOutcome,
    pub turn_in: TurnInOutcome,
    pub catch_up: Option<Unmet>,
    pub reasons: Vec<EligibilityReason>,
}
#[derive(Clone, Debug, Serialize)]
pub struct ParticipantAssessment {
    pub character: CharacterId,
    pub snapshot_id: Option<SnapshotId>,
    pub freshness: Option<Freshness>,
    pub assessment: Option<QuestAssessment>,
}
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum JointStatus {
    JointlyEligible,
    EligibleForSome,
    AllBehind,
    Blocked,
    Unknown,
    MissingOrOldSnapshots,
}
#[derive(Clone, Debug, Serialize)]
pub struct PartyQuestReport {
    pub context: DataContext,
    pub fixture: bool,
    pub revision: DatasetRevision,
    pub quest: QuestId,
    pub joint_status: JointStatus,
    pub participants: Vec<ParticipantAssessment>,
}
fn requirement(value: &Requirement) -> String {
    match value {
        Requirement::Quest { quest, state } => format!(
            "Quest {quest_id} requires {state:?} progress",
            quest_id = quest.as_str()
        ),
        Requirement::Level { level } => format!("Reach level {}", level.get()),
        Requirement::Other => "Another condition must be satisfied".into(),
    }
}
pub(crate) fn assessment(value: ParticipantEligibility) -> QuestAssessment {
    use QuestEligibilityCode::*;
    let reasons = value
        .reasons
        .into_iter()
        .map(|reason| {
            let message = match reason.code {
                RequirementDataUnknown { status, .. } => format!("Requirement data is {status:?}"),
                QuestHistoryUnknown {
                    history_quest,
                    state,
                    status,
                } => format!(
                    "Quest {} {state:?} history is unavailable ({status:?})",
                    history_quest.as_str()
                ),
                LevelUnknown { .. } => "Level is unavailable".into(),
                ConditionInputUnknown { .. } => {
                    "A condition needs an unavailable observation".into()
                }
                StageUnknown { objective, .. } => {
                    format!("Objective {} progress is unavailable", objective.as_str())
                }
                PrerequisiteUnmet { requirement: r } => requirement(&r),
                AlternativePrerequisiteUnmet { requirement: r } => {
                    format!("Alternative: {}", requirement(&r))
                }
                LevelBelowMinimum { level } => format!("Reach level {}", level.get()),
                AccessClassMismatch => "Class requirement is not satisfied".into(),
                AccessRaceMismatch => "Race requirement is not satisfied".into(),
                AccessFactionMismatch => "Faction requirement is not satisfied".into(),
                AccessFactUnknown { fact, .. } => format!("{fact:?} is unavailable"),
                ExclusiveChoiceTaken { other_quest } => {
                    format!("Quest {} excludes this choice", other_quest.as_str())
                }
                ExclusiveChoiceUnknown { other_quest, .. } => format!(
                    "Quest {} choice history is unavailable",
                    other_quest.as_str()
                ),
                AlreadyTurnedIn => "Already turned in".into(),
                ResetStateUnknown => {
                    "Reset status is unknown for an already turned-in quest".into()
                }
                RepeatabilityUnknown { .. } => "Repeatability is unavailable".into(),
                ObservationStale { .. } => "A required observation is stale".into(),
                ClockUncertain { .. } => "A required observation has uncertain capture age".into(),
                RuleExpired { .. } => "The condition has expired".into(),
                CatchUpRequired { .. } => "Catch-up progress is required before acceptance".into(),
            };
            EligibilityReason {
                phase: reason.phase,
                message,
            }
        })
        .collect();
    QuestAssessment {
        acceptance: value.acceptance,
        progress: value.progress,
        completion: value.completion,
        turn_in: value.turn_in,
        catch_up: value.catch_up,
        reasons,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use azeroth_core::{
        eligibility::{AccessFact, DefinitionField, ObservedInput, QuestEligibilityReason},
        evaluation::InputPath,
        observation::ObservationStatus,
        predicate::QuestState,
        types::{Level, ObjectiveId},
    };

    fn report(codes: Vec<QuestEligibilityCode>) -> ParticipantEligibility {
        let participant = CharacterId::new("synthetic-participant").unwrap();
        let quest = QuestId::new("synthetic-quest").unwrap();
        let phases = [
            Phase::Acceptance,
            Phase::Progress,
            Phase::Completion,
            Phase::TurnIn,
        ];
        ParticipantEligibility {
            participant: participant.clone(),
            quest: quest.clone(),
            acceptance: AcceptanceOutcome::Unknown,
            progress: ProgressOutcome::Unknown,
            completion: CompletionOutcome::Unknown,
            turn_in: TurnInOutcome::Unknown,
            catch_up: None,
            reasons: codes
                .into_iter()
                .enumerate()
                .map(|(i, code)| QuestEligibilityReason {
                    participant: participant.clone(),
                    quest: quest.clone(),
                    phase: phases[i % phases.len()],
                    code,
                })
                .collect(),
            blockers: vec![],
            warnings: vec![],
        }
    }

    #[test]
    fn unavailable_and_stale_reasons_keep_phases_without_inventing_eligibility() {
        use QuestEligibilityCode::*;
        let other = QuestId::new("synthetic-other").unwrap();
        let observed = ObservedInput::Access {
            fact: AccessFact::Class,
        };
        let input = report(vec![
            RequirementDataUnknown {
                field: DefinitionField::Prerequisites,
                status: ObservationStatus::Unsupported,
            },
            QuestHistoryUnknown {
                history_quest: other.clone(),
                state: QuestState::TurnedIn,
                status: Some(ObservationStatus::Restricted),
            },
            LevelUnknown { status: None },
            ConditionInputUnknown {
                input: InputPath::Participant,
                status: Some(ObservationStatus::Unknown),
            },
            StageUnknown {
                objective: ObjectiveId::new("synthetic-stage").unwrap(),
                status: None,
            },
            AccessFactUnknown {
                fact: AccessFact::Faction,
                status: Some(ObservationStatus::Unknown),
            },
            ExclusiveChoiceUnknown {
                other_quest: other,
                status: None,
            },
            ResetStateUnknown,
            RepeatabilityUnknown {
                status: ObservationStatus::Unsupported,
            },
            ObservationStale {
                input: observed.clone(),
            },
            ClockUncertain {
                input: observed.clone(),
            },
            RuleExpired { input: observed },
        ]);
        let expected_phases = input
            .reasons
            .iter()
            .map(|reason| reason.phase)
            .collect::<Vec<_>>();
        let actual = assessment(input);
        assert_eq!(actual.acceptance, AcceptanceOutcome::Unknown);
        assert_eq!(actual.progress, ProgressOutcome::Unknown);
        assert_eq!(actual.completion, CompletionOutcome::Unknown);
        assert_eq!(actual.turn_in, TurnInOutcome::Unknown);
        assert!(actual.catch_up.is_none());
        assert_eq!(
            actual
                .reasons
                .iter()
                .map(|reason| reason.phase)
                .collect::<Vec<_>>(),
            expected_phases
        );
        let messages = actual
            .reasons
            .iter()
            .map(|reason| reason.message.as_str())
            .collect::<Vec<_>>();
        assert_eq!(messages.len(), 12);
        assert!(messages[0].contains("Unsupported"));
        assert!(messages[1].contains("synthetic-other") && messages[1].contains("Restricted"));
        assert!(messages[4].contains("synthetic-stage"));
        assert!(messages[6].contains("choice history is unavailable"));
        assert!(messages[9].contains("stale"));
        assert!(messages[10].contains("uncertain capture age"));
        assert!(messages[11].contains("expired"));
        assert!(
            messages
                .iter()
                .all(|message| !message.contains("is satisfied"))
        );
    }

    #[test]
    fn blocked_conditions_and_alternative_catch_up_are_not_relabelled_as_eligible() {
        use QuestEligibilityCode::*;
        let level = Level::new(25).unwrap();
        let prerequisite = Requirement::Quest {
            quest: QuestId::new("synthetic-prerequisite").unwrap(),
            state: QuestState::Completed,
        };
        let catch_up = Unmet::OneOf {
            alternatives: vec![
                Unmet::Single {
                    requirement: prerequisite.clone(),
                },
                Unmet::Single {
                    requirement: Requirement::Level { level },
                },
            ],
        };
        let mut input = report(vec![
            PrerequisiteUnmet {
                requirement: prerequisite,
            },
            AlternativePrerequisiteUnmet {
                requirement: Requirement::Level { level },
            },
            PrerequisiteUnmet {
                requirement: Requirement::Other,
            },
            LevelBelowMinimum { level },
            AccessClassMismatch,
            AccessRaceMismatch,
            AccessFactionMismatch,
            ExclusiveChoiceTaken {
                other_quest: QuestId::new("synthetic-exclusive").unwrap(),
            },
            AlreadyTurnedIn,
        ]);
        input.acceptance = AcceptanceOutcome::Blocked;
        input.progress = ProgressOutcome::InProgress;
        input.completion = CompletionOutcome::Incomplete;
        input.turn_in = TurnInOutcome::NotReady;
        // The adapter preserves the typed result; it does not recompute or flatten
        // alternatives from human-readable reasons or an observed turn-in.
        let actual = assessment(input);
        assert_eq!(actual.acceptance, AcceptanceOutcome::Blocked);
        assert_eq!(actual.progress, ProgressOutcome::InProgress);
        assert_eq!(actual.completion, CompletionOutcome::Incomplete);
        assert_eq!(actual.turn_in, TurnInOutcome::NotReady);
        assert!(actual.catch_up.is_none());
        assert_eq!(actual.reasons.len(), 9);
        assert!(actual.reasons[0].message.contains("synthetic-prerequisite"));
        assert!(actual.reasons[1].message.starts_with("Alternative:"));
        assert!(actual.reasons[2].message.contains("Another condition"));
        assert!(actual.reasons[3].message.contains("25"));
        let access = actual.reasons[4..7]
            .iter()
            .map(|reason| &reason.message)
            .collect::<std::collections::BTreeSet<_>>();
        assert_eq!(access.len(), 3);
        assert!(actual.reasons[7].message.contains("synthetic-exclusive"));
        assert!(actual.reasons[8].message.contains("Already turned in"));
        let mut behind = report(vec![CatchUpRequired {
            unmet: catch_up.clone(),
        }]);
        behind.acceptance = AcceptanceOutcome::Behind;
        behind.catch_up = Some(catch_up.clone());
        let actual = assessment(behind);
        assert_eq!(actual.acceptance, AcceptanceOutcome::Behind);
        assert_eq!(actual.catch_up, Some(catch_up));
        assert!(actual.reasons[0].message.contains("before acceptance"));
    }
}
