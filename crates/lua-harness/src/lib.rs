//! Dev-only Lua 5.1 test harness for the diagnostic addon (task M0.3).
//!
//! This crate executes only this repository's own addon sources
//! (`addons/AzerothNavigator`) and the small Lua snippets written in this crate's
//! tests, inside a vendored Lua 5.1 state with a restricted standard library. It
//! never loads SavedVariables files, imported data or any text that came from a
//! player, a web page or a content file; plan section 7.2 forbids executing imported
//! Lua, and nothing here relaxes that rule. The desktop importer must parse
//! SavedVariables as data without a Lua interpreter.
//!
//! The crate is test tooling. It must never become a dependency of `azeroth-core`,
//! `azeroth-app`, `azeroth-cli` or `azeroth-desktop`.
//!
//! The sandbox approximates, and does not reproduce, the WoW Lua environment: see
//! `docs/compatibility/LUA_RUNTIME_DIFFERENCES.md`. Standalone Lua tests verify pure
//! module logic only, never Blizzard API behaviour (plan section 18.3).

use mlua::{FromLuaMulti, HookTriggers, Lua, LuaOptions, StdLib, Table, Value, VmState};
use std::{
    collections::BTreeMap,
    fmt, fs, io,
    path::{Path, PathBuf},
    sync::{
        Arc, Mutex,
        atomic::{AtomicU64, Ordering},
    },
};

/// Addon folder name; the first vararg every addon file receives.
pub const ADDON_NAME: &str = "AzerothNavigator";
/// SavedVariables global declared by the addon TOC (UNVERIFIED until M0.5).
pub const SAVED_VARIABLES_NAME: &str = "AzerothNavigatorDB";

const MEMORY_LIMIT_BYTES: usize = 64 * 1024 * 1024;
const HOOK_INTERVAL: u32 = 1000;
const MAX_HOOK_CALLS: u64 = 100_000;

/// Failure while loading or running harness Lua.
#[derive(Debug)]
pub enum HarnessError {
    Lua(mlua::Error),
    Io { path: PathBuf, source: io::Error },
    Toc(String),
}

impl fmt::Display for HarnessError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Lua(error) => write!(f, "Lua error: {error}"),
            Self::Io { path, source } => write!(f, "cannot read {}: {source}", path.display()),
            Self::Toc(message) => write!(f, "invalid TOC: {message}"),
        }
    }
}

impl std::error::Error for HarnessError {}

impl From<mlua::Error> for HarnessError {
    fn from(error: mlua::Error) -> Self {
        Self::Lua(error)
    }
}

pub type Result<T> = std::result::Result<T, HarnessError>;

/// Absolute path of the addon folder in this repository.
pub fn addon_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../addons/AzerothNavigator")
}

/// Absolute path of the repository root.
pub fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// Reads a repository file as UTF-8 text.
pub fn read_text(path: &Path) -> Result<String> {
    fs::read_to_string(path).map_err(|source| HarnessError::Io {
        path: path.to_path_buf(),
        source,
    })
}

/// A parsed addon table of contents.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Toc {
    /// `## Key: Value` directives, keys as written.
    pub directives: BTreeMap<String, String>,
    /// `# text` comment lines, without the marker.
    pub comments: Vec<String>,
    /// Listed files in load order, normalised to forward slashes.
    pub files: Vec<String>,
}

impl Toc {
    /// Parses TOC text. Tolerates CRLF line endings.
    pub fn parse(text: &str) -> Self {
        let mut toc = Self::default();
        for raw in text.lines() {
            let line = raw.trim();
            if line.is_empty() {
                continue;
            }
            if let Some(directive) = line.strip_prefix("##") {
                if let Some((key, value)) = directive.split_once(':') {
                    toc.directives
                        .insert(key.trim().to_owned(), value.trim().to_owned());
                }
            } else if let Some(comment) = line.strip_prefix('#') {
                toc.comments.push(comment.trim().to_owned());
            } else {
                toc.files.push(line.replace('\\', "/"));
            }
        }
        toc
    }

    /// Reads and parses `AzerothNavigator.toc` from the addon folder.
    pub fn read() -> Result<Self> {
        let path = addon_dir().join(format!("{ADDON_NAME}.toc"));
        Ok(Self::parse(&read_text(&path)?))
    }
}

