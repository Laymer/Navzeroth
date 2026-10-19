//! Print the world-data v1 schema; redirect explicitly to regenerate its artifact.
fn main() {
    print!("{}", azeroth_core::world_data::world_data_schema_text());
}
