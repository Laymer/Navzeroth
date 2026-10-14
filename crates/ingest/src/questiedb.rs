//! Bounded QuestieDB raw-source staging; never a gameplay dataset.
//!
//! Mapping v1 describes the four wrappers and field indices audited at
//! Questie/QuestieDB@9bc909a464ce0af80af83ce8a4f959b5f911fe88. It parses the
//! deferred literal table and validates the whole inert wrapper. It does not
//! evaluate the loader call printed in that wrapper, any correction provider,
//! derived pass, or field meaning. Records may be semantically incomplete.
//! Source identity is caller-declared; unknown build/edition never become the
//! requested target. Reports are serializable, raw gameplay rows are not.
use crate::savedvars::{self, LuaKey, LuaTable, LuaValue, source_literal_prefix};
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};

pub const ADAPTER_VERSION: &str = "questiedb-raw-staging-v1";
pub const AUDITED_REVISION: &str = "9bc909a464ce0af80af83ce8a4f959b5f911fe88";
pub const MAX_FILE_BYTES: usize = savedvars::MAX_FILE_BYTES;
pub const MAX_RECORDS: usize = 100_000;
pub const MAX_STRING_BYTES: usize = 64 * 1024;
pub const MAX_VALUES: usize = 1_000_000;
pub const MAX_TABLE_DEPTH: usize = savedvars::MAX_TABLE_DEPTH;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Entity {
    Quest,
    Npc,
    Item,
    Object,
}
impl Entity {
    pub const ALL: [Self; 4] = [Self::Quest, Self::Npc, Self::Item, Self::Object];

    fn prefix(self) -> &'static str {
        match self {
            Self::Quest => "quest",
            Self::Npc => "npc",
            Self::Item => "item",
            Self::Object => "object",
        }
    }

    /// Raw-file schema identifiers, not game-content facts. Indices are
    /// one-based. The pinned raw Item table declares 15 fields; canonical
    /// itemMeta also has teachesSpell at 16 for later correction providers.
    pub fn fields(self) -> &'static [&'static str] {
        match self {
            Self::Quest => &[
                "name",
                "startedBy",
                "finishedBy",
                "requiredLevel",
                "questLevel",
                "requiredRaces",
                "requiredClasses",
                "objectivesText",
                "triggerEnd",
                "objectives",
                "sourceItemId",
                "preQuestGroup",
                "preQuestSingle",
                "childQuests",
                "inGroupWith",
                "exclusiveTo",
                "zoneOrSort",
                "requiredSkill",
                "requiredMinRep",
                "requiredMaxRep",
                "requiredSourceItems",
                "nextQuestInChain",
                "questFlags",
                "specialFlags",
                "parentQuest",
                "reputationReward",
                "breadcrumbForQuestId",
                "breadcrumbs",
                "extraObjectives",
                "requiredSpell",
                "requiredSpecialization",
                "requiredMaxLevel",
                "availableUntilCompleted",
                "availableStartingWith",
                "requiredRanks",
                "disabledByQuest",
            ],
            Self::Npc => &[
                "name",
                "minLevelHealth",
                "maxLevelHealth",
                "minLevel",
                "maxLevel",
                "rank",
                "spawns",
                "waypoints",
                "zoneID",
                "questStarts",
                "questEnds",
                "factionID",
                "friendlyToFaction",
                "subName",
                "npcFlags",
            ],
            Self::Item => &[
                "name",
                "npcDrops",
                "objectDrops",
                "itemDrops",
                "startQuest",
                "questRewards",
                "flags",
                "foodType",
                "itemLevel",
                "requiredLevel",
                "ammoType",
                "class",
                "subClass",
                "vendors",
                "relatedQuests",
            ],
            Self::Object => &[
                "name",
                "questStarts",
                "questEnds",
                "spawns",
                "zoneID",
                "factionID",
                "waypoints",
            ],
        }
    }
}

