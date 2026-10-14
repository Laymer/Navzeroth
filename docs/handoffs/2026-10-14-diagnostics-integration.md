# Explicit diagnostics workflow integration

Owner: Codex `/root/release_review`, sole task writer. Baseline main02044c2;
diagnostics foundation c221752, backend planning dependency daf82b7 (including native DTO cb9b1be).
Branch `feature/wave5-diagnostics-integration`; source ownership released at `d135b73e6b06083ff243b231334bd9316f1ef1c0`
after normal merging the final backend freshness fix. Further source fixes require
coordination with root.

## Result and interfaces

`DatabaseHost::diagnostic_report(now)` holds the recovery read lease while
`SnapshotApplication::diagnostic_inputs` captures selected profile, current
snapshot and exact context/fixture active content under one store mutex. It
then builds from owned immutable values outside the store/recovery locks with
`build_with_context` and the existing64KiB allowlist. Capture age and individual
field age remain separate. No GameBuild capability record is available, so
capabilities remain unknown; preferences never stand in for capability evidence.
A failed/checking database startup yields unknown selected metadata and actual
observed process counters. No corrupt database contents or stored error strings
are used to generate a report.

`DatabaseHost::observe(operation,call)` starts an actual monotonic duration,
records only fixed operation/ErrorCode enums and optional duration, and returns
the original result. The collector starts observed0 per process and retains64
completion events. Foreground snapshot/session/content import attempts include
successful duplicate/no-op imports; counts describe attempts, never stored history.
No watcher import, earlier process, rejected planning registration, OS timing or
historical job statistics are claimed. Timing covers wrapped foreground work,
including relevant lock/lease waiting, rather than a universal performance budget.
Operations exceeding the allowed one-day duration report unknown duration.
Telemetry is never held over store/recovery work; cancellation/status still
reach the independent planning registry without a database lease.

Native foreground snapshot text/file, session text/file and content staging,
legacy solo/joint planning, backup and restore handlers feed the collector.
Accepted detached planning jobs record their actual terminal outcome after guarded
publication, including cancellation/failure, instead of recording enqueue success.
`JobTicket::completion_result` reads retained terminal outcome even if a caller
retrieved the result or registry retention pruned it. Native DTOs stay in the app
crate; the backend's JobAccepted and unit typed failure fix are preserved.

`preview_diagnostics` captures metadata only. `export_diagnostics(path)` recaptures
metadata and writes through existing serialize/create-new/sync helper; it refuses
existing files and preserves their bytes. UI Local diagnostics navigation invokes
neither operation automatically. Explicit preview and file path/export actions
remain separate. Errors display fixed codes only, retain a valid preview, and
reject obsolete refresh/client responses. Recovery-only navigation keeps this
explicit flow reachable after a failed database startup; gameplay views remain
unavailable there. The browser demo reports these native operations unavailable.
Nothing uploads or echoes raw messages, profile/character IDs, source text,
account paths, private exports, tokens or chat.

## Actual checks and failures

Before the backend native DTO update: full app tests passed; desktop121 unit tests,
typecheck/lint and full20 mocked Chromium cases passed, including the new
explicit preview/export/error workflow. After normal merging cb9b1be: the4
workflow tests and9registry tests passed, strict app all-target Clippy and Rust
fmt/diff checks passed; desktop typecheck/lint/build passed. After enabling
recovery-only diagnostics navigation: existing121 unit tests passed, focused
App8 tests including the newly added recovery navigation test passed, and both
diagnostics/recovery mocked Chromium cases passed. All browser evidence uses
command mocks, not native I/O or live game observations.

Initial new scope test incorrectly expected different characters with the same
context/fixture to have different active content; the actual context-scoped
behavior was correct. The test now uses a distinct synthetic build and verifies
snapshot/content scope together. Initial Clippy rejected an unused new ticket
method in the backend's path-included duplicate test module; a meaningful actual
terminal-result regression fixed this, and the later backend update replaced that
module inclusion with the public app module. A new App test initially used Playwright’s exact option in Testing Library; TypeScript rejected it and the test now uses the existing role/name convention. No failed check is claimed passed.

A local locked native cargo check was actually attempted and failed at the missing
system `dbus-1.pc` dependency before shell compilation. Actual native command
execution, selected-file behavior/permissions on Windows, installed addon version,
telemetry durations on target hardware and live gameplay remain UNVERIFIED.
Exact-head hosted Windows compilation and peer review remain required before
protected-main integration. The final backend freshness fix daf82b7 was normally merged; final18 affected
Rust tests (workflow4, registry9, service5 including store-wait expiry) and strict
app Clippy/fmt/diff pass at source d135b73. Final desktop122 unit tests across22
files, typecheck/lint/build pass. Full20 mocked Chromium cases passed before the
last recovery-navigation change; its affected diagnostics/recovery2 cases were
rerun afterward and passed. Browser port1420 is free. Next: exact combined-head
peer/hosted review, reconcile separate backend/UI PR dependencies, and update the
batch release ledger without claiming native/live acceptance.
