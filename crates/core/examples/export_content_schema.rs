//! Print the generated v2 schema; redirect explicitly to regenerate its artifact.
fn main() {
    print!(
        "{}",
        azeroth_core::content_v2::schema::reference_content_schema_text()
    );
}
