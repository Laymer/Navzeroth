//! Versioned exchange envelopes (schema version 1). Decoding is bounded and pure;
//! transports must still bound their reads before building the input string.
//! Structural validity never establishes in-game truth.
use crate::{
    error::{AppError, ErrorCode, Result, Validate, Validated, invalid},
    observation::{ClockTrust, StateSource},
    types::*,
};
use schemars::{JsonSchema, Schema, SchemaGenerator, json_schema};
use serde::{Deserialize, Deserializer, Serialize, Serializer, de::DeserializeOwned};
use std::{borrow::Cow, collections::BTreeMap, marker::PhantomData};

mod dataset;
mod evidence;
mod gear;
mod plan;
pub mod schema;
mod snapshot;
pub use dataset::*;
pub use evidence::*;
pub use gear::*;

pub const FORMAT: &str = "azeroth-navigator";
/// Decoded-size limit for every kind except reference datasets (plan section 7.1).
pub const MAX_ENVELOPE_BYTES: usize = 4 * 1024 * 1024;
/// Engineering limit for reference datasets; content bundles (M3) may chunk.
pub const MAX_DATASET_BYTES: usize = 16 * 1024 * 1024;
/// JSON nesting limit for addon-facing kinds (plan section 7.1).
pub const MAX_ADDON_JSON_DEPTH: usize = 32;
pub const MAX_EXTENSIONS: usize = 64;

#[derive(
    Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub enum ContractKind {
    CharacterSnapshot,
    PartySnapshot,
    RoutePlan,
    EvidenceBundle,
    ReferenceDataset,
    GearRecommendations,
}
impl ContractKind {
    pub const ALL: [Self; 6] = [
        Self::CharacterSnapshot,
        Self::PartySnapshot,
        Self::RoutePlan,
        Self::EvidenceBundle,
        Self::ReferenceDataset,
        Self::GearRecommendations,
    ];
    pub fn as_str(self) -> &'static str {
        match self {
            Self::CharacterSnapshot => "character_snapshot",
            Self::PartySnapshot => "party_snapshot",
            Self::RoutePlan => "route_plan",
            Self::EvidenceBundle => "evidence_bundle",
            Self::ReferenceDataset => "reference_dataset",
            Self::GearRecommendations => "gear_recommendations",
        }
    }
    pub fn max_bytes(self) -> usize {
        match self {
            Self::ReferenceDataset => MAX_DATASET_BYTES,
            _ => MAX_ENVELOPE_BYTES,
        }
    }
    /// Addon-facing kinds use the section 7.1 depth; others rely on the
    /// parser's recursion limit and the rule limits of their records.
    pub fn max_json_depth(self) -> Option<usize> {
        match self {
            Self::CharacterSnapshot | Self::PartySnapshot | Self::RoutePlan => {
                Some(MAX_ADDON_JSON_DEPTH)
            }
            _ => None,
        }
    }
    fn parse(s: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|k| k.as_str() == s)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ProducerComponent {
    Addon,
    Desktop,
    Cli,
    TestFixture,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct Producer {
    pub component: ProducerComponent,
    /// Producer application version; independent of schema and dataset versions.
    pub version: Text,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub session_id: Option<SessionId>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sequence: Option<Sequence>,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct GameContext {
    pub edition: EditionId,
    pub build: BuildId,
    pub interface: InterfaceId,
    pub locale: Locale,
}
impl GameContext {
    pub fn build_key(&self) -> BuildKey {
        BuildKey {
            edition: self.edition.clone(),
            build: self.build.clone(),
            interface: self.interface.clone(),
        }
    }
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct Provenance {
    pub source: StateSource,
    pub clock_trust: ClockTrust,
    /// Absent means the producer recorded no creation time.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub created_at_utc_ms: Option<UtcMillis>,
}
/// Versions and inputs the payload was produced from. Absent fields and an
/// empty snapshot list mean "no such dependency"; each kind fixes which apply.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct Dependencies {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dataset_revision: Option<DatasetRevision>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ruleset_version: Option<RulesetVersion>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub score_model_version: Option<ScoreModelVersion>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub snapshots: Vec<crate::entities::SnapshotRef>,
}
impl Dependencies {
    pub fn is_empty(&self) -> bool {
        self == &Self::default()
    }
}

/// The constant `format` member.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct FormatTag;
impl Serialize for FormatTag {
    fn serialize<S: Serializer>(&self, s: S) -> std::result::Result<S::Ok, S::Error> {
        s.serialize_str(FORMAT)
    }
}
impl<'de> Deserialize<'de> for FormatTag {
    fn deserialize<D: Deserializer<'de>>(d: D) -> std::result::Result<Self, D::Error> {
        if String::deserialize(d)? == FORMAT {
            Ok(Self)
        } else {
            Err(serde::de::Error::custom("Unsupported exchange format"))
        }
    }
}
impl JsonSchema for FormatTag {
    fn inline_schema() -> bool {
        true
    }
    fn schema_name() -> Cow<'static, str> {
        "FormatTag".into()
    }
    fn json_schema(_: &mut SchemaGenerator) -> Schema {
        json_schema!({"type": "string", "const": FORMAT})
    }
}

