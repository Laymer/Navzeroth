-- Bounded local feature state. Eligibility is not evidence verification or permission.
CREATE TABLE content_dispositions (
    revision_id INTEGER PRIMARY KEY REFERENCES dataset_revisions(id),
    state TEXT NOT NULL CHECK (state IN ('eligible', 'quarantined', 'revoked')),
    state_revision INTEGER NOT NULL CHECK (typeof(state_revision)='integer' AND state_revision BETWEEN 1 AND 9007199254740991),
    reason_code TEXT NOT NULL CHECK (reason_code IN ('initial_eligibility', 'operator_review', 'invalid_content', 'source_permission', 'context_mismatch', 'withdrawn_contribution')),
    changed_at_utc_ms INTEGER CHECK (changed_at_utc_ms IS NULL OR (typeof(changed_at_utc_ms)='integer' AND changed_at_utc_ms BETWEEN -9007199254740991 AND 9007199254740991)),
    last_activated_order INTEGER CHECK (last_activated_order IS NULL OR (typeof(last_activated_order)='integer' AND last_activated_order BETWEEN 1 AND 9007199254740991))
);
INSERT INTO content_dispositions (revision_id, state, state_revision, reason_code, last_activated_order)
SELECT d.id, 'eligible', 1, 'initial_eligibility',
    CASE WHEN EXISTS (SELECT 1 FROM active_datasets a WHERE a.revision_id=d.id) THEN 2
         WHEN EXISTS (SELECT 1 FROM active_datasets a WHERE a.previous_revision_id=d.id) THEN 1 ELSE NULL END
FROM dataset_revisions d;
CREATE TABLE app_settings (
    slot INTEGER PRIMARY KEY CHECK (slot=1),
    schema_version INTEGER NOT NULL CHECK (schema_version=1),
    revision INTEGER NOT NULL CHECK (typeof(revision)='integer' AND revision BETWEEN 1 AND 9007199254740991),
    document TEXT NOT NULL CHECK (typeof(document)='text' AND length(CAST(document AS BLOB)) BETWEEN 1 AND 65536)
);
CREATE TABLE current_plan (
    slot INTEGER PRIMARY KEY CHECK (slot=1),
    schema_version INTEGER NOT NULL CHECK (schema_version=1),
    revision INTEGER NOT NULL CHECK (typeof(revision)='integer' AND revision BETWEEN 1 AND 9007199254740991),
    kind TEXT,
    request_json TEXT,
    guard_json TEXT,
    summary_json TEXT,
    CHECK ((kind IS NULL AND request_json IS NULL AND guard_json IS NULL AND summary_json IS NULL) OR
        (kind IS NOT NULL AND kind IN ('solo','joint')
         AND request_json IS NOT NULL AND typeof(request_json)='text' AND length(CAST(request_json AS BLOB)) BETWEEN 1 AND 1048576
         AND guard_json IS NOT NULL AND typeof(guard_json)='text' AND length(CAST(guard_json AS BLOB)) BETWEEN 1 AND 65536
         AND summary_json IS NOT NULL AND typeof(summary_json)='text' AND length(CAST(summary_json AS BLOB)) BETWEEN 1 AND 65536))
);
INSERT INTO current_plan (slot,schema_version,revision) VALUES (1,1,1);
CREATE TABLE local_submissions (
    id TEXT PRIMARY KEY CHECK (typeof(id)='text' AND length(CAST(id AS BLOB)) BETWEEN 1 AND 64),
    fingerprint TEXT NOT NULL UNIQUE CHECK (length(fingerprint)=64 AND fingerprint NOT GLOB '*[^0-9a-f]*'),
    edition TEXT NOT NULL,
    build TEXT NOT NULL,
    interface TEXT NOT NULL,
    namespace TEXT NOT NULL CHECK (namespace IN ('demo','reference')),
    is_fixture INTEGER NOT NULL CHECK (is_fixture IN (0,1)),
    base_revision_id INTEGER NOT NULL,
    source_id TEXT NOT NULL CHECK (length(CAST(source_id AS BLOB)) BETWEEN 1 AND 128),
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