/// Every `.lua` file under the addon folder, relative, forward-slashed and sorted.
pub fn lua_files_on_disk() -> Result<Vec<String>> {
    fn walk(dir: &Path, prefix: &str, out: &mut Vec<String>) -> Result<()> {
        let io_error = |source| HarnessError::Io {
            path: dir.to_path_buf(),
            source,
        };
        for entry in fs::read_dir(dir).map_err(io_error)? {
            let entry = entry.map_err(io_error)?;
            let name = entry.file_name().to_string_lossy().into_owned();
            let relative = if prefix.is_empty() {
                name
            } else {
                format!("{prefix}/{name}")
            };
            if entry.file_type().map_err(io_error)?.is_dir() {
                walk(&entry.path(), &relative, out)?;
            } else if relative.ends_with(".lua") {
                out.push(relative);
            }
        }
        Ok(())
    }
    let mut files = Vec::new();
    walk(&addon_dir(), "", &mut files)?;
    files.sort();
    Ok(files)
}

/// How the sandbox treats globals the test did not define.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EnvMode {
    /// Unknown globals read as nil, like an unstubbed client table. `print` is captured.
    Client,
    /// Reading an undefined global or writing any global raises an error. Used to prove
    /// that pure modules touch no client global.
    Strict,
}

/// A fresh Lua 5.1 state plus a restricted, WoW-like global table.
pub struct Sandbox {
    lua: Lua,
    env: Table,
    ns: Table,
    printed: Arc<Mutex<Vec<String>>>,
}

const ALLOWED_BASE: [&str; 15] = [
    "assert",
    "error",
    "getmetatable",
    "ipairs",
    "next",
    "pairs",
    "pcall",
    "rawequal",
    "rawget",
    "rawset",
    "select",
    "setmetatable",
    "tonumber",
    "tostring",
    "type",
];

impl Sandbox {
    /// Creates a state with only the string, table and math libraries, a bounded
    /// memory budget and an instruction budget, and a global table that exposes a
    /// whitelist of base functions (no `load*`, `dofile`, `require`, `io`, `os`,
    /// `debug`, `package`, `getfenv` or `setfenv`).
    pub fn new(mode: EnvMode) -> Result<Self> {
        let lua = Lua::new_with(
            StdLib::STRING | StdLib::TABLE | StdLib::MATH,
            LuaOptions::default(),
        )?;
        lua.set_memory_limit(MEMORY_LIMIT_BYTES)?;
        let calls = Arc::new(AtomicU64::new(0));
        lua.set_hook(
            HookTriggers {
                every_nth_instruction: Some(HOOK_INTERVAL),
                ..HookTriggers::default()
            },
            move |_, _| {
                if calls.fetch_add(1, Ordering::Relaxed) >= MAX_HOOK_CALLS {
                    return Err(mlua::Error::runtime("instruction budget exhausted"));
                }
                Ok(VmState::Continue)
            },
        )?;

        let globals = lua.globals();
        // `string.dump` would expose bytecode; remove it from the shared library table too.
        let string_lib: Table = globals.get("string")?;
        string_lib.raw_set("dump", Value::Nil)?;

        let env = lua.create_table()?;
        for name in ALLOWED_BASE {
            env.raw_set(name, globals.raw_get::<Value>(name)?)?;
        }
        env.raw_set("unpack", globals.raw_get::<Value>("unpack")?)?;
        for library in ["string", "table", "math"] {
            let source: Table = globals.get(library)?;
            let copy = lua.create_table()?;
            for pair in source.pairs::<Value, Value>() {
                let (key, value) = pair?;
                copy.raw_set(key, value)?;
            }
            env.raw_set(library, copy)?;
        }
        env.raw_set("_G", env.clone())?;

        let printed = Arc::new(Mutex::new(Vec::new()));
        if mode == EnvMode::Client {
            let sink = Arc::clone(&printed);
            let print = lua.create_function(move |_, values: mlua::Variadic<Value>| {
                let line = values
                    .iter()
                    .map(|value| match value {
                        Value::String(text) => text.to_string_lossy().to_string(),
                        other => format!("<{}>", other.type_name()),
                    })
                    .collect::<Vec<_>>()
                    .join(" ");
                if let Ok(mut lines) = sink.lock() {
                    lines.push(line);
                }
                Ok(())
            })?;
            env.raw_set("print", print)?;
        } else {
            lua.load(
                r#"
                local env = ...
                setmetatable(env, {
                    __index = function(_, key)
                        error("read of undeclared global '" .. tostring(key) .. "'", 2)
                    end,
                    __newindex = function(_, key)
                        error("write to global '" .. tostring(key) .. "'", 2)
                    end,
                })
                "#,
            )
            .set_name("=harness-strict")
            .call::<()>(env.clone())?;
        }

        let ns = lua.create_table()?;
        Ok(Self {
            lua,
            env,
            ns,
            printed,
        })
    }

