//! Source-neutral knowledge convergence. No I/O or clock reads, and no inference
//! that a complete target inventory exists. Origins are never silently relabelled.
use crate::{
    entities::Confidence, observation::ClockTrust, release::DOMAINS, source_admission::Origin,
    sources::precedence_rank,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{cmp::Ordering, collections::BTreeSet};
// Limits (V2.8c, ADR 0007 §7). The whole generated world measured 2,864,420
// facts (2,862,735 generated plus 1,685 adapter facts), 621 MB of retained fact
// documents and a 3.36 GB pretty catalog. Build-wide limits are about twice that;
// per-document limits are unchanged (to_facts.py writes to them).
/// Facts in one build (all inputs together).
pub const MAX_FACTS: usize = 6_000_000;
/// Facts in one knowledge_v1 document.
pub const MAX_DOCUMENT_FACTS: usize = 250_000;
pub const MAX_DOCUMENT_BYTES: usize = 8 * 1024 * 1024;
/// Encoded catalog bytes: both the summed compact candidates and the pretty catalog.
pub const MAX_CATALOG_BYTES: u64 = 8 * 1024 * 1024 * 1024;
/// Refuses a build-wide fact count above [`MAX_FACTS`].
pub fn check_fact_capacity(facts: usize) -> Result<()> {
    if facts > MAX_FACTS {
        return Err("Source knowledge exceeds fact capacity");
    }
    Ok(())
}
/// Refuses an encoded catalog size above [`MAX_CATALOG_BYTES`].
pub fn check_catalog_bytes(bytes: u64) -> Result<()> {
    if bytes > MAX_CATALOG_BYTES {
        return Err("Source catalog exceeds encoded byte bound");
    }
    Ok(())
}
pub type Result<T> = std::result::Result<T, &'static str>;
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "snake_case", deny_unknown_fields)]
pub enum Datum {
    Known { value: Value },
    Unknown,
    Unsupported,
    Restricted,
    Stale { last_value: Option<Value> },
    NotApplicable,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Fact {
    pub domain: String,
    /// Includes all variant/applicability conditions, not just a display name.
    pub record: String,
    pub field: String,
    pub datum: Datum,
    pub confidence: Confidence,
    /// Actual observation/publication date, never download time or Git ordering.
    pub observed_at_utc_ms: Option<i64>,
    #[serde(default)]
    pub clock_trust: ClockTrust,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Document {
    pub schema_version: u32,
    pub facts: Vec<Fact>,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Candidate {
    pub source_id: String,
    pub source_revision: String,
    pub source_file: String,
    pub source_sha256: String,
    pub origin: Origin,
    pub fact: Fact,
}
#[derive(Debug, Serialize)]
pub struct Entry {
    pub origin: Origin,
    pub domain: String,
    pub record: String,
    pub field: String,
    pub preferred: usize,
    pub usable_for_target: bool,
    /// All observations, including unavailable and losing conflicting evidence.
    pub candidates: Vec<Candidate>,
    pub conflict: bool,
}
#[derive(Debug, Serialize)]
pub struct Catalog {
    pub format: &'static str,
    pub schema_version: u32,
    pub target: Origin,
    pub as_of_utc_ms: i64,
    pub precedence: &'static str,
    pub publication_authorized: bool,
    pub modification_notice: &'static str,
    pub entries: Vec<Entry>,
    pub domains: Vec<DomainReport>,
}
#[derive(Debug, Serialize)]
pub struct DomainReport {
    pub domain: &'static str,
    pub source_facts: usize,
    pub target_facts: usize,
    pub conflicts: usize,
    /// Counts of imported facts cannot establish the independently required set.
    pub required_inventory: Option<usize>,
}
fn text(s: &str) -> bool {
    !s.trim().is_empty() && s.len() <= 512 && !s.chars().any(char::is_control)
}
fn content(value: &Value, depth: usize, count: &mut usize) -> bool {
    *count += 1;
    if depth > 32 || *count > 8192 {
        return false;
    }
    match value {
        Value::Array(a) => a.iter().all(|v| content(v, depth + 1, count)),
        Value::Object(a) => a
            .iter()
            .all(|(k, v)| text(k) && content(v, depth + 1, count)),
        Value::String(s) => s.len() <= 65536,
        Value::Number(n) => n
            .as_i64()
            .map(|i| i.unsigned_abs() <= crate::types::MAX_SAFE_INTEGER)
            .or_else(|| n.as_u64().map(|i| i <= crate::types::MAX_SAFE_INTEGER))
            .unwrap_or_else(|| {
                n.as_f64().is_some_and(|f| {
                    f.is_finite() && f.abs() <= crate::types::MAX_SAFE_INTEGER as f64
                })
            }),
        _ => true,
    }
}
impl Fact {
    pub fn validate(&self, as_of: i64) -> Result<()> {
        if !DOMAINS.contains(&self.domain.as_str())
            || !text(&self.record)
            || !text(&self.field)
            || self.observed_at_utc_ms.is_some_and(|t| t < 0 || t > as_of)
        {
            return Err("Invalid knowledge fact identity or date");
        }
        let value = match &self.datum {
            Datum::Known { value } => {
                if value.is_null() {
                    return Err("Known fact cannot be null");
                }
                Some(value)
            }
            Datum::Stale { last_value } => last_value.as_ref(),
            _ => None,
        };
        if let Some(v) = value
            && (!content(v, 0, &mut 0)
                || serde_json::to_vec(v)
                    .map_err(|_| "Invalid fact value")?
                    .len()
                    > 65536)
        {
            return Err("Knowledge fact exceeds value bounds");
        }
        Ok(())
    }
}
impl Document {
    pub fn parse(bytes: &[u8], as_of: i64) -> Result<Self> {
        let raw = crate::strict_json::value(bytes, MAX_DOCUMENT_BYTES)?;
        let doc: Self = serde_json::from_value(raw).map_err(|_| "Invalid knowledge document")?;
        if doc.schema_version != 1 || doc.facts.len() > MAX_DOCUMENT_FACTS {
            return Err("Unsupported knowledge document or capacity");
        }
        let mut ids = BTreeSet::new();
        for f in &doc.facts {
            f.validate(as_of)?;
            if !ids.insert((&f.domain, &f.record, &f.field)) {
                return Err("Duplicate fact in source document");
            }
        }
        Ok(doc)
    }
}
fn reliability(c: &Candidate) -> u8 {
    match c.fact.datum {
        Datum::Stale { .. } => 0,
        _ => precedence_rank(c.fact.confidence),
    }
}
/// Most reliable first. Only comparable, supported known observations participate
/// in recency; unknown dates stay unknown. Lexical ties are stable, not freshness.
pub fn compare(a: &Candidate, b: &Candidate) -> Ordering {
    reliability(b)
        .cmp(&reliability(a))
        .then_with(|| {
            let date = |c: &Candidate| {
                if c.fact.clock_trust == ClockTrust::Trusted {
                    c.fact.observed_at_utc_ms
                } else {
                    None
                }
            };
            date(b).cmp(&date(a))
        })
        .then_with(|| a.source_id.cmp(&b.source_id))
        .then_with(|| a.source_revision.cmp(&b.source_revision))
        .then_with(|| a.source_file.cmp(&b.source_file))
        .then_with(|| a.source_sha256.cmp(&b.source_sha256))
}
pub fn converge(target: Origin, as_of: i64, candidates: Vec<Candidate>) -> Result<Catalog> {
    if as_of < 0
        || as_of.unsigned_abs() > crate::types::MAX_SAFE_INTEGER
        || target.edition.as_deref().is_none_or(|s| !text(s))
        || target.build.as_deref().is_none_or(|s| !text(s))
        || check_fact_capacity(candidates.len()).is_err()
    {
        return Err("Invalid knowledge target, date or capacity");
    }
    let mut encoded_bytes = 0u64;
    for c in &candidates {
        c.fact.validate(as_of)?;
        encoded_bytes += serde_json::to_vec(c)
            .map_err(|_| "Invalid candidate encoding")?
            .len() as u64;
        if check_catalog_bytes(encoded_bytes).is_err() {
            return Err("Aggregate source facts exceed catalog byte bound");
        }
        if !text(&c.source_id)
            || !text(&c.source_file)
            || c.source_revision.len() != 40 && c.source_revision.len() != 64
            || !c
                .source_revision
                .bytes()
                .all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase())
            || c.source_sha256.len() != 64
            || !c
                .source_sha256
                .bytes()
                .all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase())
            || c.origin.edition.as_deref().is_some_and(|s| !text(s))
            || c.origin.build.as_deref().is_some_and(|s| !text(s))
        {
            return Err("Invalid knowledge provenance");
        }
        if c.fact.confidence == Confidence::VerifiedOnBuild
            && (c.origin.edition.is_none() || c.origin.build.is_none())
        {
            return Err("Verified fact requires a recorded origin build");
        }
    }
    // Work on indices so millions of candidates are neither cloned nor re-keyed
    // (V2.8c memory bound). One source identity must carry identical evidence;
    // exact repeats count once.
    fn identity(c: &Candidate) -> [&str; 6] {
        [
            &c.source_id,
            &c.source_revision,
            &c.source_file,
            &c.fact.domain,
            &c.fact.record,
            &c.fact.field,
        ]
    }
    let mut order: Vec<usize> = (0..candidates.len()).collect();
    order.sort_by(|&a, &b| {
        identity(&candidates[a])
            .cmp(&identity(&candidates[b]))
            .then(a.cmp(&b))
    });
    let mut keep = vec![true; candidates.len()];
    for pair in order.windows(2) {
        let (a, b) = (&candidates[pair[0]], &candidates[pair[1]]);
        if identity(a) == identity(b) {
            if a != b {
                return Err("Source fact identity reused with different evidence");
            }
            keep[pair[1]] = false;
        }
    }
    // Unknown build/edition is not a shared context. Isolate such observations
    // by source revision; never join two unrelated unknown origins.
    let scopes: Vec<String> = candidates
        .iter()
        .map(|c| {
            if c.origin.edition.is_none() || c.origin.build.is_none() {
                format!("{}:{}", c.source_id, c.source_revision)
            } else {
                String::new()
            }
        })
        .collect();
    let group_key = |i: usize| {
        let c = &candidates[i];
        (
            c.origin.edition.as_deref().unwrap_or_default(),
            c.origin.build.as_deref().unwrap_or_default(),
            scopes[i].as_str(),
            c.fact.domain.as_str(),
            c.fact.record.as_str(),
            c.fact.field.as_str(),
        )
    };
    let mut order: Vec<usize> = (0..candidates.len()).filter(|&i| keep[i]).collect();
    drop(keep);
    order.sort_by(|&a, &b| group_key(a).cmp(&group_key(b)).then(a.cmp(&b)));
    let mut runs = Vec::new();
    let mut start = 0;
    for end in 1..=order.len() {
        if end == order.len() || group_key(order[end]) != group_key(order[start]) {
            runs.push(end - start);
            start = end;
        }
    }
    drop(scopes);
    let mut slots: Vec<Option<Candidate>> = candidates.into_iter().map(Some).collect();
    let mut taken = order.into_iter();
    let mut entries = Vec::with_capacity(runs.len());
    for run in runs {
        let mut group: Vec<Candidate> = taken
            .by_ref()
            .take(run)
            .map(|i| slots[i].take().expect("each candidate is grouped once"))
            .collect();
        group.sort_by(compare);
        let first = &group[0];
        let conflict = group
            .iter()
            .skip(1)
            .any(|c| c.fact.datum != first.fact.datum);
        let usable = first.origin == target
            && matches!(first.fact.datum, Datum::Known { .. })
            && matches!(
                first.fact.confidence,
                Confidence::VerifiedOnBuild
                    | Confidence::PublisherDocumented
                    | Confidence::LimitedSample
            )
            && !conflict;
        entries.push(Entry {
            origin: first.origin.clone(),
            domain: first.fact.domain.clone(),
            record: first.fact.record.clone(),
            field: first.fact.field.clone(),
            preferred: 0,
            usable_for_target: usable,
            candidates: group,
            conflict,
        });
    }
    let domains = DOMAINS
        .iter()
        .map(|&domain| DomainReport {
            domain,
            source_facts: entries
                .iter()
                .filter(|e| e.domain == domain)
                .map(|e| e.candidates.len())
                .sum(),
            target_facts: entries
                .iter()
                .filter(|e| e.domain == domain && e.usable_for_target)
                .count(),
            conflicts: entries
                .iter()
                .filter(|e| e.domain == domain && e.conflict)
                .count(),
            required_inventory: None,
        })
        .collect();
    Ok(Catalog {
        format: "navzeroth-source-knowledge",
        schema_version: 1,
        target,
        as_of_utc_ms: as_of,
        precedence: "exact origin isolation; reliability; newest evidenced date; stable source/revision/file/hash tie",
        publication_authorized: false,
        modification_notice: "Transformed by Navzeroth contributors on 2026-10-15; preserve source licence terms and notices in NOTICES.txt and registry.json; exact originals/licences accompany the artifact.",
        entries,
        domains,
    })
}
