# World-data example fixtures

Inputs for the `validate_world_data` and `validate_knowledge` examples, run under
coverage by `scripts/coverage/collect-rust.mjs`. `shards/` holds one valid shard,
one invalid shard and the runner bookkeeping the validator must skip (`.written/`,
`facts/`, `run-manifest.json`, non-JSON files); `facts/` holds one valid and one
invalid knowledge_v1 document. Both runs therefore exit 1.
