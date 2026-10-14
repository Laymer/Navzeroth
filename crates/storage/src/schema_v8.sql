-- One bounded local proposal per party; acceptance is an explicit local declaration.
CREATE TABLE party_plan_proposals (
    party_id TEXT PRIMARY KEY REFERENCES party_profiles(party_id) ON DELETE CASCADE,
    proposal_id TEXT NOT NULL CHECK (length(proposal_id) = 64),
    revision INTEGER NOT NULL CHECK (revision > 0),
    document TEXT NOT NULL CHECK (length(document) <= 2097152)
) WITHOUT ROWID;
