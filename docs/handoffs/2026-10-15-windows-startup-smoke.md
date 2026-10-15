# Windows first-start smoke investigation

Owner/sole writer Codex `/root`; baseline `32c5e2d`, claim `653908e`.
Isolated branch `fix/windows-startup-smoke`.

PR143 package jobs on runs38746649696 and38751564696 found a running process but
no SQLite store after the workflow's fixed20 seconds, then killed the process.
An unchanged earlier revision passed a rerun. The root cause is not established.
Tauri initializes configured windows before the setup callback; slow WebView
initialization is plausible, but process liveness cannot rule out database errors.

Extracted the observation into `scripts/windows/First-StartSmoke.ps1`: refuse
preexisting store/WAL/SHM without deletion, fail immediately on process exit,
poll under a monotonic120-second bound, retain at least20-second liveness and
require a nonempty file to remain observed for5 seconds. Record timing and
best-effort process/window, WebView, app-data metadata and Windows application
error diagnostics before process cleanup. No relaunch, production change or
publication/candidate bypass. Existing uninstall/user-data/addon checks remain.
File observation is explicitly not database migration or UI acceptance evidence.

Actual local checks: eight synthetic-process harness checks pass under PowerShell7
on Linux: stale store/journal, alive/no store, empty store, delayed creation,
minimum survival and dwell enforcement, exited process with a store. Whitespace
check passes. The acceptance reviewer independently inspected the workflow,
scripts and docs and reran all eight checks successfully, with no important
finding. New harness runs in the hosted Windows packaging job; native behavior
and current-source hosted results remain pending when this handoff was written.
No native interactive or game check is claimed.

Next: await all nine exact-head hosted checks, integrate through protected main
in source-review/staging order and verify resulting main. Keep failed historical
runs as evidence. Publisher inquiry still awaits the user's reply email before
submission; no personal contact data belongs in Git.
