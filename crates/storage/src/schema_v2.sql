-- Reference content is independent of mutable player snapshot/profile state.
CREATE TABLE dataset_revisions (
    id INTEGER PRIMARY KEY,
    edition TEXT NOT NULL,
    build TEXT NOT NULL,
    interface TEXT NOT NULL,
    namespace TEXT NOT NULL CHECK (namespace IN ('demo', 'reference')),
    is_fixture INTEGER NOT NULL CHECK (is_fixture IN (0, 1)),
    revision TEXT NOT NULL,
    document TEXT NOT NULL,
    UNIQUE (edition, build, interface, namespace, is_fixture, revision),
    UNIQUE (id, edition, build, interface, namespace, is_fixture)
);
CREATE TABLE dataset_manifests (
    revision_id INTEGER PRIMARY KEY REFERENCES dataset_revisions(id),
    manifest_version INTEGER NOT NULL CHECK (manifest_version = 1),
    content_sha256 TEXT NOT NULL CHECK (length(content_sha256) = 64),
    coverage_document TEXT NOT NULL
);
CREATE TABLE source_records (
    revision_id INTEGER NOT NULL REFERENCES dataset_revisions(id),
    source_id TEXT NOT NULL,
    document TEXT NOT NULL,
    PRIMARY KEY (revision_id, source_id)
);
CREATE TABLE source_conflicts (
    revision_id INTEGER NOT NULL REFERENCES dataset_revisions(id),
    conflict_index INTEGER NOT NULL CHECK (conflict_index >= 0),
    document TEXT NOT NULL,
    PRIMARY KEY (revision_id, conflict_index)
);
CREATE TABLE active_datasets (
    edition TEXT NOT NULL,
    build TEXT NOT NULL,
    interface TEXT NOT NULL,
    namespace TEXT NOT NULL CHECK (namespace IN ('demo', 'reference')),
    is_fixture INTEGER NOT NULL CHECK (is_fixture IN (0, 1)),
    revision_id INTEGER NOT NULL,
    previous_revision_id INTEGER,
    PRIMARY KEY (edition, build, interface, namespace, is_fixture),
    FOREIGN KEY (revision_id, edition, build, interface, namespace, is_fixture)
        REFERENCES dataset_revisions(id, edition, build, interface, namespace, is_fixture),
    FOREIGN KEY (previous_revision_id, edition, build, interface, namespace, is_fixture)
        REFERENCES dataset_revisions(id, edition, build, interface, namespace, is_fixture)
);
CREATE TRIGGER dataset_revisions_no_update BEFORE UPDATE ON dataset_revisions
BEGIN SELECT RAISE(ABORT, 'immutable revision'); END;
CREATE TRIGGER dataset_revisions_no_delete BEFORE DELETE ON dataset_revisions
BEGIN SELECT RAISE(ABORT, 'immutable revision'); END;
CREATE TRIGGER dataset_manifests_no_update BEFORE UPDATE ON dataset_manifests
BEGIN SELECT RAISE(ABORT, 'immutable manifest'); END;
CREATE TRIGGER dataset_manifests_no_delete BEFORE DELETE ON dataset_manifests
BEGIN SELECT RAISE(ABORT, 'immutable manifest'); END;
CREATE TRIGGER source_records_no_update BEFORE UPDATE ON source_records
BEGIN SELECT RAISE(ABORT, 'immutable source'); END;
CREATE TRIGGER source_records_no_delete BEFORE DELETE ON source_records
BEGIN SELECT RAISE(ABORT, 'immutable source'); END;
CREATE TRIGGER source_conflicts_no_update BEFORE UPDATE ON source_conflicts
BEGIN SELECT RAISE(ABORT, 'immutable conflict'); END;
CREATE TRIGGER source_conflicts_no_delete BEFORE DELETE ON source_conflicts
BEGIN SELECT RAISE(ABORT, 'immutable conflict'); END;
-- The manifest seals its revision once all child records are inserted.
CREATE TRIGGER source_records_sealed BEFORE INSERT ON source_records
WHEN EXISTS (SELECT 1 FROM dataset_manifests WHERE revision_id=NEW.revision_id)
BEGIN SELECT RAISE(ABORT, 'sealed revision'); END;
CREATE TRIGGER source_conflicts_sealed BEFORE INSERT ON source_conflicts
WHEN EXISTS (SELECT 1 FROM dataset_manifests WHERE revision_id=NEW.revision_id)
BEGIN SELECT RAISE(ABORT, 'sealed revision'); END;
