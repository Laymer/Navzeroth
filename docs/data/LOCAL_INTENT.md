# Local settings and saved session intent

Section8.1/12.1 uses one bounded `app_settings` row and one replaceable
`current_plan` slot in database schema9. These are desktop intent, separate from
imported character observations. No raw snapshot, ANAV1/Lua export, full planning
report, chat, token, or event stream is stored in the plan slot.

## Settings

`get_app_settings` returns a revision and version1 settings. `update_app_settings`
accepts strict JSON `{schema_version:1, expected_revision, settings}`. Settings
include nullable selected character/party, nullable explicitly chosen installation
path and export-file path/format, manual planning defaults, text/contrast/motion
choices, and requested reference-window topmost preference. Paths are local,
4096 UTF-8 bytes maximum, and reject NUL. The whole document is at most64KiB.

Saving settings does not read/scan an installation, import a file, start watching,
show a window or enable integrations. Imports has an explicit button to fill its
existing file editor from the saved export choice. Import and watch remain separate
explicit actions. Planner/Party also explicitly apply saved manual defaults.
These defaults never overwrite imported boolean preferences or infer game facts.
Selected profiles survive restart. Missing subjects remain unavailable.

Accessibility choices apply to the ordinary desktop/reference presentation;
requested topmost remains distinct from observed native window state. Windows
still start closed. Native DPI/focus/monitor behavior is not established by browser
tests. Official API support stays unverified; hosted sharing/telemetry stay disabled.

## Saved plan

After a successful solo/joint job, “Save on Today” requests a pin using only its
server job ID and expected slot revision. The server retains a separate compact
descriptor at guarded successful publication. Consuming the existing report is
still one-shot. Pin metadata shares the registry's16-terminal-job retention;
failed/canceled/foreign/pruned jobs cannot replace the slot.

The descriptor holds the canonical typed request (1MiB maximum), an original
dependency guard (64KiB maximum), and ordered original-step summary (64KiB maximum,
up to128 original objectives). It preserves exact context/fixture, ordered party
cohort, snapshot IDs/hashes, immutable content identity/hash, original disposition
counter, declared evaluation/publication UTC, freshness policy and rule/model
versions. No cluster-internal IDs replace original objectives.

Pin/clear use compare-and-swap. Clear increments the seeded slot's revision and
nulls all payload columns; it never deletes/resets the row. Settings reset also
retains monotonic revision. Revisions are exact JS-safe integers; exhaustion and
failed replacement leave prior state intact.

## Read and trust boundaries

Today/reference distinguish empty, historical and current. Current requires the
selected subject to match, original dependencies to remain exact, all original
captures to remain fresh, typed field classes to remain stable, and content
eligibility counter to remain unchanged. Restrict/release/reactivate cannot revive
an original guard even when key/hash match. Validation shares a SQLite writer
reservation with publication. Bounded projection serialization precedes final
validation; bounded descriptor serialization precedes pin validation/CAS.

Guard reconstruction uses the original trusted evaluation clock; it never
recaptures a new disposition counter or runs the optimizer. Changed/deleted
snapshots, changed cohort, content replacement/restriction, field expiry, newly
usable future fields, clock rollback outside the stable interval, missing time,
and changed rule/model versions keep the saved intent historical. Historical
steps remain inspectable, but do not render a current next action. Local expiry
also suppresses next-action text before the next minute read.

Recovered SQLite contents cannot authenticate a successful calculation. Recovery
integration must validate all documents, then call
`mark_recovered_plan_guard` to mark every populated slot `recovered_reference`,
preserving its original inputs, versions, summary and revision. It stays historical
until a new successful job is explicitly pinned. This feature does not claim that
shared recovery registration is complete before the coordinator integrates it.

Today includes original snapshot and dataset references. ReferenceView exposes
the same readonly plan metadata, with at most8 displayed steps and explicit
original count/truncation, within its existing64KiB bound. The reference webview
still permits only `get_reference_view`; it cannot access settings paths, pin,
modify settings, watch files, or manage windows.

All persisted documents reject duplicate keys at every depth, future schemas,
ignored nested fields, excessive bytes/collections, and incoherent identities or
request/summary references. Typed recovery validators are public storage functions;
errors do not echo input. Local saved intent does not authenticate participant
acceptance, report addon progress, establish source truth or verify gameplay.
