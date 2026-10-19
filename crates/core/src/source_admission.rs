//! Explicit source admission before retrieval. Pins identify reviewed bytes; they
//! do not prove ownership or authenticate a grant. No network or source execution.
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

pub const MAX_REGISTRY_BYTES: usize = 1_048_576;
pub const MAX_FILE_BYTES: u64 = 8 * 1024 * 1024;
pub const MAX_RETRIEVAL_BYTES: u64 = 1024 * 1024 * 1024;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Rejection {
    InvalidRegistry,
    UnsupportedVersion,
    Capacity,
    UnknownSource,
    RightsNotAdmitted,
    FileNotPermitted,
    FieldNotPermitted,
    MethodNotPermitted,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Registry {
    pub schema_version: u32,
    pub sources: Vec<Source>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Source {
    pub id: String,
    /// Immutable upstream revision, independently of target validation.
    pub revision: String,
    pub publisher: String,
    pub origin: Origin,
    pub rights: Rights,
    pub retrieval: Retrieval,
    pub files: Vec<PermittedFile>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Origin {
    pub edition: Option<String>,
    pub build: Option<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Rights {
    pub status: Status,
    pub license: License,
    pub license_identifier: Option<String>,
    pub reviewer: Option<String>,
    pub attribution: Option<String>,
    /// Full obligations include redistribution, adaptation, retention and notices.
    pub obligations: Option<String>,
    pub evidence: Vec<Evidence>,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Status {
    Unreviewed,
    Rejected,
    Admitted,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum License {
    Unknown,
    Noncommercial,
    Permissive,
    ShareAlike,
    /// The owner explicitly accepts this source's terms (ADR 0006 §14). Never
    /// implied: admission also needs evidence whose scope cites that amendment.
    OwnerAccepted,
}
/// The ADR amendment an `owner_accepted` admission must cite in evidence scope.
pub const OWNER_ACCEPTANCE_CITATION: &str = "ADR 0006 §14";
/// The evidence that carries the citation must point at the ADR itself.
pub const OWNER_ACCEPTANCE_DOCUMENT: &str = "docs/decisions/0006-v1-program.md";
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Evidence {
    pub url: String,
    pub revision: String,
    pub sha256: String,
    pub reviewed_scope: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Retrieval {
    pub method: Method,
    pub max_total_bytes: u64,
    pub minimum_interval_ms: u64,
    pub max_requests: u32,
    /// A reviewed access-method grant, not merely a public URL.
    pub access_evidence: Option<Evidence>,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Method {
    ManualLocal,
    HttpsPinned,
    /// Opt-in, bounded, rate-limited retrieval by `data-generators/`. Builds,
    /// tests and `source-sync` never fetch it; generated material is retained.
    HttpsRateLimited,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PermittedFile {
    pub path: String,
    pub url: String,
    pub sha256: String,
    pub bytes: u64,
    /// Exact field names. Wildcards and implicit all-field permission are absent.
    pub fields: Vec<String>,
    pub transformation: String,
}
/// The adapter must enforce these limits, hash actual bytes, reject redirects,
/// preserve immutable snapshots and parse only inert data. Admission is not
/// target-build verification, conflict resolution or distribution readiness.
#[derive(Debug)]
pub struct Authorization<'a> {
    pub source: &'a Source,
    pub file: &'a PermittedFile,
}
fn text(s: &str) -> bool {
    !s.trim().is_empty() && s.len() <= 4096 && !s.chars().any(char::is_control)
}
fn optional(s: &Option<String>) -> bool {
    s.as_deref().is_none_or(text)
}
fn hash(s: &str) -> bool {
    s.len() == 64
        && s.bytes()
            .all(|c| c.is_ascii_digit() || (b'a'..=b'f').contains(&c))
}
fn revision(s: &str) -> bool {
    (s.len() == 40 || s.len() == 64)
        && s.bytes()
            .all(|c| c.is_ascii_digit() || (b'a'..=b'f').contains(&c))
}
fn url(s: &str) -> bool {
    let Some(rest) = s.strip_prefix("https://") else {
        return false;
    };
    if !text(s) || !s.is_ascii() || s.contains(['@', '#', '?', '\\', ' ']) {
        return false;
    }
    let authority = rest.split('/').next().unwrap_or("");
    // Literal spaces in immutable upstream filenames have one canonical URI
    // spelling. No encoded separators, dots, percent or alternate authority
    // forms are admitted. Paths are never decoded into local filenames.
    if rest[authority.len()..].replace("%20", "").contains('%') {
        return false;
    }
    // Deliberately narrow: DNS names only, default HTTPS port, no credentials,
    // encoded authority, IP literals, trailing dot or alternate parser forms.
    authority.len() <= 253
        && authority.contains('.')
        && authority.rsplit('.').next().is_some_and(|label| {
            label.bytes().all(|c| c.is_ascii_alphabetic()) || label.starts_with("xn--")
        })
        && authority.split('.').all(|label| {
            !label.is_empty()
                && label.len() <= 63
                && label.as_bytes()[0].is_ascii_alphanumeric()
                && label.as_bytes()[label.len() - 1].is_ascii_alphanumeric()
                && label
                    .bytes()
                    .all(|c| c.is_ascii_alphanumeric() || c == b'-')
        })
}
fn path(s: &str) -> bool {
    crate::types::AssetPath::new(s.to_ascii_lowercase()).is_ok()
}
fn evidence(e: &Evidence) -> bool {
    url(&e.url) && revision(&e.revision) && hash(&e.sha256) && text(&e.reviewed_scope)
}
impl Registry {
    pub fn parse(bytes: &[u8]) -> Result<Self, Rejection> {
        if bytes.len() > MAX_REGISTRY_BYTES {
            return Err(Rejection::Capacity);
        }
        let registry: Self =
            serde_json::from_slice(bytes).map_err(|_| Rejection::InvalidRegistry)?;
        registry.validate()?;
        Ok(registry)
    }
    pub fn validate(&self) -> Result<(), Rejection> {
        if self.schema_version != 1 {
            return Err(Rejection::UnsupportedVersion);
        }
        if self.sources.len() > 128 {
            return Err(Rejection::Capacity);
        }
        let mut ids = BTreeSet::new();
        for s in &self.sources {
            if !text(&s.id)
                || !ids.insert(&s.id)
                || !text(&s.publisher)
                || !revision(&s.revision)
                || !optional(&s.origin.edition)
                || !optional(&s.origin.build)
            {
                return Err(Rejection::InvalidRegistry);
            }
            if s.files.len() > 1024 || s.rights.evidence.len() > 64 {
                return Err(Rejection::Capacity);
            }
            let r = &s.rights;
            if !optional(&r.license_identifier)
                || !optional(&r.reviewer)
                || !optional(&r.attribution)
                || !optional(&r.obligations)
                || !r.evidence.iter().all(evidence)
            {
                return Err(Rejection::InvalidRegistry);
            }
            // Owner acceptance is explicit: it must cite the ADR amendment.
            // Owner-accepted and generator-retrieved sources register their
            // generated files later, so an empty list is allowed for them;
            // authorize() still refuses every file that is not listed.
            let owner = r.license == License::OwnerAccepted;
            let later = owner || s.retrieval.method == Method::HttpsRateLimited;
            if r.status == Status::Admitted
                && (!matches!(
                    r.license,
                    License::Permissive | License::ShareAlike | License::OwnerAccepted
                ) || r.license_identifier.is_none()
                    || r.reviewer.is_none()
                    || r.attribution.is_none()
                    || r.obligations.is_none()
                    || r.evidence.is_empty()
                    || (owner
                        && !r.evidence.iter().any(|e| {
                            e.reviewed_scope.contains(OWNER_ACCEPTANCE_CITATION)
                                && e.url.contains(OWNER_ACCEPTANCE_DOCUMENT)
                        }))
                    || (!later && s.files.is_empty()))
            {
                return Err(Rejection::RightsNotAdmitted);
            }
            let t = &s.retrieval;
            if t.max_total_bytes == 0
                || t.max_total_bytes > MAX_RETRIEVAL_BYTES
                || t.max_requests == 0
                || t.max_requests > 1024
                || t.minimum_interval_ms < 1000
                || t.access_evidence.as_ref().is_some_and(|e| !evidence(e))
            {
                return Err(Rejection::InvalidRegistry);
            }
            if r.status == Status::Admitted && t.access_evidence.is_none() {
                return Err(Rejection::RightsNotAdmitted);
            }
            let mut paths = BTreeSet::new();
            let mut total = 0u64;
            for f in &s.files {
                if !path(&f.path)
                    || !paths.insert(f.path.to_ascii_lowercase())
                    || !url(&f.url)
                    || !hash(&f.sha256)
                    || f.bytes == 0
                    || f.bytes > MAX_FILE_BYTES
                    || !text(&f.transformation)
                {
                    return Err(Rejection::InvalidRegistry);
                }
                if f.fields.is_empty() || f.fields.len() > 256 {
                    return Err(Rejection::Capacity);
                }
                let mut fields = BTreeSet::new();
                if f.fields
                    .iter()
                    .any(|v| !text(v) || v.contains(['*', '?']) || !fields.insert(v))
                {
                    return Err(Rejection::InvalidRegistry);
                }
                total += f.bytes;
            }
            if total > t.max_total_bytes || s.files.len() > t.max_requests as usize {
                return Err(Rejection::Capacity);
            }
        }
        Ok(())
    }
    pub fn authorize(
        &self,
        source_id: &str,
        file_path: &str,
        fields: &[&str],
        method: Method,
    ) -> Result<Authorization<'_>, Rejection> {
        self.validate()?;
        let source = self
            .sources
            .iter()
            .find(|s| s.id == source_id)
            .ok_or(Rejection::UnknownSource)?;
        if source.rights.status != Status::Admitted {
            return Err(Rejection::RightsNotAdmitted);
        }
        if source.retrieval.method != method {
            return Err(Rejection::MethodNotPermitted);
        }
        let file = source
            .files
            .iter()
            .find(|f| f.path == file_path)
            .ok_or(Rejection::FileNotPermitted)?;
        if fields.is_empty()
            || fields.len() > 256
            || fields
                .iter()
                .any(|field| !file.fields.iter().any(|f| f == field))
        {
            return Err(Rejection::FieldNotPermitted);
        }
        Ok(Authorization { source, file })
    }
}
