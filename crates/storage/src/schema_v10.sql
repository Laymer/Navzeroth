-- Preserve schema9 history while accepting the existing256-byte registered SourceId contract.
CREATE TABLE local_submissions_next (
    id TEXT PRIMARY KEY CHECK (typeof(id)='text' AND length(CAST(id AS BLOB)) BETWEEN 1 AND 64),
    fingerprint TEXT NOT NULL UNIQUE CHECK (length(fingerprint)=64 AND fingerprint NOT GLOB '*[^0-9a-f]*'),
    edition TEXT NOT NULL,
    build TEXT NOT NULL,
    interface TEXT NOT NULL,
    namespace TEXT NOT NULL CHECK (namespace IN ('demo','reference')),
    is_fixture INTEGER NOT NULL CHECK (is_fixture IN (0,1)),
    base_revision_id INTEGER NOT NULL,
    source_id TEXT NOT NULL CHECK (length(CAST(source_id AS BLOB)) BETWEEN 1 AND 256),
    state TEXT NOT NULL CHECK (state IN ('pending','accepted','rejected','withdrawn','revoked')),
    state_revision INTEGER NOT NULL CHECK (typeof(state_revision)='integer' AND state_revision BETWEEN 1 AND 9007199254740991),
    consent_version INTEGER NOT NULL CHECK (consent_version=1),
    document TEXT CHECK (document IS NULL OR (typeof(document)='text' AND length(CAST(document AS BLOB)) BETWEEN 1 AND 16384)),
    decision_code TEXT NOT NULL CHECK (decision_code IN ('pending_review','accepted_local_review','rejected_local_review','withdrawn_by_submitter','revoked_local_review')),
    created_at_utc_ms INTEGER CHECK (created_at_utc_ms IS NULL OR (typeof(created_at_utc_ms)='integer' AND created_at_utc_ms BETWEEN -9007199254740991 AND 9007199254740991)),
    changed_at_utc_ms INTEGER CHECK (changed_at_utc_ms IS NULL OR (typeof(changed_at_utc_ms)='integer' AND changed_at_utc_ms BETWEEN -9007199254740991 AND 9007199254740991)),
    CHECK (document IS NOT NULL OR state IN ('withdrawn','revoked')),
    CHECK (state!='withdrawn' OR document IS NULL),
    CHECK ((state='pending' AND decision_code='pending_review')
        OR (state='accepted' AND decision_code='accepted_local_review')
        OR (state='rejected' AND decision_code='rejected_local_review')
        OR (state='withdrawn' AND decision_code='withdrawn_by_submitter')
        OR (state='revoked' AND decision_code='revoked_local_review')),
    FOREIGN KEY (base_revision_id,edition,build,interface,namespace,is_fixture)
        REFERENCES dataset_revisions(id,edition,build,interface,namespace,is_fixture),
    FOREIGN KEY (base_revision_id,source_id) REFERENCES source_records(revision_id,source_id)
);

INSERT INTO local_submissions_next SELECT * FROM local_submissions;
DROP TABLE local_submissions;
ALTER TABLE local_submissions_next RENAME TO local_submissions;
