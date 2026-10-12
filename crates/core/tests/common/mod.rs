use serde_json::{Value, json};
pub fn context() -> Value {
    json!({"build":{"edition":"wow_forever","build":"SYNTHETIC","interface":"SYNTHETIC"},"namespace":"demo"})
}
pub fn unknown() -> Value {
    json!({"status":"unknown"})
}
pub fn snapshot() -> Value {
    json!({"schema_version":1,"id":"snap1","character":"char1","context":context(),"capture":{},"source_revision":"demo-r1","level":unknown(),"role":unknown(),"location":unknown(),"preferences":{},"equipment":{},"quests":{},"objectives":{}})
}
