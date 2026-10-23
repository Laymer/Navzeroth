# Windows: build, install, update and remove

Task M9.4 (packaging automation) with the installation and removal instructions
of M9.5. Status: IMPLEMENTED_UNVERIFIED. The hosted Windows runner builds the
installer, installs it silently, starts the application and uninstalls it (see
the [RC publication receipt](../releases/1.0.0-rc.1-publication.json) and
the earlier [0.9.2 publication record](../handoffs/2026-10-17-v0.9.2-publication.md)
(earlier: [0.9.1](../handoffs/2026-10-16-v0.9.1-publication.md), [0.8.1 evidence](../handoffs/2026-10-14-final-program-audit.md)),
the [original handoff](../handoffs/2026-10-13-M9.4-windows-packaging.md) and
[PR #39 review](../handoffs/2026-10-13-PR39-review.md)). Interactive
installer pages, a non-runner Windows machine and the addon on a live Forever
client are not fully verified. The independently verified public download is identified by the
[installation guide](../site/installation.html) and
[versioned public metadata](../site/release.json); a prepared source version does
not establish publication. Historical RC and older receipts above keep their
original scope. The prepared 1.0.0 dataset target is Forever 1.60.1.70450 / interface 16001,
with historical 1.12.1/70245 origins retained, under
[ADR 0007](../decisions/0007-world-data-automation.md),
[ADR 0008](../decisions/0008-unsigned-stable-release.md) and
[ADR 0009](../decisions/0009-automated-draft-release-acceptance.md).
Live-addon admission remains experimental/unverified; no frozen launch build or
full live acceptance is implied. Current exact-installer verification is recorded
separately in the [v1 release gates](../releases/V1_RELEASE_PROCESS.md).

Release files (built by [`Build-Windows.ps1`](../../scripts/windows/Build-Windows.ps1)):

| File | Contents |
| --- | --- |
| `AzerothNavigator-<version>-x64-setup.exe` | Per-user NSIS installer: the application, a copy of the addon and its helper script |
| `AzerothNavigator-addon-<version>.zip` | The `AzerothNavigator` addon folder alone, for manual installation |
| `SHA256SUMS.txt` | SHA-256 of the installer, addon and included metadata |
| `SOURCE_COMMIT.txt` | Exact packaged source commit |
| `CANDIDATE_NOTES.md` | Version-specific notes when present |

The 1.0.0-rc.1 release has all five files. Its checksum manifest covers the
four payload files, not itself. A local `-AddonOnly` build omits the installer;
an untagged version without notes also omits `CANDIDATE_NOTES.md`. Neither is the
complete Windows release asset set. The prepared unsigned stable contract additionally requires the content/source
material archive, RELEASE_MATERIALS.json and RELEASE_ASSETS.json: eight release
assets in total, rather than the historical RC's five. It bundles verified
world/authored packs, notices, source materials and signed content receipts.
This describes required prepared scope, not completed publication or Windows
acceptance. Installer Authenticode status and content-receipt signatures are
separate checks. Dependency notices and retained sources are also verified against
the committed source.

## For users

### Install the application

1. Download the setup file, `SHA256SUMS.txt`, `SOURCE_COMMIT.txt` and notes from the
   linked project's release page. Check the version and evidence limitations.
2. Compare `Get-FileHash .\AzerothNavigator-1.0.0-rc.1-x64-setup.exe -Algorithm SHA256`
   with its line in `SHA256SUMS.txt` (substitute the chosen version). A matching
   checksum shows the download is intact; it does
   not prove who published it (plan §8.7). Download only from the project's release page.
3. Run the setup file. It installs for the current Windows user into
   `%LOCALAPPDATA%\Azeroth Navigator` and needs no administrator rights.
   - Until releases are code-signed, Windows SmartScreen may show "Windows protected
     your PC". Choose *More info* > *Run anyway* only for a file from the release page
     whose checksum matches.
   - The application uses Microsoft Edge WebView2. If the runtime is missing, this
     installer is configured to download Microsoft's bootstrapper, so that first
     installation requires network access. The local workflows are designed to run
     offline after setup; acceptance on actual Windows hardware remains required.
4. At the end the installer asks whether to install the World of Warcraft addon now
   (next section). You can say no and do it later.

### Install the addon

The addon lives in the game's `Interface\AddOns` folder. Where the Forever client keeps
that folder is **unverified**, so nothing guesses it: you choose it.

**With the helper** — Start menu > *Azeroth Navigator - WoW addon*, choose *Install*,
then select the game's `Interface\AddOns` folder (or the game folder that contains it).
The helper:

- refuses a folder that is not named `AddOns`, and never creates one;
- attempts to detect running processes under the conventional game folder and
  refuses when one is detected; this is not proof the client is closed;
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
Record the actual client build before capture. The 1.0.0-rc.1 adapters experimentally admit
1.60.1 builds 70338, 70374, 70425 and 70450, each with interface 16001.
70425 and 70450 remain development-only and UNVERIFIED; full acceptance for the
pending post-launch v1 target is not enabled by this installer.

### Update

Use **Backup and recovery > Create consistent backup** before an upgrade; see
[database recovery](DATABASE_RECOVERY.md). Keep that private backup outside both
the installation and application-data directories. Copying only the live SQLite
file is not a consistent backup.

Run the newer setup file over the installed one. Updates are designed to retain
application data; interactive upgrade acceptance remains unverified. If the helper recorded
an AddOns folder, the installer offers to update the addon there too, so the application
and the addon keep the same version. Older versions cannot be installed over newer ones;
uninstall first to go back. An older application may refuse a newer database schema;
retain a compatible pre-upgrade backup rather than treating executable downgrade as
a database migration. Silent installation keeps an existing game addon unchanged.

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
`-Action Uninstall -RemoveSavedVariables`, or by hand. That switch covers the named
`.lua` and `.lua.bak` files in every account under the selected game folder; use
`-WhatIf` to inspect the scope first. A silent uninstall retains application data
and the game addon, as checked by the hosted packaging smoke test. Recovery
originals are siblings of the live database inside application data, so deleting
application data can also remove those originals. Keep preservation copies outside
application-data directories before choosing deletion; external backups have their
own explicit retention decisions.

## For builders

Build on a native Windows checkout (plan §5.2). A WSL path such as
`\\wsl.localhost\...` is not supported: clone the repository again on a Windows drive.

```powershell
git clone --config core.autocrlf=false https://github.com/Laymer/Navzeroth
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
   runs `cargo test --locked --workspace` and the desktop unit tests (skip with `-SkipTests`).
4. Runs `tauri build --bundles nsis`; Tauri downloads its pinned NSIS on first use.
5. Packages the addon (sorted entries, commit timestamp), writes the exact commit
   to `SOURCE_COMMIT.txt`, copies version-specific notes when present, and writes
   `SHA256SUMS.txt`.
   Rebuilding the same checkout with the same .NET runtime gives the same zip; a
   checkout with other line endings (Git's `core.autocrlf` on Windows) gives a
   different one, because the addon files then differ.
6. Fails if the build changed a tracked file.

Other options: `-ExpectedVersion v0.8.1` (fail on mismatch), `-OutputDirectory`,
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

This hook is for local builds. The explicit 1.0.0 release policy permits unsigned
Windows artifacts, verified as NotSigned with their exact hashes/source. Other releases follow their committed
version-specific policy; optional SignPath configuration is documented in
[code signing](CODE_SIGNING.md), without claiming approval or signing availability.
Obtaining a certificate or a signing account is an owner decision (AGENTS.md).

## For maintainers: releases

The workflow [`package-windows.yml`](../../.github/workflows/package-windows.yml) runs on
pull requests targeting main, pushes to main, manual dispatch and version tags. Each
run tests the addon helper with Windows PowerShell 5.1 and PowerShell 7, builds with
Windows PowerShell 5.1, tests the first-start observation harness and installs silently.
First start requires a fresh, nonempty SQLite store within a bounded 120 s
observation, at least 20 s of process survival and 5 s with the store present.
Early process exit fails immediately. A preexisting store or journal fails without
deletion. Failures report process, WebView, app-data and Windows error-event
diagnostics before stopping the process. File presence does not prove completed
migrations or interactive readiness. The workflow then uninstalls, checks retained
user/addon data and uploads the release files as a workflow artifact (kept 14 days).

Tag runs also require version-specific release notes and draft the complete
policy-required asset set. The historical RC set had five files; stable 1.0.0 also
requires the three content/source material assets described above. Versions
beginning `0.` or containing `-` are marked prerelease. The tag-only **Draft GitHub release** job does not replace the ten
required PR/main checks.

For the authorized 1.0.0 program, follow the
[v1 release process](../releases/V1_RELEASE_PROCESS.md): exact admitted content and
source closure, all protected checks, stability, source-platform parity, independent
review and eight hosted installer observations remain mandatory. Schema 5 then
requires independently reviewed automated UI/native acceptance of the exact
independently downloaded draft installer before publication. The real UI must
activate the bundled content, create a source-backed engineering plan, and prove
restart/persistence; no human result is fabricated. Human smoke and supplemental
Windows 11 upgrade remain separate, unperformed observations. A historical five-file
package, a prepared build, or synthetic browser tests cannot satisfy these gates.

Publication is already authorized for this program once all mandatory gates pass.
It remains an explicit coordinator action; the workflow never publishes by itself.

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