    /// The underlying Lua state, for creating test inputs such as byte strings.
    pub fn lua(&self) -> &Lua {
        &self.lua
    }

    /// The table that plays the client's `_G` for every chunk this sandbox runs.
    pub fn env(&self) -> &Table {
        &self.env
    }

    /// The shared addon namespace table passed as the second vararg to addon files.
    pub fn ns(&self) -> &Table {
        &self.ns
    }

    /// Lines captured from `print` in [`EnvMode::Client`].
    pub fn printed(&self) -> Vec<String> {
        self.printed
            .lock()
            .map(|lines| lines.clone())
            .unwrap_or_default()
    }

    /// Runs a repository-owned snippet inside the sandbox environment. The snippet
    /// receives the addon namespace as the local `ns`.
    pub fn run<R: FromLuaMulti>(&self, chunk_name: &str, code: &str) -> Result<R> {
        let source = format!("local ns = ... ; {code}");
        Ok(self
            .lua
            .load(source)
            .set_name(format!("={chunk_name}"))
            .set_environment(self.env.clone())
            .call::<R>(self.ns.clone())?)
    }

    /// Loads one addon file (path relative to the addon folder) the way the client
    /// does: with the addon name and the shared namespace as `...`.
    pub fn load_addon_file(&self, relative: &str) -> Result<()> {
        let source = read_text(&addon_dir().join(relative))?;
        self.lua
            .load(source)
            .set_name(format!("@{ADDON_NAME}/{relative}"))
            .set_environment(self.env.clone())
            .call::<()>((ADDON_NAME, self.ns.clone()))?;
        Ok(())
    }

    /// Loads every TOC-listed file in TOC order; returns the order used.
    pub fn load_toc_files(&self) -> Result<Vec<String>> {
        let toc = Toc::read()?;
        if toc.files.is_empty() {
            return Err(HarnessError::Toc("no files listed".to_owned()));
        }
        for file in &toc.files {
            self.load_addon_file(file)?;
        }
        Ok(toc.files)
    }
}

/// What a source-scan hit is.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HitKind {
    Identifier,
    StringLiteral,
}

/// A forbidden name found in Lua source.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Hit {
    pub line: usize,
    pub name: String,
    pub kind: HitKind,
}

/// Names the addon must never reference: dynamic code loading, environment
/// manipulation, and standard libraries that the client sandbox restricts. A string
/// literal equal to one of them is also reported (`_G["load"]`).
pub const FORBIDDEN_NAMES: [&str; 14] = [
    "load",
    "loadstring",
    "loadfile",
    "dofile",
    "setfenv",
    "getfenv",
    "debug",
    "io",
    "os",
    "require",
    "package",
    "dump",
    "module",
    "newproxy",
];