/// Caller declarations, not independently verified source/build evidence.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize)]
pub struct SourceOrigin {
    pub revision: Option<String>,
    pub edition: Option<String>,
    pub build: Option<String>,
}
impl SourceOrigin {
    fn validate(&self) -> Result<()> {
        if self.revision.as_ref().is_some_and(|revision| {
            revision.len() != 40
                || !revision
                    .bytes()
                    .all(|c| c.is_ascii_digit() || (b'a'..=b'f').contains(&c))
        }) {
            return Err(Rejection::Origin);
        }
        for value in [&self.edition, &self.build].into_iter().flatten() {
            if value.is_empty()
                || value.len() > 64
                || !value
                    .bytes()
                    .all(|c| c.is_ascii_alphanumeric() || b"._-".contains(&c))
            {
                return Err(Rejection::Origin);
            }
        }
        if self
            .edition
            .as_deref()
            .is_some_and(|edition| edition != "wow_forever")
        {
            return Err(Rejection::EditionMismatch);
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Rejection {
    FileTooLarge,
    NotUtf8,
    Wrapper,
    Schema,
    RecordLimit,
    RecordId,
    RecordShape,
    FieldIndex,
    Origin,
    EditionMismatch,
    MixedOrigins,
    DuplicateInput,
    EmptyInput,
    UnknownCorrection,
    Literal(savedvars::Rejection),
}
impl std::fmt::Display for Rejection {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::FileTooLarge => "Source file exceeds its size limit",
            Self::NotUtf8 => "Source file is not UTF-8",
            Self::Wrapper => "Source file does not match the supported raw wrapper",
            Self::Schema => "Source field declarations do not match the audited schema",
            Self::RecordLimit => "Source file holds too many records",
            Self::RecordId => "Source record ID must be a positive u32 integer",
            Self::RecordShape => "Source records must be literal tables",
            Self::FieldIndex => "Source record contains an unknown field index",
            Self::Origin => "Source identity is malformed or exceeds its bounds",
            Self::EditionMismatch => "This adapter only stages declared Forever or unknown sources",
            Self::MixedOrigins => "Source inputs have different declared origins",
            Self::DuplicateInput => "Source input is repeated",
            Self::EmptyInput => "At least one raw source table is required",
            Self::UnknownCorrection => "Correction path is not in the audited inventory",
            Self::Literal(_) => "Source contains unsupported or malformed bounded literal data",
        })
    }
}
impl std::error::Error for Rejection {}
impl From<savedvars::Rejection> for Rejection {
    fn from(value: savedvars::Rejection) -> Self {
        Self::Literal(value)
    }
}
type Result<T> = std::result::Result<T, Rejection>;

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct FileFingerprint {
    pub bytes: usize,
    pub sha256: String,
}
fn fingerprint(input: &[u8]) -> FileFingerprint {
    FileFingerprint {
        bytes: input.len(),
        sha256: format!("{:x}", Sha256::digest(input)),
    }
}
fn bounded_text(input: &[u8]) -> Result<&str> {
    if input.len() > MAX_FILE_BYTES {
        return Err(Rejection::FileTooLarge);
    }
    std::str::from_utf8(input).map_err(|_| Rejection::NotUtf8)
}

#[derive(Debug, PartialEq)]
pub struct StagedTable {
    entity: Entity,
    origin: SourceOrigin,
    fingerprint: FileFingerprint,
    records: BTreeMap<u32, LuaTable>,
}
impl StagedTable {
    pub fn entity(&self) -> Entity {
        self.entity
    }
    pub fn origin(&self) -> &SourceOrigin {
        &self.origin
    }
    pub fn fingerprint(&self) -> &FileFingerprint {
        &self.fingerprint
    }
    pub fn records(&self) -> &BTreeMap<u32, LuaTable> {
        &self.records
    }
}

