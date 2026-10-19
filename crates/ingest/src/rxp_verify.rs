//! Route replay verifier for RXP-format guides (task V2.3), ported from the owner's
//! `verify2.py`.
//!
//! The verifier replays a parsed [`GuideFile`] for one character and checks the
//! route design against a pluggable [`FactSource`]: quest existence, prerequisites
//! before accept, required level, race/class availability, repeatable and seasonal
//! quests, quest-log capacity, accept before turn-in, double turn-ins, objective
//! and reward indexes, zone and flight-node names, coordinates, header hygiene and
//! bracket balance.
//!
//! Facts are typed and may be unknown. A check whose fact is unknown is reported
//! as `unverified`, never as `ok`. A directive, tag or filter the importer does not
//! interpret is `unsupported`. A replay is clean only with zero errors, unverified
//! and unsupported findings. Guide text (names, notes) never appears in a report;
//! findings carry line numbers, quest IDs and static check names only.
use crate::rxp::{
    Applies, Character, Class, Directive, Edition, Guide, GuideFile, HeaderTag, Item, QuestId,
    Race, Step, StepTag, XpKind,
};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

// ---------------------------------------------------------------------------
// Facts
// ---------------------------------------------------------------------------

/// A typed fact that may be unknown. `Unknown` is never treated as a pass.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Fact<T> {
    Known(T),
    Unknown,
}
impl<T> Fact<T> {
    pub fn known(&self) -> Option<&T> {
        match self {
            Self::Known(v) => Some(v),
            Self::Unknown => None,
        }
    }
}
impl<T> From<Option<T>> for Fact<T> {
    fn from(value: Option<T>) -> Self {
        value.map_or(Self::Unknown, Self::Known)
    }
}

