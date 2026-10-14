//! Bounded local intent. Stored plans are references, never imported observations.
use crate::{
    ProfileKey, SqliteStore,
    content::ContentRevisionKey,
    error::{SqliteResult, invalid},
};
use azeroth_core::{error::Result, types::*};
use rusqlite::{OptionalExtension, params};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
#[path = "today_json.rs"]
mod json;
pub const MAX_DOCUMENT: usize = 65536;
pub const MAX_REQUEST: usize = 1048576;
pub const MAX_REVISION: i64 = 9007199254740991;
pub fn decode<T: serde::de::DeserializeOwned + Serialize>(text: &str, limit: usize) -> Result<T> {
    json::decode(text, limit)
}
pub fn encode<T: Serialize>(value: &T, limit: usize) -> Result<String> {
    struct Output {
        bytes: Vec<u8>,
        limit: usize,
    }
    impl std::io::Write for Output {
        fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
            if bytes.len() > self.limit.saturating_sub(self.bytes.len()) {
                return Err(std::io::Error::other("local state bound"));
            }
            self.bytes.extend_from_slice(bytes);
            Ok(bytes.len())
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }
    let mut out = Output {
        bytes: Vec::new(),
        limit,
    };
    serde_json::to_writer(&mut out, value)
        .map_err(|_| invalid("Local state exceeds serialization bound"))?;
    String::from_utf8(out.bytes).map_err(|_| invalid("Invalid local state encoding"))
}
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CandidateEstimate {
    pub objective: ObjectiveId,
    pub node: NodeId,
    pub utility: u32,
    pub completion_ms: Millis,
    pub service_ms: Millis,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct PlanningRequest {
    pub profile_id: String,
    pub snapshot_id: SnapshotId,
    pub revision: DatasetRevision,
    pub goal_profile: SourceId,
    pub origin: NodeId,
    pub stopping_node: NodeId,
    pub budget_ms: Millis,
    pub stopping_service_ms: Millis,
    pub candidates: Vec<CandidateEstimate>,
    pub excluded: BTreeSet<ObjectiveId>,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct JointParticipantInput {
    pub profile_id: String,
    pub snapshot_id: SnapshotId,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct JointEstimate {
    pub objective: ObjectiveId,
    pub node: NodeId,
    /// Explicit same-unit contributions; every member must have positive utility.
    pub utilities: BTreeMap<String, u32>,
    pub completion_ms: Millis,
    pub service_ms: Millis,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct JointPlanningRequest {
    pub party_id: PartyId,
    pub participants: Vec<JointParticipantInput>,
    pub revision: DatasetRevision,
    pub goal_profile: SourceId,
    pub origin: NodeId,
    pub stopping_node: NodeId,
    pub budget_ms: Millis,
    pub stopping_service_ms: Millis,
    pub candidates: Vec<JointEstimate>,
    pub excluded: BTreeSet<ObjectiveId>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum Subject {
    Character { profile_id: String },
    Party { party_id: PartyId },
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum ExportFormat {
    Anav1Text,
    SavedVariables { global: String, key: String },
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExportSelection {
    pub path: String,
    pub format: ExportFormat,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Contrast {
    System,
    High,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Motion {
    System,
    Reduced,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Accessibility {
    pub text_scale_percent: u16,
    pub contrast: Contrast,
    pub motion: Motion,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ManualDefaults {
    pub goal_profile: Option<SourceId>,
    pub budget_ms: Millis,
    pub stopping_service_ms: Millis,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Settings {
    pub schema_version: u32,
    pub selected_subject: Option<Subject>,
    pub selected_installation: Option<String>,
    pub selected_export: Option<ExportSelection>,
    pub manual_planning_defaults: ManualDefaults,
    pub accessibility: Accessibility,
    pub reference_always_on_top: bool,
}
impl Default for Settings {
    fn default() -> Self {
        Self {
            schema_version: 1,
            selected_subject: None,
            selected_installation: None,
            selected_export: None,
            manual_planning_defaults: ManualDefaults {
                goal_profile: None,
                budget_ms: Millis::new(3600000).expect("constant"),
                stopping_service_ms: Millis::new(0).expect("constant"),
            },
            accessibility: Accessibility {
                text_scale_percent: 100,
                contrast: Contrast::System,
                motion: Motion::System,
            },
            reference_always_on_top: false,
        }
    }
}
fn path(value: &str) -> bool {
    !value.is_empty() && value.len() <= 4096 && !value.contains('\0')
}
fn identifier(value: &str) -> bool {
    value.len() <= 128
        && value
            .as_bytes()
            .first()
            .is_some_and(|b| b.is_ascii_alphabetic() || *b == b'_')
        && value
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'_')
}
fn profile(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}
impl Settings {
    pub fn validate(&self) -> Result<()> {
        if self.schema_version !=1 || ![100,125,150,175,200].contains(&self.accessibility.text_scale_percent)
            || self.selected_installation.as_ref().is_some_and(|p| !path(p))
            || self.selected_subject.as_ref().is_some_and(|s| matches!(s, Subject::Character {profile_id} if !profile(profile_id)))
            || self.selected_export.as_ref().is_some_and(|v| !path(&v.path) || matches!(&v.format, ExportFormat::SavedVariables{global,key} if !identifier(global) || !identifier(key))) {
            return Err(invalid("Invalid local settings"));
        }
        encode(self, MAX_DOCUMENT)?;
        Ok(())
    }
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SavedProfile {
    pub profile_id: String,
    pub key: ProfileKey,
    pub snapshot_id: SnapshotId,
    pub snapshot_hash: Sha256Hex,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GuardDescriptor {
    pub schema_version: u32,
    pub subject: Subject,
    pub profiles: Vec<SavedProfile>,
    pub content: ContentRevisionKey,
    pub content_hash: Sha256Hex,
    pub evaluation_utc_ms: UtcMillis,
    pub publication_utc_ms: UtcMillis,
    pub max_age_ms: Millis,
    pub ruleset_version: String,
    pub model_version: String,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SavedStep {
    pub objective: ObjectiveId,
    pub node: NodeId,
    pub completion_ms: Millis,
    pub service_ms: Millis,
    pub expected_session_ms: Millis,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SavedTermination {
    NoFeasibleCandidate,
    StepLimit,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PlanSummary {
    pub schema_version: u32,
    pub goal_profile: SourceId,
    pub steps: Vec<SavedStep>,
    pub expected_total_ms: Millis,
    pub termination: SavedTermination,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PlanKind {
    Solo,
    Joint,
}
impl PlanKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Solo => "solo",
            Self::Joint => "joint",
        }
    }
}
#[derive(Clone, Debug)]
pub struct PlanDescriptor {
    pub kind: PlanKind,
    pub request_json: String,
    pub guard: GuardDescriptor,
    pub summary: PlanSummary,
}
impl PlanDescriptor {
    pub fn validate(&self) -> Result<()> {
        let g = &self.guard;
        let s = &self.summary;
        if g.schema_version != 1
            || s.schema_version != 1
            || g.profiles.is_empty()
            || g.profiles.len() > 40
            || s.steps.len() > 128
            || g.profiles.iter().any(|p| {
                !profile(&p.profile_id)
                    || p.key.context != g.content.scope.context
                    || p.key.fixture != g.content.scope.fixture
            })
            || g.profiles
                .iter()
                .map(|p| &p.profile_id)
                .collect::<BTreeSet<_>>()
                .len()
                != g.profiles.len()
            || g.profiles
                .iter()
                .map(|p| &p.key)
                .collect::<BTreeSet<_>>()
                .len()
                != g.profiles.len()
            || g.publication_utc_ms.get() < g.evaluation_utc_ms.get()
            || g.ruleset_version.is_empty()
            || g.ruleset_version.len() > 128
            || g.model_version.is_empty()
            || g.model_version.len() > 128
            || s.steps
                .iter()
                .map(|x| &x.objective)
                .collect::<BTreeSet<_>>()
                .len()
                != s.steps.len()
            || s.steps
                .windows(2)
                .any(|w| w[0].expected_session_ms > w[1].expected_session_ms)
            || s.steps
                .last()
                .is_some_and(|x| x.expected_session_ms > s.expected_total_ms)
        {
            return Err(invalid("Invalid saved plan dependencies"));
        }
        let (goal, revision, candidates, excluded) = match self.kind {
            PlanKind::Solo => {
                let r: PlanningRequest = decode(&self.request_json, MAX_REQUEST)?;
                if g.profiles.len() != 1
                    || !matches!(&g.subject, Subject::Character{profile_id} if profile_id==&r.profile_id && profile_id==&g.profiles[0].profile_id)
                    || r.snapshot_id != g.profiles[0].snapshot_id
                {
                    return Err(invalid("Saved solo identity differs"));
                }
                (
                    r.goal_profile,
                    r.revision,
                    r.candidates
                        .iter()
                        .map(|c| {
                            (
                                c.objective.clone(),
                                (c.node.clone(), c.completion_ms, c.service_ms),
                            )
                        })
                        .collect::<BTreeMap<_, _>>(),
                    r.excluded,
                )
            }
            PlanKind::Joint => {
                let r: JointPlanningRequest = decode(&self.request_json, MAX_REQUEST)?;
                if !(2..=40).contains(&r.participants.len())
                    || !matches!(&g.subject,Subject::Party{party_id} if party_id==&r.party_id)
                    || r.participants.len() != g.profiles.len()
                    || r.participants.iter().zip(&g.profiles).any(|(p, g)| {
                        p.profile_id != g.profile_id || p.snapshot_id != g.snapshot_id
                    })
                    || r.candidates.iter().any(|c| {
                        c.utilities.len() != g.profiles.len()
                            || g.profiles
                                .iter()
                                .any(|p| !c.utilities.contains_key(&p.profile_id))
                    })
                {
                    return Err(invalid("Saved joint identity differs"));
                }
                (
                    r.goal_profile,
                    r.revision,
                    r.candidates
                        .iter()
                        .map(|c| {
                            (
                                c.objective.clone(),
                                (c.node.clone(), c.completion_ms, c.service_ms),
                            )
                        })
                        .collect::<BTreeMap<_, _>>(),
                    r.excluded,
                )
            }
        };
        if goal != s.goal_profile
            || revision != g.content.revision
            || candidates.len() > 128
            || excluded.len() > 128
            || s.steps.iter().any(|step| {
                excluded.contains(&step.objective)
                    || candidates.get(&step.objective)
                        != Some(&(step.node.clone(), step.completion_ms, step.service_ms))
            })
        {
            return Err(invalid("Saved plan summary differs from declared request"));
        }
        encode(g, MAX_DOCUMENT)?;
        encode(s, MAX_DOCUMENT)?;
        Ok(())
    }
}
#[derive(Clone, Debug, Serialize)]
pub struct SettingsRecord {
    pub revision: i64,
    pub settings: Settings,
}
#[derive(Clone, Debug)]
pub struct CurrentPlanRecord {
    pub revision: i64,
    pub plan: Option<PlanDescriptor>,
}
fn next(revision: i64) -> Result<i64> {
    if revision < 0 || revision >= MAX_REVISION {
        Err(invalid("Local revision exhausted"))
    } else {
        Ok(revision + 1)
    }
}
/// Typed validation for recovery; no filesystem, activation or optimizer calls.
pub fn validate_settings_document(text: &str) -> Result<()> {
    let s: Settings = decode(text, MAX_DOCUMENT)?;
    s.validate()
}
pub fn validate_current_plan_documents(
    kind: &str,
    request: &str,
    guard: &str,
    summary: &str,
) -> Result<()> {
    descriptor(kind, request.to_owned(), guard, summary)?.validate()
}
fn descriptor(kind: &str, request: String, guard: &str, summary: &str) -> Result<PlanDescriptor> {
    Ok(PlanDescriptor {
        kind: match kind {
            "solo" => PlanKind::Solo,
            "joint" => PlanKind::Joint,
            _ => return Err(invalid("Invalid saved plan kind")),
        },
        request_json: request,
        guard: decode(guard, MAX_DOCUMENT)?,
        summary: decode(summary, MAX_DOCUMENT)?,
    })
}
impl SqliteStore {
    /// Intent writes and their dependency checks share one writer reservation.
    pub fn with_intent_transaction<T>(&self, call: impl FnOnce(&Self) -> Result<T>) -> Result<T> {
        if !self.conn.is_autocommit() {
            return Err(invalid("Intent transaction cannot nest"));
        }
        let tx = rusqlite::Transaction::new_unchecked(
            &self.conn,
            rusqlite::TransactionBehavior::Immediate,
        )
        .db()?;
        let value = call(self)?;
        tx.commit().db()?;
        Ok(value)
    }
    pub fn settings(&self) -> Result<SettingsRecord> {
        let row: Option<(i64, String)> = self
            .conn
            .query_row(
                "SELECT revision,document FROM app_settings WHERE slot=1",
                [],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .optional()
            .db()?;
        match row {
            Some((revision, text)) => {
                let settings: Settings = decode(&text, MAX_DOCUMENT)?;
                settings.validate()?;
                Ok(SettingsRecord { revision, settings })
            }
            None => Ok(SettingsRecord {
                revision: 0,
                settings: Settings::default(),
            }),
        }
    }
    /// Caller holds the immediate transaction when combining dependency validation.
    pub fn replace_settings(&self, expected: i64, settings: &Settings) -> Result<SettingsRecord> {
        settings.validate()?;
        let revision = next(expected)?;
        let document = encode(settings, MAX_DOCUMENT)?;
        let count = if expected == 0 {
            self.conn.execute("INSERT OR IGNORE INTO app_settings(slot,schema_version,revision,document) VALUES(1,1,1,?1)",[document]).db()?
        } else {
            self.conn
                .execute(
                    "UPDATE app_settings SET revision=?1,document=?2 WHERE slot=1 AND revision=?3",
                    params![revision, document, expected],
                )
                .db()?
        };
        if count != 1 {
            return Err(invalid("Local settings revision changed"));
        }
        Ok(SettingsRecord {
            revision,
            settings: settings.clone(),
        })
    }
    pub fn current_plan(&self) -> Result<CurrentPlanRecord> {
        type Row = (
            i64,
            Option<String>,
            Option<String>,
            Option<String>,
            Option<String>,
        );
        let (revision,kind,request,guard,summary):Row=self.conn.query_row("SELECT revision,kind,request_json,guard_json,summary_json FROM current_plan WHERE slot=1",[],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?,r.get(4)?))).db()?;
        let plan = match (kind, request, guard, summary) {
            (None, None, None, None) => None,
            (Some(k), Some(r), Some(g), Some(s)) => {
                let p = descriptor(&k, r, &g, &s)?;
                p.validate()?;
                Some(p)
            }
            _ => return Err(invalid("Invalid saved plan slot")),
        };
        Ok(CurrentPlanRecord { revision, plan })
    }
    pub fn replace_current_plan(
        &self,
        expected: i64,
        plan: Option<&PlanDescriptor>,
    ) -> Result<i64> {
        let revision = next(expected)?;
        let (kind, request, guard, summary) = match plan {
            Some(p) => {
                p.validate()?;
                (
                    Some(p.kind.as_str()),
                    Some(p.request_json.as_str()),
                    Some(encode(&p.guard, MAX_DOCUMENT)?),
                    Some(encode(&p.summary, MAX_DOCUMENT)?),
                )
            }
            None => (None, None, None, None),
        };
        let count=self.conn.execute("UPDATE current_plan SET revision=?1,kind=?2,request_json=?3,guard_json=?4,summary_json=?5 WHERE slot=1 AND revision=?6",params![revision,kind,request,guard,summary,expected]).db()?;
        if count != 1 {
            return Err(invalid("Saved plan revision changed"));
        }
        Ok(revision)
    }
}