/// The constant `kind` member of an envelope with payload `P`.
pub struct KindTag<P>(PhantomData<fn() -> P>);
impl<P> KindTag<P> {
    pub fn new() -> Self {
        Self(PhantomData)
    }
}
impl<P> Default for KindTag<P> {
    fn default() -> Self {
        Self::new()
    }
}
impl<P> Clone for KindTag<P> {
    fn clone(&self) -> Self {
        Self::new()
    }
}
impl<P> PartialEq for KindTag<P> {
    fn eq(&self, _: &Self) -> bool {
        true
    }
}
impl<P: Payload> std::fmt::Debug for KindTag<P> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(P::KIND.as_str())
    }
}
impl<P: Payload> Serialize for KindTag<P> {
    fn serialize<S: Serializer>(&self, s: S) -> std::result::Result<S::Ok, S::Error> {
        s.serialize_str(P::KIND.as_str())
    }
}
impl<'de, P: Payload> Deserialize<'de> for KindTag<P> {
    fn deserialize<D: Deserializer<'de>>(d: D) -> std::result::Result<Self, D::Error> {
        if String::deserialize(d)? == P::KIND.as_str() {
            Ok(Self::new())
        } else {
            Err(serde::de::Error::custom("Exchange kind does not match"))
        }
    }
}
impl<P: Payload> JsonSchema for KindTag<P> {
    fn inline_schema() -> bool {
        true
    }
    fn schema_name() -> Cow<'static, str> {
        format!("KindTag_{}", P::KIND.as_str()).into()
    }
    fn json_schema(_: &mut SchemaGenerator) -> Schema {
        json_schema!({"type": "string", "const": P::KIND.as_str()})
    }
}

/// Unknown top-level envelope members, retained and re-serialized in key order.
/// Nested domain records ignore unknown members instead.
#[derive(Clone, Debug, Default, PartialEq, Serialize)]
#[serde(transparent)]
pub struct Extensions(BTreeMap<String, serde_json::Value>);
impl Extensions {
    pub fn new(entries: impl IntoIterator<Item = (String, serde_json::Value)>) -> Result<Self> {
        let mut map = BTreeMap::new();
        for (key, value) in entries {
            check_extension_key(&key)?;
            if map.len() >= MAX_EXTENSIONS || map.insert(key, value).is_some() {
                return Err(invalid("Duplicate extension or extension limit exceeded"));
            }
        }
        Ok(Self(map))
    }
    pub fn get(&self, key: &str) -> Option<&serde_json::Value> {
        self.0.get(key)
    }
    pub fn len(&self) -> usize {
        self.0.len()
    }
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}
fn check_extension_key(key: &str) -> Result<()> {
    if key.is_empty() || key.len() > 256 || key.chars().any(char::is_control) {
        return Err(invalid(
            "Extension key must contain 1..256 bytes without controls",
        ));
    }
    Ok(())
}
impl<'de> Deserialize<'de> for Extensions {
    fn deserialize<D: Deserializer<'de>>(d: D) -> std::result::Result<Self, D::Error> {
        struct Visitor;
        impl<'de> serde::de::Visitor<'de> for Visitor {
            type Value = Extensions;
            fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
                f.write_str("bounded unique extension members")
            }
            fn visit_map<A: serde::de::MapAccess<'de>>(
                self,
                mut a: A,
            ) -> std::result::Result<Extensions, A::Error> {
                let mut map = BTreeMap::new();
                while let Some(key) = a.next_key::<String>()? {
                    check_extension_key(&key).map_err(serde::de::Error::custom)?;
                    if map.len() >= MAX_EXTENSIONS || map.contains_key(&key) {
                        return Err(serde::de::Error::custom(
                            "Duplicate extension or extension limit exceeded",
                        ));
                    }
                    let value = a.next_value()?;
                    map.insert(key, value);
                }
                Ok(Extensions(map))
            }
        }
        d.deserialize_map(Visitor)
    }
}

