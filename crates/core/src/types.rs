//! Edition-scoped identities and checked units. No IDs or clocks are generated here.
use crate::error::{AppError, ErrorCode, Result, invalid};
use schemars::{JsonSchema, Schema, SchemaGenerator, json_schema};
use serde::{Deserialize, Serialize};
use std::borrow::Cow;

pub const MAX_SAFE_INTEGER: u64 = 9_007_199_254_740_991;
/// Structural approximation of identifier rules for JSON Schema consumers.
/// Rust remains authoritative: it counts UTF-8 bytes, not code points.
/// `\p{White_Space}` matches Rust's `trim`; JavaScript's `\s` would also match U+FEFF.
pub const IDENTIFIER_PATTERN: &str =
    r"^[^\p{Cc}\p{White_Space}](?:[^\p{Cc}]*[^\p{Cc}\p{White_Space}])?$";
pub const TEXT_PATTERN: &str = r"^(?:[^\p{Cc}]|[\t\n])*$";
pub const COPPER_PATTERN: &str = "^(?:0|[1-9][0-9]{0,19})$";

/// Implements `JsonSchema` with a fixed, named schema.
macro_rules! fixed_schema {
    ($name:ident, $schema:tt) => {
        impl JsonSchema for $name {
            fn schema_name() -> Cow<'static, str> {
                stringify!($name).into()
            }
            fn json_schema(_: &mut SchemaGenerator) -> Schema {
                json_schema!($schema)
            }
        }
    };
}

macro_rules! ids {
    ($($name:ident),+ $(,)?) => { $(
        #[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
        #[serde(try_from = "String", into = "String")]
        pub struct $name(String);
        impl $name {
            pub fn new(value: impl Into<String>) -> Result<Self> { Self::try_from(value.into()) }
            pub fn as_str(&self) -> &str { &self.0 }
        }
        impl TryFrom<String> for $name {
            type Error = AppError;
            fn try_from(value: String) -> Result<Self> {
                if value.is_empty() || value.len() > 256 || value.trim() != value || value.chars().any(char::is_control) {
                    return Err(invalid("Identifier must contain 1..256 bytes without edge whitespace or controls"));
                }
                Ok(Self(value))
            }
        }
        impl From<$name> for String { fn from(value: $name) -> Self { value.0 } }
        fixed_schema!($name, {"type": "string", "minLength": 1, "maxLength": 256, "pattern": IDENTIFIER_PATTERN});
    )+ };
}
ids!(
    EditionId,
    BuildId,
    InterfaceId,
    Locale,
    CharacterId,
    SnapshotId,
    PartyId,
    ItemId,
    VariantId,
    InstanceId,
    AcquisitionId,
    QuestId,
    ObjectiveId,
    MapId,
    FloorId,
    NodeId,
    EdgeId,
    PlanId,
    RuleId,
    EvidenceId,
    SessionId,
    ProviderId,
    DatasetRevision,
    RulesetVersion,
    ScoreModelVersion,
    RoleId,
    PreferenceId,
    SlotId,
    AttributeId,
    SourceId,
    StepId,
    CapabilityId,
    ImageId,
    LayerId,
    PinId
);

macro_rules! integer {
    ($name:ident, $base:ty, $wire:literal, $valid:expr, $min:expr, $max:expr) => {
        fixed_schema!($name, {"type": "integer", "minimum": $min, "maximum": $max});
        #[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
        #[serde(try_from = $wire, into = $wire)]
        pub struct $name($base);
        impl $name {
            pub fn new(value: $base) -> Result<Self> {
                Self::try_from(value)
            }
            pub fn get(self) -> $base {
                self.0
            }
        }
        impl TryFrom<$base> for $name {
            type Error = AppError;
            fn try_from(value: $base) -> Result<Self> {
                if !($valid)(value) {
                    return Err(invalid("Integer is outside the permitted range"));
                }
                Ok(Self(value))
            }
        }
        impl From<$name> for $base {
            fn from(value: $name) -> Self {
                value.0
            }
        }
    };
}
const NEG_SAFE: i64 = -(MAX_SAFE_INTEGER as i64);
integer!(
    Millis,
    u64,
    "u64",
    |n: u64| n <= MAX_SAFE_INTEGER,
    0,
    MAX_SAFE_INTEGER
);
integer!(
    Sequence,
    u64,
    "u64",
    |n: u64| n <= MAX_SAFE_INTEGER,
    0,
    MAX_SAFE_INTEGER
);
integer!(
    UtcMillis,
    i64,
    "i64",
    |n: i64| n.unsigned_abs() <= MAX_SAFE_INTEGER,
    NEG_SAFE,
    MAX_SAFE_INTEGER
);
integer!(
    SafeInteger,
    i64,
    "i64",
    |n: i64| n.unsigned_abs() <= MAX_SAFE_INTEGER,
    NEG_SAFE,
    MAX_SAFE_INTEGER
);
integer!(Level, u16, "u16", |n: u16| n > 0, 1, u16::MAX);
// Image width, height or frame divisor in pixels: 1..=16384 (engineering limit).
integer!(
    PixelExtent,
    u32,
    "u32",
    |n: u32| (1..=16_384).contains(&n),
    1,
    16_384
);
pub const MAX_PIXEL_EXTENT: u32 = 16_384;

