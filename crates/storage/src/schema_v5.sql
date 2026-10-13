-- Immutable recorder events and derived summaries, scoped by exact profile key.
CREATE TABLE sessions (
    profile_id INTEGER NOT NULL REFERENCES profiles (id) ON DELETE CASCADE,
    session_id TEXT NOT NULL,
    content_sha256 TEXT NOT NULL CHECK (length(content_sha256) = 64),
    document TEXT NOT NULL CHECK (length(document) <= 32768),
    summary_document TEXT NOT NULL,
    PRIMARY KEY (profile_id, session_id)
) WITHOUT ROWID;
