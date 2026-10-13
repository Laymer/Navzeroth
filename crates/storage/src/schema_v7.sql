-- Explicit user annotations; immutable imported events remain untouched.
CREATE TABLE session_calibration_annotations (
    profile_id INTEGER NOT NULL,
    session_id TEXT NOT NULL,
    document TEXT NOT NULL CHECK (length(document) <= 2048),
    PRIMARY KEY (profile_id, session_id),
    FOREIGN KEY (profile_id, session_id) REFERENCES sessions (profile_id, session_id) ON DELETE CASCADE
) WITHOUT ROWID;