/// Scans Lua source for forbidden identifiers and string literals, ignoring comments.
pub fn scan_forbidden(source: &str) -> Vec<Hit> {
    let bytes = source.as_bytes();
    let mut hits = Vec::new();
    let (mut i, mut line) = (0usize, 1usize);
    while i < bytes.len() {
        let c = bytes[i];
        match c {
            b'\n' => {
                line += 1;
                i += 1;
            }
            b'-' if bytes.get(i + 1) == Some(&b'-') => {
                i += 2;
                if let Some(level) = long_bracket_level(bytes, i) {
                    let (end, lines) = skip_long(bytes, i, level);
                    line += lines;
                    i = end;
                } else {
                    while i < bytes.len() && bytes[i] != b'\n' {
                        i += 1;
                    }
                }
            }
            b'"' | b'\'' => {
                let quote = c;
                let start_line = line;
                let mut literal = Vec::new();
                i += 1;
                while i < bytes.len() && bytes[i] != quote && bytes[i] != b'\n' {
                    if bytes[i] == b'\\' && i + 1 < bytes.len() {
                        literal.push(bytes[i + 1]);
                        i += 2;
                    } else {
                        literal.push(bytes[i]);
                        i += 1;
                    }
                }
                i += 1;
                report(&mut hits, start_line, &literal, HitKind::StringLiteral);
            }
            b'[' => {
                if let Some(level) = long_bracket_level(bytes, i) {
                    let start_line = line;
                    let (end, lines) = skip_long(bytes, i, level);
                    let body = &bytes[(i + level + 2).min(end)..end.saturating_sub(level + 2)];
                    report(
                        &mut hits,
                        start_line,
                        body.trim_ascii(),
                        HitKind::StringLiteral,
                    );
                    line += lines;
                    i = end;
                } else {
                    i += 1;
                }
            }
            _ if c == b'_' || c.is_ascii_alphabetic() => {
                let start = i;
                while i < bytes.len() && (bytes[i] == b'_' || bytes[i].is_ascii_alphanumeric()) {
                    i += 1;
                }
                report(&mut hits, line, &bytes[start..i], HitKind::Identifier);
            }
            _ => i += 1,
        }
    }
    hits
}

fn report(hits: &mut Vec<Hit>, line: usize, text: &[u8], kind: HitKind) {
    if let Ok(name) = std::str::from_utf8(text)
        && FORBIDDEN_NAMES.contains(&name)
    {
        hits.push(Hit {
            line,
            name: name.to_owned(),
            kind,
        });
    }
}

/// If a long bracket (`[[`, `[=[`, ...) opens at `at`, returns its level.
fn long_bracket_level(bytes: &[u8], at: usize) -> Option<usize> {
    if bytes.get(at) != Some(&b'[') {
        return None;
    }
    let mut level = 0;
    while bytes.get(at + 1 + level) == Some(&b'=') {
        level += 1;
    }
    (bytes.get(at + 1 + level) == Some(&b'[')).then_some(level)
}

/// Returns the index just past the closing bracket and the newlines crossed.
fn skip_long(bytes: &[u8], at: usize, level: usize) -> (usize, usize) {
    let mut closing = vec![b']'];
    closing.extend(std::iter::repeat_n(b'=', level));
    closing.push(b']');
    let mut i = at + level + 2;
    let mut lines = 0;
    while i < bytes.len() {
        if bytes[i..].starts_with(&closing) {
            return (i + closing.len(), lines);
        }
        if bytes[i] == b'\n' {
            lines += 1;
        }
        i += 1;
    }
    (bytes.len(), lines)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scanner_flags_identifiers_and_literals_but_not_comments() {
        let source = "-- load here is a comment\nlocal a = loadstring\nlocal b = _G[\"os\"]\n\
                      --[[ io\n dofile ]] local c = 1\nlocal d = [==[ debug ]==]\n";
        let hits = scan_forbidden(source);
        let found: Vec<(usize, &str, &HitKind)> = hits
            .iter()
            .map(|hit| (hit.line, hit.name.as_str(), &hit.kind))
            .collect();
        assert_eq!(
            found,
            vec![
                (2, "loadstring", &HitKind::Identifier),
                (3, "os", &HitKind::StringLiteral),
                (6, "debug", &HitKind::StringLiteral),
            ]
        );
    }

    #[test]
    fn scanner_does_not_match_longer_identifiers() {
        assert!(scan_forbidden("local loaded, osx, ioctl, requirement = 1").is_empty());
    }

    #[test]
    fn toc_parser_separates_directives_comments_and_files() {
        let toc = Toc::parse("## Interface: 16001\r\n# note\r\n\r\nCore\\A.lua\r\nB.lua\r\n");
        assert_eq!(toc.directives["Interface"], "16001");
        assert_eq!(toc.comments, vec!["note"]);
        assert_eq!(toc.files, vec!["Core/A.lua", "B.lua"]);
    }
}
