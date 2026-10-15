//! Read-only ForeverDB 0.34.0 quest-source staging. No Lua execution, acquisition,
//! admission, playable conversion or persistence. Namespace labels are source
//! declarations, never independently verified target-game evidence.
use crate::savedvars::{self, LuaKey, LuaTable, LuaValue, source_literal_prefix};
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};

pub const ADAPTER_VERSION: &str = "foreverdb-quest-staging-v1";
pub const MAX_FILE_BYTES: usize = savedvars::MAX_FILE_BYTES;
pub const MAX_VALUES: usize = crate::questiedb::MAX_VALUES;
pub const MAX_QUESTS: usize = 100_000;
const FOREVER: &[&str] = &[
    "qfQuests",
    "qfPoi",
    "qfSeen",
    "qfNpcNames",
    "qfNpcSpots",
    "qfItems",
    "qfArea",
    "qfByMap",
];
const CLASSIC: &[&str] = &[
    "qcQuests",
    "qcNpc",
    "qcObj",
    "qcItem",
    "qcByMap",
    "qcExcl",
    "dgClassic",
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Rejection {
    Capacity,
    Wrapper,
    Schema,
    Version,
    Metadata,
    Quest,
    Literal(savedvars::Rejection),
}
impl std::fmt::Display for Rejection {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::Capacity => "ForeverDB source exceeds staging bounds",
            Self::Wrapper => "ForeverDB source is not a supported inert namespace wrapper",
            Self::Schema => "ForeverDB source assignments differ from the supported schema",
            Self::Version => "ForeverDB addon version has not been reviewed by this adapter",
            Self::Metadata => "ForeverDB origin metadata is invalid",
            Self::Quest => "ForeverDB quest identity or compact record is invalid",
            Self::Literal(_) => "ForeverDB source contains unsupported or malformed literal data",
        })
    }
}
impl std::error::Error for Rejection {}
type Result<T> = std::result::Result<T, Rejection>;

