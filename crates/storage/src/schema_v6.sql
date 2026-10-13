-- Bounded reminder preferences/history, isolated by the full persisted profile.
CREATE TABLE reminder_state (
    profile_id INTEGER PRIMARY KEY REFERENCES profiles (id) ON DELETE CASCADE,
    document TEXT NOT NULL CHECK (length(document) <= 1048576)
);
