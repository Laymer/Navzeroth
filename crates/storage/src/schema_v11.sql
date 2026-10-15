-- Preserve every v1 document/hash and the shared activation/disposition history.
-- SQLite's added constant default does not rewrite immutable revision content.
ALTER TABLE dataset_revisions ADD COLUMN content_version INTEGER NOT NULL DEFAULT 1
    CHECK (content_version IN (1, 2));
