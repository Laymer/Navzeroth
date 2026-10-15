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
