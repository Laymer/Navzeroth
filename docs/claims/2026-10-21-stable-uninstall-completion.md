# Stable uninstall completion before the RC1 baseline install

Owner: root. Baseline 8b12aec1bb28bdb35a3014c365ec500da8876af0. Sole writer in /tmp/navzeroth-stable-uninstall-completion, branch fix/stable-uninstall-completion. Scope: Native-Smoke-Helpers.ps1 (shared wait helper, installer error detail), Stable-Acceptance.ps1 (Remove-OwnedInstallation), Test-NativeSmokeHelpers.ps1 and a focused handoff.

V17 hosted HOSTED-UPGRADE-RC1 failed with the RC1 installer exiting 2 about one second after the 1.0.0 uninstall. The NSIS uninstaller relaunches itself and removes its Uninstall registry entry after the files, so waiting only for the executable lets a silent installer see 1.0.0 and refuse the downgrade. No runtime, content, canonical metadata or key edits; no limit or acceptance requirement is relaxed.
