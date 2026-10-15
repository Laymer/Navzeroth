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

## Source-identified development checks — 2026-10-15

The operator exited the game and authorized the guided update. The coordinator
backed up the prior addon and project SavedVariables privately, installed main
`e45eed1b2bde95b9a565f144ec81d49758c05c90` through its existing installer and
verified all21 installed addon files against Git blobs, no extra files and
unchanged SavedVariables hashes. Version text remains0.8.1.

The operator then ran the requested probe/status/snapshot/reload sequence and
reported `/dump GetBuildInfo()` as1.60.1,70374,Oct12th2026,16001; visible level30,
no bugs and matching values. The actual selected persisted file passes bounded
inert parsing and the existing character snapshot contract: fixturefalse,
matching target tuple, Known level30,30quest records,19equipment slots,
16known equipment observations and0objective records. All30quest completion
flags remain unreadable; this is a binding investigation, not evidence of
incomplete quests. Diagnostic capability statuses remain16untested/0failed.
Raw files and backups remain private.

The operator also completed manual recorder start/two samples/stop/export/reload
and reported all commands successful. The persisted session validates against
the existing session contract: fixturefalse, matching70374tuple,7events and
maximum relative elapsed time46,102ms. This is a manual unclassified recording;
it does not establish quest turn-ins, automatic advancement/deviation, active
play time, full session analysis or final candidate acceptance.

Clock confirmation succeeded by operator report and a subsequent persisted
capture recorded Trusted. After reload, the operator made a new capture without
confirmation; it warned of uncertainty and the saved snapshot recorded Uncertain.
The first reset-check disk read still contained the earlier trusted capture; a
completed repeat capture/reload supplied the new uncertain record. The operator
also observed both C_QuestLog.IsComplete and IsQuestFlaggedCompleted as functions;
return values were not yet observed with the patched reader.
These are narrow development observations with an inspected source revision.
They do not update aggregate release acceptance rows, establish complete
inventories/rights or bind a frozen1.0.0candidate. Windows PowerShell5.1 helper
checks passed separately with synthetic inputs; no interactive desktop acceptance
is inferred.


## Checked completion-reader update — 2026-10-15

After main840939f passed all nine hosted checks and the three source jobs, the
operator confirmed WoW closed; the coordinator also checked no WowB process was
running. Prior addon and project SavedVariables were backed up privately. The
existing installer installed all21addon files from840939f, verified against exact
Git bytes with no extras; the project SavedVariables SHA stayed unchanged. The
operator then saved/reloaded without errors on another character, confirmed
visible level13, and confirmed the exact exported5completed/9incomplete quest
counts. Actual strict inspection reports fixturefalse, target70374tuple, Known
level13,14quest records with all14completion booleans readable,19equipment slots
with10known and0objectives; clockuncertain. This is narrow development verification
of the separate getter, not turn-in/history/objective or final acceptance evidence.
The earlier level30capture remains historical; use different anonymous profiles
for distinct characters. Raw exports remain private.

The operator reports C_EventUtils.IsEventValid and IsCallbackEvent as functions,
and IsEventValid returned true for QUEST_TURNED_IN and QUEST_LOG_UPDATE. This is
only reported event availability; callback dispatch and payload remain unverified.
The opt-in recorder implementation uses only QUEST_TURNED_IN, with no log polling.


## Opt-in turn-in callback and service import — 2026-10-15

PR151 final head9cf62df passed all nine checks plus three source jobs and merged
as83f38c8. Its checked preview package source809ba1f has the identical entire tree.
All package hashes and21addon file bytes were verified. With the operator-confirmed
game closed, installed those files, saved a private backup and verified project
SavedVariables unchanged. Version stays0.8.1; main follow-up checks are separate.

The operator used a distinct anonymous profile for the level13character, explicitly
started quest mode, turned in exactly one quest and stopped/exported/reloaded with
no errors. Actual strict inspection validates fixturefalse and the target tuple,
five events, one quest-turn-in event and41,604ms maximum session-relative elapsed.
An isolated in-memory desktop application service imported the actual snapshot and
recording together:0ms observed,41,604ms unclassified,0ms gap. This is actual data
consumption on the Linux service, not native Windows UI acceptance or active time.
Raw exports remain private. No objective/turn-in history mapping, automatic plan
advancement/deviation, loot fact or final acceptance row is inferred. Paused-event
exclusion and restricted conditions remain synthetic-only for this binding.
