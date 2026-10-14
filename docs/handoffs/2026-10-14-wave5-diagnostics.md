# Section16.2 allowlisted explicit local diagnostics

Owner: Codex /root/backlog_audit, sole feature writer.
Baseline main6eec505, after published v0.4.0 at2d5de64.
Worktree /tmp/navzeroth-wave5-diagnostics; feature/wave5-diagnostics;
draft PR[70](https://github.com/Laymer/Navzeroth/pull/70).
Read STATUS, wave5 coordination and plan16.2. Only shared edit is root-approved
additive `pub mod diagnostics;` in app lib. Recovery/root retain shared state,
native command registration, global command client and App/navigation ownership.

## Implemented

`diagnostics::build` creates schema1 bounded Serialize-only metadata;
`build_with_context` additionally accepts an explicit FreshnessContext and optional
compatible GameBuild capability record. Current stored character/dataset schemas
have no GameBuild capability map, so native integration passes no capability
record and current checks remain unknown. Preferences are never capabilities.
`serialize` refuses reports above64KiB; `write_new` writes only a newly selected
file, refuses overwrite/symlinks, syncs success and removes an incomplete file
it created on write failure. Unix creation permissions are0600.

Imported values/identities never form export text. Versions derive from trusted
package version metadata; bundled addon is distinct from unknown installed addon.
OS and edition are fixed categories. Only ASCII-decimal u32 build/interface values
survive; arbitrary/synthetic labels become unknown. Selected snapshot level/role/
location availability, individual field freshness and capture freshness expose no
values, IDs or absolute timestamps. Field and snapshot age remain separate.

Dataset revision is explicitly a SHA256 fingerprint of revision bytes, not the
raw imported label or a profile hash. Coverage contains aggregate predefined
collection/field counts; source labels/URLs/license text, record IDs and finding
messages/references are excluded. Incompatible context/fixture suppresses dataset
coverage and sets a visible mismatch flag. Historical70338level/clock evidence
is separately labelled limited_user_report, with installed_revision_match unknown.
It cannot verify the currently installed addon or snapshot source.

Optional capability inputs retain only fixed registered keys, enum checks,
verified_available true/false/unknown and freshness. Restricted/stale remain
explicit; unknown capability IDs and all evidence identifiers are dropped.
Root must supply trustworthy capability evidence provenance if this future input
is ever enabled; current stored workflow provides none.

`DiagnosticTelemetry` is a64-entry ring of enum operation/error codes and optional
measured duration, with a visible truncation flag. Counters describe only the
current process collector. None/default counters remain unknown; `observed()`
starts actual process observation at0. Invalid duration is refused without
changing telemetry. No arbitrary message/log-string API exists. No upload occurs.

Standalone DiagnosticsPanel/DiagnosticsClient provide separate explicit preview
and export actions. Mounting does neither. Export recaptures current metadata,
uses the exact selected path and exposes no path in report/receipt/errors.
Unknown counters display unknown; fixed error codes preserve an existing preview.
Refresh/client replacement rejects obsolete responses. No shared wiring was
performed while recovery owns it.

## Actual checks

Six Rust integration tests pass: planted secrets in imported identifiers,
versions, build labels, free text/source-policy fields and unknown keys; schema/
byte bounds; capability false/restricted/stale; unknown versus actual0; ring
retention/truncation; context mismatch; create-new refusal preserving existing
file and stored profile state. App lib tests pass9, including dedicated numeric
build and byte-cap regressions. Locked app all-target Clippy and Rust fmt check
pass. Four DiagnosticsPanel tests pass; desktop typecheck/lint pass. Root review found
the frontend coverage-class casing mismatch and refresh busy-state race; both
were corrected. The new deferred old/new preview regression failed on87df42d
source and passes after correction. An old completion cannot unlock or replace
a newer pending request; Rust and frontend assert snake_case coverage classes.

Dependency setup used documented `pnpm install --frozen-lockfile`. An initial
npm-ci attempt could not run because this project uses pnpm and has no npm lock;
it produced no tracked lockfile or source change. Checks above use documented
pnpm commands. No native/live result or unrelated test run is claimed.

## Remaining integration and review

Root captures selected snapshot/content and telemetry consistently under the
recovery operation lease, then supplies immutable clones and `context(system_now)`
to `build_with_context(...,Some(&clock),None)`. Preview/export wrappers and explicit
Settings/navigation wiring remain a separate root-owned integration step. Real
foreground import/job timing/error recording must initialize and feed the typed
collector; no historical timings/counts are invented by this feature.

This draft alone does not establish user-facing native export. Current-head
hosted CI, source-locked peer review, native file-dialog/path behavior and actual
installed-addon/Windows/live acceptance remain pending or unverified. Next:
complete shared integration after recovery ownership release, review privacy and
failure behavior at exact combined head, then record actual checks before merge.