/// Lowercase hexadecimal SHA-256 digest. Core never hashes; it carries declared digests.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct Sha256Hex(String);
impl Sha256Hex {
    pub fn new(value: impl Into<String>) -> Result<Self> {
        Self::try_from(value.into())
    }
    pub fn as_str(&self) -> &str {
        &self.0
    }
}
impl TryFrom<String> for Sha256Hex {
    type Error = AppError;
    fn try_from(value: String) -> Result<Self> {
        if value.len() != 64
            || !value
                .bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
        {
            return Err(invalid(
                "Digest must be 64 lowercase hexadecimal characters",
            ));
        }
        Ok(Self(value))
    }
}
impl From<Sha256Hex> for String {
    fn from(value: Sha256Hex) -> Self {
        value.0
    }
}
fixed_schema!(Sha256Hex, {"type": "string", "pattern": "^[0-9a-f]{64}$"});

pub const ASSET_PATH_PATTERN: &str =
    "^[a-z0-9_-](?:[a-z0-9._-]*[a-z0-9_-])?(?:/[a-z0-9_-](?:[a-z0-9._-]*[a-z0-9_-])?)*$";
const WINDOWS_DEVICE_NAMES: [&str; 22] = [
    "con", "prn", "aux", "nul", "com1", "com2", "com3", "com4", "com5", "com6", "com7", "com8",
    "com9", "lpt1", "lpt2", "lpt3", "lpt4", "lpt5", "lpt6", "lpt7", "lpt8", "lpt9",
];
/// Relative, traversal-free content path (plan sections 8.7 and 16.4): lowercase
/// segments of `[a-z0-9._-]` that start and end with `[a-z0-9_-]`, joined by `/`,
/// at most 256 bytes. `.`/`..` segments, absolute paths, backslashes, colons and
/// Windows device names are rejected. It is resolved only against a declared
/// content root; the resolver must still refuse symlinks that escape that root.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct AssetPath(String);
impl AssetPath {
    pub fn new(value: impl Into<String>) -> Result<Self> {
        Self::try_from(value.into())
    }
    pub fn as_str(&self) -> &str {
        &self.0
    }
    pub fn segments(&self) -> impl Iterator<Item = &str> {
        self.0.split('/')
    }
}
impl TryFrom<String> for AssetPath {
    type Error = AppError;
    fn try_from(value: String) -> Result<Self> {
        const ERROR: &str = "Asset path must be a relative, lowercase, traversal-free path";
        if value.is_empty() || value.len() > 256 {
            return Err(invalid(ERROR));
        }
        for segment in value.split('/') {
            let bytes = segment.as_bytes();
            let edge =
                |b: u8| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_' || b == b'-';
            if bytes.is_empty()
                || !edge(bytes[0])
                || !edge(bytes[bytes.len() - 1])
                || !bytes.iter().all(|&b| edge(b) || b == b'.')
            {
                return Err(invalid(ERROR));
            }
            let stem = segment.split('.').next().unwrap_or(segment);
            if WINDOWS_DEVICE_NAMES.contains(&stem) {
                return Err(invalid(ERROR));
            }
        }
        Ok(Self(value))
    }
}
impl From<AssetPath> for String {
    fn from(value: AssetPath) -> Self {
        value.0
    }
}
fixed_schema!(AssetPath, {"type": "string", "minLength": 1, "maxLength": 256, "pattern": ASSET_PATH_PATTERN});

/// A point in an image frame, in full-frame pixels. The origin is the frame's
/// top-left corner; x grows rightward and y downward.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct PixelPoint {
    pub x: FiniteNumber,
    pub y: FiniteNumber,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct Copper(u64);