struct Cursor<'a> {
    rest: &'a str,
}
impl Cursor<'_> {
    fn skip(&mut self) -> Result<()> {
        loop {
            self.rest = self.rest.trim_start_matches([' ', '\t', '\r', '\n']);
            if let Some(comment) = self.rest.strip_prefix("--") {
                if comment.starts_with('[') {
                    return Err(Rejection::Wrapper);
                }
                self.rest = comment.split_once('\n').map_or("", |(_, r)| r);
            } else {
                return Ok(());
            }
        }
    }
    fn token(&mut self, text: &str) -> Result<()> {
        self.skip()?;
        self.rest = self.rest.strip_prefix(text).ok_or(Rejection::Wrapper)?;
        Ok(())
    }
    fn word(&mut self, text: &str) -> Result<()> {
        self.token(text)?;
        if self
            .rest
            .as_bytes()
            .first()
            .is_some_and(|b| b.is_ascii_alphanumeric() || *b == b'_')
        {
            return Err(Rejection::Wrapper);
        }
        Ok(())
    }
    fn identifier(&mut self) -> Result<String> {
        self.skip()?;
        let n = self
            .rest
            .bytes()
            .take_while(|b| b.is_ascii_alphanumeric() || *b == b'_')
            .count();
        if n == 0 || n > 64 {
            return Err(Rejection::Wrapper);
        }
        let name = self.rest[..n].to_owned();
        self.rest = &self.rest[n..];
        Ok(name)
    }
}
fn values(value: &LuaValue) -> usize {
    1 + match value {
        LuaValue::Table(t) => t.values().map(values).sum(),
        _ => 0,
    }
}
fn parse(input: &[u8], expected: &[&str]) -> Result<BTreeMap<String, LuaTable>> {
    if input.len() > MAX_FILE_BYTES {
        return Err(Rejection::Capacity);
    }
    let text = std::str::from_utf8(input).map_err(|_| Rejection::Wrapper)?;
    let mut c = Cursor { rest: text };
    c.word("local")?;
    c.word("_")?;
    c.token(",")?;
    c.word("ns")?;
    c.token("=")?;
    c.token("...")?;
    let mut tables = BTreeMap::new();
    let mut count = 0usize;
    loop {
        c.skip()?;
        if c.rest.is_empty() {
            break;
        }
        if tables.len() >= expected.len() {
            return Err(Rejection::Schema);
        }
        c.word("ns")?;
        c.token(".")?;
        let name = c.identifier()?;
        if !expected.contains(&name.as_str()) || tables.contains_key(&name) {
            return Err(Rejection::Schema);
        }
        c.token("=")?;
        let (value, used) = source_literal_prefix(c.rest).map_err(Rejection::Literal)?;
        count = count
            .checked_add(values(&value))
            .ok_or(Rejection::Capacity)?;
        if count > MAX_VALUES {
            return Err(Rejection::Capacity);
        }
        let LuaValue::Table(table) = value else {
            return Err(Rejection::Schema);
        };
        c.rest = &c.rest[used..];
        tables.insert(name, table);
    }
    if tables.len() != expected.len() {
        return Err(Rejection::Schema);
    }
    Ok(tables)
}
fn string(t: &LuaTable, key: &str) -> Result<String> {
    match t.get(&LuaKey::Str(key.to_owned())) {
        Some(LuaValue::Str(s)) => Ok(s.clone()),
        _ => Err(Rejection::Metadata),
    }
}
fn integer(t: &LuaTable, key: &str) -> Result<u64> {
    match t.get(&LuaKey::Str(key.to_owned())) {
        Some(LuaValue::Int(n)) => u64::try_from(*n).map_err(|_| Rejection::Metadata),
        _ => Err(Rejection::Metadata),
    }
}
#[derive(Debug, Clone, Serialize)]
pub struct Metadata {
    pub addon_version: String,
    pub data_revision: String,
    pub bundle_origin_build: String,
    pub built_epoch: u64,
    pub level_cap: u64,
}
fn metadata(t: &LuaTable) -> Result<Metadata> {
    let keys: BTreeSet<_> = t.keys().cloned().collect();
    if keys
        != ["addon", "build", "built", "cap", "site", "version"]
            .into_iter()
            .map(|s| LuaKey::Str(s.to_owned()))
            .collect()
    {
        return Err(Rejection::Metadata);
    }
    let addon_version = string(t, "addon")?;
    if addon_version != "0.34.0" {
        return Err(Rejection::Version);
    }
    let bundle_origin_build = string(t, "build")?;
    let suffix = bundle_origin_build
        .strip_prefix("1.60.1.")
        .ok_or(Rejection::Metadata)?;
    if suffix.is_empty() || suffix.len() > 10 || !suffix.bytes().all(|b| b.is_ascii_digit()) {
        return Err(Rejection::Metadata);
    }
    let data_revision = string(t, "version")?;
    if data_revision.is_empty()
        || data_revision.len() > 80
        || !data_revision
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b".-_".contains(&b))
        || string(t, "site")? != "https://foreverdb.net"
    {
        return Err(Rejection::Metadata);
    }
    let level_cap = integer(t, "cap")?;
    if level_cap > 255 {
        return Err(Rejection::Metadata);
    }
    Ok(Metadata {
        addon_version,
        data_revision,
        bundle_origin_build,
        built_epoch: integer(t, "built")?,
        level_cap,
    })
}
fn quest_ids(table: &LuaTable, fields: usize) -> Result<Vec<u32>> {
    if table.len() > MAX_QUESTS {
        return Err(Rejection::Capacity);
    }
    table
        .iter()
        .map(|(key, value)| {
            let LuaKey::Int(n) = key else {
                return Err(Rejection::Quest);
            };
            let id = u32::try_from(*n)
                .ok()
                .filter(|n| *n > 0)
                .ok_or(Rejection::Quest)?;
            let LuaValue::Str(record) = value else {
                return Err(Rejection::Quest);
            };
            if record.split('|').count() != fields {
                return Err(Rejection::Quest);
            }
            Ok(id)
        })
        .collect()
}
#[derive(Debug, Serialize)]
pub struct FileReport {
    pub bytes: usize,
    pub sha256: String,
    pub assignments: BTreeMap<String, usize>,
}
fn file_report(input: &[u8], tables: &BTreeMap<String, LuaTable>) -> FileReport {
    FileReport {
        bytes: input.len(),
        sha256: format!("{:x}", Sha256::digest(input)),
        assignments: tables.iter().map(|(n, t)| (n.clone(), t.len())).collect(),
    }
}
#[derive(Debug, Serialize)]
pub struct QuestInventory {
    /// The file's namespace is a declared provenance stratum, not validation.
    pub declared_stratum: &'static str,
    pub declared_edition: &'static str,
    pub declared_record_build: Option<String>,
    pub source_quest_ids: Vec<u32>,
}
#[derive(Debug, Serialize)]
pub struct Report {
    pub report_version: u32,
    pub adapter_version: &'static str,
    pub metadata: Metadata,
    pub files: BTreeMap<&'static str, FileReport>,
    pub quest_inventories: Vec<QuestInventory>,
    pub overlapping_quest_ids: Vec<u32>,
    pub independently_required_quest_ids: Option<Vec<u32>>,
    pub target_validation: &'static str,
    pub rights_admission: &'static str,
    pub publication_allowed: bool,
    pub blockers: Vec<&'static str>,
}
/// Source values stay private and non-serializable. Consumers may inspect the
/// literal facts; no stage type is convertible to an admitted catalog.
#[derive(Debug)]
pub struct StagedBundle {
    metadata: Metadata,
    forever: BTreeMap<String, LuaTable>,
    classic: BTreeMap<String, LuaTable>,
    files: BTreeMap<&'static str, FileReport>,
    forever_ids: Vec<u32>,
    classic_ids: Vec<u32>,
}
impl StagedBundle {
    pub fn table(&self, stratum: Stratum, name: &str) -> Option<&LuaTable> {
        match stratum {
            Stratum::Forever => self.forever.get(name),
            Stratum::Classic => self.classic.get(name),
        }
    }
    pub fn report(&self) -> Report {
        let classic: BTreeSet<_> = self.classic_ids.iter().copied().collect();
        Report {
            report_version: 1,
            adapter_version: ADAPTER_VERSION,
            metadata: self.metadata.clone(),
            files: self
                .files
                .iter()
                .map(|(name, f)| {
                    (
                        *name,
                        FileReport {
                            bytes: f.bytes,
                            sha256: f.sha256.clone(),
                            assignments: f.assignments.clone(),
                        },
                    )
                })
                .collect(),
            quest_inventories: vec![
                QuestInventory {
                    declared_stratum: "forever_file",
                    declared_edition: "wow_forever",
                    declared_record_build: None,
                    source_quest_ids: self.forever_ids.clone(),
                },
                QuestInventory {
                    declared_stratum: "classic_cmangos_file",
                    declared_edition: "wow_classic",
                    declared_record_build: None,
                    source_quest_ids: self.classic_ids.clone(),
                },
            ],
            overlapping_quest_ids: self
                .forever_ids
                .iter()
                .copied()
                .filter(|id| classic.contains(id))
                .collect(),
            independently_required_quest_ids: None,
            target_validation: "unverified",
            rights_admission: "unreviewed",
            publication_allowed: false,
            blockers: vec![
                "Source rights and corresponding-source obligations require review",
                "Target 70374 applicability is unverified",
                "Independent complete target inventory is unavailable",
                "Raw compact fields and other catalog domains are not normalized",
            ],
        }
    }
}
#[derive(Clone, Copy)]
pub enum Stratum {
    Forever,
    Classic,
}
/// All three complete wrappers must parse. Unknown assignments, duplicate keys,
/// executable suffixes and unsupported versions fail without partial output.
pub fn stage(meta: &[u8], forever: &[u8], classic: &[u8]) -> Result<StagedBundle> {
    let meta_tables = parse(meta, &["meta"])?;
    let metadata = metadata(&meta_tables["meta"])?;
    let forever_tables = parse(forever, FOREVER)?;
    let classic_tables = parse(classic, CLASSIC)?;
    let forever_ids = quest_ids(&forever_tables["qfQuests"], 8)?;
    let classic_ids = quest_ids(&classic_tables["qcQuests"], 14)?;
    let files = [
        ("ForeverDB/Data/Meta.lua", file_report(meta, &meta_tables)),
        (
            "ForeverDB/Data/QuestForever.lua",
            file_report(forever, &forever_tables),
        ),
        (
            "ForeverDB/Data/QuestClassic.lua",
            file_report(classic, &classic_tables),
        ),
    ]
    .into_iter()
    .collect();
    Ok(StagedBundle {
        metadata,
        forever: forever_tables,
        classic: classic_tables,
        files,
        forever_ids,
        classic_ids,
    })
}
