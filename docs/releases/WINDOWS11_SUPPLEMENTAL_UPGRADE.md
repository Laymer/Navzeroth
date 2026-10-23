# Supplemental personal Windows11 upgrade from0.8.1

This is an opt-in engineering observation, **not a release gate** or a substitute
for schema5 exact-draft hosted UI/native acceptance. Unavailable or unperformed
local work remains unperformed. A relevant defect observed locally still holds
publication until investigated. No local operation has been performed by preparing
this launcher. Existing hosted drivers retain their disposable-host guards.

## Source-backed paths and limits

The default NSIS installation is `%LOCALAPPDATA%\Azeroth Navigator`, with
`azeroth-desktop.exe`. Tauri's `io.github.laymer.azeroth-navigator` identifier and
`app_data_dir()` locate Windows roaming data at
`%APPDATA%\io.github.laymer.azeroth-navigator`: `snapshots.sqlite3` and any SQLite
sidecars, `local-data-v1`, and other files are all preserved. The corresponding
`%LOCALAPPDATA%\io.github.laymer.azeroth-navigator` holds addon-location/WebView
state and is also preserved when present. References:
`apps/desktop/src-tauri/tauri.conf.json`, `src/lib.rs` startup,
`scripts/windows/Stable-Acceptance.ps1`, and the addon installer's state directory.
These are repository defaults, not assertions that a personal machine currently
uses them. The launcher refuses another installed baseline or a running app;
custom installations require explicit review rather than guessed discovery.

Backups stay outside every Git worktree, with inheritance disabled and access
restricted to the invoking Windows identity. They contain private user data: do
not upload the backup, manifest, paths or exports to GitHub. Preserve normal disk
protection and sufficient free space. Backup inventory is bounded to100000 files,
16GiB per root and8GiB per file. Manifest reads are bounded to32MiB. Reparse paths
are refused. Source, copy and pre-upgrade inventories must match exact SHA256 and
lengths. This is an offline closed-app copy, not an adversarial filesystem sandbox
or SQLite online-backup protocol; do not change files concurrently. Incomplete
backups have no completed manifest and cannot authorize the upgrade.

## Concrete invocation, only after exact draft verification

First independently download and verify the exact draft assets using the release
verifier: release/asset/source identities, unsigned status, checksums and content
receipts remain required. Select the installer SHA256 and40-character source
commit from that independently verified evidence, not by accepting the local
file's computed hash as its own authority. Retain its downloaded `SOURCE_COMMIT.txt`.
The launcher additionally checks that marker, installer version1.0.0 and unsigned
status. It does not repeat the complete release verifier or establish provenance
merely from a matching local source marker.

Close the app normally, including any background instance. Use ordinary Windows
PowerShell or PowerShell7; do not alter execution policy. If local policy refuses
this script, stop and arrange an allowed execution method with the owner. Substitute
only the already verified concrete values below. Choose a new private backup
folder under an existing parent outside Git and outside application data.

```powershell
$arguments = @{
  Installer = 'C:\verified-draft\AzerothNavigator-1.0.0-x64-setup.exe'
  InstallerSha256 = '<independently verified lowercase SHA256>'
  SourceCommit = '<independently verified lowercase source commit>'
  SourceCommitFile = 'C:\verified-draft\SOURCE_COMMIT.txt'
  BackupDirectory = 'D:\private-backups\navzeroth-before-1.0.0'
}
.\scripts\windows\Start-SupplementalUpgrade.ps1 -Phase Backup @arguments
```

Review the private backup manifest and available storage before proceeding. The
following is a separate explicit action. The owner supplies elevation through the
normal UAC prompt; the launcher cannot approve it and does not suppress prompts,
change security policy, install silently, stop processes or control networking.

```powershell
.\scripts\windows\Start-SupplementalUpgrade.ps1 -Phase Upgrade @arguments
```

The launcher rechecks original0.8.1 executable/data and all backup files before
opening the exact installer via `Start-Process -Verb RunAs`. Follow installer
prompts. Decline an automatic application launch on the finish screen; first start
is a separate observation. Cancellation is not a pass. The20-minute wait is finite;
on timeout it leaves the interactive installer untouched and requires manual
inspection, not a second overlapping attempt. Never run two upgrades concurrently.

`upgrade-observation.json` records installer source/hash/exit status and installed
executable version/hash only. It explicitly records no human smoke. A successful
installer exit/version check does not prove first start, database health, content
activation, planning, settings or recovery. Afterward separately record actual
first start, imported profiles/settings, visible bundle revision/default reset,
representative source-backed plan and restart/persistence if the owner performs
them. Record actual Windows version, elevation prompts, resource use and defects;
do not label these steps passed from the launcher's output. Share only reviewed,
redacted observations, never private backups. No personal snapshot becomes source
admission or live-game verification.

Do not automatically downgrade the upgraded database or overwrite personal state
on failure. Preserve the backup and failing state separately; determine a supported
rollback/recovery path before restoring. A0.8.1 binary must not be pointed at an
upgraded database without a proven compatible restore. Keep the backup until the
owner has verified recovery and retention requirements.

## Verification of launcher preparation

`pwsh -NoProfile -File scripts/windows/Test-SupplementalUpgrade.ps1` exercises inert
temporary files and helper refusals, not an installer, elevation, actual Windows
ACLs, a private profile or the full supplemental workflow. Native Windows execution
and the0.8.1 upgrade remain unverified until actually performed.
