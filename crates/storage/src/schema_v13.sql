-- Durable travel choices belong to immutable revisions, not process memory.
CREATE TABLE content_travel_choices (
    revision_id INTEGER PRIMARY KEY REFERENCES dataset_revisions(id),
    accept_inferred INTEGER NOT NULL CHECK(accept_inferred IN (0,1))
);
CREATE TABLE bundled_content_defaults (
    scope_json TEXT PRIMARY KEY CHECK(length(scope_json)<=4096),
    app_version TEXT NOT NULL CHECK(length(app_version)<=128),
    previous_app_version TEXT,
    revision_id INTEGER NOT NULL REFERENCES dataset_revisions(id)
);