/// Header facts every payload is checked against.
pub struct EnvelopeHeader<'a> {
    pub context: DataContext,
    pub game: &'a GameContext,
    pub fixture: bool,
    pub producer: &'a Producer,
    pub dependencies: &'a Dependencies,
}
pub trait Payload: Serialize + DeserializeOwned + JsonSchema + Sized {
    const KIND: ContractKind;
    fn validate_payload(&self, header: &EnvelopeHeader) -> Result<()>;
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(bound(deserialize = "P: Payload"))]
#[schemars(rename = "{P}Envelope", bound = "P: Payload")]
pub struct EnvelopeData<P: Payload> {
    pub format: FormatTag,
    pub kind: KindTag<P>,
    pub schema_version: SchemaVersion,
    /// True for synthetic engineering data. Production planning rejects it.
    pub fixture: bool,
    pub producer: Producer,
    pub game: GameContext,
    pub namespace: DatasetNamespace,
    pub provenance: Provenance,
    #[serde(default, skip_serializing_if = "Dependencies::is_empty")]
    pub dependencies: Dependencies,
    pub payload: P,
    #[serde(flatten)]
    #[schemars(skip)]
    pub extensions: Extensions,
}
pub type Envelope<P> = Validated<EnvelopeData<P>>;
impl<P: Payload> EnvelopeData<P> {
    pub fn data_context(&self) -> DataContext {
        DataContext {
            build: self.game.build_key(),
            namespace: self.namespace,
        }
    }
}
impl<P: Payload> Validate for EnvelopeData<P> {
    fn validate(&self) -> Result<()> {
        if self.producer.sequence.is_some() && self.producer.session_id.is_none() {
            return Err(invalid("Producer sequence requires a producer session ID"));
        }
        if self.producer.version.as_str().trim().is_empty() {
            return Err(invalid("Producer version must not be empty"));
        }
        if self.namespace == DatasetNamespace::Demo && !self.fixture {
            return Err(invalid("Demo namespace data must be marked as a fixture"));
        }
        if self.fixture != (self.provenance.source == StateSource::SyntheticFixture) {
            return Err(invalid(
                "Fixture flag must agree with synthetic fixture provenance",
            ));
        }
        if self.producer.component == ProducerComponent::TestFixture && !self.fixture {
            return Err(invalid("Test fixture producers must mark fixture data"));
        }
        self.payload.validate_payload(&EnvelopeHeader {
            context: self.data_context(),
            game: &self.game,
            fixture: self.fixture,
            producer: &self.producer,
            dependencies: &self.dependencies,
        })
    }
}

/// Checks bracket nesting before parsing, ignoring brackets inside strings.
fn depth_within(input: &str, max: usize) -> bool {
    let (mut depth, mut in_string, mut escaped) = (0usize, false, false);
    for b in input.bytes() {
        if in_string {
            match (escaped, b) {
                (true, _) => escaped = false,
                (false, b'\\') => escaped = true,
                (false, b'"') => in_string = false,
                _ => {}
            }
            continue;
        }
        match b {
            b'"' => in_string = true,
            b'{' | b'[' => {
                depth += 1;
                if depth > max {
                    return false;
                }
            }
            b'}' | b']' => depth = depth.saturating_sub(1),
            _ => {}
        }
    }
    true
}

#[derive(Deserialize)]
struct Header {
    #[serde(default)]
    format: serde_json::Value,
    #[serde(default)]
    schema_version: serde_json::Value,
    #[serde(default)]
    kind: serde_json::Value,
}
fn parse_header(input: &str, max_bytes: usize) -> Result<ContractKind> {
    if input.len() > max_bytes {
        return Err(AppError::new(
            ErrorCode::ImportTooLarge,
            "Exchange document exceeds its size limit",
            false,
        ));
    }
    let header: Header =
        serde_json::from_str(input).map_err(|_| invalid("Invalid exchange JSON header"))?;
    if header.format.as_str() != Some(FORMAT) {
        return Err(invalid("Not an Azeroth Navigator exchange document"));
    }
    let version = header
        .schema_version
        .as_u64()
        .ok_or_else(|| invalid("Exchange schema version must be an unsigned integer"))?;
    SchemaVersion::try_from(u32::try_from(version).unwrap_or(u32::MAX))?;
    header
        .kind
        .as_str()
        .and_then(ContractKind::parse)
        .ok_or_else(|| invalid("Unsupported exchange kind"))
}

