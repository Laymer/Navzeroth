-- Immutable companions; manual state never replaces an imported observation.
-- Parent snapshot deletion/pruning also removes its now-unusable companions.
CREATE TABLE progression_snapshots (
    profile_id INTEGER NOT NULL REFERENCES profiles(id) ON DELETE CASCADE,
    snapshot_row_id INTEGER NOT NULL REFERENCES snapshots(id) ON DELETE CASCADE,
    origin TEXT NOT NULL CHECK (origin IN ('imported', 'manual')),
    companion_id TEXT NOT NULL,
    content_sha256 TEXT NOT NULL CHECK (length(content_sha256) = 64),
    document TEXT NOT NULL CHECK (length(CAST(document AS BLOB)) <= 4194304),
    PRIMARY KEY (profile_id, origin, companion_id)
) WITHOUT ROWID;
CREATE INDEX progression_parent ON progression_snapshots(snapshot_row_id, origin);