/// No normalization, gameplay validation, correction application or activation.
/// Only the audited wrapper's fixed loader declaration is recognized as inert
/// syntax. Additional calls/statements before, inside or after it are rejected.
pub fn stage_raw(input: &[u8], entity: Entity, origin: SourceOrigin) -> Result<StagedTable> {
    let text = bounded_text(input)?;
    origin.validate()?;
    let mut outer = Wrapper { remaining: text };
    outer.expect("local QuestieDB = QuestieLoader:ImportModule(\"QuestieDB\");")?;
    outer.expect(&format!("QuestieDB.{}Keys", entity.prefix()))?;
    outer.expect("=")?;
    let declarations = outer.literal()?;
    let LuaValue::Table(declarations) = declarations else {
        return Err(Rejection::Schema);
    };
    if declarations.len() != entity.fields().len() {
        return Err(Rejection::Schema);
    }
    for (index, name) in entity.fields().iter().enumerate() {
        if declarations.get(&LuaKey::Str((*name).to_owned()))
            != Some(&LuaValue::Int(
                i64::try_from(index + 1).map_err(|_| Rejection::Schema)?,
            ))
        {
            return Err(Rejection::Schema);
        }
    }
    outer.expect(&format!("QuestieDB.{}Data", entity.prefix()))?;
    outer.expect("=")?;
    outer.expect("[[")?;
    // The first delimiter ends a Lua long string, including inside quoted text
    // of the inner data. Never search for the last delimiter and skip suffixes.
    let (body, suffix) = outer.remaining.split_once("]]").ok_or(Rejection::Wrapper)?;
    let mut inner = Wrapper { remaining: body };
    inner.expect("return")?;
    let LuaValue::Table(rows) = inner.literal()? else {
        return Err(Rejection::RecordShape);
    };
    inner.end()?;
    outer.remaining = suffix;
    outer.end()?;
    if rows.len() > MAX_RECORDS {
        return Err(Rejection::RecordLimit);
    }
    let mut records = BTreeMap::new();
    for (key, value) in rows {
        let LuaKey::Int(id) = key else {
            return Err(Rejection::RecordId);
        };
        let id = u32::try_from(id)
            .ok()
            .filter(|id| *id != 0)
            .ok_or(Rejection::RecordId)?;
        let LuaValue::Table(fields) = value else {
            return Err(Rejection::RecordShape);
        };
        for key in fields.keys() {
            let LuaKey::Int(index) = key else {
                return Err(Rejection::FieldIndex);
            };
            if *index <= 0 || usize::try_from(*index).map_or(true, |i| i > entity.fields().len()) {
                return Err(Rejection::FieldIndex);
            }
        }
        records.insert(id, fields);
    }
    Ok(StagedTable {
        entity,
        origin,
        fingerprint: fingerprint(input),
        records,
    })
}

struct Wrapper<'a> {
    remaining: &'a str,
}
impl Wrapper<'_> {
    fn skip(&mut self) -> Result<()> {
        loop {
            self.remaining = self.remaining.trim_start_matches([' ', '\t', '\r', '\n']);
            if !self.remaining.starts_with("--") {
                return Ok(());
            }
            if self.remaining.starts_with("--[[") || self.remaining.starts_with("--[=") {
                return Err(Rejection::Wrapper);
            }
            self.remaining = self.remaining.split_once('\n').map_or("", |(_, tail)| tail);
        }
    }
    fn expect(&mut self, token: &str) -> Result<()> {
        self.skip()?;
        self.remaining = self
            .remaining
            .strip_prefix(token)
            .ok_or(Rejection::Wrapper)?;
        Ok(())
    }
    fn literal(&mut self) -> Result<LuaValue> {
        let (value, consumed) = source_literal_prefix(self.remaining)?;
        self.remaining = &self.remaining[consumed..];
        Ok(value)
    }
    fn end(&mut self) -> Result<()> {
        self.skip()?;
        if self.remaining.is_empty() {
            Ok(())
        } else {
            Err(Rejection::Wrapper)
        }
    }
}

/// Exact pinned inventory; a different source revision remains unaudited even
/// when all these paths are supplied. None is applied by this adapter.
pub const CORRECTION_PATHS: [&str; 18] = [
    "src/corrections/Forever/legacy/classicQuestFixes.lua",
    "src/corrections/Forever/legacy/classicNPCFixes.lua",
    "src/corrections/Forever/legacy/classicItemFixes.lua",
    "src/corrections/Forever/legacy/classicObjectFixes.lua",
    "src/corrections/Forever/legacy/classicQuestReputationFixes.lua",
    "src/corrections/Forever/legacy/itemStartFixes.lua",
    "src/corrections/Forever/generated/foreverBaseNpc.lua",
    "src/corrections/Forever/generated/foreverBaseObject.lua",
    "src/corrections/Forever/generated/foreverBaseQuest.lua",
    "src/corrections/Forever/generated/foreverBaseItem.lua",
    "src/corrections/Forever/traces/foreverQuestTraces.lua",
    "src/corrections/Forever/traces/foreverNpcTraces.lua",
    "src/corrections/Forever/traces/foreverItemTraces.lua",
    "src/corrections/Forever/traces/foreverObjectTraces.lua",
    "src/corrections/Forever/foreverQuestFixes.lua",
    "src/corrections/Forever/foreverNPCFixes.lua",
    "src/corrections/Forever/foreverItemFixes.lua",
    "src/corrections/Forever/foreverObjectFixes.lua",
];

