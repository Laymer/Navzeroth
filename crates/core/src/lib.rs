//! Domain types, travel, objectives, gear, rules, and explanations for Azeroth Navigator.
//!
//! This crate is a placeholder created by task M1.1 to prove that the Cargo
//! workspace wires together. Task M1.2 replaces this module with the real
//! domain types, observation semantics, schema versioning, and application
//! errors described in the implementation plan (section 6).
//!
//! `core` must stay independent of Tauri, SQLite, network I/O, and the live
//! game (plan section 4.3). Do not add those dependencies here.

/// Placeholder confirming the crate builds and links. M1.2 removes this.
pub fn placeholder() -> &'static str {
    "azeroth-core: domain types not yet implemented (see plan M1.2)"
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn placeholder_reports_pending_work() {
        assert_eq!(
            placeholder(),
            "azeroth-core: domain types not yet implemented (see plan M1.2)"
        );
    }
}
