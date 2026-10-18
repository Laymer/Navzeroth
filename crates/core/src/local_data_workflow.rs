//! Durable local intent and deterministic selected-build change detection, without I/O.
use crate::local_data::{PerformancePreset, is_sha256};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
pub const CONTENT_LOCALES: &[&str] = &[
    "enUS", "enGB", "deDE", "esES", "esMX", "frFR", "itIT", "koKR", "ptBR", "ruRU", "zhCN", "zhTW",
];
#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum StartupMode {
    #[default]
    Later,
    Snapshots,
    LocalResearch,
}
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct SelectionChoices {
    pub locales: Vec<String>,
    pub preset: PerformancePreset,
    pub change_detection: bool,
    pub baseline_sha256: Option<String>,
}
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Preferences {
    pub format_version: u32,
    pub first_run_completed: bool,
    pub startup_mode: StartupMode,
    pub selections: BTreeMap<String, SelectionChoices>,
}
impl Default for Preferences {
    fn default() -> Self {
        Self {
            format_version: 1,
            first_run_completed: false,
            startup_mode: StartupMode::Later,
            selections: BTreeMap::new(),
        }
    }
}
impl Preferences {
    pub fn validate(&self) -> Result<(), &'static str> {
        if self.format_version != 1 || self.selections.len() > 16 {
            return Err("Unsupported local workflow preferences");
        }
        for (id, choices) in &self.selections {
            if !is_sha256(id)
                || choices.locales.len() > CONTENT_LOCALES.len()
                || choices
                    .locales
                    .iter()
                    .any(|l| !CONTENT_LOCALES.contains(&l.as_str()))
                || choices.locales.windows(2).any(|w| w[0] >= w[1])
                || choices
                    .baseline_sha256
                    .as_ref()
                    .is_some_and(|h| !is_sha256(h))
                || choices.change_detection && choices.baseline_sha256.is_none()
            {
                return Err("Invalid local selection choices");
            }
        }
        Ok(())
    }
}
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct PreferencesRecord {
    pub format_version: u32,
    pub revision: u64,
    pub preferences: Preferences,
}
/// Legacy empty selections retain original declared-locale order. New callers select explicitly.
pub fn select_locales(
    declared: &[String],
    requested: &[String],
) -> Result<Vec<String>, &'static str> {
    if requested.is_empty() {
        return Ok(declared.to_vec());
    }
    if requested.len() > CONTENT_LOCALES.len()
        || requested.windows(2).any(|w| w[0] >= w[1])
        || requested
            .iter()
            .any(|l| !CONTENT_LOCALES.contains(&l.as_str()) || !declared.contains(l))
    {
        return Err("Requested content locale is unsupported or not installed");
    }
    Ok(requested.to_vec())
}
#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum PollStatus {
    Idle,
    WaitingForStableFile,
    Ready,
    Staged,
    Missing,
    RetryExhausted,
}
pub struct ChangePoller {
    baseline: String,
    candidate: Option<String>,
    since: u64,
    last_now: u64,
    attempts: u8,
    staged: bool,
}
impl ChangePoller {
    pub fn new(baseline: &str) -> Result<Self, &'static str> {
        if !is_sha256(baseline) {
            return Err("Invalid change baseline");
        }
        Ok(Self {
            baseline: baseline.into(),
            candidate: None,
            since: 0,
            last_now: 0,
            attempts: 0,
            staged: false,
        })
    }
    pub fn poll(&mut self, now_ms: u64, hash: Option<&str>) -> Result<PollStatus, &'static str> {
        if now_ms < self.last_now {
            return Err("Change detector monotonic time moved backwards");
        }
        self.last_now = now_ms;
        let Some(hash) = hash else {
            self.candidate = None;
            self.attempts = 0;
            self.staged = false;
            return Ok(PollStatus::Missing);
        };
        if !is_sha256(hash) {
            return Err("Invalid observed build identity");
        }
        if hash == self.baseline {
            self.candidate = None;
            self.attempts = 0;
            self.staged = false;
            return Ok(PollStatus::Idle);
        }
        if self.candidate.as_deref() != Some(hash) {
            self.candidate = Some(hash.into());
            self.since = now_ms;
            self.attempts = 0;
            self.staged = false;
        }
        if self.staged {
            return Ok(PollStatus::Staged);
        }
        if now_ms.saturating_sub(self.since) < 750 {
            return Ok(PollStatus::WaitingForStableFile);
        }
        if self.attempts >= 3 {
            return Ok(PollStatus::RetryExhausted);
        }
        self.attempts += 1;
        Ok(PollStatus::Ready)
    }
    pub fn staged(&mut self) {
        self.staged = true;
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum MaterialKind {
    HotfixCache,
    QuestCache,
    CreatureCache,
    GameObjectCache,
    ItemCache,
    Snapshot,
    Session,
    Guide,
    AuthoredPack,
}
impl MaterialKind {
    pub fn input_kind(self) -> Option<crate::local_data_inputs::InputKind> {
        use crate::local_data_inputs::InputKind;
        Some(match self {
            Self::HotfixCache => InputKind::HotfixCache,
            Self::QuestCache => InputKind::QuestCache,
            Self::CreatureCache => InputKind::CreatureCache,
            Self::GameObjectCache => InputKind::GameObjectCache,
            Self::ItemCache => InputKind::ItemCache,
            Self::Snapshot => InputKind::CharacterSnapshot,
            Self::Session => InputKind::SessionEvents,
            _ => return None,
        })
    }
}

/// Explicit dependencies for one local research delivery, never activation intent.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct DeliveryChoices {
    pub format_version: u32,
    pub media: Vec<String>,
    pub inputs: Vec<String>,
    pub guide_facts: bool,
    pub preset: PerformancePreset,
}
impl DeliveryChoices {
    pub fn validate(&self) -> Result<(), &'static str> {
        if self.format_version != 1
            || [&self.media, &self.inputs].into_iter().any(|ids| {
                ids.len() > 16
                    || ids.iter().any(|id| !is_sha256(id))
                    || ids.windows(2).any(|pair| pair[0] >= pair[1])
            })
        {
            return Err("Invalid selected delivery dependencies");
        }
        Ok(())
    }
}
