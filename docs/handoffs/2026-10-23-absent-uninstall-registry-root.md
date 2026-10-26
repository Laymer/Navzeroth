# Absent uninstall registry root repair

Owner: desktop_review. Baseline dee9998a; claim4772ddbd preceded code.
Frozen code e9baa342230d0ef300081a7a28f9710c90416892; isolated branch
fix/absent-uninstall-registry-root, `/tmp/navzeroth-uninstall-root-guard`.

Hosted probe39916898751 compiled successfully (1128.766s), but fixture preflight
failed because HKCU's Uninstall root did not exist. Installation, marker issuance,
cleanup and recovery workload did not execute. Its native/transport regression
passes are distinct from the unexecuted recovery measurement.

Only Get-InstalledDisplayVersion changes: the existing HKCU scope is retained.
A typed ItemNotFoundException from the initial exact-root lookup means no
registration. Access/provider errors and all subsequent enumeration/property
failures still propagate, including a root disappearing after initial lookup.
An unrelated entry without DisplayName is ignored explicitly under StrictMode;
a matching entry must have a nonblank string DisplayVersion. Normal matching
version strings retain their exact value. No HKLM scan, timeout or guard waiver.

Actual focused regressions:12 PASS each under Linux PowerShell7.6.6, native
Windows PowerShell5.1 and portable native PowerShell7.6.7. Invoke existing
Test-NativeSmokeHelpers.ps1 with -RegistryOnly: scoped inert provider mocks cover
absent/present-empty roots, unrelated/normal/missing-name entries, invalid/missing
versions, lookup denial, existing-root enumeration/vanish and property failures.
The new switch returns before the original synthetic-process tests; no real
registry access/writes, application/process controls, installer or network actions
ran. Original baseline helper fails the new absent-root check (negative control).
Existing broader process tests were not rerun for this narrow registry change.

Exact author native commands used the existing portable/native executables:

- `powershell.exe -NoProfile -ExecutionPolicy Bypass -File "$(wslpath -w /tmp/navzeroth-uninstall-root-guard/scripts/windows/Test-NativeSmokeHelpers.ps1)" -RegistryOnly`
- `/tmp/navzeroth-native-pwsh-7.6.7/pwsh.exe -NoProfile -ExecutionPolicy Bypass -File "$(wslpath -w /tmp/navzeroth-uninstall-root-guard/scripts/windows/Test-NativeSmokeHelpers.ps1)" -RegistryOnly`

Both author native runs used process-only ExecutionPolicy Bypass; no persistent
policy changed. Initial nativePS7 UNC launch without that option refused script
signature before tests. Root independently cleared code and ran12 LinuxPS7 plus
12 nativePS7 tests on hash-verified WindowsTemp copies, without an override.
Root nativePS5 normal-File invocation was denied by script policy BEFORE tests;
it is not a pass. Tooling independently cleared code and ran12 nativePS5 tests.
These differing invocation scopes do not establish hosted installation acceptance.
git diff --check passes. Logs retained externally:

- /tmp/navzeroth-uninstall-registry-linux.log and -native5.log:
  SHA2563c72b1f115a7384906cd5c1ee37720a254201d66be62afc1646146cc3ae3794c
- /tmp/navzeroth-uninstall-registry-native7.log:
  SHA2561d2f0e6074d894014acbf84b9d1996fc665f779d5e27a76e7721aeefb11720a9
- /tmp/navzeroth-uninstall-registry-old-control.log:
  SHA2563dd33a73c142b3e2a26970eef68d8ae206eb262c6d635ac01fdad62b8baafe9a

Next: independent root/tooling review, then coordinator-owned probe integration
and hosted measurement. No dispatch, release/canonical change or recovery pass
is claimed. Local unpublished website branch ee659bb2 remains untouched.
