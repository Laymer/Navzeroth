//! Validated domain records. `*Data` are editable drafts; public record aliases
//! wrap them in `Validated`, so construction and deserialization share invariants.
use crate::error::{Result, invalid};
use std::collections::BTreeSet;
pub mod reference;
pub use reference::*;

pub(crate) fn bounded(len: usize) -> Result<()> {
    if len > 1024 {
        Err(invalid("Record collection exceeds 1024 entries"))
    } else {
        Ok(())
    }
}
pub(crate) fn unique<T: Ord>(values: impl IntoIterator<Item = T>) -> Result<()> {
    let mut seen = BTreeSet::new();
    for value in values {
        if !seen.insert(value) {
            return Err(invalid("Duplicate record identifier"));
        }
        bounded(seen.len())?;
    }
    Ok(())
}
pub mod character;
pub use character::*;
pub mod spatial;
pub use spatial::*;
pub mod planning;
pub use planning::*;
