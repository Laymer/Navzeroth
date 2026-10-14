//! Parser for the literal subset of a WoW SavedVariables file (plan section 7.2).
//!
//! A SavedVariables file is Lua source, but this module treats it purely as
//! data: a hand-written recursive-descent reader over bytes. No Lua engine, no
//! `load`, no `dofile`, no shell. Anything that would need evaluation is
//! rejected, never executed.
//!
//! # Accepted grammar
//!
//! ```text
//! file    = { ws | ";" | assign }
//! assign  = Name ws "=" ws value                 (Name: [A-Za-z_][A-Za-z0-9_]*, not a Lua keyword)
//! value   = table | string | number | "true" | "false"
//! table   = "{" [ field { sep field } [ sep ] ] "}"
//! field   = "[" ( string | integer ) "]" "=" value | Name "=" value | value
//! sep     = "," | ";"
//! ws      = SP | HT | LF | CR | "--" <text up to LF>
//! number  = ["-"] digits ["." digits] [("e"|"E") ["+"|"-"] digits]
//! ```
//!
//! Strings use `'` or `"` and these escapes: `\\ \" \' \a \b \f \n \r \t \v`,
//! `\ddd` (decimal, at most 255), `\xXX`, `\z` (skips following whitespace) and
//! `\u{X...}` (a Unicode scalar value). A raw line break inside a string, an
//! unknown escape, or a decoded string that is not valid UTF-8 is rejected;
//! nothing is converted lossily. Positional fields take integer keys from 1 as
//! in Lua.
//!
//! # Rejected
//!
//! `function`, call syntax, global or name references as values, `nil`, `..`
//! concatenation, arithmetic and other operators (only a minus directly
//! attached to a number is accepted), long strings and long comments
//! (`[[`, `[=[`, `--[[`), hexadecimal numbers, `inf`/`nan`, non-integer or
//! boolean keys, duplicate keys or globals, Lua keywords as names, and any
//! trailing garbage. A byte order mark is rejected (the file must be plain
//! UTF-8).
//!
//! # Bounds
//!
//! 8 MiB per file, 32 nested tables, ~4 MiB per string, 262,144 values and 64
//! top-level assignments. Nesting is checked before recursing, so the stack
//! depth is bounded regardless of input.
//!
//! # Unverified
//!
//! How the real client serializes SavedVariables (escape style, key order,
//! number formats such as `1e+15`, comments, line endings) is UNVERIFIED until
//! M0.5 runs on a live client. This subset may need widening; the per-line
//! `-- [1]` comments real files carry are accepted on that expectation.
use crate::error::invalid;
use azeroth_core::{
    contract::MAX_ENVELOPE_BYTES,
    error::{AppError, ErrorCode, Result},
};
use std::collections::BTreeMap;

pub const MAX_FILE_BYTES: usize = 8 * 1024 * 1024;
pub const MAX_TABLE_DEPTH: usize = 32;
pub const MAX_STRING_BYTES: usize = MAX_ENVELOPE_BYTES + 1024;
pub const MAX_VALUES: usize = 262_144;
pub const MAX_GLOBALS: usize = 64;
const MAX_NUMBER_BYTES: usize = 64;

/// A parsed Lua literal. Tables are ordered maps, so output is deterministic.
#[derive(Clone, Debug, PartialEq)]
pub enum LuaValue {
    /// Explicit source-table nil. SavedVariables entry points still reject nil.
    Nil,
    Bool(bool),
    Int(i64),
    Float(f64),
    Str(String),
    Table(LuaTable),
}
/// Table keys are integers or strings, as written (`["k"]`, `[1]`, `name`) or
/// implied by position.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum LuaKey {
    Int(i64),
    Str(String),
}
pub type LuaTable = BTreeMap<LuaKey, LuaValue>;

/// The top-level assignments of one file.
#[derive(Clone, Debug, PartialEq)]
pub struct ParsedFile {
    globals: BTreeMap<String, LuaValue>,
}
impl ParsedFile {
    pub fn globals(&self) -> &BTreeMap<String, LuaValue> {
        &self.globals
    }
    pub fn get(&self, name: &str) -> Option<&LuaValue> {
        self.globals.get(name)
    }
}

