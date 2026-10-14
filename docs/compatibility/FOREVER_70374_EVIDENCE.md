# Forever 1.60.1.70374 evidence log

This log preserves reported observations for the requested frozen baseline.
The [v1 acceptance runbook](V1_ACCEPTANCE_RUNBOOK.md) defines the required checks;
the coordinator-owned [acceptance manifest](../../release/v1/acceptance-evidence.json)
records release-gate dispositions. Earlier 70338 evidence remains historical.

## Client identification reported on 2026-10-14

Source: the user reported the result of `/dump GetBuildInfo()` in the task
conversation. No screenshot, diagnostic export, installed addon source revision,
or native Windows environment record accompanied this result.

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
| OS, locale, display mode, combat conditions | Unverified | Operator environment record pending |

This establishes a reported match for the requested version/build/interface.
It does not establish field readability, event support, clock trust, restrictions,
plan advancement, SavedVariables persistence, or native Windows acceptance.
`LIVE-BUILD` remains **unverified** until the installed candidate and operating
conditions are recorded and diagnostic behavior is checked.

Next operator evidence requested by the coordinator: `/anavdiag probe`,
`/anavdiag status`, Windows version, and whether the checks ran out of combat.
Record the actual results here before updating any release acceptance status.
