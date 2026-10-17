//! Inert importer for RXP-format leveling guides (task V2.3).
//!
//! A guide file is a Lua source that calls `RXPGuides.RegisterGuide([[ ... ]])`
//! once per section. This module never runs that Lua and has no Lua engine: it
//! scans the text line by line for the exact call form, takes the long-string body
//! as data, and parses the RXP guide grammar inside it (header tags, `<<` filters,
//! `step` blocks and their directives). Anything outside that grammar is kept as an
//! explicit unsupported item with its line number; nothing is silently dropped.
//!
//! The grammar is ported from the owner's guide generator (`gen.py`/`verify2.py`,
//! kept outside the repository) and the RXP documentation the owner keeps with it.
//! Parsed facts in a guide (quest IDs, coordinates, names) are route design, not
//! admitted content: the verifier in [`crate::rxp_verify`] checks them against a
//! separate fact source.
//!
//! Every size is bounded. Oversize input is refused as a whole, never truncated.
use serde::Serialize;
use std::collections::BTreeSet;

pub const MAX_FILE_BYTES: usize = 4 * 1024 * 1024;
pub const MAX_LINE_BYTES: usize = 2048;
pub const MAX_GUIDES: usize = 512;
pub const MAX_STEPS_PER_GUIDE: usize = 4096;
pub const MAX_TOTAL_STEPS: usize = 32_768;
pub const MAX_LINES_PER_STEP: usize = 256;
pub const MAX_HEADER_LINES: usize = 64;
pub const MAX_ISSUES: usize = 4096;
/// Header values and names (zone, flight node) are short labels.
pub const MAX_LABEL_BYTES: usize = 160;

const OPENER: &str = "RXPGuides.RegisterGuide([[";
const CLOSER: &str = "]]";

pub type QuestId = u32;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Rejection {
    /// The file exceeds [`MAX_FILE_BYTES`].
    TooLarge,
    /// The file is not UTF-8 or contains a NUL byte.
    Encoding,
    /// A line exceeds [`MAX_LINE_BYTES`].
    LineTooLong,
    /// Guide, step, line or issue counts exceed their bounds.
    Capacity,
}
impl std::fmt::Display for Rejection {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::TooLarge => "Guide file exceeds the 4 MiB limit",
            Self::Encoding => "Guide file is not NUL-free UTF-8 text",
            Self::LineTooLong => "Guide file has a line longer than 2048 bytes",
            Self::Capacity => "Guide file exceeds the guide, step or line bounds",
        })
    }
}
impl std::error::Error for Rejection {}