/// Why a file was rejected. A closed set of codes: none carries content or position.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Rejection {
    FileTooLarge,
    StringTooLong,
    TooManyValues,
    TooManyGlobals,
    TooDeep,
    /// Input ended inside a construct.
    Truncated,
    Syntax,
    Function,
    Call,
    NameReference,
    Nil,
    Operator,
    Concatenation,
    LongBracket,
    BadNumber,
    BadKey,
    BadEscape,
    RawLineBreak,
    NotUtf8,
    DuplicateKey,
    DuplicateGlobal,
    ReservedName,
}
impl Rejection {
    pub fn message(self) -> &'static str {
        match self {
            Self::FileTooLarge => "SavedVariables file exceeds its size limit",
            Self::StringTooLong => "SavedVariables string exceeds its size limit",
            Self::TooManyValues => "SavedVariables file holds too many values",
            Self::TooManyGlobals => "SavedVariables file holds too many top-level assignments",
            Self::TooDeep => "SavedVariables table nesting depth exceeded",
            Self::Truncated => "SavedVariables file ends unexpectedly",
            Self::Syntax => "SavedVariables file is not a literal table assignment",
            Self::Function => "SavedVariables files must not contain functions",
            Self::Call => "SavedVariables files must not contain calls",
            Self::NameReference => "SavedVariables values must be literals, not names",
            Self::Nil => "SavedVariables files must not contain nil",
            Self::Operator => "SavedVariables files must not contain operators",
            Self::Concatenation => "SavedVariables files must not concatenate",
            Self::LongBracket => "SavedVariables files must not contain long strings or comments",
            Self::BadNumber => "SavedVariables number is not a supported decimal literal",
            Self::BadKey => "SavedVariables table key must be a string or an integer",
            Self::BadEscape => "SavedVariables string has an unsupported escape",
            Self::RawLineBreak => "SavedVariables string contains a raw line break",
            Self::NotUtf8 => "SavedVariables string is not valid UTF-8",
            Self::DuplicateKey => "SavedVariables table repeats a key",
            Self::DuplicateGlobal => "SavedVariables file repeats a top-level name",
            Self::ReservedName => "SavedVariables name is a Lua keyword",
        }
    }
    pub fn code(self) -> ErrorCode {
        match self {
            Self::FileTooLarge | Self::StringTooLong | Self::TooManyValues => {
                ErrorCode::ImportTooLarge
            }
            _ => ErrorCode::InvalidInput,
        }
    }
}
impl From<Rejection> for AppError {
    fn from(rejection: Rejection) -> Self {
        AppError::new(rejection.code(), rejection.message(), false)
    }
}

/// Parse a SavedVariables file. Never executes anything.
pub fn parse(input: &str) -> Result<ParsedFile> {
    parse_detailed(input).map_err(AppError::from)
}

/// Like [`parse`], with the closed rejection reason instead of an `AppError`.
pub fn parse_detailed(input: &str) -> std::result::Result<ParsedFile, Rejection> {
    if input.len() > MAX_FILE_BYTES {
        return Err(Rejection::FileTooLarge);
    }
    let mut parser = Parser {
        bytes: input.as_bytes(),
        pos: 0,
        values: 0,
        policy: LiteralPolicy::SAVED_VARIABLES,
    };
    Ok(ParsedFile {
        globals: parser.file()?,
    })
}

/// Return the export string stored at `global[key]`. Both names are caller
/// parameters: the addon's global (`AzerothNavigatorDB`) and key layout are
/// UNVERIFIED until M0.5 and nothing here hard-codes them. The assumed layout
/// is `AzerothNavigatorDB = { ["export"] = "ANAV1:{...}" }`. Other globals
/// and keys are parsed within bounds, then ignored and never modified.
pub fn extract_export<'a>(file: &'a ParsedFile, global: &str, key: &str) -> Result<&'a str> {
    let Some(LuaValue::Table(table)) = file.get(global) else {
        return Err(invalid(
            "SavedVariables global is missing or is not a table",
        ));
    };
    match table.get(&LuaKey::Str(key.to_owned())) {
        Some(LuaValue::Str(export)) => Ok(export),
        _ => Err(invalid(
            "SavedVariables export is missing or is not a string",
        )),
    }
}

