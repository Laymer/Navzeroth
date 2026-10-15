# Reference-content v2 schema

This is a separate content format. Snapshot/plan exchange remains v1. The generated
schema closes fixed records, preserves typed dynamic maps and encodes explicit
format/version and observation distinctions. Rust decoding additionally refuses
duplicate JSON keys and enforces graph, context, provenance and capacity rules.
Structural validity never establishes rights, completeness or game observations.

Regenerate with:

```sh
cargo run --locked -p azeroth-core --example export_content_schema > schemas/v2/reference-content.schema.json
cargo test --locked -p azeroth-core --test content_v2 regenerate_content_fixture -- --ignored
```

Validate with `cargo test --locked -p azeroth-core --test content_v2` and
`node scripts/validate-content-v2-schema.mjs` after the documented pnpm install.
The fixture is entirely synthetic. Capacity PR5 must regenerate after replacing
the intermediate nested v1 payload with the dedicated catalog payload.