/// Decode one envelope of kind `P::KIND`. Order: size, header (format, schema
/// version, kind), nesting depth, typed structure, then semantic validation.
/// Errors never include imported content.
pub fn decode_envelope<P: Payload>(input: &str) -> Result<Envelope<P>> {
    if parse_header(input, P::KIND.max_bytes())? != P::KIND {
        return Err(invalid("Exchange kind does not match the expected kind"));
    }
    decode_checked(input)
}
fn decode_checked<P: Payload>(input: &str) -> Result<Envelope<P>> {
    if let Some(max) = P::KIND.max_json_depth()
        && !depth_within(input, max)
    {
        return Err(invalid("Exchange JSON nesting depth exceeded"));
    }
    let data: EnvelopeData<P> =
        serde_json::from_str(input).map_err(|_| invalid("Invalid exchange structure or record"))?;
    Envelope::try_new(data)
}

/// Any supported exchange document, dispatched on its `kind`.
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(untagged)]
pub enum AnyExchange {
    CharacterSnapshot(Box<Envelope<crate::entities::CharacterSnapshot>>),
    PartySnapshot(Box<Envelope<crate::entities::PartySnapshot>>),
    RoutePlan(Box<Envelope<crate::entities::RoutePlan>>),
    EvidenceBundle(Box<Envelope<EvidenceBundle>>),
    ReferenceDataset(Box<Envelope<ReferenceDataset>>),
    GearRecommendations(Box<Envelope<GearRecommendationSet>>),
}
impl AnyExchange {
    pub fn kind(&self) -> ContractKind {
        match self {
            Self::CharacterSnapshot(_) => ContractKind::CharacterSnapshot,
            Self::PartySnapshot(_) => ContractKind::PartySnapshot,
            Self::RoutePlan(_) => ContractKind::RoutePlan,
            Self::EvidenceBundle(_) => ContractKind::EvidenceBundle,
            Self::ReferenceDataset(_) => ContractKind::ReferenceDataset,
            Self::GearRecommendations(_) => ContractKind::GearRecommendations,
        }
    }
}
/// Decode any supported kind. The size check uses the largest kind limit
/// first, then the declared kind's own limit.
pub fn decode_any(input: &str) -> Result<AnyExchange> {
    let kind = parse_header(input, MAX_DATASET_BYTES)?;
    if input.len() > kind.max_bytes() {
        return Err(AppError::new(
            ErrorCode::ImportTooLarge,
            "Exchange document exceeds its size limit",
            false,
        ));
    }
    Ok(match kind {
        ContractKind::CharacterSnapshot => {
            AnyExchange::CharacterSnapshot(Box::new(decode_checked(input)?))
        }
        ContractKind::PartySnapshot => AnyExchange::PartySnapshot(Box::new(decode_checked(input)?)),
        ContractKind::RoutePlan => AnyExchange::RoutePlan(Box::new(decode_checked(input)?)),
        ContractKind::EvidenceBundle => {
            AnyExchange::EvidenceBundle(Box::new(decode_checked(input)?))
        }
        ContractKind::ReferenceDataset => {
            AnyExchange::ReferenceDataset(Box::new(decode_checked(input)?))
        }
        ContractKind::GearRecommendations => {
            AnyExchange::GearRecommendations(Box::new(decode_checked(input)?))
        }
    })
}

pub(crate) fn mismatch(message: &'static str) -> AppError {
    AppError::new(ErrorCode::BuildMismatch, message, false)
}
pub(crate) fn unresolved(message: &'static str) -> AppError {
    AppError::new(ErrorCode::DatasetIncomplete, message, false)
}
pub(crate) fn same_context(header: &EnvelopeHeader, context: &DataContext) -> Result<()> {
    if &header.context != context {
        return Err(mismatch(
            "Record edition, build, interface or namespace differs from its envelope",
        ));
    }
    Ok(())
}
pub(crate) fn no_dependencies(header: &EnvelopeHeader) -> Result<()> {
    if !header.dependencies.is_empty() {
        return Err(invalid("This exchange kind declares no dependencies"));
    }
    Ok(())
}
