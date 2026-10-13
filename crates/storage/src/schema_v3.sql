-- Manual party membership retains identity when a character is deleted.
CREATE TABLE party_profiles (
    party_id TEXT PRIMARY KEY
) WITHOUT ROWID;
CREATE TABLE party_participants (
    party_id TEXT NOT NULL REFERENCES party_profiles (party_id) ON DELETE CASCADE,
    position INTEGER NOT NULL CHECK (position >= 0 AND position < 40),
    profile_id INTEGER REFERENCES profiles (id) ON DELETE SET NULL,
    profile_key TEXT NOT NULL,
    PRIMARY KEY (party_id, position),
    UNIQUE (party_id, profile_key)
) WITHOUT ROWID;
CREATE INDEX party_participants_profile ON party_participants (profile_id);
