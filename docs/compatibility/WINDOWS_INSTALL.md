# Windows: build, install, update and remove

Task M9.4 (packaging automation) with the installation and removal instructions
of M9.5. Status: IMPLEMENTED_UNVERIFIED. The hosted Windows runner builds the
installer, installs it silently, starts the application and uninstalls it (see
the [original handoff](../handoffs/2026-10-13-M9.4-windows-packaging.md) and
[PR #39 review](../handoffs/2026-10-13-PR39-review.md)). Interactive
installer pages, a non-runner Windows machine and the addon on a live Forever
client are not verified.

Release files (built by [`Build-Windows.ps1`](../../scripts/windows/Build-Windows.ps1)):

| File | Contents |
| --- | --- |
| `AzerothNavigator-<version>-x64-setup.exe` | Per-user NSIS installer: the application, a copy of the addon and its helper script |
| `AzerothNavigator-addon-<version>.zip` | The `AzerothNavigator` addon folder alone, for manual installation |
| `SHA256SUMS.txt` | SHA-256 of both files |

## For users

### Install the application

1. Download the setup file and `SHA256SUMS.txt` from the project's GitHub release page.
2. Optionally compare `Get-FileHash .\AzerothNavigator-<version>-x64-setup.exe` with the
   line in `SHA256SUMS.txt`. A matching checksum shows the download is intact; it does
   not prove who published it (plan §8.7). Download only from the project's release page.
3. Run the setup file. It installs for the current Windows user into
   `%LOCALAPPDATA%\Azeroth Navigator` and needs no administrator rights.
   - Until releases are code-signed, Windows SmartScreen may show "Windows protected
     your PC". Choose *More info* > *Run anyway* only for a file from the release page
     whose checksum matches.
   - The application uses Microsoft Edge WebView2. Windows 11 includes it. If it is
     missing, the installer downloads Microsoft's WebView2 bootstrapper once; after that
     the application works offline.
4. At the end the installer asks whether to install the World of Warcraft addon now
   (next section). You can say no and do it later.

### Install the addon

The addon lives in the game's `Interface\AddOns` folder. Where the Forever client keeps
that folder is **unverified**, so nothing guesses it: you choose it.

**With the helper** — Start menu > *Azeroth Navigator - WoW addon*, choose *Install*,
then select the game's `Interface\AddOns` folder (or the game folder that contains it).
The helper:

- refuses a folder that is not named `AddOns`, and never creates one;
- refuses while a program from that game folder is running;
- writes only `AddOns\AzerothNavigator`, and replaces it only if its TOC title is
  `Azeroth Navigator Diagnostics` exactly; a different addon with that folder name is left untouched;
- copies to a temporary sibling folder, checks every file (SHA-256), then swaps it in,
  restoring the previous version if the swap fails (a later cleanup or location-recording
  error can occur after the new version is installed);
- remembers the AddOns folder in
  `%LOCALAPPDATA%\io.github.laymer.azeroth-navigator\addon-location.txt` so updates and
  the uninstaller can find it.

From a terminal (same script; `-WhatIf` shows what would change):

```powershell
$helper = "$env:LOCALAPPDATA\Azeroth Navigator\addon\Install-AzerothNavigatorAddon.ps1"
powershell -ExecutionPolicy Bypass -File $helper -Action Install -AddOnsPath 'D:\Games\WoW Forever\Interface\AddOns'
powershell -ExecutionPolicy Bypass -File $helper -Action Status
powershell -ExecutionPolicy Bypass -File $helper -Action Uninstall -RemoveSavedVariables -WhatIf
```

**Manually** — extract `AzerothNavigator-addon-<version>.zip` into the AddOns folder so
that `AddOns\AzerothNavigator\AzerothNavigator.toc` exists, as described in
[ADDON_INSTALL.md](ADDON_INSTALL.md).

Then start the game and enable *Azeroth Navigator Diagnostics* in the AddOns list.

### Update

Run the newer setup file over the installed one. Your data stays. If the helper recorded
an AddOns folder, the installer offers to update the addon there too, so the application
and the addon keep the same version. Older versions cannot be installed over newer ones;
uninstall first to go back.

### Where things are

| What | Where |
| --- | --- |
| Application, addon copy, helper, uninstaller | `%LOCALAPPDATA%\Azeroth Navigator\` |
| Your data (snapshot store `snapshots.sqlite3`) | `%APPDATA%\io.github.laymer.azeroth-navigator\` |
| WebView2 cache and the recorded AddOns folder | `%LOCALAPPDATA%\io.github.laymer.azeroth-navigator\` |
| Start menu entries | *Azeroth Navigator*, *Azeroth Navigator - WoW addon* |
| Apps & features entry | current user (`HKCU\...\Uninstall`) |
| The addon (only if you installed it) | `<game>\Interface\AddOns\AzerothNavigator\` |

### Uninstall

Settings > Apps > *Azeroth Navigator* > Uninstall. The uninstaller:

- asks whether to remove the addon from the recorded AddOns folder (only the
  `AzerothNavigator` folder; other addons, SavedVariables and game settings stay);
- removes your data only if you tick *Delete the application data*;
- removes its program files, shortcuts and Apps & features entry.

The addon's SavedVariables (`WTF\Account\<account>\SavedVariables\AzerothNavigator.lua`,
conventional layout, unverified for Forever) are removed only by the helper with
`-Action Uninstall -RemoveSavedVariables`, or by hand.

## For builders

Build on a native Windows checkout (plan §5.2). A WSL path such as
`\\wsl.localhost\...` is not supported: clone the repository again on a Windows drive.

```powershell
git clone https://github.com/Laymer/Navzeroth
cd Navzeroth
powershell -ExecutionPolicy Bypass -File scripts\windows\Build-Windows.ps1 -InstallPrerequisites
```

The files land in `dist\windows\` (ignored by git). The script:

1. Checks that the version is the same in `Cargo.toml` (`[workspace.package]`),
   `tauri.conf.json`, both `package.json` files, the addon TOC and `Core/Version.lua`.
2. Checks the toolchain pinned by the repository and, with `-InstallPrerequisites`,
   installs what is missing with winget (accepting the package agreements):

   | Prerequisite | Pinned by | Installed as |
   | --- | --- | --- |
   | Visual Studio C++ build tools | — | `Microsoft.VisualStudio.2022.BuildTools` with the VCTools workload |
   | Rust (x86_64-pc-windows-msvc) | `rust-toolchain.toml` | `Rustlang.Rustup`, then the pinned toolchain |
   | Node.js | `.node-version` | `OpenJS.NodeJS --version <pinned>` |
   | Corepack and pnpm | CI's Corepack version, `packageManager` | isolated Corepack under `%LOCALAPPDATA%\navzeroth-build` |

3. Installs locked dependencies (`pnpm install --frozen-lockfile`, `cargo fetch --locked`),
   runs `cargo test --workspace` and the desktop unit tests (skip with `-SkipTests`).
4. Runs `tauri build --bundles nsis`; Tauri downloads its pinned NSIS on first use.
5. Packages the addon (sorted entries, commit timestamp) and writes `SHA256SUMS.txt`.
   Rebuilding the same checkout with the same .NET runtime gives the same zip; a
   checkout with other line endings (Git's `core.autocrlf` on Windows) gives a
   different one, because the addon files then differ.
6. Fails if the build changed a tracked file.

Other options: `-ExpectedVersion v0.2.0` (fail on mismatch), `-OutputDirectory`,
`-AllowDirty`, and `-AddonOnly` (addon package only; also runs with PowerShell 7 on
Linux or macOS). `scripts\windows\Test-AddonInstaller.ps1` checks the helper against a
synthetic game folder.

### Signing

Without signing, the installer works but SmartScreen warns. To sign, set
`NAVZEROTH_SIGN_COMMAND` to a signing command; Tauri runs it for the application and the
installer. Tauri splits the value on single spaces and replaces the word `%1` with the
file to sign, so `%1` must be a separate, unquoted word and no part may contain a space
(put the signing tool on `PATH`). Example with a certificate in the Windows store, from
a Developer PowerShell where `signtool` is on `PATH`:

```powershell
$env:NAVZEROTH_SIGN_COMMAND = 'signtool sign /fd sha256 /tr http://timestamp.digicert.com /td sha256 /sha1 <thumbprint> %1'
```

In GitHub Actions, store the same value as the repository secret
`NAVZEROTH_SIGN_COMMAND` (plus whatever credentials the signing tool reads). Obtaining a
certificate or a cloud signing account is an owner decision (AGENTS.md).

## For maintainers: releases

The workflow [`package-windows.yml`](../../.github/workflows/package-windows.yml) runs on
pull requests that touch packaging inputs, on manual dispatch and on version tags. Each
run tests the addon helper with Windows PowerShell 5.1 and PowerShell 7, builds with
Windows PowerShell 5.1, installs silently, starts the application for 20 s, uninstalls,
and uploads the release files as a workflow artifact (kept 14 days).

To prepare a release:

1. Set the new version in the six files listed above.
2. Write `docs/releases/<version>.md` (plan M9.5: limitations and verification evidence);
   a tag without it fails.
3. Merge to main through the usual pull request.
4. Tag the merged commit: `git tag v<version>` and `git push origin v<version>`.
5. The workflow creates a **draft** GitHub release with the three files (versions `0.x`
   and `-pre` versions are marked pre-release). Review it, then publish it by hand.
   Publishing is the owner's decision (plan §20.2); the workflow never publishes.

## Decisions

- **NSIS, per user.** No administrator rights, and uninstall hooks for the addon. MSI
  would need WiX and a per-machine install.
- **The addon ships twice.** Inside the installer (with the helper, for one download)
  and as a zip (for manual installation). Both come from the same folder at the same
  commit.
- **No game-folder guessing and no automatic addon install.** The Forever AddOns
  location is unverified; the user picks it once, and silent installs never touch it.
- **No auto-updater.** An updater needs a publisher trust configuration and signed
  update metadata (plan §8.7); executable releases stay explicit, user-run installers.
- **Draft releases only.** Building and drafting are automated; publishing is not.
