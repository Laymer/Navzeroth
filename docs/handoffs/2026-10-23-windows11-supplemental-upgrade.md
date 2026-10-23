# Supplemental Windows11 launcher preparation

Owner world_review; baseline9d30d716; claimd95e3c2b. Separate single-writer branch
`test/windows11-supplemental-upgrade` and worktree. Only new launcher/test/runbook
and this task's claim/handoff are changed. No existing hosted guard or release
acceptance evidence changes.

The launcher requires explicit independently selected exact draft source/hash and
downloaded source marker, unsigned1.0.0 installer and installed0.8.1; it refuses
running apps. Separate Backup/Upgrade invocations preserve default installation,
roaming database/local-data and local addon/WebView state with exact inventories.
Private copies are outside Git with current-user-only inherited ACLs; no completed
manifest is written until copying and before/after hash checks finish. Upgrade
rechecks original and backup identities and invokes normal interactive RunAs/UAC.
It never approves elevation, launches the app, changes policy/network controls or
claims smoke. Runbook distinguishes default repository paths from uninspected
personal paths and defers execution to a concrete independently verified draft.

Actual self-verification: Linux PowerShell7 inert helper tests9PASS; source parsing
included by dot source. First run exposed DirectoryInfo.Parent losing PowerShell's
PSIsContainer extension; corrected to the actual .NET DirectoryInfo type and reran
successfully. No real installer, Windows ACL/elevation, local private state or
network adapter operation was executed. Full orchestration remains unverified.

Next: independent review of this small opt-in preparation, then supply verified
concrete draft identities and ask the owner for ordinary elevation when executing.
Unavailable supplemental checks do not block schema5; actual relevant defects do.