/// Who may take a quest.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Allowed<T: Ord> {
    Any,
    Only(BTreeSet<T>),
}
impl<T: Ord> Allowed<T> {
    fn allows(&self, value: &T) -> bool {
        match self {
            Self::Any => true,
            Self::Only(set) => set.contains(value),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QuestFacts {
    pub required_level: Fact<u8>,
    /// Quest level, used by the XP estimate's grey-out penalty.
    pub level: Fact<u8>,
    /// Any one of these must be turned in first (QuestieDB `preQuestSingle` shape).
    pub prerequisites_any: Fact<Vec<QuestId>>,
    /// All of these must be turned in first (`preQuestGroup` shape).
    pub prerequisites_all: Fact<Vec<QuestId>>,
    pub races: Fact<Allowed<Race>>,
    pub classes: Fact<Allowed<Class>>,
    pub repeatable: Fact<bool>,
    pub seasonal: Fact<bool>,
    /// Number of trackable objectives (`.complete id,n` indexes 1..=n).
    pub objectives: Fact<u8>,
    /// Number of selectable rewards (`.turnin id,n` indexes 1..=n).
    pub reward_choices: Fact<u8>,
    pub xp: Fact<u32>,
}
impl QuestFacts {
    pub fn unknown() -> Self {
        Self {
            required_level: Fact::Unknown,
            level: Fact::Unknown,
            prerequisites_any: Fact::Unknown,
            prerequisites_all: Fact::Unknown,
            races: Fact::Unknown,
            classes: Fact::Unknown,
            repeatable: Fact::Unknown,
            seasonal: Fact::Unknown,
            objectives: Fact::Unknown,
            reward_choices: Fact::Unknown,
            xp: Fact::Unknown,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum QuestRecord {
    Known(QuestFacts),
    /// The source asserts the quest does not exist (for this edition/build).
    Absent,
    Unknown,
}

/// Pluggable fact source. Implementations must answer `Unknown` rather than guess.
pub trait FactSource {
    fn quest(&self, id: QuestId) -> QuestRecord;
    /// Is this a map zone name as the addon's `.goto`/`.zone` uses it?
    fn zone(&self, name: &str) -> Fact<bool>;
    /// Is this a flight-master node name usable with `.fly`/`.fp`?
    fn flight_node(&self, name: &str) -> Fact<bool>;
    fn quest_log_capacity(&self) -> Fact<u16>;
    /// May this class learn this spell from a trainer?
    fn spell_trainable(&self, spell: u32, class: Class) -> Fact<bool>;
    /// Experience needed to go from `level` to `level + 1`.
    fn level_xp(&self, level: u8) -> Fact<u32>;
    /// The game edition these facts describe. Never mixed silently with another.
    fn edition(&self) -> Fact<Edition>;
    /// The client build these facts were taken from (`.build.info` label).
    fn client_build(&self) -> Fact<String>;
}

/// Where a facts file says its facts came from (ADR 0006 §3; owner decision
/// 2026-10-17: guide facts must be confirmed locally).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FactOrigin {
    /// Version-2 projected research: never an evidence claim or acceptance.
    UnverifiedResearch,
    /// Hand-made test facts. Never evidence about the game.
    Synthetic,
    /// The user's own in-game observations through the permitted addon.
    LocalObservation,
    /// The user's own client data (local extraction).
    LocalClientData,
    /// Admitted published-licence material, within its admitted scope.
    AdmittedSource,
    OriginalAuthoring,
}

pub const FACTS_FORMAT: &str = "navzeroth-guide-facts";
pub const FACTS_VERSION: u32 = 1;
pub const RESEARCH_FACTS_VERSION: u32 = 2;
pub const MAX_FACTS_BYTES: usize = 8 * 1024 * 1024;
pub const MAX_FACT_QUESTS: usize = 50_000;
pub const MAX_FACT_NAMES: usize = 4096;
pub const MAX_FACT_LIST: usize = 64;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FactsRejection {
    TooLarge,
    Json,
    Format,
    Capacity,
    Value,
}
impl std::fmt::Display for FactsRejection {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::TooLarge => "Facts file exceeds the 8 MiB limit",
            Self::Json => "Facts file is not valid unique-key JSON of the facts schema",
            Self::Format => "Facts file has an unsupported navzeroth-guide-facts version or origin",
            Self::Capacity => "Facts file exceeds the quest, name or list bounds",
            Self::Value => "Facts file has an out-of-range or invalid value",
        })
    }
}
impl std::error::Error for FactsRejection {}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct NameSet {
    known: Vec<String>,
    /// When true, names not listed are known to be invalid.
    complete: bool,
}
#[derive(Deserialize)]
#[serde(untagged)]
enum AllowedJson<T> {
    Any(AnyTag),
    Only(Vec<T>),
}
#[derive(Deserialize)]
#[serde(rename_all = "snake_case")]
enum AnyTag {
    Any,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct QuestJson {
    required_level: Option<u8>,
    level: Option<u8>,
    prerequisites_any: Option<Vec<QuestId>>,
    prerequisites_all: Option<Vec<QuestId>>,
    races: Option<AllowedJson<Race>>,
    classes: Option<AllowedJson<Class>>,
    repeatable: Option<bool>,
    seasonal: Option<bool>,
    objectives: Option<u8>,
    reward_choices: Option<u8>,
    xp: Option<u32>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct QuestsJson {
    records: BTreeMap<String, QuestJson>,
    #[serde(default)]
    absent: Vec<QuestId>,
    /// When true, quests not listed are known to be absent.
    complete: bool,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct SpellJson {
    classes: Vec<Class>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct FactsJson {
    format: String,
    version: u32,
    origin: FactOrigin,
    edition: Edition,
    /// `null` or absent: the build is unknown.
    client_build: Option<String>,
    quest_log_capacity: Option<u16>,
    level_xp: Option<Vec<u32>>,
    zones: Option<NameSet>,
    flight_nodes: Option<NameSet>,
    quests: Option<QuestsJson>,
    #[serde(default)]
    spells: BTreeMap<String, SpellJson>,
}

/// A fact source read from a `navzeroth-guide-facts` v1 JSON document.
#[derive(Debug, Clone)]
pub struct JsonFacts {
    /// Self-declared by the file. An evidence claim still needs binding to
    /// real observation or extraction records before it counts.
    pub origin: FactOrigin,
    pub edition: Edition,
    pub client_build: Option<String>,
    quest_log_capacity: Option<u16>,
    level_xp: Option<Vec<u32>>,
    zones: Option<(BTreeSet<String>, bool)>,
    flight_nodes: Option<(BTreeSet<String>, bool)>,
    quests: BTreeMap<QuestId, QuestFacts>,
    absent: BTreeSet<QuestId>,
    quests_complete: bool,
    spells: BTreeMap<u32, BTreeSet<Class>>,
}

fn id(text: &str) -> Result<u32, FactsRejection> {
    let canonical = !text.is_empty()
        && text.len() <= 9
        && !text.starts_with('0')
        && text.bytes().all(|b| b.is_ascii_digit());
    canonical
        .then(|| text.parse::<u32>().ok())
        .flatten()
        .filter(|&v| v > 0)
        .ok_or(FactsRejection::Value)
}
fn names(set: Option<NameSet>) -> Result<Option<(BTreeSet<String>, bool)>, FactsRejection> {
    let Some(set) = set else { return Ok(None) };
    if set.known.len() > MAX_FACT_NAMES {
        return Err(FactsRejection::Capacity);
    }
    let mut out = BTreeSet::new();
    for name in set.known {
        if name.trim() != name || name.is_empty() || name.len() > crate::rxp::MAX_LABEL_BYTES {
            return Err(FactsRejection::Value);
        }
        if !out.insert(name) {
            return Err(FactsRejection::Value);
        }
    }
    Ok(Some((out, set.complete)))
}
fn allowed<T: Ord>(value: Option<AllowedJson<T>>) -> Result<Fact<Allowed<T>>, FactsRejection> {
    Ok(match value {
        None => Fact::Unknown,
        Some(AllowedJson::Any(AnyTag::Any)) => Fact::Known(Allowed::Any),
        Some(AllowedJson::Only(list)) => {
            let len = list.len();
            let set: BTreeSet<T> = list.into_iter().collect();
            if set.is_empty() || set.len() != len {
                return Err(FactsRejection::Value);
            }
            Fact::Known(Allowed::Only(set))
        }
    })
}
fn quest_list(list: Option<Vec<QuestId>>) -> Result<Fact<Vec<QuestId>>, FactsRejection> {
    match list {
        Some(l) if l.len() > MAX_FACT_LIST || l.contains(&0) => Err(FactsRejection::Value),
        other => Ok(other.into()),
    }
}

impl JsonFacts {
    pub fn parse(bytes: &[u8]) -> Result<Self, FactsRejection> {
        if bytes.len() > MAX_FACTS_BYTES {
            return Err(FactsRejection::TooLarge);
        }
        let value = azeroth_core::strict_json::value(bytes, MAX_FACTS_BYTES)
            .map_err(|_| FactsRejection::Json)?;
        if value.get("format").and_then(|v| v.as_str()) != Some(FACTS_FORMAT)
            || !matches!(value.get("version").and_then(|v| v.as_u64()), Some(1 | 2))
        {
            return Err(FactsRejection::Format);
        }
        if (value.get("version").and_then(|v| v.as_u64()) == Some(2))
            != (value.get("origin").and_then(|v| v.as_str()) == Some("unverified_research"))
        {
            return Err(FactsRejection::Format);
        }
        let raw: FactsJson = serde_json::from_value(value).map_err(|_| FactsRejection::Json)?;
        debug_assert!(raw.format == FACTS_FORMAT && matches!(raw.version, 1 | 2));
        if (raw.version == 2) != (raw.origin == FactOrigin::UnverifiedResearch) {
            return Err(FactsRejection::Format);
        }
        if let Some(build) = &raw.client_build
            && (build.is_empty()
                || build.len() > 64
                || !build
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || b"._-".contains(&b)))
        {
            return Err(FactsRejection::Value);
        }
        if raw.quest_log_capacity == Some(0) {
            return Err(FactsRejection::Value);
        }
        if let Some(table) = &raw.level_xp
            && (table.is_empty() || table.len() > 99 || table.contains(&0))
        {
            return Err(FactsRejection::Value);
        }
        let mut quests = BTreeMap::new();
        let mut absent = BTreeSet::new();
        let mut quests_complete = false;
        if let Some(q) = raw.quests {
            if q.records.len() + q.absent.len() > MAX_FACT_QUESTS {
                return Err(FactsRejection::Capacity);
            }
            quests_complete = q.complete;
            for (key, r) in q.records {
                let facts = QuestFacts {
                    required_level: r.required_level.into(),
                    level: r.level.into(),
                    prerequisites_any: quest_list(r.prerequisites_any)?,
                    prerequisites_all: quest_list(r.prerequisites_all)?,
                    races: allowed(r.races)?,
                    classes: allowed(r.classes)?,
                    repeatable: r.repeatable.into(),
                    seasonal: r.seasonal.into(),
                    objectives: r.objectives.into(),
                    reward_choices: r.reward_choices.into(),
                    xp: r.xp.into(),
                };
                if quests.insert(id(&key)?, facts).is_some() {
                    return Err(FactsRejection::Value);
                }
            }
            for q in q.absent {
                if q == 0 || quests.contains_key(&q) || !absent.insert(q) {
                    return Err(FactsRejection::Value);
                }
            }
        }
        if raw.spells.len() > MAX_FACT_QUESTS {
            return Err(FactsRejection::Capacity);
        }
        let mut spells = BTreeMap::new();
        for (key, s) in raw.spells {
            let len = s.classes.len();
            let set: BTreeSet<Class> = s.classes.into_iter().collect();
            if set.len() != len {
                return Err(FactsRejection::Value);
            }
            spells.insert(id(&key)?, set);
        }
        Ok(Self {
            origin: raw.origin,
            edition: raw.edition,
            client_build: raw.client_build,
            quest_log_capacity: raw.quest_log_capacity,
            level_xp: raw.level_xp,
            zones: names(raw.zones)?,
            flight_nodes: names(raw.flight_nodes)?,
            quests,
            absent,
            quests_complete,
            spells,
        })
    }
}
fn lookup(set: &Option<(BTreeSet<String>, bool)>, name: &str) -> Fact<bool> {
    match set {
        Some((names, _)) if names.contains(name) => Fact::Known(true),
        Some((_, true)) => Fact::Known(false),
        _ => Fact::Unknown,
    }
}
impl FactSource for JsonFacts {
    fn quest(&self, id: QuestId) -> QuestRecord {
        if let Some(q) = self.quests.get(&id) {
            QuestRecord::Known(q.clone())
        } else if self.absent.contains(&id) || self.quests_complete {
            QuestRecord::Absent
        } else {
            QuestRecord::Unknown
        }
    }
    fn zone(&self, name: &str) -> Fact<bool> {
        lookup(&self.zones, name)
    }
    fn flight_node(&self, name: &str) -> Fact<bool> {
        lookup(&self.flight_nodes, name)
    }
    fn quest_log_capacity(&self) -> Fact<u16> {
        self.quest_log_capacity.into()
    }
    fn spell_trainable(&self, spell: u32, class: Class) -> Fact<bool> {
        self.spells
            .get(&spell)
            .map(|classes| classes.contains(&class))
            .into()
    }
    fn level_xp(&self, level: u8) -> Fact<u32> {
        self.level_xp
            .as_ref()
            .and_then(|t| t.get(usize::from(level).checked_sub(1)?).copied())
            .into()
    }
    fn edition(&self) -> Fact<Edition> {
        Fact::Known(self.edition)
    }
    fn client_build(&self) -> Fact<String> {
        self.client_build.clone().into()
    }
}

// ---------------------------------------------------------------------------
// Report
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Status {
    Ok,
    Error,
    Warning,
    Unverified,
    Unsupported,
    /// Holds only because the guide says so (the `N-M` section-name level), with
    /// no fact behind it. Not ok, and it blocks a clean replay.
    Asserted,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Check {
    // file / guide structure
    BracketBalance,
    HeaderRequired,
    HeaderDuplicate,
    HeaderTag,
    HeaderLine,
    Edition,
    FactsEdition,
    FactsBuild,
    GuideName,
    NextGuide,
    GuideFilter,
    // step structure
    StepFilter,
    StepTag,
    LineFilter,
    Directive,
    Text,
    PlaceholderText,
    StepLocation,
    // quest replay
    QuestExists,
    DoubleAccept,
    ReAcceptDone,
    PrerequisiteAny,
    PrerequisiteAll,
    RequiredLevel,
    RaceAllowed,
    ClassAllowed,
    Repeatable,
    Seasonal,
    QuestLogCapacity,
    CompleteInLog,
    ObjectiveIndex,
    TurnInInLog,
    DoubleTurnIn,
    ObjectivesMarked,
    RewardIndex,
    Abandon,
    LeftInLog,
    // places and training
    Zone,
    Coordinate,
    FlightNode,
    TrainSpell,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Finding {
    pub check: Check,
    pub status: Status,
    pub line: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub quest: Option<QuestId>,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum StepState {
    Replayed,
    /// Excluded by a `<<` filter for this character.
    Filtered,
    /// Skipped by a failed `.isOnQuest`/`.isQuestComplete`/`.isQuestTurnedIn`/`.isQuestAvailable` condition.
    Conditional,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct StepReport {
    pub guide: usize,
    pub step: usize,
    pub line: usize,
    pub state: StepState,
    pub ok: usize,
    pub findings: Vec<Finding>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct GuideReport {
    pub guide: usize,
    pub line: usize,
    pub applies: Applies,
    pub steps: usize,
    pub ok: usize,
    pub findings: Vec<Finding>,
}
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize)]
pub struct Totals {
    pub guides: usize,
    pub steps: usize,
    pub replayed_steps: usize,
    pub skipped_steps: usize,
    pub ok: usize,
    pub errors: usize,
    pub warnings: usize,
    pub unverified: usize,
    pub unsupported: usize,
    pub asserted: usize,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Report {
    pub format: &'static str,
    pub version: u32,
    pub character: Character,
    pub facts_origin: Option<FactOrigin>,
    pub facts_edition: Option<Edition>,
    pub facts_build: Option<String>,
    /// The facts declare a local or admitted origin; synthetic facts never do.
    /// The origin is self-declared by the facts file, so this is a claim that
    /// still needs binding to real observation or extraction records.
    pub facts_are_evidence: bool,
    /// Zero error, unverified, unsupported and asserted findings.
    pub replay_clean: bool,
    /// `replay_clean` with facts that claim evidence. Synthetic facts never pass.
    pub passes: bool,
    pub totals: Totals,
    pub file_findings: Vec<Finding>,
    pub guides: Vec<GuideReport>,
    pub steps: Vec<StepReport>,
    /// Quests still in the log when the replay ends.
    pub left_in_log: Vec<QuestId>,
    pub max_log: usize,
    pub level_gate: u8,
    /// XP-model level at the end, when every turned-in quest had known XP facts.
    pub estimated_level: Option<u8>,
}
pub const REPORT_FORMAT: &str = "navzeroth-guide-verify";
pub const REPORT_VERSION: u32 = 1;

/// Kill/rested-XP multiplier on quest XP in the owner's level model (`gen.py`).
pub const KILL_FACTOR: f64 = 2.2;
pub const MAX_LEVEL: u8 = 60;

// ---------------------------------------------------------------------------
// Replay
// ---------------------------------------------------------------------------

struct Sink<'a> {
    findings: &'a mut Vec<Finding>,
    ok: &'a mut usize,
}
impl Sink<'_> {
    fn push(&mut self, check: Check, status: Status, line: usize, quest: Option<QuestId>) {
        if status == Status::Ok {
            *self.ok += 1;
        } else {
            self.findings.push(Finding {
                check,
                status,
                line,
                quest,
            });
        }
    }
    fn fact(&mut self, check: Check, fact: Fact<bool>, line: usize, quest: Option<QuestId>) {
        let status = match fact {
            Fact::Known(true) => Status::Ok,
            Fact::Known(false) => Status::Error,
            Fact::Unknown => Status::Unverified,
        };
        self.push(check, status, line, quest);
    }
}

struct Estimate {
    level: u8,
    xp: f64,
    /// False once any turned-in quest lacked XP, quest level or the level table.
    complete: bool,
}

struct Replay<'f> {
    facts: &'f dyn FactSource,
    character: Character,
    log: BTreeMap<QuestId, BTreeSet<u8>>,
    done: BTreeSet<QuestId>,
    /// Level guaranteed by `.xp` gates (the addon makes the player reach them).
    level: u8,
    /// Level the guide only declares through `N-M` section names.
    declared: u8,
    estimate: Estimate,
    max_log: usize,
    /// Labels of replayed steps, for `#requires`.
    labels_done: BTreeSet<String>,
}

impl Replay<'_> {
    fn gate(&mut self, level: u8) {
        self.level = self.level.max(level);
        if self.estimate.level < level {
            self.estimate.level = level;
            self.estimate.xp = 0.0;
        }
    }
    fn gain(&mut self, quest: &QuestFacts) {
        let (Some(&xp), Some(&level)) = (quest.xp.known(), quest.level.known()) else {
            self.estimate.complete = false;
            return;
        };
        let diff = i32::from(self.estimate.level) - i32::from(level);
        let factor = match diff {
            ..=5 => 1.0,
            6 => 0.8,
            7 => 0.6,
            8 => 0.4,
            9 => 0.2,
            _ => 0.1,
        };
        self.estimate.xp += f64::from(xp) * factor * KILL_FACTOR;
        while self.estimate.level < MAX_LEVEL {
            let Fact::Known(next) = self.facts.level_xp(self.estimate.level) else {
                self.estimate.complete = false;
                return;
            };
            if self.estimate.xp < f64::from(next) {
                break;
            }
            self.estimate.xp -= f64::from(next);
            self.estimate.level += 1;
        }
    }

    fn accept(&mut self, sink: &mut Sink, line: usize, quest: QuestId, skip_state: bool) {
        let record = self.facts.quest(quest);
        let facts = match record {
            QuestRecord::Absent => {
                sink.push(Check::QuestExists, Status::Error, line, Some(quest));
                return;
            }
            QuestRecord::Unknown => {
                sink.push(Check::QuestExists, Status::Unverified, line, Some(quest));
                QuestFacts::unknown()
            }
            QuestRecord::Known(f) => {
                sink.push(Check::QuestExists, Status::Ok, line, Some(quest));
                f
            }
        };
        if skip_state {
            return; // optional pickups are not part of the default log (verify2)
        }
        let q = Some(quest);
        sink.push(
            Check::DoubleAccept,
            if self.log.contains_key(&quest) {
                Status::Error
            } else {
                Status::Ok
            },
            line,
            q,
        );
        sink.push(
            Check::ReAcceptDone,
            if self.done.contains(&quest) {
                Status::Error
            } else {
                Status::Ok
            },
            line,
            q,
        );
        match &facts.prerequisites_any {
            Fact::Known(list) => sink.push(
                Check::PrerequisiteAny,
                if list.is_empty() || list.iter().any(|p| self.done.contains(p)) {
                    Status::Ok
                } else {
                    Status::Error
                },
                line,
                q,
            ),
            Fact::Unknown => sink.push(Check::PrerequisiteAny, Status::Unverified, line, q),
        }
        match &facts.prerequisites_all {
            Fact::Known(list) => {
                if list.is_empty() {
                    sink.push(Check::PrerequisiteAll, Status::Ok, line, q);
                }
                for p in list {
                    let status = if self.done.contains(p) {
                        Status::Ok
                    } else {
                        Status::Error
                    };
                    sink.push(Check::PrerequisiteAll, status, line, q);
                }
            }
            Fact::Unknown => sink.push(Check::PrerequisiteAll, Status::Unverified, line, q),
        }
        let level = match facts.required_level {
            Fact::Known(required) if required <= self.level => Status::Ok,
            Fact::Known(required) if self.estimate.complete && required <= self.estimate.level => {
                Status::Warning // reached only by the XP estimate
            }
            // Only the section name claims the level: not a verified pass.
            Fact::Known(required) if required <= self.declared => Status::Asserted,
            Fact::Known(_) if self.estimate.complete => Status::Error,
            Fact::Known(_) => Status::Unverified,
            Fact::Unknown => Status::Unverified,
        };
        sink.push(Check::RequiredLevel, level, line, q);
        sink.fact(
            Check::RaceAllowed,
            match &facts.races {
                Fact::Known(a) => Fact::Known(a.allows(&self.character.race)),
                Fact::Unknown => Fact::Unknown,
            },
            line,
            q,
        );
        sink.fact(
            Check::ClassAllowed,
            match &facts.classes {
                Fact::Known(a) => Fact::Known(a.allows(&self.character.class)),
                Fact::Unknown => Fact::Unknown,
            },
            line,
            q,
        );
        sink.fact(Check::Repeatable, negate(&facts.repeatable), line, q);
        sink.fact(Check::Seasonal, negate(&facts.seasonal), line, q);
        self.log.entry(quest).or_default();
        self.max_log = self.max_log.max(self.log.len());
        let capacity = match self.facts.quest_log_capacity() {
            Fact::Known(c) => Fact::Known(self.log.len() <= usize::from(c)),
            Fact::Unknown => Fact::Unknown,
        };
        sink.fact(Check::QuestLogCapacity, capacity, line, q);
    }

    fn complete(
        &mut self,
        sink: &mut Sink,
        line: usize,
        quest: QuestId,
        objective: u8,
        optional: bool,
    ) {
        let q = Some(quest);
        let Some(marked) = self.log.get_mut(&quest) else {
            if !optional {
                sink.push(Check::CompleteInLog, Status::Error, line, q);
            }
            return;
        };
        sink.push(Check::CompleteInLog, Status::Ok, line, q);
        marked.insert(objective);
        let status = match self.facts.quest(quest) {
            QuestRecord::Known(f) => match f.objectives {
                Fact::Known(n) if objective >= 1 && objective <= n => Status::Ok,
                Fact::Known(_) => Status::Error,
                Fact::Unknown => Status::Unverified,
            },
            QuestRecord::Absent => Status::Error,
            QuestRecord::Unknown => Status::Unverified,
        };
        sink.push(Check::ObjectiveIndex, status, line, q);
    }

    fn turn_in(
        &mut self,
        sink: &mut Sink,
        line: usize,
        quest: QuestId,
        reward: Option<u8>,
        optional: bool,
    ) {
        let q = Some(quest);
        let facts = match self.facts.quest(quest) {
            QuestRecord::Absent => {
                sink.push(Check::QuestExists, Status::Error, line, q);
                return;
            }
            QuestRecord::Unknown => {
                sink.push(Check::QuestExists, Status::Unverified, line, q);
                QuestFacts::unknown()
            }
            QuestRecord::Known(f) => {
                sink.push(Check::QuestExists, Status::Ok, line, q);
                f
            }
        };
        if let Some(n) = reward {
            let status = match facts.reward_choices {
                Fact::Known(choices) if n >= 1 && n <= choices => Status::Ok,
                Fact::Known(_) => Status::Error,
                Fact::Unknown => Status::Unverified,
            };
            sink.push(Check::RewardIndex, status, line, q);
        }
        if !self.log.contains_key(&quest) {
            if optional {
                return;
            }
            if self.done.contains(&quest) {
                sink.push(Check::DoubleTurnIn, Status::Error, line, q);
            } else {
                sink.push(Check::TurnInInLog, Status::Error, line, q);
            }
            return;
        }
        sink.push(Check::TurnInInLog, Status::Ok, line, q);
        sink.push(Check::DoubleTurnIn, Status::Ok, line, q);
        let marked = self.log.remove(&quest).unwrap_or_default();
        if !optional {
            let status = match facts.objectives {
                // Invalid indexes are reported independently and must not stand in
                // for a missing declared objective. Zero objectives is complete.
                Fact::Known(n) if (1..=n).all(|index| marked.contains(&index)) => Status::Ok,
                Fact::Known(_) => Status::Warning,
                Fact::Unknown => Status::Unverified,
            };
            sink.push(Check::ObjectivesMarked, status, line, q);
        }
        // The replay treats this turn-in as done (the quest leaves the log), so it
        // also gains its XP, as verify2.py does for every turn-in from the log.
        self.gain(&facts);
        self.done.insert(quest);
    }

    fn place(&self, sink: &mut Sink, check: Check, line: usize, name: &str) {
        let fact = match check {
            Check::FlightNode => match (self.facts.flight_node(name), self.facts.zone(name)) {
                // verify2 also accepts a zone name as a `.fly` destination.
                (Fact::Known(true), _) | (_, Fact::Known(true)) => Fact::Known(true),
                (Fact::Known(false), Fact::Known(false)) => Fact::Known(false),
                _ => Fact::Unknown,
            },
            _ => self.facts.zone(name),
        };
        sink.fact(check, fact, line, None);
    }

    /// Evaluate step conditions; `false` when the addon would skip the step.
    /// Lines excluded by their own `<< filter` don't apply and are not evaluated.
    fn conditions_hold(&self, step: &Step) -> bool {
        let requires = step.tags.iter().all(|(_, t)| match t {
            StepTag::Requires(label) => self.labels_done.contains(label),
            _ => true,
        });
        let applies = |l: &&crate::rxp::StepLine| {
            l.filter
                .as_ref()
                .is_none_or(|f| f.applies(self.character) != Applies::No)
        };
        requires
            && step.lines.iter().filter(applies).all(|l| match &l.item {
                Item::Directive { directive, .. } => match directive {
                    Directive::IsOnQuest(qs) => qs.iter().all(|q| self.log.contains_key(q)),
                    // Objective progress is not modelled: in the log counts as complete.
                    Directive::IsQuestComplete(q) => self.log.contains_key(q),
                    Directive::IsQuestTurnedIn(q) => self.done.contains(q),
                    Directive::IsQuestAvailable(q) => {
                        !self.log.contains_key(q) && !self.done.contains(q)
                    }
                    _ => true,
                },
                _ => true,
            })
    }

    fn step(&mut self, step: &Step, sink: &mut Sink) {
        let optional = step.optional();
        let mut has_command = false;
        let mut has_location = false;
        let quest_state = step.lines.iter().any(|l| {
            matches!(
                l.item,
                Item::Directive {
                    directive: Directive::Accept { .. }
                        | Directive::TurnIn { .. }
                        | Directive::Complete { .. }
                        | Directive::Abandon(_),
                    ..
                }
            )
        });
        for (line, tag) in &step.tags {
            let unsupported = match tag {
                StepTag::Unsupported | StepTag::Level(None) => true,
                // `#level n` shows the step only from level n: decidable only when
                // an `.xp` gate already guarantees n.
                StepTag::Level(Some(n)) => *n > self.level,
                // A `#completewith` step may complete before its own lines run, so
                // its quest-state lines may or may not happen.
                StepTag::CompleteWith(_) => quest_state,
                StepTag::Label(label) => {
                    self.labels_done.insert(label.clone());
                    false
                }
                StepTag::Optional | StepTag::Sticky | StepTag::Requires(_) => false,
            };
            if unsupported {
                sink.push(Check::StepTag, Status::Unsupported, *line, None);
            }
        }
        for l in &step.lines {
            if let Some(filter) = &l.filter {
                match filter.applies(self.character) {
                    Applies::No => continue,
                    Applies::Yes => {}
                    Applies::Unsupported => {
                        sink.push(Check::LineFilter, Status::Unsupported, l.line, None)
                    }
                }
            }
            let directive = match &l.item {
                Item::Directive { directive, .. } => directive,
                Item::Note(..) | Item::Comment => continue,
                Item::UnsupportedText => {
                    sink.push(Check::Text, Status::Unsupported, l.line, None);
                    continue;
                }
            };
            has_command = true;
            let line = l.line;
            match directive {
                Directive::Accept { quest, conditional } => {
                    // `-id` applies `.isOnQuest`: shown only while already on the
                    // quest, where accepting is a no-op. Either way no log change.
                    self.accept(sink, line, *quest, optional || *conditional);
                }
                Directive::TurnIn {
                    quest,
                    reward,
                    conditional,
                } => self.turn_in(sink, line, *quest, *reward, optional || *conditional),
                Directive::Complete { quest, objective } => {
                    self.complete(sink, line, *quest, *objective, optional)
                }
                Directive::Abandon(quest) => {
                    let status = if self.log.remove(quest).is_some() {
                        Status::Ok
                    } else {
                        Status::Warning
                    };
                    sink.push(Check::Abandon, status, line, Some(*quest));
                }
                Directive::Goto { zone, x, y } => {
                    has_location = true;
                    self.place(sink, Check::Zone, line, zone);
                    // Both negative is documented RXP (a dashed map line).
                    let range = 0.0..=100.0;
                    let inside = (range.contains(x) && range.contains(y))
                        || (*x < 0.0 && *y < 0.0 && range.contains(&-x) && range.contains(&-y));
                    sink.push(
                        Check::Coordinate,
                        if inside { Status::Ok } else { Status::Error },
                        line,
                        None,
                    );
                }
                Directive::Zone(zone) => {
                    has_location = true;
                    self.place(sink, Check::Zone, line, zone);
                }
                Directive::Fly(node) | Directive::FlightPath(node) => {
                    has_location |= matches!(directive, Directive::Fly(_));
                    self.place(sink, Check::FlightNode, line, node);
                }
                Directive::Train(spell) => {
                    has_location = true;
                    let fact = self.facts.spell_trainable(*spell, self.character.class);
                    sink.fact(Check::TrainSpell, fact, line, None);
                }
                Directive::Xp { level, kind } => {
                    if *kind == XpKind::Reach {
                        self.gate(*level);
                    }
                }
                Directive::Trainer | Directive::Hearth | Directive::DeathSkip => {
                    has_location = true
                }
                Directive::Malformed => sink.push(Check::Directive, Status::Error, line, None),
                Directive::Unsupported => {
                    sink.push(Check::Directive, Status::Unsupported, line, None)
                }
                // Route instructions without a checkable fact: recorded, not counted as ok.
                Directive::MaxLevel(_)
                | Directive::Home
                | Directive::Vendor
                | Directive::Mob(_)
                | Directive::Use(_)
                | Directive::Collect { .. }
                | Directive::Gossip(_)
                | Directive::IsOnQuest(_)
                | Directive::IsQuestComplete(_)
                | Directive::IsQuestTurnedIn(_)
                | Directive::IsQuestAvailable(_) => {}
            }
        }
        if has_command && !has_location {
            sink.push(Check::StepLocation, Status::Warning, step.line, None);
        }
    }
}

fn negate(fact: &Fact<bool>) -> Fact<bool> {
    match fact {
        Fact::Known(v) => Fact::Known(!v),
        Fact::Unknown => Fact::Unknown,
    }
}

/// Lower bound from a `N-M ...` section name (the owner's naming convention).
fn declared_start_level(name: &str) -> Option<u8> {
    let (start, rest) = name.split_once('-')?;
    let end: String = rest.chars().take_while(char::is_ascii_digit).collect();
    if end.is_empty()
        || start.is_empty()
        || start.len() > 2
        || !start.bytes().all(|b| b.is_ascii_digit())
    {
        return None;
    }
    start.parse().ok().filter(|l| (1..=MAX_LEVEL).contains(l))
}

fn header_checks(guide: &Guide, names: &BTreeMap<String, usize>, sink: &mut Sink) {
    let mut seen: BTreeMap<&'static str, usize> = BTreeMap::new();
    let mut editions = BTreeSet::new();
    for h in &guide.header {
        let key = match &h.tag {
            HeaderTag::Name(_) => "name",
            HeaderTag::DisplayName(_) => "displayname",
            HeaderTag::Next(_) => "next",
            HeaderTag::Group(_) => "group",
            HeaderTag::Subgroup(_) => "subgroup",
            HeaderTag::DefaultFor(f) => {
                if f.as_ref().is_none_or(|f| f.has_unsupported()) {
                    sink.push(Check::HeaderTag, Status::Unsupported, h.line, None);
                }
                "defaultfor"
            }
            HeaderTag::Version(v) => {
                if v.is_none() {
                    sink.push(Check::HeaderTag, Status::Error, h.line, None);
                }
                "version"
            }
            HeaderTag::Edition(e) => {
                editions.insert(*e);
                "edition"
            }
            HeaderTag::Unsupported => {
                sink.push(Check::HeaderTag, Status::Unsupported, h.line, None);
                continue;
            }
        };
        if key != "edition" {
            *seen.entry(key).or_default() += 1;
            if seen[key] > 1 {
                sink.push(Check::HeaderDuplicate, Status::Error, h.line, None);
            }
        }
        if let HeaderTag::Next(next) = &h.tag {
            for n in next {
                let status = if names.contains_key(n) {
                    Status::Ok
                } else {
                    Status::Warning
                };
                sink.push(Check::NextGuide, status, h.line, None);
            }
        }
    }
    for required in ["name", "group", "version"] {
        let status = if seen.contains_key(required) {
            Status::Ok
        } else {
            Status::Error
        };
        sink.push(Check::HeaderRequired, status, guide.line, None);
    }
    // Exactly one edition tag: editions are never mixed silently.
    let status = if editions.len() == 1 {
        Status::Ok
    } else {
        Status::Error
    };
    sink.push(Check::Edition, status, guide.line, None);
    for &line in &guide.unsupported_header_lines {
        sink.push(Check::HeaderLine, Status::Unsupported, line, None);
    }
    for &line in &guide.placeholder_lines {
        sink.push(Check::PlaceholderText, Status::Error, line, None);
    }
    if guide.filters.len() > 1 {
        sink.push(Check::GuideFilter, Status::Error, guide.filters[1].0, None);
    }
}

/// The editions a file declares, for callers that report them.
pub fn editions(file: &GuideFile) -> BTreeSet<Edition> {
    file.guides
        .iter()
        .flat_map(|g| &g.header)
        .filter_map(|h| match h.tag {
            HeaderTag::Edition(e) => Some(e),
            _ => None,
        })
        .collect()
}

/// The character named by the first guide's `#defaultfor` (or its `<<` filter).
pub fn default_character(file: &GuideFile) -> Option<Character> {
    let guide = file.guides.first()?;
    guide
        .header
        .iter()
        .find_map(|h| match &h.tag {
            HeaderTag::DefaultFor(Some(f)) => f.single_character(),
            _ => None,
        })
        .or_else(|| guide.filters.first()?.1.as_ref()?.single_character())
}

/// Replay `file` for `character` against `facts`.
pub fn verify(
    file: &GuideFile,
    facts: &dyn FactSource,
    character: Character,
    origin: Option<FactOrigin>,
) -> Report {
    let mut report = Report {
        format: REPORT_FORMAT,
        version: if origin == Some(FactOrigin::UnverifiedResearch) {
            2
        } else {
            REPORT_VERSION
        },
        character,
        facts_origin: origin,
        facts_edition: facts.edition().known().copied(),
        facts_build: facts.client_build().known().cloned(),
        facts_are_evidence: origin
            .is_some_and(|o| !matches!(o, FactOrigin::Synthetic | FactOrigin::UnverifiedResearch)),
        replay_clean: false,
        passes: false,
        totals: Totals::default(),
        file_findings: Vec::new(),
        guides: Vec::new(),
        steps: Vec::new(),
        left_in_log: Vec::new(),
        max_log: 0,
        level_gate: 1,
        estimated_level: None,
    };
    let mut file_ok = 0;
    {
        let mut sink = Sink {
            findings: &mut report.file_findings,
            ok: &mut file_ok,
        };
        for issue in &file.issues {
            sink.push(Check::BracketBalance, Status::Error, issue.line, None);
        }
        if file.guides.is_empty() {
            sink.push(Check::BracketBalance, Status::Error, 1, None);
        } else if file.issues.is_empty() {
            sink.push(Check::BracketBalance, Status::Ok, 1, None);
        }
        if editions(file).len() > 1 {
            sink.push(Check::Edition, Status::Error, 1, None);
        }
        let build = match facts.client_build() {
            Fact::Known(_) => Status::Ok,
            Fact::Unknown => Status::Unverified,
        };
        sink.push(Check::FactsBuild, build, 1, None);
    }
    let mut names = BTreeMap::new();
    for (index, guide) in file.guides.iter().enumerate() {
        if let Some(name) = guide.name()
            && names.insert(name.to_owned(), index).is_some()
        {
            report.file_findings.push(Finding {
                check: Check::GuideName,
                status: Status::Error,
                line: guide.line,
                quest: None,
            });
        }
    }
    let all_labels: BTreeSet<&str> = file
        .guides
        .iter()
        .flat_map(|g| &g.steps)
        .flat_map(|s| &s.tags)
        .filter_map(|(_, t)| match t {
            StepTag::Label(l) => Some(l.as_str()),
            _ => None,
        })
        .collect();
    let mut replay = Replay {
        facts,
        character,
        log: BTreeMap::new(),
        done: BTreeSet::new(),
        level: 1,
        declared: 1,
        estimate: Estimate {
            level: 1,
            xp: 0.0,
            complete: true,
        },
        max_log: 0,
        labels_done: BTreeSet::new(),
    };
    let mut totals = Totals {
        guides: file.guides.len(),
        ok: file_ok,
        ..Totals::default()
    };
    for (gi, guide) in file.guides.iter().enumerate() {
        let mut g = GuideReport {
            guide: gi,
            line: guide.line,
            applies: Applies::Yes,
            steps: guide.steps.len(),
            ok: 0,
            findings: Vec::new(),
        };
        let mut sink = Sink {
            findings: &mut g.findings,
            ok: &mut g.ok,
        };
        header_checks(guide, &names, &mut sink);
        // The guide's edition must be the facts' edition (verify2.py required
        // `#forever`; here the facts name the target edition).
        let guide_editions: BTreeSet<Edition> = guide
            .header
            .iter()
            .filter_map(|h| match h.tag {
                HeaderTag::Edition(e) => Some(e),
                _ => None,
            })
            .collect();
        if let Some(edition) = guide_editions.first() {
            let status = match facts.edition() {
                Fact::Known(target) if guide_editions.len() == 1 && *edition == target => {
                    Status::Ok
                }
                Fact::Known(_) => Status::Error,
                Fact::Unknown => Status::Unverified,
            };
            sink.push(Check::FactsEdition, status, guide.line, None);
        }
        g.applies = match guide.filters.first() {
            None => Applies::Yes,
            Some((line, None)) => {
                sink.push(Check::GuideFilter, Status::Unsupported, *line, None);
                Applies::Unsupported
            }
            Some((line, Some(f))) => {
                let a = f.applies(character);
                if a == Applies::Unsupported {
                    sink.push(Check::GuideFilter, Status::Unsupported, *line, None);
                }
                a
            }
        };
        if g.applies != Applies::No
            && let Some(level) = guide.name().and_then(declared_start_level)
        {
            replay.declared = replay.declared.max(level);
        }
        totals.steps += guide.steps.len();
        for (si, step) in guide.steps.iter().enumerate() {
            let mut s = StepReport {
                guide: gi,
                step: si,
                line: step.line,
                state: StepState::Replayed,
                ok: 0,
                findings: Vec::new(),
            };
            let mut step_sink = Sink {
                findings: &mut s.findings,
                ok: &mut s.ok,
            };
            let filtered = g.applies == Applies::No
                || match &step.filter {
                    None => false,
                    Some(None) => {
                        step_sink.push(Check::StepFilter, Status::Unsupported, step.line, None);
                        false
                    }
                    Some(Some(f)) => match f.applies(character) {
                        Applies::No => true,
                        Applies::Yes => false,
                        Applies::Unsupported => {
                            step_sink.push(Check::StepFilter, Status::Unsupported, step.line, None);
                            false
                        }
                    },
                };
            if filtered {
                s.state = StepState::Filtered;
            } else if !replay.conditions_hold(step) {
                s.state = StepState::Conditional;
                for (line, tag) in &step.tags {
                    if let StepTag::Requires(label) = tag
                        && !all_labels.contains(label.as_str())
                    {
                        step_sink.push(Check::StepTag, Status::Error, *line, None);
                    }
                }
            } else {
                replay.step(step, &mut step_sink);
            }
            if s.state == StepState::Replayed {
                totals.replayed_steps += 1;
            } else {
                totals.skipped_steps += 1;
            }
            report.steps.push(s);
        }
        report.guides.push(g);
    }
    for quest in replay.log.keys() {
        report.file_findings.push(Finding {
            check: Check::LeftInLog,
            status: Status::Warning,
            line: 0,
            quest: Some(*quest),
        });
    }
    let all = report
        .file_findings
        .iter()
        .chain(report.guides.iter().flat_map(|g| &g.findings))
        .chain(report.steps.iter().flat_map(|s| &s.findings));
    for f in all {
        match f.status {
            Status::Ok => totals.ok += 1,
            Status::Error => totals.errors += 1,
            Status::Warning => totals.warnings += 1,
            Status::Unverified => totals.unverified += 1,
            Status::Unsupported => totals.unsupported += 1,
            Status::Asserted => totals.asserted += 1,
        }
    }
    totals.ok += report.guides.iter().map(|g| g.ok).sum::<usize>()
        + report.steps.iter().map(|s| s.ok).sum::<usize>();
    report.replay_clean = totals.errors == 0
        && totals.unverified == 0
        && totals.unsupported == 0
        && totals.asserted == 0;
    report.passes = report.replay_clean && report.facts_are_evidence;
    report.totals = totals;
    report.left_in_log = replay.log.keys().copied().collect();
    report.max_log = replay.max_log;
    report.level_gate = replay.level;
    report.estimated_level = replay.estimate.complete.then_some(replay.estimate.level);
    report
}
