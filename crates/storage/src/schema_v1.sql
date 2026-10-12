-- Schema version 1: character snapshots only (plan sections 8.1 and 8.2).
-- Profiles are scoped by character, edition, build, interface, dataset
-- namespace and the fixture flag, so snapshots of different contexts never
-- share a current pointer or history (CASE-005, CASE-015).
CREATE TABLE profiles (
    id INTEGER PRIMARY KEY,
    character_id TEXT NOT NULL,
    edition TEXT NOT NULL,
    build TEXT NOT NULL,
    interface TEXT NOT NULL,
    namespace TEXT NOT NULL CHECK (namespace IN ('demo', 'reference')),
    is_fixture INTEGER NOT NULL CHECK (is_fixture IN (0, 1)),
    current_snapshot_id INTEGER REFERENCES snapshots (id) ON DELETE SET NULL,
    UNIQUE (character_id, edition, build, interface, namespace, is_fixture)
);

CREATE TABLE profile_preferences (
    profile_id INTEGER NOT NULL REFERENCES profiles (id) ON DELETE CASCADE,
    preference_id TEXT NOT NULL,
    status TEXT NOT NULL CHECK (
        status IN ('known', 'unknown', 'unsupported', 'restricted', 'not_applicable')
    ),
    -- NULL unless status is known; false (0) is distinct from unknown (NULL).
    value INTEGER CHECK (value IN (0, 1)),
    document TEXT NOT NULL,
    PRIMARY KEY (profile_id, preference_id),
    CHECK ((status = 'known') = (value IS NOT NULL))
) WITHOUT ROWID;

CREATE TABLE snapshots (
    id INTEGER PRIMARY KEY,
    profile_id INTEGER NOT NULL REFERENCES profiles (id) ON DELETE CASCADE,
    snapshot_id TEXT NOT NULL,
    content_sha256 TEXT NOT NULL CHECK (length(content_sha256) = 64),
    producer_component TEXT NOT NULL,
    producer_version TEXT NOT NULL,
    producer_session_id TEXT,
    producer_sequence INTEGER,
    game_locale TEXT NOT NULL,
    source_revision TEXT NOT NULL,
    capture_source TEXT NOT NULL,
    capture_clock_trust TEXT NOT NULL CHECK (capture_clock_trust IN ('trusted', 'uncertain')),
    capture_observed_at_utc_ms INTEGER,
    capture_session_id TEXT,
    capture_session_elapsed_ms INTEGER,
    capture_sequence INTEGER,
    received_at_utc_ms INTEGER,
    -- The full validated envelope; the source of truth, re-validated on read.
    document TEXT NOT NULL,
    UNIQUE (profile_id, content_sha256),
    UNIQUE (profile_id, snapshot_id)
);
CREATE INDEX snapshots_profile_capture
    ON snapshots (profile_id, capture_observed_at_utc_ms, capture_sequence);

-- Queryable status per observation. Values stay in `snapshots.document`, so
-- unknown, unsupported, restricted, not applicable, false and zero are never
-- collapsed into NULL or 0 here.
CREATE TABLE snapshot_observations (
    id INTEGER PRIMARY KEY,
    snapshot_id INTEGER NOT NULL REFERENCES snapshots (id) ON DELETE CASCADE,
    field TEXT NOT NULL,
    entry_key TEXT NOT NULL DEFAULT '',
    sub_key TEXT NOT NULL DEFAULT '',
    status TEXT NOT NULL CHECK (
        status IN ('known', 'unknown', 'unsupported', 'restricted', 'not_applicable')
    ),
    clock_trust TEXT NOT NULL CHECK (clock_trust IN ('trusted', 'uncertain')),
    observed_at_utc_ms INTEGER,
    UNIQUE (snapshot_id, field, entry_key, sub_key)
);

-- Redacted audit trail: statuses, error codes and static messages only. It is
-- pruned to a fixed row count; it is not a debug log (plan section 16).
CREATE TABLE imports (
    id INTEGER PRIMARY KEY,
    source_kind TEXT NOT NULL CHECK (
        source_kind IN ('file', 'clipboard', 'saved_variables', 'fixture')
    ),
    status TEXT NOT NULL CHECK (status IN ('stored', 'history_only', 'duplicate', 'failed')),
    error_code TEXT,
    message TEXT,
    snapshot_id INTEGER REFERENCES snapshots (id) ON DELETE SET NULL,
    received_at_utc_ms INTEGER,
    CHECK ((status = 'failed') = (error_code IS NOT NULL))
);
