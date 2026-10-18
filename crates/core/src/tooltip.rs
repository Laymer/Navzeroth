//! Inert, bounded research tooltip interpolation. No scripting or markup execution.
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
pub const MAX_TEXT: usize = 16_384;
pub const MAX_DOCUMENTS: usize = 4096;
pub const MAX_DEPTH: usize = 16;
pub const MAX_TOKENS: usize = 4096;
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Document {
    pub text: String,
    /// Explicit reviewed scalar substitutions, e.g. s1, d. Never derive game math here.
    pub scalars: BTreeMap<String, String>,
}
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum Gap {
    MissingDocument,
    MissingScalar,
    UnsupportedExpression,
    Cycle,
    Capacity,
}
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Evaluation {
    pub text: String,
    pub gaps: Vec<Gap>,
    pub verified_for_gameplay: bool,
}
struct Context<'a> {
    documents: &'a BTreeMap<u32, Document>,
    active: BTreeSet<u32>,
    gaps: BTreeSet<Gap>,
    tokens: usize,
}
impl Context<'_> {
    fn append(&mut self, out: &mut String, text: &str) {
        if out.len().saturating_add(text.len()) <= MAX_TEXT {
            out.push_str(text);
        } else {
            self.gaps.insert(Gap::Capacity);
        }
    }
    fn render(&mut self, id: u32) -> String {
        if self.active.contains(&id) {
            self.gaps.insert(Gap::Cycle);
            return "[cycle]".into();
        }
        if self.active.len() >= MAX_DEPTH {
            self.gaps.insert(Gap::Capacity);
            return "[capacity]".into();
        }
        let Some(doc) = self.documents.get(&id) else {
            self.gaps.insert(Gap::MissingDocument);
            return "[missing]".into();
        };
        self.active.insert(id);
        let mut out = String::new();
        let mut rest = doc.text.as_str();
        while let Some(start) = rest.find('$') {
            self.append(&mut out, &rest[..start]);
            rest = &rest[start + 1..];
            self.tokens += 1;
            if self.tokens > MAX_TOKENS {
                self.gaps.insert(Gap::Capacity);
                break;
            }
            if let Some(tail) = rest.strip_prefix('$') {
                self.append(&mut out, "$");
                rest = tail;
                continue;
            }
            // Only $s1..$s32, $d, $m1..$m32 and $M1..$M32 are scalar tokens.
            // $@spelldesc<id> is an explicit document dependency. Everything else stays opaque.
            if let Some(tail) = rest.strip_prefix("@spelldesc") {
                let n = tail.bytes().take_while(u8::is_ascii_digit).count();
                if n > 0
                    && n <= 10
                    && let Ok(target) = tail[..n].parse::<u32>()
                    && target != 0
                {
                    let nested = self.render(target);
                    self.append(&mut out, &nested);
                    rest = &tail[n..];
                    continue;
                }
            } else if rest.starts_with('d') {
                if rest
                    .as_bytes()
                    .get(1)
                    .is_none_or(|b| !b.is_ascii_alphanumeric())
                {
                    if let Some(v) = doc.scalars.get("d") {
                        self.append(&mut out, v);
                    } else {
                        self.gaps.insert(Gap::MissingScalar);
                        self.append(&mut out, "[unknown]");
                    }
                    rest = &rest[1..];
                    continue;
                }
            } else if rest.starts_with(['s', 'm', 'M']) {
                let n = rest.as_bytes()[1..]
                    .iter()
                    .take_while(|b| b.is_ascii_digit())
                    .count();
                if n > 0
                    && n <= 2
                    && let Ok(index) = rest[1..1 + n].parse::<u8>()
                    && (1..=32).contains(&index)
                    && rest
                        .as_bytes()
                        .get(1 + n)
                        .is_none_or(|b| !b.is_ascii_alphanumeric())
                {
                    let key = &rest[..1 + n];
                    if let Some(v) = doc.scalars.get(key) {
                        self.append(&mut out, v);
                    } else {
                        self.gaps.insert(Gap::MissingScalar);
                        self.append(&mut out, "[unknown]");
                    }
                    rest = &rest[1 + n..];
                    continue;
                }
            }
            self.gaps.insert(Gap::UnsupportedExpression);
            self.append(&mut out, "$");
            // Preserve unsupported source text; output is plain text, never HTML/Lua.
        }
        self.append(&mut out, rest);
        self.active.remove(&id);
        out
    }
}
/// Input quotas are checked before recursion; untrusted values never become instructions.
pub fn evaluate(documents: &BTreeMap<u32, Document>, id: u32) -> Result<Evaluation, &'static str> {
    if documents.len() > MAX_DOCUMENTS
        || documents.iter().any(|(id, d)| {
            *id == 0
                || d.text.len() > MAX_TEXT
                || d.scalars.len() > 97
                || d.scalars
                    .iter()
                    .any(|(k, v)| k.len() > 3 || v.len() > 128 || v.contains('$'))
        })
        || documents
            .values()
            .map(|d| d.text.len() + d.scalars.values().map(String::len).sum::<usize>())
            .sum::<usize>()
            > 4 * 1024 * 1024
    {
        return Err("Tooltip input capacity exceeded");
    }
    let mut context = Context {
        documents,
        active: BTreeSet::new(),
        gaps: BTreeSet::new(),
        tokens: 0,
    };
    let text = context.render(id);
    Ok(Evaluation {
        text,
        gaps: context.gaps.into_iter().collect(),
        verified_for_gameplay: false,
    })
}
/// Enumerate only supported document references, honoring escaped dollar signs.
/// Consumers can load the requested closure without admitting an entire catalog.
pub fn references(text: &str) -> Result<Vec<u32>, &'static str> {
    if text.len() > MAX_TEXT {
        return Err("Tooltip text capacity exceeded");
    }
    let mut result = BTreeSet::new();
    let mut rest = text;
    let mut tokens = 0;
    while let Some(start) = rest.find('$') {
        rest = &rest[start + 1..];
        tokens += 1;
        if tokens > MAX_TOKENS {
            return Err("Tooltip token capacity exceeded");
        }
        if let Some(tail) = rest.strip_prefix('$') {
            rest = tail;
            continue;
        }
        if let Some(tail) = rest.strip_prefix("@spelldesc") {
            let n = tail.bytes().take_while(u8::is_ascii_digit).count();
            if n > 0
                && n <= 10
                && let Ok(id) = tail[..n].parse::<u32>()
                && id != 0
            {
                result.insert(id);
                rest = &tail[n..];
            }
        }
    }
    Ok(result.into_iter().collect())
}
