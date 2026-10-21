# V17 candidate result: RC1 upgrade failed

Candidate 8b12aec1 (the signed V17 closure) ran in GitHub run 39603870404 (Windows package). Every other workflow passed: CI (Stability skipped, not passed), Coverage, Independent source knowledge and Pages. The Windows package job failed at step 55, which correctly refused draft readiness.

Hosted acceptance (installer SHA256 837a04f8089d5a45088af217cb3c7c1f42b3d79ccfecf7ddd256d1c1c372d06d):

- Passed: HOSTED-INSTALL, HOSTED-RESOURCES, HOSTED-FIRST-START, HOSTED-PERFORMANCE, HOSTED-PERSISTENCE-RECOVERY and HOSTED-OFFLINE. The offline row, which failed in V16, passed with the repaired adapter handling. Persistence and recovery finished well inside the 1200 s bound (about 775 s of workload time in this run; V16 had a 12 s margin).
- Failed: HOSTED-UPGRADE-RC1. Error: `Installer ...\stable-rc1\AzerothNavigator-1.0.0-rc.1-x64-setup.exe exit code 2`, recorded about one second after the case began.
- Not run: HOSTED-DOWNGRADE-REFUSAL.
- Owner smoke: unverified, as always at this stage. The nine canonical observation rows stay unverified.

Diagnosis (inferred from the code and the timing, not observed directly): Remove-OwnedInstallation waited only for azeroth-desktop.exe to disappear. The NSIS uninstaller relaunches itself from TEMP and removes its Uninstall registry entry after the files, so the silent RC1 installer started while 1.0.0 was still registered and refused the downgrade. Exit code 2 is the same code the downgrade smoke records. The repair is in PR 268 (Wait-NativeSmokeUninstalled). It is not yet verified on a hosted runner.

Checked and ruled out as further upgrade risks: RC1 and this candidate ship identical resource paths, so no RC1-only files remain to break the exact installed-tree inventory, and the app settings type is unchanged since v1.0.0-rc.1.

Archived under release/history/2026-10-21-*-v17-* (signed bytes, cases, raw reports, recorded overlay, workflow, logs). Rust raw coverage profiles were not archived because of disk space; the run artifacts stay on GitHub until they expire. The case source_commit is the pull request merge ref 7361c873, while the candidate is the PR head 8b12aec1.

Next: independent review and merge of PR 268, then V18 (bind preparation, independent content review, unsigned preparation, signing with the owner's external keys, independent closure checks) and one candidate push. No main merge, tag or publication.
