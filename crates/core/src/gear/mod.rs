//! Gear core (M5.1-M5.3): role profile structure, equipment eligibility, valid
//! configuration enumeration and fixed-point model score breakdowns.
//!
//! Everything here is pure: inputs are passed explicitly, no clock, I/O or
//! randomness. Only `Truth::True` produces a definite affirmative; unknown,
//! unsupported, restricted and not-applicable observations stay distinct and
//! never become eligible, valid or zero. Scores are *model scores* under a
//! labelled weight source; they are never measured damage or survivability.
//! Weights shipped with tests are SYNTHETIC engineering assumptions, not game
//! truth, and nothing here claims Classic behaviour holds for Forever.
//!
//! Wave-1 types are internal library types: they are not registered in the
//! contract schema. Practical ranking and acquisition value (M5.4) are not
//! implemented and nothing in this module is ever labelled `Ranking::Practical`.
pub mod inputs;
pub mod profile;

pub use inputs::*;
pub use profile::*;

/// Hard cap on enumerated slot assignments per scenario (search space bound).
pub const MAX_CONFIGURATIONS: usize = 4096;
/// Label required by plan section 10.1 for non-exhaustive catalogs.
pub const RANKING_LABEL: &str = "Best among verified candidates";
/// Every score produced here is a model score under a versioned weight source.
pub const MODEL_SCORE_LABEL: &str = "model score";
/// The only arithmetic version implemented: i64 thousandths, half away from zero.
pub const GEAR_MODEL_VERSION: &str = "gear-model-v1";
/// Fixed-point scale: one unit is stored as 1000 thousandths.
pub const FIXED_POINT_SCALE: i64 = 1000;
