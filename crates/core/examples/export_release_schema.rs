//! Regenerate release structural schemas; runtime assessment remains authoritative.
fn main() {
    let evidence = match std::env::args().nth(1).as_deref() {
        Some("inventory") => false,
        Some("evidence") => true,
        _ => panic!("usage: export_release_schema inventory|evidence"),
    };
    print!(
        "{}",
        azeroth_core::release::release_contract_schema_text(evidence)
    );
}
