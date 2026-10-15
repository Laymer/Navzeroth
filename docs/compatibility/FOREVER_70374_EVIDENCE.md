# Forever 1.60.1.70374 evidence log

This log preserves reported observations for the requested frozen baseline.
The [v1 acceptance runbook](V1_ACCEPTANCE_RUNBOOK.md) defines the required checks;
the coordinator-owned [acceptance manifest](../../release/v1/acceptance-evidence.json)
records release-gate dispositions. Earlier 70338 evidence remains historical.

## Client identification reported on 2026-10-14

Source: the user reported the result of `/dump GetBuildInfo()` in the task
conversation, then Windows 11, out-of-combat conditions, error-free probe/status
commands and addon version `0.8.1` displayed by `/anavdiag` with no arguments.
No screenshot, serialized diagnostic export or installed addon source revision
accompanied these reports.

Reported values, preserving the empty and single-space final strings:

```text
1.60.1 - 70374 - Oct 12 2026 - 16001 - "" - " "
```

| Field | Reported value | Evidence scope |
| --- | --- | --- |
| Version | `1.60.1` | User-reported client result |
| Build | `70374` | User-reported client result |
| Build date | `Oct 12 2026` | User-reported client result |
| Interface | `16001` | User-reported client result |
| Remaining returned values | `""`, `" "` | Retained without interpreting their meaning |
| Edition | Target `wow_forever` | GetBuildInfo tuple alone does not establish edition |
| Addon version | `0.8.1` | User-reported no-argument `/anavdiag` output; installed files not inspected |
| App version, source commit and archive hashes | Unverified | Installation evidence pending; version text alone does not bind source |
| OS | Windows 11 | User report: "I am running windows 11 out of combat"; exact OS build/hardware pending |
| Combat condition | Out of combat | Same user report; no combat-transition acceptance established |
| Diagnostic commands | Reportedly no errors | User report: "anavdiag probe and status indicate no errors"; actual output pending |
| Locale, display mode, hardware, WebView2 | Unverified | Detailed operator environment record pending |

This establishes a reported match for the requested version/build/interface.
It does not establish field readability, event support, clock trust, restrictions,
plan advancement, SavedVariables persistence, or native Windows acceptance.
`LIVE-BUILD` remains **unverified** until the installed candidate, remaining
operating conditions and diagnostic output are recorded. The user-reported
error-free commands establish only that narrow reported diagnostic flow.

Next operator evidence: actual probe/status screenshot/transcription or sanitized
`diagnostic_export` after reload, plus installed source/artifact identity. Record
actual results here before updating any release acceptance status. Detailed
Windows environment collection follows the runbook.

## Development candidate prepared for the next guided check

PR118 head `8ec4edcf706d5d0e6c5485a1d86717090dc88117` passed all nine hosted
checks. Its package source `004058f863c3fe11472ddaa09d106c67e4f2c86a` is the
GitHub PR merge candidate with the identical tree. Root downloaded the five-file
artifact and verified the four checksum entries, source metadata, installer MZ
header, exact source notes and all 21 addon file bytes. See the
[machine-readable candidate record](../releases/candidates/70374-004058f.json).
These are automated/artifact checks, not user installation or live acceptance.

The user was given the exact candidate link and instructions to install with the
game closed, retain SavedVariables, then run `/anavdiag probe` and `/anavdiag copy`
out of combat and return the selectable diagnostic output. The candidate still
displays `0.8.1`; version text alone cannot distinguish it from the published ZIP.
No reply or observation for this new candidate is recorded yet. All mandatory
aggregate release rows remain unverified.
