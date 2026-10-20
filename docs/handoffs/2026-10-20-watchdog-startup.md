# Synthetic watchdog startup regression repair

Owner desktop_review; baseline97a81de9fd8d07270ed86f55ca9bf445b010e60a;
claim0e77fa19; frozen TEST-ONLY code21eebc3e. Isolated worktree
/tmp/navzeroth-stable-watchdog-startup. Only Test-StableAcceptanceHelpers.ps1
changes, plus this claim/handoff; no production helper/driver, runtime, workflow,
20minute workload/60minute suite/170minute job, data, receipt or key changes.

Actual V14package39497088109 at PR source8d9fc3922e3433b45755ef68602a62c054faee4e
failed early PS7 helper checks: the real parent2s watchdog fired, then the test
could not read its synthetic child's progress file. No installer/native/eight-case
producer ran. Full log /tmp/navzeroth-v14-package-39497088109.log. Root retains
failure/source closure separately. Exact hosted child startup cause is UNPROVEN;
no Node stderr or child initialization checkpoint was retained.

Actual native Windows reproduction used portable OFFICIAL PowerShell7.6.7 and
repo-pinned WindowsNode24.15.0 outside Git, process-local PATH/ExecutionPolicy only.
The UNMODIFIED baseline test passed26checks locally; cold-start failure was NOT
reproduced. Local portable/UNC timing is not the hosted runner's exact environment.
A deterministic test now models legitimate startup before the first checkpoint:
child deliberately delays5s, former2s real parent must timeout with no progress or
finally marker and a nonpassing watchdog; a distinct report with finite15s TEST
allowance must reach the pending IPC, then actual parent kill retains progress,
nonpassing watchdog, exact timeout error and absent finally marker. No assertions
are skipped or weakened. This is startup allowance for a synthetic regression,
not a larger production budget or evidence that real recovery now completes.
Node version/path are printed to diagnose future actual runner differences.

Actual checks:
- Native Windows PowerShell7.6.7 +WindowsNode24.15.0: baseline26PASS; repaired26PASS
  including both real parent timeout controls.
- Linux PowerShell7.6.6 +actual default LinuxNode24.19.0: repaired26PASS.
- Native Windows PowerShell5.1:25PASS; PS7-only parent regression explicitly NOT
  executed under5.1 (argument-list API is unavailable there).
- gitdiffcheckPASS. Unchanged Node/runtime tests not redundantly rerun.
No installed application, network adapter or WoW operation was performed.

Official portable artifacts/checksum verification:
https://github.com/PowerShell/PowerShell/releases/download/v7.6.7/PowerShell-7.6.7-win-x64.zip
106364316B SHA256d2f952ebdbf24386cb8923553aa58e41a846db35a6155ea04c58142a4b16dbb7;
matched official hashes.sha256 (itself6c2d5f481cc20c83df45d5c29e15f0d7e9e1db82eb5b8f7f2fe33b107f308325).
https://nodejs.org/dist/v24.15.0/win-x64/node.exe
SHA2563331e1ffe19874215472217c5e94f5a0c6d8e18c4ac7111d3937aa0ad5e9b4a5,
matched official v24.15.0/SHASUMS256.txt. Extracted artifacts are under
/tmp/navzeroth-native-pwsh-7.6.7 and /tmp/navzeroth-native-node-24.15.0. Extracted
executables needed local u+x before WSL invocation; no persistent installation or
machine execution policy changes. Actual versions were executed and confirmed.

Native command wrapper /tmp/navzeroth-run-native-watchdog.ps1 prepends ONLY its
process PATH and invokes this worktree's test. Native PS7 executable
/tmp/navzeroth-native-pwsh-7.6.7/pwsh.exe -NoProfile -ExecutionPolicy Bypass -File
\\wsl.localhost\kali-linux\tmp\navzeroth-run-native-watchdog.ps1.
Linux command /tmp/navzeroth-pwsh/pwsh -NoProfile -File scripts/windows/Test-StableAcceptanceHelpers.ps1.
Logs /tmp/navzeroth-watchdog-startup-{native-before,native-after,linux-after,ps51-after}.log:
3bc35194d67ae46641dba9ea3e111854e40e721845993c7e5e6da779731bdead,
4899433194d65f3f5cca11337175bc0864f80d0de861d0eb14c7d710ebc807d1,
33d457017d390d2cd81ef8606e9981a06c08b97c7c10b93012d7771f619553fd,
fe45eb24ea998c71a30f47e514f75d8606c61963dda4bc7c0285282611ab99ad respectively.

Next independent review and routine draft PR, parent archival/normal integration
and renewed exact source bindings, then fresh actual hosted checks. V14 failure,
V13 recovery timeout and all missing installed cases remain unpromoted.