type Parsed<T> = std::result::Result<T, Rejection>;

struct Parser<'a> {
    bytes: &'a [u8],
    pos: usize,
    values: usize,
    policy: LiteralPolicy,
}

/// Source staging reuses the literal reader without widening SavedVariables.
/// Kept private to ingest; callers cannot raise parser budgets.
#[derive(Clone, Copy)]
struct LiteralPolicy {
    allow_nil: bool,
    exact_integers: bool,
    max_values: usize,
    max_string_bytes: usize,
    max_identifier_bytes: Option<usize>,
}
impl LiteralPolicy {
    const SAVED_VARIABLES: Self = Self {
        allow_nil: false,
        exact_integers: false,
        max_values: MAX_VALUES,
        max_string_bytes: MAX_STRING_BYTES,
        max_identifier_bytes: None,
    };
    const SOURCE: Self = Self {
        allow_nil: true,
        exact_integers: true,
        max_values: crate::questiedb::MAX_VALUES,
        max_string_bytes: crate::questiedb::MAX_STRING_BYTES,
        max_identifier_bytes: Some(crate::questiedb::MAX_STRING_BYTES),
    };
}

/// Parse one literal prefix for an independently validated source wrapper.
/// The caller must validate the entire remaining wrapper and file size. Nil
/// stays a present value and consumes its positional index, unlike Lua runtime
/// table deletion. Overlarge integer tokens cannot silently round into floats.
pub(crate) fn source_literal_prefix(input: &str) -> Parsed<(LuaValue, usize)> {
    if input.len() > MAX_FILE_BYTES {
        return Err(Rejection::FileTooLarge);
    }
    let mut parser = Parser {
        bytes: input.as_bytes(),
        pos: 0,
        values: 0,
        policy: LiteralPolicy::SOURCE,
    };
    parser.skip()?;
    let value = parser.value(1)?;
    Ok((value, parser.pos))
}

const KEYWORDS: [&str; 22] = [
    "and", "break", "do", "else", "elseif", "end", "false", "for", "function", "goto", "if", "in",
    "local", "nil", "not", "or", "repeat", "return", "then", "true", "until", "while",
];
fn is_ident_start(b: u8) -> bool {
    b.is_ascii_alphabetic() || b == b'_'
}
fn is_ident_byte(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'_'
}

