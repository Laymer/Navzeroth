//! Regenerate with cargo run --locked -p azeroth-app --example export_progression_schema.
fn main() {
    print!("{}", azeroth_app::progression::api_schema_text());
}