#[derive(Debug, PartialEq)]
pub struct UnsupportedCorrection {
    path: &'static str,
    origin: SourceOrigin,
    fingerprint: FileFingerprint,
}

/// Inventory a provider without evaluating any Lua or applying any row. Even
/// a literal-looking provider is unsupported: precedence, symbolic operations,
/// branches and derived passes require a separately reviewed adapter.
pub fn inspect_correction(
    input: &[u8],
    path: &str,
    origin: SourceOrigin,
) -> Result<UnsupportedCorrection> {
    bounded_text(input)?;
    origin.validate()?;
    let path = *CORRECTION_PATHS
        .iter()
        .find(|known| **known == path)
        .ok_or(Rejection::UnknownCorrection)?;
    Ok(UnsupportedCorrection {
        path,
        origin,
        fingerprint: fingerprint(input),
    })
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct RawFileSummary {
    pub entity: Entity,
    pub fingerprint: FileFingerprint,
    pub parsed_records: usize,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct CorrectionSummary {
    pub path: &'static str,
    pub fingerprint: FileFingerprint,
    pub disposition: &'static str,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct StagingReport {
    pub adapter_version: &'static str,
    pub audited_revision: &'static str,
    pub caller_declared_origin: SourceOrigin,
    pub declared_revision_matches_audit: bool,
    pub raw_files: Vec<RawFileSummary>,
    pub missing_raw_entities: Vec<Entity>,
    pub unsupported_corrections: Vec<CorrectionSummary>,
    pub missing_corrections: Vec<&'static str>,
    pub corrections_applied: usize,
    pub correction_coverage_complete: bool,
    pub derived_passes_applied: bool,
    pub source_build_verified: bool,
    pub target_inventory_complete: bool,
    pub distribution_eligible: bool,
    pub playable_dataset: bool,
}

/// Aggregate in deterministic entity/path order; do not merge conflicting or
/// unknown-versus-known source identities, duplicate files or separate editions.
pub fn summarize(
    tables: &[StagedTable],
    corrections: &[UnsupportedCorrection],
) -> Result<StagingReport> {
    let origin = &tables.first().ok_or(Rejection::EmptyInput)?.origin;
    let mut raw = BTreeMap::new();
    for table in tables {
        if &table.origin != origin {
            return Err(Rejection::MixedOrigins);
        }
        if raw
            .insert(
                table.entity,
                RawFileSummary {
                    entity: table.entity,
                    fingerprint: table.fingerprint.clone(),
                    parsed_records: table.records.len(),
                },
            )
            .is_some()
        {
            return Err(Rejection::DuplicateInput);
        }
    }
    let mut unsupported = BTreeMap::new();
    for correction in corrections {
        if &correction.origin != origin {
            return Err(Rejection::MixedOrigins);
        }
        if unsupported
            .insert(
                correction.path,
                CorrectionSummary {
                    path: correction.path,
                    fingerprint: correction.fingerprint.clone(),
                    disposition: "unsupported_symbolic_or_executable_provider_not_applied",
                },
            )
            .is_some()
        {
            return Err(Rejection::DuplicateInput);
        }
    }
    let supplied: BTreeSet<_> = unsupported.keys().copied().collect();
    Ok(StagingReport {
        adapter_version: ADAPTER_VERSION,
        audited_revision: AUDITED_REVISION,
        caller_declared_origin: origin.clone(),
        declared_revision_matches_audit: origin.revision.as_deref() == Some(AUDITED_REVISION),
        missing_raw_entities: Entity::ALL
            .into_iter()
            .filter(|e| !raw.contains_key(e))
            .collect(),
        raw_files: raw.into_values().collect(),
        missing_corrections: CORRECTION_PATHS
            .into_iter()
            .filter(|p| !supplied.contains(p))
            .collect(),
        unsupported_corrections: unsupported.into_values().collect(),
        corrections_applied: 0,
        correction_coverage_complete: false,
        derived_passes_applied: false,
        source_build_verified: false,
        target_inventory_complete: false,
        distribution_eligible: false,
        playable_dataset: false,
    })
}