impl Parser<'_> {
    fn peek(&self) -> Option<u8> {
        self.bytes.get(self.pos).copied()
    }
    fn peek_at(&self, offset: usize) -> Option<u8> {
        self.bytes.get(self.pos + offset).copied()
    }
    fn count_value(&mut self) -> Parsed<()> {
        self.values += 1;
        if self.values > self.policy.max_values {
            return Err(Rejection::TooManyValues);
        }
        Ok(())
    }

    /// Skip whitespace and `--` line comments. Long comments are rejected.
    fn skip(&mut self) -> Parsed<()> {
        loop {
            match self.peek() {
                Some(b' ' | b'\t' | b'\r' | b'\n') => self.pos += 1,
                Some(b'-') if self.peek_at(1) == Some(b'-') => {
                    if self.peek_at(2) == Some(b'[') && matches!(self.peek_at(3), Some(b'[' | b'='))
                    {
                        return Err(Rejection::LongBracket);
                    }
                    self.pos += 2;
                    while self.peek().is_some_and(|c| c != b'\n') {
                        self.pos += 1;
                    }
                }
                _ => return Ok(()),
            }
        }
    }

    /// What was found where a separator or the end of a value was expected.
    fn unexpected(&self) -> Rejection {
        match (self.peek(), self.peek_at(1)) {
            (None, _) => Rejection::Truncated,
            (Some(b'.'), Some(b'.')) => Rejection::Concatenation,
            (Some(b'(' | b'"' | b'\'' | b'{' | b':'), _) => Rejection::Call,
            (Some(c), _) if b"+-*/%^#<>~&|=".contains(&c) => Rejection::Operator,
            _ => Rejection::Syntax,
        }
    }

    fn ident(&mut self) -> &str {
        let start = self.pos;
        while self.peek().is_some_and(is_ident_byte) {
            self.pos += 1;
        }
        // Identifier bytes are ASCII, so this slice is valid UTF-8.
        std::str::from_utf8(&self.bytes[start..self.pos]).unwrap_or_default()
    }

    fn owned_ident(&mut self) -> Parsed<String> {
        let limit = self.policy.max_identifier_bytes;
        let name = self.ident();
        if limit.is_some_and(|limit| name.len() > limit) {
            return Err(Rejection::StringTooLong);
        }
        Ok(name.to_owned())
    }

    fn file(&mut self) -> Parsed<BTreeMap<String, LuaValue>> {
        let mut globals = BTreeMap::new();
        loop {
            self.skip()?;
            match self.peek() {
                None => return Ok(globals),
                Some(b';') => {
                    self.pos += 1;
                    continue;
                }
                Some(c) if is_ident_start(c) => {}
                Some(_) => return Err(Rejection::Syntax),
            }
            let name = self.owned_ident()?;
            if KEYWORDS.contains(&name.as_str()) {
                return Err(if name == "function" {
                    Rejection::Function
                } else {
                    Rejection::ReservedName
                });
            }
            self.skip()?;
            match (self.peek(), self.peek_at(1)) {
                (Some(b'='), next) if next != Some(b'=') => self.pos += 1,
                _ => return Err(self.unexpected()),
            }
            self.skip()?;
            let value = self.value(1)?;
            if globals.len() >= MAX_GLOBALS {
                return Err(Rejection::TooManyGlobals);
            }
            if globals.insert(name, value).is_some() {
                return Err(Rejection::DuplicateGlobal);
            }
            // A value is followed by whitespace, `;`, another assignment or the end.
            self.skip()?;
            if let Some(c) = self.peek()
                && c != b';'
                && !is_ident_start(c)
            {
                return Err(self.unexpected());
            }
        }
    }

    /// `depth` is the nesting level a table starting here would have.
    fn value(&mut self, depth: usize) -> Parsed<LuaValue> {
        self.count_value()?;
        match self.peek() {
            None => Err(Rejection::Truncated),
            Some(b'{') => self.table(depth),
            Some(b'"' | b'\'') => self.string().map(LuaValue::Str),
            Some(b'-' | b'0'..=b'9') => self.number(),
            Some(b'[') => Err(Rejection::LongBracket),
            Some(c) if is_ident_start(c) => {
                let name = self.owned_ident()?;
                match name.as_str() {
                    "true" => Ok(LuaValue::Bool(true)),
                    "false" => Ok(LuaValue::Bool(false)),
                    "nil" if self.policy.allow_nil => Ok(LuaValue::Nil),
                    "nil" => Err(Rejection::Nil),
                    "function" => Err(Rejection::Function),
                    _ => {
                        self.skip()?;
                        Err(match self.peek() {
                            Some(b'(' | b'"' | b'\'' | b'{' | b':') => Rejection::Call,
                            _ => Rejection::NameReference,
                        })
                    }
                }
            }
            Some(_) => Err(self.unexpected()),
        }
    }

    fn table(&mut self, depth: usize) -> Parsed<LuaValue> {
        if depth > MAX_TABLE_DEPTH {
            return Err(Rejection::TooDeep);
        }
        self.pos += 1;
        let mut table = LuaTable::new();
        let mut next_index = 1i64;
        loop {
            self.skip()?;
            match self.peek() {
                None => return Err(Rejection::Truncated),
                Some(b'}') => {
                    self.pos += 1;
                    return Ok(LuaValue::Table(table));
                }
                _ => {}
            }
            let (key, value) = self.field(depth, &mut next_index)?;
            if table.insert(key, value).is_some() {
                return Err(Rejection::DuplicateKey);
            }
            self.skip()?;
            match self.peek() {
                Some(b',' | b';') => self.pos += 1,
                Some(b'}') => {}
                _ => return Err(self.unexpected()),
            }
        }
    }

    fn field(&mut self, depth: usize, next_index: &mut i64) -> Parsed<(LuaKey, LuaValue)> {
        match self.peek() {
            Some(b'[') => {
                if matches!(self.peek_at(1), Some(b'[' | b'=')) {
                    return Err(Rejection::LongBracket);
                }
                self.pos += 1;
                self.skip()?;
                let key = self.bracket_key()?;
                self.skip()?;
                match self.peek() {
                    Some(b']') => self.pos += 1,
                    _ => return Err(self.unexpected()),
                }
                self.skip()?;
                match (self.peek(), self.peek_at(1)) {
                    (Some(b'='), next) if next != Some(b'=') => self.pos += 1,
                    _ => return Err(self.unexpected()),
                }
                self.skip()?;
                Ok((key, self.value(depth + 1)?))
            }
            Some(c) if is_ident_start(c) => {
                let start = self.pos;
                let name = self.owned_ident()?;
                self.skip()?;
                if self.peek() == Some(b'=') && self.peek_at(1) != Some(b'=') {
                    if KEYWORDS.contains(&name.as_str()) {
                        return Err(Rejection::ReservedName);
                    }
                    self.pos += 1;
                    self.skip()?;
                    Ok((LuaKey::Str(name), self.value(depth + 1)?))
                } else {
                    // A positional value such as `true`; names are rejected there.
                    self.pos = start;
                    self.positional(depth, next_index)
                }
            }
            _ => self.positional(depth, next_index),
        }
    }

    fn positional(&mut self, depth: usize, next_index: &mut i64) -> Parsed<(LuaKey, LuaValue)> {
        let value = self.value(depth + 1)?;
        let key = LuaKey::Int(*next_index);
        // At most MAX_VALUES positional values exist, so this cannot overflow.
        *next_index += 1;
        Ok((key, value))
    }

    fn bracket_key(&mut self) -> Parsed<LuaKey> {
        match self.peek() {
            None => Err(Rejection::Truncated),
            Some(b'"' | b'\'') => self.string().map(LuaKey::Str),
            Some(b'-' | b'0'..=b'9') => match self.number()? {
                LuaValue::Int(n) => Ok(LuaKey::Int(n)),
                _ => Err(Rejection::BadKey),
            },
            Some(b'[') => Err(Rejection::LongBracket),
            Some(c) if is_ident_start(c) => Err(Rejection::NameReference),
            Some(_) => Err(Rejection::BadKey),
        }
    }

    fn digits(&mut self) -> usize {
        let start = self.pos;
        while self.peek().is_some_and(|c| c.is_ascii_digit()) {
            self.pos += 1;
        }
        self.pos - start
    }

    fn number(&mut self) -> Parsed<LuaValue> {
        let start = self.pos;
        if self.peek() == Some(b'-') {
            self.pos += 1;
            if !self.peek().is_some_and(|c| c.is_ascii_digit()) {
                return Err(self.unexpected_after_minus());
            }
        }
        self.digits();
        let mut integral = true;
        if self.peek() == Some(b'.') && self.peek_at(1).is_some_and(|c| c.is_ascii_digit()) {
            integral = false;
            self.pos += 1;
            self.digits();
        }
        if matches!(self.peek(), Some(b'e' | b'E')) {
            integral = false;
            self.pos += 1;
            if matches!(self.peek(), Some(b'+' | b'-')) {
                self.pos += 1;
            }
            if self.digits() == 0 {
                return Err(Rejection::BadNumber);
            }
        }
        // `0x10`, `12abc`, `1.e5` and similar are not decimal literals.
        if self.peek().is_some_and(is_ident_byte) || self.pos - start > MAX_NUMBER_BYTES {
            return Err(Rejection::BadNumber);
        }
        let text = std::str::from_utf8(&self.bytes[start..self.pos]).unwrap_or_default();
        if integral && let Ok(n) = text.parse::<i64>() {
            return Ok(LuaValue::Int(n));
        }
        if integral && self.policy.exact_integers {
            return Err(Rejection::BadNumber);
        }
        match text.parse::<f64>() {
            Ok(f) if f.is_finite() => Ok(LuaValue::Float(f)),
            _ => Err(Rejection::BadNumber),
        }
    }
    fn unexpected_after_minus(&self) -> Rejection {
        if self.peek().is_none() {
            Rejection::Truncated
        } else {
            Rejection::Operator
        }
    }

    fn string(&mut self) -> Parsed<String> {
        let quote = self.bytes[self.pos];
        self.pos += 1;
        let mut out: Vec<u8> = Vec::new();
        loop {
            let Some(c) = self.peek() else {
                return Err(Rejection::Truncated);
            };
            self.pos += 1;
            match c {
                _ if c == quote => break,
                b'\n' | b'\r' => return Err(Rejection::RawLineBreak),
                b'\\' => self.escape(&mut out)?,
                _ => out.push(c),
            }
            if out.len() > self.policy.max_string_bytes {
                return Err(Rejection::StringTooLong);
            }
        }
        String::from_utf8(out).map_err(|_| Rejection::NotUtf8)
    }

    fn escape(&mut self, out: &mut Vec<u8>) -> Parsed<()> {
        let Some(c) = self.peek() else {
            return Err(Rejection::Truncated);
        };
        self.pos += 1;
        match c {
            b'a' => out.push(7),
            b'b' => out.push(8),
            b'f' => out.push(12),
            b'n' => out.push(b'\n'),
            b'r' => out.push(b'\r'),
            b't' => out.push(b'\t'),
            b'v' => out.push(11),
            b'\\' | b'"' | b'\'' => out.push(c),
            b'0'..=b'9' => {
                let mut value = u32::from(c - b'0');
                for _ in 0..2 {
                    match self.peek() {
                        Some(d) if d.is_ascii_digit() => {
                            value = value * 10 + u32::from(d - b'0');
                            self.pos += 1;
                        }
                        _ => break,
                    }
                }
                out.push(u8::try_from(value).map_err(|_| Rejection::BadEscape)?);
            }
            b'x' => {
                let high = self.hex_digit()?;
                let low = self.hex_digit()?;
                out.push(high * 16 + low);
            }
            b'z' => {
                while matches!(
                    self.peek(),
                    Some(b' ' | b'\t' | b'\r' | b'\n' | 0x0b | 0x0c)
                ) {
                    self.pos += 1;
                }
            }
            b'u' => {
                if self.peek() != Some(b'{') {
                    return Err(Rejection::BadEscape);
                }
                self.pos += 1;
                let (mut value, mut count) = (0u32, 0usize);
                while let Some(d) = self.peek().and_then(|d| char::from(d).to_digit(16)) {
                    value = value.checked_mul(16).ok_or(Rejection::BadEscape)? + d;
                    count += 1;
                    self.pos += 1;
                    if count > 8 {
                        return Err(Rejection::BadEscape);
                    }
                }
                if count == 0 || self.peek() != Some(b'}') {
                    return Err(Rejection::BadEscape);
                }
                self.pos += 1;
                let ch = char::from_u32(value).ok_or(Rejection::BadEscape)?;
                let mut buffer = [0u8; 4];
                out.extend_from_slice(ch.encode_utf8(&mut buffer).as_bytes());
            }
            _ => return Err(Rejection::BadEscape),
        }
        Ok(())
    }

    fn hex_digit(&mut self) -> Parsed<u8> {
        let digit = self
            .peek()
            .and_then(|d| char::from(d).to_digit(16))
            .ok_or(Rejection::BadEscape)?;
        self.pos += 1;
        // A hexadecimal digit is below 16.
        Ok(u8::try_from(digit).unwrap_or_default())
    }
}

#[cfg(test)]
mod source_budget_tests {
    use super::*;

    #[test]
    fn nil_holes_and_empty_tables_consume_the_source_value_budget() {
        let parse_with_budget = |input: &str, budget| {
            let mut parser = Parser {
                bytes: input.as_bytes(),
                pos: 0,
                values: 0,
                policy: LiteralPolicy {
                    max_values: budget,
                    ..LiteralPolicy::SOURCE
                },
            };
            parser.value(1)
        };
        // Root plus four explicit values; nil is not discarded to evade limits.
        assert!(parse_with_budget("{nil,{},false,0}", 5).is_ok());
        assert_eq!(
            parse_with_budget("{nil,{},false,0}", 4),
            Err(Rejection::TooManyValues)
        );
        assert_eq!(
            parse_with_budget("{nil,nil,nil,nil}", 4),
            Err(Rejection::TooManyValues)
        );
        assert_eq!(
            LiteralPolicy::SOURCE.max_values,
            crate::questiedb::MAX_VALUES
        );
        assert_eq!(
            LiteralPolicy::SOURCE.max_string_bytes,
            crate::questiedb::MAX_STRING_BYTES
        );
    }
}
