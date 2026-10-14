# Forever 1.60.1.70374 evidence log

This log preserves reported observations for the requested frozen baseline.
The [v1 acceptance runbook](V1_ACCEPTANCE_RUNBOOK.md) defines the required checks;
the coordinator-owned [acceptance manifest](../../release/v1/acceptance-evidence.json)
records release-gate dispositions. Earlier 70338 evidence remains historical.

## Client identification reported on 2026-10-14

Source: the user reported the result of `/dump GetBuildInfo()` in the task
conversation, then Windows 11, out-of-combat conditions and error-free probe/status
commands. No screenshot, serialized diagnostic export or installed addon source
revision accompanied these reports.

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
| Addon/app version and source commit | Unverified | Installation evidence pending |
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

Next operator evidence requested by the coordinator: `/anavdiag` with no arguments
for the displayed addon version, actual probe/status screenshot/transcription or
sanitized `diagnostic_export` after reload, and installed source identity. Record
actual results here before updating any release acceptance status. Detailed
Windows environment collection follows the runbook.