impl Copper {
    pub fn new(value: u64) -> Self {
        Self(value)
    }
    pub fn get(self) -> u64 {
        self.0
    }
}
impl TryFrom<String> for Copper {
    type Error = AppError;
    fn try_from(s: String) -> Result<Self> {
        if s.is_empty()
            || s.len() > 20
            || !s.bytes().all(|b| b.is_ascii_digit())
            || (s.len() > 1 && s.starts_with('0'))
        {
            return Err(invalid(
                "Currency must be a canonical unsigned decimal string",
            ));
        }
        s.parse()
            .map(Self)
            .map_err(|_| invalid("Currency overflow"))
    }
}
fixed_schema!(Copper, {
    "description": "Unsigned integer copper as a canonical base-10 string (Rust also rejects values above u64)",
    "type": "string",
    "pattern": COPPER_PATTERN
});
impl From<Copper> for String {
    fn from(n: Copper) -> Self {
        n.0.to_string()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(try_from = "f64", into = "f64")]
pub struct FiniteNumber(f64);
impl FiniteNumber {
    pub fn new(n: f64) -> Result<Self> {
        Self::try_from(n)
    }
    pub fn get(self) -> f64 {
        self.0
    }
}
impl TryFrom<f64> for FiniteNumber {
    type Error = AppError;
    fn try_from(n: f64) -> Result<Self> {
        if !n.is_finite() || n.abs() > MAX_SAFE_INTEGER as f64 {
            return Err(invalid(
                "Number must be finite and within the safe numeric range",
            ));
        }
        Ok(Self(n))
    }
}
fixed_schema!(FiniteNumber, {"type": "number", "minimum": NEG_SAFE, "maximum": MAX_SAFE_INTEGER});
impl From<FiniteNumber> for f64 {
    fn from(n: FiniteNumber) -> Self {
        n.0
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(try_from = "f64", into = "f64")]
pub struct Normalized(f64);
impl Normalized {
    pub fn new(n: f64) -> Result<Self> {
        Self::try_from(n)
    }
    pub fn get(self) -> f64 {
        self.0
    }
}
impl TryFrom<f64> for Normalized {
    type Error = AppError;
    fn try_from(n: f64) -> Result<Self> {
        if !n.is_finite() || !(0.0..=1.0).contains(&n) {
            return Err(invalid("Coordinate must be finite and in [0,1]"));
        }
        Ok(Self(n))
    }
}
fixed_schema!(Normalized, {"type": "number", "minimum": 0, "maximum": 1});
impl From<Normalized> for f64 {
    fn from(n: Normalized) -> Self {
        n.0
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "u32", into = "u32")]
pub struct SchemaVersion;
impl SchemaVersion {
    pub const V1: Self = Self;
}
impl TryFrom<u32> for SchemaVersion {
    type Error = AppError;
    fn try_from(n: u32) -> Result<Self> {
        if n == 1 {
            Ok(Self)
        } else {
            Err(AppError::new(
                ErrorCode::UnsupportedSchema,
                "Unsupported schema version; supported version is 1",
                false,
            ))
        }
    }
}
fixed_schema!(SchemaVersion, {"type": "integer", "const": 1});
impl From<SchemaVersion> for u32 {
    fn from(_: SchemaVersion) -> Self {
        1
    }
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, JsonSchema)]
pub struct BuildKey {
    pub edition: EditionId,
    pub build: BuildId,
    pub interface: InterfaceId,
}
#[derive(
    Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub enum DatasetNamespace {
    Demo,
    Reference,
}
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, JsonSchema)]
pub struct DataContext {
    pub build: BuildKey,
    pub namespace: DatasetNamespace,
}
impl DataContext {
    pub fn ensure_compatible(&self, other: &Self) -> Result<()> {
        if self != other {
            return Err(AppError::new(
                ErrorCode::BuildMismatch,
                "Edition, build, interface or dataset namespace mismatch",
                false,
            ));
        }
        Ok(())
    }
}
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, JsonSchema)]
pub struct ItemKey {
    pub context: DataContext,
    pub item_id: ItemId,
    pub variant: VariantId,
}
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, JsonSchema)]
pub struct MapKey {
    pub context: DataContext,
    pub map_id: MapId,
    pub floor: FloorId,
    pub instance: Option<InstanceId>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct Text(String);
impl Text {
    pub fn new(s: impl Into<String>) -> Result<Self> {
        Self::try_from(s.into())
    }
    pub fn as_str(&self) -> &str {
        &self.0
    }
}
impl TryFrom<String> for Text {
    type Error = AppError;
    fn try_from(s: String) -> Result<Self> {
        if s.len() > 4096 || s.chars().any(|c| c.is_control() && c != '\n' && c != '\t') {
            return Err(invalid(
                "Text exceeds limits or contains unsupported controls",
            ));
        }
        Ok(Self(s))
    }
}
fixed_schema!(Text, {"type": "string", "maxLength": 4096, "pattern": TEXT_PATTERN});
impl From<Text> for String {
    fn from(t: Text) -> Self {
        t.0
    }
}

/// A normalized position on one named map/floor/instance space. The origin is
/// the map's top-left corner; x grows eastward (right) and y southward (down),
/// both in [0,1], matching the C_Map map-position convention. This orientation
/// is a documented convention, unverified on the Forever client until the M0.4
/// checks exist. Positions on different `MapKey`s are never comparable. `map_id`
/// values follow the addon-exported uiMapID as a decimal string (ADR 0002).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct Position {
    pub map: MapKey,
    pub x: Normalized,
    pub y: Normalized,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[schemars(rename = "Bounds")]
pub struct BoundsData {
    pub min_x: Normalized,
    pub min_y: Normalized,
    pub max_x: Normalized,
    pub max_y: Normalized,
}
impl crate::error::Validate for BoundsData {
    fn validate(&self) -> Result<()> {
        if self.min_x.get() > self.max_x.get() || self.min_y.get() > self.max_y.get() {
            return Err(invalid("Area bounds are inverted"));
        }
        Ok(())
    }
}
pub type Bounds = crate::error::Validated<BoundsData>;
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct SupportedArea {
    pub map: MapKey,
    pub bounds: Option<Bounds>,
}
impl SupportedArea {
    pub fn contains(&self, point: &Position) -> Result<bool> {
        self.map.context.ensure_compatible(&point.map.context)?;
        Ok(self.map == point.map
            && self.bounds.as_ref().is_none_or(|b| {
                point.x.get() >= b.min_x.get()
                    && point.x.get() <= b.max_x.get()
                    && point.y.get() >= b.min_y.get()
                    && point.y.get() <= b.max_y.get()
            }))
    }
}

/// Bounded, deterministic maps reject duplicate JSON keys instead of losing data.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(transparent)]
pub struct RecordMap<K: Ord, V>(std::collections::BTreeMap<K, V>);
impl<K: Ord, V> Default for RecordMap<K, V> {
    fn default() -> Self {
        Self(std::collections::BTreeMap::new())
    }
}
impl<K: Ord, V> RecordMap<K, V> {
    pub fn new(entries: impl IntoIterator<Item = (K, V)>) -> Result<Self> {
        let mut map = std::collections::BTreeMap::new();
        for (k, v) in entries {
            if map.len() >= 1024 || map.insert(k, v).is_some() {
                return Err(invalid("Duplicate key or record map limit exceeded"));
            }
        }
        Ok(Self(map))
    }
}
impl<K: Ord, V> std::ops::Deref for RecordMap<K, V> {
    type Target = std::collections::BTreeMap<K, V>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
impl<K: Ord + JsonSchema, V: JsonSchema> JsonSchema for RecordMap<K, V> {
    fn inline_schema() -> bool {
        true
    }
    fn schema_name() -> Cow<'static, str> {
        format!("RecordMap_{}_{}", K::schema_name(), V::schema_name()).into()
    }
    fn schema_id() -> Cow<'static, str> {
        format!("RecordMap<{},{}>", K::schema_id(), V::schema_id()).into()
    }
    fn json_schema(generator: &mut SchemaGenerator) -> Schema {
        json_schema!({
            "type": "object",
            "propertyNames": generator.subschema_for::<K>(),
            "additionalProperties": generator.subschema_for::<V>(),
            "maxProperties": 1024
        })
    }
}
impl<'de, K: Ord + Deserialize<'de>, V: Deserialize<'de>> Deserialize<'de> for RecordMap<K, V> {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> std::result::Result<Self, D::Error> {
        struct Visitor<K, V>(std::marker::PhantomData<(K, V)>);
        impl<'de, K: Ord + Deserialize<'de>, V: Deserialize<'de>> serde::de::Visitor<'de>
            for Visitor<K, V>
        {
            type Value = RecordMap<K, V>;
            fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
                f.write_str("a bounded map with unique keys")
            }
            fn visit_map<A: serde::de::MapAccess<'de>>(
                self,
                mut a: A,
            ) -> std::result::Result<Self::Value, A::Error> {
                let mut map = std::collections::BTreeMap::new();
                while let Some(key) = a.next_key::<K>()? {
                    if map.len() >= 1024 || map.contains_key(&key) {
                        return Err(serde::de::Error::custom(
                            "Duplicate key or record map limit exceeded",
                        ));
                    }
                    map.insert(key, a.next_value()?);
                }
                Ok(RecordMap(map))
            }
        }
        d.deserialize_map(Visitor(std::marker::PhantomData))
    }
}
