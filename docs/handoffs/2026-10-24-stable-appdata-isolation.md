# V21.1: stable acceptance application-data isolation

Owner: Claude Code release continuation
(docs/claims/2026-10-23-claude-stable-release-continuation.md).
Baseline: candidate b81e55c5 (PR #239); branch fix/stable-acceptance-appdata-isolation.

## V21 Windows package run 39990706776

Attempt 1 (2026-10-23): INSTALL and RESOURCES passed. FIRST-START failed:
"Installed native engineering workload failed." The driver's progress file stops
in the `endpoint` phase at 28 ms. The driver exited non-zero about 19 s later
without recording `cleanup` or writing its report. Its stderr does not reach the
job log, so the cause is unknown. Runner image 20261021.618; the last passing
FIRST-START ran on 20261002.596. The other cases did not run.

Attempt 2 (2026-10-24, re-run of the same commit): INSTALL, RESOURCES,
FIRST-START, PERFORMANCE, PERSISTENCE-RECOVERY and HOSTED-OFFLINE passed. OFFLINE
used the ADR 0010 firewall isolation: isolated, external connection refused,
restored. UPGRADE-RC1 failed. The RC1 baseline workload reported
`Database recovery required` 0.9 s after start, before any work.
DOWNGRADE-REFUSAL did not run. The draft gate refused both attempts.

## Cause

The app resolves its data folder with `app_data_dir()`, which calls
`SHGetKnownFolderPath(FOLDERID_RoamingAppData)` (dirs-sys 0.5.0). The known-folder
API ignores the `APPDATA` environment variable. The recovery report shows the real
path `C:\Users\runneradmin\AppData\Roaming\io.github.laymer.azeroth-navigator`.
Stable-Acceptance.ps1 set `$env:APPDATA` to `isolated-appdata` and, before RC1,
to `upgrade-appdata`. Neither override took effect, so:

- RC1 opened the candidate's database from the earlier cases and refused it.
- Every stable acceptance run so far started on data the job left behind. The
  native smoke upgrade step populates a database, and its silent uninstall keeps
  user data by design. FIRST-START "fresh" was therefore not fresh. The
  PERFORMANCE limitation "fresh database after resource verification" was untrue
  for V19, V20 and V21 attempts. No such results were accepted as release evidence.

## Repair

- `Get-StableAppDataDirectory` resolves the Roaming known folder through
  `Environment.GetFolderPath`, the same known-folder API the app uses.
- Acceptance refuses to start while that application data exists.
- The workflow removes the native smoke data before acceptance, as the earlier
  smoke steps already do.
- `Move-StableAppDataAside` moves the candidate's data to
  `candidate-appdata` (retained, never deleted) before the RC1 baseline. The
  result is recorded in `raw/appdata-aside.json`.
- The ineffective `APPDATA` overrides and their restoration are removed.

## Checks run by the author

- Test-StableAcceptanceHelpers.ps1: Linux PS7 57/57, native Windows PS7 58/58,
  native PS5.1 57/57. Five new checks cover the known folder ignoring an APPDATA
  override, refused identifiers, an intact move, absent data, and refusals that
  keep the data.
- package-windows-workflow.test.mjs 6/6; Test-StableWorkflowGate.ps1 12/12;
  actionlint clean.

## Not verified

- A hosted Windows run with the repair.
- The attempt 1 FIRST-START failure, whose cause is unknown. It may recur.

## Next

1. Independent review, then merge into release/stable-1.0.0-version.
2. The change is production (outside docs/ and release/). Renew the V22
   fingerprint, functional binding, content review, preparation, receipt and
   closure, as V21 did after PR #285.
3. Push the V22 candidate and read all eight cases.
