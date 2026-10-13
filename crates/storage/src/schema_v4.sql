-- Profile and snapshot identities survive pruning; no document is rewritten.
CREATE TABLE snapshot_promotions (
    id INTEGER PRIMARY KEY,
    profile_id INTEGER NOT NULL REFERENCES profiles(id) ON DELETE CASCADE,
    previous_snapshot_id TEXT NOT NULL,
    selected_snapshot_id TEXT NOT NULL,
    reason TEXT NOT NULL CHECK(length(reason) BETWEEN 1 AND 1024),
    operated_at_utc_ms INTEGER NOT NULL
);