// ---------------------------------------------------------------------------
// Characters and filters
// ---------------------------------------------------------------------------

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, serde::Deserialize,
)]
pub enum Race {
    Human,
    Dwarf,
    NightElf,
    Gnome,
    Orc,
    Undead,
    Tauren,
    Troll,
}
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, serde::Deserialize,
)]
pub enum Class {
    Warrior,
    Paladin,
    Hunter,
    Rogue,
    Priest,
    Shaman,
    Mage,
    Warlock,
    Druid,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
pub enum Faction {
    Alliance,
    Horde,
}
impl Race {
    pub const ALL: [Race; 8] = [
        Self::Human,
        Self::Dwarf,
        Self::NightElf,
        Self::Gnome,
        Self::Orc,
        Self::Undead,
        Self::Tauren,
        Self::Troll,
    ];
    pub fn faction(self) -> Faction {
        match self {
            Self::Human | Self::Dwarf | Self::NightElf | Self::Gnome => Faction::Alliance,
            _ => Faction::Horde,
        }
    }
    /// RXP token, with the client's `Scourge` alias for Undead.
    pub fn from_token(token: &str) -> Option<Self> {
        Some(match token {
            "Human" => Self::Human,
            "Dwarf" => Self::Dwarf,
            "NightElf" => Self::NightElf,
            "Gnome" => Self::Gnome,
            "Orc" => Self::Orc,
            "Undead" | "Scourge" => Self::Undead,
            "Tauren" => Self::Tauren,
            "Troll" => Self::Troll,
            _ => return None,
        })
    }
}
impl Class {
    pub const ALL: [Class; 9] = [
        Self::Warrior,
        Self::Paladin,
        Self::Hunter,
        Self::Rogue,
        Self::Priest,
        Self::Shaman,
        Self::Mage,
        Self::Warlock,
        Self::Druid,
    ];
    pub fn from_token(token: &str) -> Option<Self> {
        Some(match token {
            "Warrior" => Self::Warrior,
            "Paladin" => Self::Paladin,
            "Hunter" => Self::Hunter,
            "Rogue" => Self::Rogue,
            "Priest" => Self::Priest,
            "Shaman" => Self::Shaman,
            "Mage" => Self::Mage,
            "Warlock" => Self::Warlock,
            "Druid" => Self::Druid,
            _ => return None,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct Character {
    pub race: Race,
    pub class: Class,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Term {
    Race(Race),
    Class(Class),
    Faction(Faction),
    /// A token this importer does not evaluate (edition/realm tokens such as
    /// `tbc`, misspellings). Kept, never guessed.
    Unsupported,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Literal {
    pub negated: bool,
    pub term: Term,
}
/// RXP filter: `/` is OR, spaces are AND, `!` is NOT, with precedence
/// `!` > space > `/` (RXP documentation). Stored as a disjunction of conjunctions.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Filter {
    pub any_of: Vec<Vec<Literal>>,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Applies {
    Yes,
    No,
    /// The filter contains a token that cannot be evaluated.
    Unsupported,
}
impl Filter {
    fn parse(text: &str) -> Option<Self> {
        if text.contains("<<") {
            return None; // chained `<< a << b` is not evaluated
        }
        let mut any_of = Vec::new();
        for alternative in text.split('/') {
            let mut all = Vec::new();
            for token in alternative.split_whitespace() {
                let (negated, name) = match token.strip_prefix('!') {
                    Some(rest) => (true, rest),
                    None => (false, token),
                };
                let term = if let Some(r) = Race::from_token(name) {
                    Term::Race(r)
                } else if let Some(c) = Class::from_token(name) {
                    Term::Class(c)
                } else if name == "Alliance" {
                    Term::Faction(Faction::Alliance)
                } else if name == "Horde" {
                    Term::Faction(Faction::Horde)
                } else {
                    Term::Unsupported
                };
                all.push(Literal { negated, term });
            }
            if all.is_empty() {
                return None;
            }
            any_of.push(all);
        }
        (!any_of.is_empty()).then_some(Self { any_of })
    }
    pub fn has_unsupported(&self) -> bool {
        self.any_of
            .iter()
            .flatten()
            .any(|l| l.term == Term::Unsupported)
    }
    pub fn applies(&self, c: Character) -> Applies {
        let mut unsupported = false;
        for all in &self.any_of {
            let mut conj = Some(true);
            for literal in all {
                let value = match literal.term {
                    Term::Race(r) => Some(r == c.race),
                    Term::Class(k) => Some(k == c.class),
                    Term::Faction(f) => Some(f == c.race.faction()),
                    Term::Unsupported => None,
                }
                .map(|v| v != literal.negated);
                conj = match (conj, value) {
                    (Some(false), _) | (_, Some(false)) => Some(false),
                    (Some(true), Some(true)) => Some(true),
                    _ => None,
                };
            }
            match conj {
                Some(true) => return Applies::Yes,
                None => unsupported = true,
                Some(false) => {}
            }
        }
        if unsupported {
            Applies::Unsupported
        } else {
            Applies::No
        }
    }
    /// The single race and class this filter names positively, if exactly one each.
    pub fn single_character(&self) -> Option<Character> {
        let [all] = self.any_of.as_slice() else {
            return None;
        };
        let races: BTreeSet<_> = all
            .iter()
            .filter_map(|l| match l.term {
                Term::Race(r) if !l.negated => Some(r),
                _ => None,
            })
            .collect();
        let classes: BTreeSet<_> = all
            .iter()
            .filter_map(|l| match l.term {
                Term::Class(c) if !l.negated => Some(c),
                _ => None,
            })
            .collect();
        match (races.len(), classes.len()) {
            (1, 1) => Some(Character {
                race: *races.first()?,
                class: *classes.first()?,
            }),
            _ => None,
        }
    }
}

// ---------------------------------------------------------------------------
// Parsed structure
// ---------------------------------------------------------------------------

/// Structural problems found while scanning. The verifier reports each as an error.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum StructureIssue {
    /// A `RegisterGuide([[` block has no `]]` before end of file or before the next block.
    UnclosedBlock,
    /// `]]` is not followed by `)`.
    MalformedCloser,
    /// `]]` appears outside a guide block.
    StrayCloser,
    /// Non-comment code outside a guide block (another call form, `[==[`, Lua code).
    TopLevelCode,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct Issue {
    pub line: usize,
    pub issue: StructureIssue,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Edition {
    Classic,
    Era,
    Som,
    Tbc,
    Wotlk,
    Cata,
    Forever,
}
impl Edition {
    fn from_tag(tag: &str) -> Option<Self> {
        Some(match tag {
            "classic" => Self::Classic,
            "era" => Self::Era,
            "som" => Self::Som,
            "tbc" => Self::Tbc,
            "wotlk" => Self::Wotlk,
            "cata" => Self::Cata,
            "forever" => Self::Forever,
            _ => return None,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HeaderTag {
    Name(String),
    DisplayName(String),
    /// `#next a;b` names one or more following guides.
    Next(Vec<String>),
    Group(String),
    Subgroup(String),
    DefaultFor(Option<Filter>),
    Version(Option<u32>),
    Edition(Edition),
    /// A tag this importer does not interpret, or a tag value that is malformed
    /// or over the label bound.
    Unsupported,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HeaderLine {
    pub line: usize,
    pub tag: HeaderTag,
}
#[derive(Debug, Clone, PartialEq)]
pub struct Guide {
    /// Line of `RXPGuides.RegisterGuide([[`.
    pub line: usize,
    pub header: Vec<HeaderLine>,
    /// Guide-level `<< ...` filters with their lines (more than one is reported).
    pub filters: Vec<(usize, Option<Filter>)>,
    /// Header lines that are neither a tag, a filter nor blank.
    pub unsupported_header_lines: Vec<usize>,
    pub steps: Vec<Step>,
}
impl Guide {
    pub fn name(&self) -> Option<&str> {
        self.header.iter().find_map(|h| match &h.tag {
            HeaderTag::Name(n) => Some(n.as_str()),
            _ => None,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StepTag {
    Optional,
    Sticky,
    Label(String),
    CompleteWith(String),
    Requires(String),
    Level(Option<u8>),
    Unsupported,
}
#[derive(Debug, Clone, PartialEq)]
pub struct Step {
    pub line: usize,
    /// `step << filter`; `Some(None)` is a filter that does not parse.
    pub filter: Option<Option<Filter>>,
    pub tags: Vec<(usize, StepTag)>,
    pub lines: Vec<StepLine>,
}
impl Step {
    pub fn optional(&self) -> bool {
        self.tags.iter().any(|(_, t)| *t == StepTag::Optional)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct StepLine {
    pub line: usize,
    pub item: Item,
    /// A trailing ` << filter` on a directive line.
    pub filter: Option<Filter>,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NoteKind {
    /// `>>text`, alone or after a directive.
    Default,
    /// `*text`
    StepWindow,
    /// `+text`
    Objective,
}
#[derive(Debug, Clone, PartialEq)]
pub enum Item {
    Directive {
        directive: Directive,
        /// Text after `>>`, kept as data.
        note: Option<String>,
    },
    Note(NoteKind, String),
    /// A `--` line inside a guide body.
    Comment,
    /// A line in a step that is not a directive, note, tag or comment.
    UnsupportedText,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum XpKind {
    /// `.xp N`, `.xp N.f`, `.xp N+x`, `.xp N-x`, `.xp N,1`: afterwards the player is at least level N.
    Reach,
    /// `.xp <N,1`: completes while below level N; no level gate.
    Below,
}
#[derive(Debug, Clone, PartialEq)]
pub enum Directive {
    Accept {
        quest: QuestId,
        /// `.accept -id` applies `.isOnQuest`-style skipping in RXP.
        conditional: bool,
    },
    TurnIn {
        quest: QuestId,
        reward: Option<u8>,
        conditional: bool,
    },
    Complete {
        quest: QuestId,
        objective: u8,
    },
    Abandon(QuestId),
    Goto {
        zone: String,
        x: f64,
        y: f64,
    },
    Fly(String),
    FlightPath(String),
    Zone(String),
    Train(u32),
    Trainer,
    Xp {
        level: u8,
        kind: XpKind,
    },
    MaxLevel(u8),
    Home,
    Hearth,
    Vendor,
    Mob(String),
    Use(u32),
    Collect {
        item: u32,
        count: u32,
    },
    Gossip(u32),
    DeathSkip,
    IsOnQuest(Vec<QuestId>),
    IsQuestComplete(QuestId),
    IsQuestTurnedIn(QuestId),
    IsQuestAvailable(QuestId),
    /// A recognized directive whose arguments do not parse.
    Malformed,
    /// A directive this importer does not interpret. Kept, never dropped.
    Unsupported,
}

#[derive(Debug, Clone, PartialEq)]
pub struct GuideFile {
    pub guides: Vec<Guide>,
    pub issues: Vec<Issue>,
}

// ---------------------------------------------------------------------------
// Scanning
// ---------------------------------------------------------------------------

struct Budget {
    steps: usize,
    issues: usize,
}
impl Budget {
    fn issue(
        &mut self,
        issues: &mut Vec<Issue>,
        line: usize,
        issue: StructureIssue,
    ) -> Result<(), Rejection> {
        self.issues += 1;
        if self.issues > MAX_ISSUES {
            return Err(Rejection::Capacity);
        }
        issues.push(Issue { line, issue });
        Ok(())
    }
}

/// Parse a guide file as data. No Lua is executed.
pub fn parse(bytes: &[u8]) -> Result<GuideFile, Rejection> {
    if bytes.len() > MAX_FILE_BYTES {
        return Err(Rejection::TooLarge);
    }
    if bytes.contains(&0) {
        return Err(Rejection::Encoding);
    }
    let text = std::str::from_utf8(bytes).map_err(|_| Rejection::Encoding)?;
    let text = text.strip_prefix('\u{feff}').unwrap_or(text);
    let mut budget = Budget {
        steps: 0,
        issues: 0,
    };
    let mut issues = Vec::new();
    let mut guides = Vec::new();
    // (start line, body lines)
    let mut open: Option<(usize, Vec<(usize, &str)>)> = None;
    for (index, raw) in text.split('\n').enumerate() {
        let number = index + 1;
        let raw = raw.strip_suffix('\r').unwrap_or(raw);
        if raw.len() > MAX_LINE_BYTES {
            return Err(Rejection::LineTooLong);
        }
        let trimmed = raw.trim();
        if let Some(rest) = trimmed.strip_prefix(OPENER) {
            if let Some((start, _)) = open.take() {
                budget.issue(&mut issues, start, StructureIssue::UnclosedBlock)?;
            }
            open = Some((number, Vec::new()));
            if let Some(done) = take_body_line(&mut open, number, rest, &mut issues, &mut budget)? {
                push_guide(&mut guides, done, &mut budget)?;
            }
            continue;
        }
        if open.is_some() {
            if let Some(done) = take_body_line(&mut open, number, raw, &mut issues, &mut budget)? {
                push_guide(&mut guides, done, &mut budget)?;
            }
            continue;
        }
        if trimmed.is_empty() || (trimmed.starts_with("--") && !trimmed.starts_with("--[")) {
            continue;
        }
        let issue = if trimmed.contains(CLOSER) {
            StructureIssue::StrayCloser
        } else {
            StructureIssue::TopLevelCode
        };
        budget.issue(&mut issues, number, issue)?;
    }
    if let Some((start, _)) = open {
        budget.issue(&mut issues, start, StructureIssue::UnclosedBlock)?;
    }
    Ok(GuideFile { guides, issues })
}

type Block<'a> = (usize, Vec<(usize, &'a str)>);

/// Add one body line to the open block; returns the block when `]]` closes it.
fn take_body_line<'a>(
    open: &mut Option<Block<'a>>,
    number: usize,
    line: &'a str,
    issues: &mut Vec<Issue>,
    budget: &mut Budget,
) -> Result<Option<Block<'a>>, Rejection> {
    let Some((_, body)) = open.as_mut() else {
        return Ok(None);
    };
    if let Some(at) = line.find(CLOSER) {
        let (inside, after) = line.split_at(at);
        if !inside.trim().is_empty() {
            body.push((number, inside));
        }
        let after = after[CLOSER.len()..].trim();
        let closed = after
            .strip_prefix(')')
            .is_some_and(|rest| rest.trim().is_empty() || rest.trim() == ";");
        if !closed {
            budget.issue(issues, number, StructureIssue::MalformedCloser)?;
        }
        return Ok(open.take());
    }
    if body.len() >= MAX_STEPS_PER_GUIDE * 8 {
        return Err(Rejection::Capacity);
    }
    body.push((number, line));
    Ok(None)
}

fn push_guide(
    guides: &mut Vec<Guide>,
    (start, body): Block<'_>,
    budget: &mut Budget,
) -> Result<(), Rejection> {
    if guides.len() >= MAX_GUIDES {
        return Err(Rejection::Capacity);
    }
    let guide = parse_guide(start, &body)?;
    budget.steps += guide.steps.len();
    if budget.steps > MAX_TOTAL_STEPS {
        return Err(Rejection::Capacity);
    }
    guides.push(guide);
    Ok(())
}

fn label(value: &str) -> Option<String> {
    let value = value.trim();
    (!value.is_empty() && value.len() <= MAX_LABEL_BYTES && !value.chars().any(char::is_control))
        .then(|| value.to_owned())
}

fn parse_header_tag(tag: &str, value: &str) -> HeaderTag {
    let labelled = |f: fn(String) -> HeaderTag| label(value).map_or(HeaderTag::Unsupported, f);
    match tag {
        "name" => labelled(HeaderTag::Name),
        "displayname" => labelled(HeaderTag::DisplayName),
        "group" => labelled(HeaderTag::Group),
        "subgroup" => labelled(HeaderTag::Subgroup),
        "next" => {
            let names: Option<Vec<String>> = value.split(';').map(label).collect();
            match names {
                Some(names) if !names.is_empty() && names.len() <= 16 => HeaderTag::Next(names),
                _ => HeaderTag::Unsupported,
            }
        }
        "defaultfor" => HeaderTag::DefaultFor(Filter::parse(value)),
        "version" => HeaderTag::Version(small_number(value.trim())),
        other => match Edition::from_tag(other) {
            Some(edition) if value.trim().is_empty() => HeaderTag::Edition(edition),
            _ => HeaderTag::Unsupported,
        },
    }
}

fn split_tag(line: &str) -> Option<(&str, &str)> {
    let rest = line.strip_prefix('#')?;
    let end = rest.find(char::is_whitespace).unwrap_or(rest.len());
    let (tag, value) = rest.split_at(end);
    (!tag.is_empty()).then_some((tag, value))
}

fn parse_guide(start: usize, body: &[(usize, &str)]) -> Result<Guide, Rejection> {
    let mut guide = Guide {
        line: start,
        header: Vec::new(),
        filters: Vec::new(),
        unsupported_header_lines: Vec::new(),
        steps: Vec::new(),
    };
    let mut header_lines = 0;
    for &(number, raw) in body {
        let line = raw.trim();
        if line.is_empty() {
            continue;
        }
        if line == "step" || line.starts_with("step ") || line.starts_with("step\t") {
            if guide.steps.len() >= MAX_STEPS_PER_GUIDE {
                return Err(Rejection::Capacity);
            }
            let rest = line[4..].trim();
            let filter = if rest.is_empty() {
                None
            } else {
                Some(
                    rest.strip_prefix("<<")
                        .and_then(|f| Filter::parse(f.trim())),
                )
            };
            guide.steps.push(Step {
                line: number,
                filter,
                tags: Vec::new(),
                lines: Vec::new(),
            });
            continue;
        }
        if let Some(step) = guide.steps.last_mut() {
            if step.tags.len() + step.lines.len() >= MAX_LINES_PER_STEP {
                return Err(Rejection::Capacity);
            }
            if let Some((tag, value)) = split_tag(line) {
                step.tags.push((number, parse_step_tag(tag, value)));
            } else {
                step.lines.push(parse_step_line(number, line));
            }
            continue;
        }
        header_lines += 1;
        if header_lines > MAX_HEADER_LINES {
            return Err(Rejection::Capacity);
        }
        if let Some((tag, value)) = split_tag(line) {
            guide.header.push(HeaderLine {
                line: number,
                tag: parse_header_tag(tag, value),
            });
        } else if let Some(filter) = line.strip_prefix("<<") {
            guide.filters.push((number, Filter::parse(filter.trim())));
        } else if !line.starts_with("--") {
            guide.unsupported_header_lines.push(number);
        }
    }
    Ok(guide)
}

fn parse_step_tag(tag: &str, value: &str) -> StepTag {
    let value = value.trim();
    match (tag, value.is_empty()) {
        ("optional", true) => StepTag::Optional,
        ("sticky", true) => StepTag::Sticky,
        ("label", false) => label(value).map_or(StepTag::Unsupported, StepTag::Label),
        ("completewith", false) => label(value).map_or(StepTag::Unsupported, StepTag::CompleteWith),
        ("requires", false) => label(value).map_or(StepTag::Unsupported, StepTag::Requires),
        ("level", false) => StepTag::Level(small_number(value)),
        _ => StepTag::Unsupported,
    }
}

fn parse_step_line(number: usize, line: &str) -> StepLine {
    let plain = |item| StepLine {
        line: number,
        item,
        filter: None,
    };
    if line.starts_with("--") {
        return plain(Item::Comment);
    }
    if let Some(text) = line.strip_prefix(">>") {
        return plain(Item::Note(NoteKind::Default, text.trim().to_owned()));
    }
    if let Some(text) = line.strip_prefix('*') {
        return plain(Item::Note(NoteKind::StepWindow, text.trim().to_owned()));
    }
    if let Some(text) = line.strip_prefix('+') {
        return plain(Item::Note(NoteKind::Objective, text.trim().to_owned()));
    }
    let Some(rest) = line.strip_prefix('.') else {
        return plain(Item::UnsupportedText);
    };
    // Split off the `>>` note, then a ` << filter` (before the note, or trailing the
    // note when every token is supported), then a `--` label comment
    // (`.complete 7,1 --Kobold Vermin`).
    let (mut command, mut note) = match rest.split_once(">>") {
        Some((c, n)) => (c, Some(n.trim())),
        None => (rest, None),
    };
    let mut filter = None;
    if let Some(at) = command.find(" << ") {
        filter = Filter::parse(command[at + 4..].trim());
        command = &command[..at];
        if filter.is_none() {
            return plain(Item::Directive {
                directive: Directive::Malformed,
                note: note.map(str::to_owned),
            });
        }
    } else if let Some(text) = note
        && let Some(at) = text.rfind(" << ")
        && let Some(f) = Filter::parse(text[at + 4..].trim()).filter(|f| !f.has_unsupported())
    {
        filter = Some(f);
        note = Some(text[..at].trim());
    }
    let note = note.map(str::to_owned);
    let command = command.split_once(" --").map_or(command, |(c, _)| c).trim();
    let end = command.find(char::is_whitespace).unwrap_or(command.len());
    let (name, args) = command.split_at(end);
    StepLine {
        line: number,
        item: Item::Directive {
            directive: parse_directive(name, args.trim()),
            note,
        },
        filter,
    }
}

fn small_number<T: std::str::FromStr>(text: &str) -> Option<T> {
    (!text.is_empty() && text.len() <= 9 && text.bytes().all(|b| b.is_ascii_digit()))
        .then(|| text.parse().ok())
        .flatten()
}
fn quest_id(text: &str) -> Option<(QuestId, bool)> {
    let (conditional, digits) = match text.trim().strip_prefix('-') {
        Some(d) => (true, d),
        None => (false, text.trim()),
    };
    small_number::<QuestId>(digits)
        .filter(|&q| q > 0)
        .map(|q| (q, conditional))
}
fn coordinate(text: &str) -> Option<f64> {
    let text = text.trim();
    let ok = !text.is_empty()
        && text.len() <= 12
        && text
            .bytes()
            .all(|b| b.is_ascii_digit() || b == b'.' || b == b'-');
    ok.then(|| text.parse::<f64>().ok())
        .flatten()
        .filter(|v| v.is_finite())
}

fn parse_directive(name: &str, args: &str) -> Directive {
    let parts: Vec<&str> = args.split(',').map(str::trim).collect();
    let single = || (parts.len() == 1).then(|| parts[0]);
    let parsed = match name {
        "accept" => {
            quest_id(parts[0]).map(|(quest, conditional)| Directive::Accept { quest, conditional })
        }
        "turnin" => quest_id(parts[0]).and_then(|(quest, conditional)| {
            let reward = match parts.get(1) {
                None => None,
                Some(r) => Some(small_number::<u8>(r)?),
            };
            (parts.len() <= 2).then_some(Directive::TurnIn {
                quest,
                reward,
                conditional,
            })
        }),
        "complete" => match parts.as_slice() {
            [q, o] => quest_id(q)
                .filter(|(_, c)| !c)
                .zip(small_number::<u8>(o))
                .map(|((quest, _), objective)| Directive::Complete { quest, objective }),
            _ => None,
        },
        "abandon" => single()
            .and_then(quest_id)
            .filter(|(_, c)| !c)
            .map(|(q, _)| Directive::Abandon(q)),
        "goto" | "waypoint" => match parts.as_slice() {
            [zone, x, y, extra @ ..]
                if extra.len() <= 3 && extra.iter().all(|e| coordinate(e).is_some()) =>
            {
                label(zone)
                    .zip(coordinate(x).zip(coordinate(y)))
                    .map(|(zone, (x, y))| Directive::Goto { zone, x, y })
            }
            _ => None,
        },
        "fly" => label(args).map(Directive::Fly),
        "fp" => label(args).map(Directive::FlightPath),
        "zone" => label(args).map(Directive::Zone),
        "mob" => label(args).map(Directive::Mob),
        "train" => single().and_then(small_number).map(Directive::Train),
        "xp" => parse_xp(args),
        "maxlevel" => single().and_then(small_number).map(Directive::MaxLevel),
        "use" => single().and_then(small_number).map(Directive::Use),
        "gossipoption" => single().and_then(small_number).map(Directive::Gossip),
        "collect" => match parts.as_slice() {
            [item, count, ..] if parts.len() <= 5 => small_number(item)
                .zip(small_number(count))
                .map(|(item, count)| Directive::Collect { item, count }),
            _ => None,
        },
        "isOnQuest" => parts
            .iter()
            .map(|p| quest_id(p).filter(|(_, c)| !c).map(|(q, _)| q))
            .collect::<Option<Vec<_>>>()
            .filter(|v| !v.is_empty() && v.len() <= 16)
            .map(Directive::IsOnQuest),
        "isQuestComplete" => single()
            .and_then(quest_id)
            .map(|(q, _)| Directive::IsQuestComplete(q)),
        "isQuestTurnedIn" => single()
            .and_then(quest_id)
            .map(|(q, _)| Directive::IsQuestTurnedIn(q)),
        "isQuestAvailable" => single()
            .and_then(quest_id)
            .map(|(q, _)| Directive::IsQuestAvailable(q)),
        "trainer" | "home" | "hs" | "vendor" | "deathskip" if args.is_empty() => Some(match name {
            "trainer" => Directive::Trainer,
            "home" => Directive::Home,
            "hs" => Directive::Hearth,
            "vendor" => Directive::Vendor,
            _ => Directive::DeathSkip,
        }),
        "trainer" | "home" | "hs" | "vendor" | "deathskip" => None,
        _ => return Directive::Unsupported,
    };
    parsed.unwrap_or(Directive::Malformed)
}

fn parse_xp(args: &str) -> Option<Directive> {
    let (body, skip) = match args.split_once(',') {
        Some((b, s)) => (b.trim(), Some(s.trim())),
        None => (args.trim(), None),
    };
    if skip.is_some_and(|s| s != "1") {
        return None;
    }
    if let Some(level) = body.strip_prefix('<') {
        return small_number(level.trim()).map(|level| Directive::Xp {
            level,
            kind: XpKind::Below,
        });
    }
    let end = body.find(['.', '+', '-']).unwrap_or(body.len());
    let (level, rest) = body.split_at(end);
    if !rest.is_empty() && small_number::<u32>(&rest[1..]).is_none() {
        return None;
    }
    small_number::<u8>(level)
        .filter(|l| (1..=100).contains(l))
        .map(|level| Directive::Xp {
            level,
            kind: XpKind::Reach,
        })
}
