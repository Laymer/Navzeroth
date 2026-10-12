//! Edition-scoped identities and checked units. No IDs or clocks are generated here.
use crate::error::{AppError, ErrorCode, Result, invalid};
use serde::{Deserialize, Serialize};

pub const MAX_SAFE_INTEGER: u64 = 9_007_199_254_740_991;

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
    CapabilityId
);

macro_rules! integer {
    ($name:ident, $base:ty, $wire:literal, $valid:expr) => {
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
integer!(Millis, u64, "u64", |n: u64| n <= MAX_SAFE_INTEGER);
integer!(Sequence, u64, "u64", |n: u64| n <= MAX_SAFE_INTEGER);
integer!(UtcMillis, i64, "i64", |n: i64| n.unsigned_abs()
    <= MAX_SAFE_INTEGER);
integer!(SafeInteger, i64, "i64", |n: i64| n.unsigned_abs()
    <= MAX_SAFE_INTEGER);
integer!(Level, u16, "u16", |n: u16| n > 0);

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
        if !n.is_finite() {
            return Err(invalid("Number must be finite"));
        }
        Ok(Self(n))
    }
}
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
impl From<SchemaVersion> for u32 {
    fn from(_: SchemaVersion) -> Self {
        1
    }
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct BuildKey {
    pub edition: EditionId,
    pub build: BuildId,
    pub interface: InterfaceId,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DatasetNamespace {
    Demo,
    Reference,
}
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
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
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct ItemKey {
    pub context: DataContext,
    pub item_id: ItemId,
    pub variant: VariantId,
}
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
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
impl From<Text> for String {
    fn from(t: Text) -> Self {
        t.0
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Position {
    pub map: MapKey,
    pub x: Normalized,
    pub y: Normalized,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
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
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
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
