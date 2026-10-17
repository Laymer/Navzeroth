# Code signing

Task V0.4 of the 1.0.0 program ([ADR 0006](../decisions/0006-v1-program.md) §9).
Status: workflow and verifier implemented; **no signed build exists yet**. Signing
starts once the owner's SignPath Foundation application is approved and the
repository is configured (below). Until then every installer is unsigned, and a
stable `v1.0.0` tag fails the package job on purpose.

## Code signing policy

Free code signing provided by [SignPath.io](https://about.signpath.io/), certificate
by [SignPath Foundation](https://signpath.org/).

- Signed files: the Windows installer `AzerothNavigator-<version>-x64-setup.exe`,
  built from this repository by the `Windows package` workflow on a version tag.
  The application executable and uninstaller inside the installer aren't signed
  separately (see Limitations).
- Committers and reviewers: [the repository's collaborators](https://github.com/Laymer/Navzeroth/graphs/contributors).
  Changes reach `main` through pull requests that must pass the nine required
  checks; the owner reviews them (branch protection doesn't enforce reviews).
- Approvers: the project owner, [Laymer](https://github.com/Laymer), approves every
  release-signing request in SignPath by hand.
- Privacy: this program will not transfer any information to other networked
  systems unless specifically requested by the user or the person installing or
  operating it. The desktop application has no network client; its content
  security policy allows only local IPC and local map images. The in-game addon
  stays inside the game's addon environment. Exception: if Microsoft Edge WebView2
  is missing, the installer downloads Microsoft's WebView2 bootstrapper
  (`webviewInstallMode: downloadBootstrapper` in `tauri.windows.conf.json`).

## How the workflow signs

In [`package-windows.yml`](../../.github/workflows/package-windows.yml), after
`Build-Windows.ps1` writes the unsigned release files and before the silent-install
and native smoke tests (so those tests run on the signed file):

1. **Signing mode.** A version tag uses the `release-signing` policy when the
   `SIGNPATH_ORGANIZATION_ID` variable is set. If it isn't set, a 0.x or prerelease
   tag continues unsigned with a warning, and a stable tag fails. A manual run with
   `test_signing` uses the `test-signing` policy. Pull requests and main pushes don't sign.
2. The installer alone is uploaded as the artifact `unsigned-installer-<sha>`.
3. `signpath/github-action-submit-signing-request` (pinned v3.0) submits it and waits
   up to two hours for the owner's approval (15 minutes for test signing).
4. [`Complete-SignedInstaller.ps1`](../../scripts/windows/Complete-SignedInstaller.ps1)
   checks the returned file and then replaces the installer:
   - It must equal the unsigned build byte for byte, except the PE CheckSum and
     certificate-table entry, followed only by zero padding and one certificate
     table ([`Signing.ps1`](../../scripts/windows/Signing.ps1)). So the signing
     service can't have changed the program.
   - It must carry an Authenticode signature. For release signing, the chain must
     be `Valid` with a timestamp, and the signer subject must contain
     `SIGNPATH_EXPECTED_SUBJECT` when that variable is set.
   - Then `RELEASE_ASSETS.json` (when present) and `SHA256SUMS.txt` are regenerated,
     and `Assert-ReleaseArtifacts` re-verifies the exact asset set against source.

The draft-release job is unchanged; it drafts from the artifact that now holds the
signed installer.

## Application (prepared answers)

Researched 2026-10-17 from <https://signpath.org/terms>. The form at
<https://signpath.org/apply> could not be read by the agent; paste these where
they fit and adjust to the form's actual fields.

| Question | Answer |
| --- | --- |
| Project name | Azeroth Navigator (Navzeroth) |
| Repository | https://github.com/Laymer/Navzeroth (public) |
| Licence | BSD-3-Clause (OSI-approved), no commercial dual licence |
| Home page | https://laymer.github.io/Navzeroth/ (has the "Code signing policy" section) |
| Download page | https://github.com/Laymer/Navzeroth/releases (latest: 0.9.2 developer prerelease) |
| Description | Offline Windows desktop planner and in-game addon for World of Warcraft: Forever: imports addon snapshots, plans quest routes, compares gear, and records sessions. The app makes no network connections; the installer downloads Microsoft's WebView2 bootstrapper only if WebView2 is missing. No memory reading, injection or input automation. |
| What to sign | One NSIS installer per release, `AzerothNavigator-<version>-x64-setup.exe`, built by GitHub Actions (`.github/workflows/package-windows.yml`) on a version tag |
| Build system | GitHub Actions on GitHub-hosted `windows-2025` runners, from tagged source |
| Team | Laymer: author, reviewer and approver. All team members use MFA on GitHub and SignPath (**owner to confirm before applying**). |
| Privacy | The program transfers no information to other networked systems unless the user requests it |
| Uninstall | The installer registers an uninstaller (Apps & features); install and removal are documented in `docs/compatibility/WINDOWS_INSTALL.md` |

Open question for the owner before applying. The terms forbid "any proprietary,
non open-source component" in the signed software. Today's installer contains the
project's own BSD-3-Clause code and addon, OFL fonts, and third-party open-source
components with their notices shipped (Tauri and other Rust crates, CascLib (MIT),
zlib, wowdbdefs-rs, ed25519 and others; see `crates/local-data/THIRD_PARTY.md` and
`apps/desktop/THIRD_PARTY.md`). It also compiles in the WoWDBDefs table
`definitions` (`crates/local-data/build.rs`), which are data under **CC BY-SA 4.0**,
a free licence but not an OSI-approved one. Mention that in the application and
ask whether it's acceptable.

The same question applies with more force to 1.0.0: ADR 0006 plans content packs
from granted sources (ForeverDB, Questie, foreverchanges.pro, Olympus News
Network) whose terms are not OSI licences. Either keep such packs out of the signed
installer (deliver them as separate release materials or locally built content),
or get SignPath's answer before relying on signing for 1.0.0.

The terms also want the phrase "Code signing policy" on the download page. The
current download page is the 0.9.2 release, which doesn't link it yet: before
applying, add a line such as "Code signing policy:
https://laymer.github.io/Navzeroth/#code-signing" to the 0.9.2 release description
on GitHub (the description only; the release assets stay unchanged).

After acceptance, also set on SignPath's side what its terms require: file metadata
restrictions with product name "Azeroth Navigator", and manual approval for every
release-signing request. Each stable release's notes should link the code signing
policy, because the terms ask for it on the download page as well.

Verified for these answers (2026-10-17, independent review of PR #173): the app's
CSP and capabilities allow no remote origins and the Windows dependency tree has no
HTTP client; the addon never calls its addon-message APIs; BSD-3-Clause licences;
the per-user uninstaller and the removal documentation; the Pages URL.

## Owner setup (one time)

Only the owner can do steps 1 to 3: they need your account, identity and acceptance
of SignPath's terms.

1. **Apply** at <https://signpath.org/> (SignPath Foundation, free for open source).
   Use the public repository URL, the BSD-3-Clause licence, the release page
   <https://github.com/Laymer/Navzeroth/releases> and this page as the code signing
   policy. Use multi-factor authentication on your GitHub and SignPath accounts.
2. After approval, in SignPath: create the project (slug `Navzeroth`, or set
   `SIGNPATH_PROJECT_SLUG`), link the GitHub repository as a trusted build system,
   and keep the two policies `test-signing` and `release-signing`. Release signing
   requires manual approval by you; restrict it to version tags (`refs/tags/v*`).
3. Add an artifact configuration (the default one, or set
   `SIGNPATH_ARTIFACT_CONFIGURATION_SLUG`):

   ```xml
   <artifact-configuration xmlns="http://signpath.io/artifact-configuration/v1">
     <zip-file>
       <pe-file path="AzerothNavigator-*-x64-setup.exe" max-matches="1">
         <authenticode-sign />
       </pe-file>
     </zip-file>
   </artifact-configuration>
   ```

4. Create a SignPath API token for a CI user that may submit to the project. Then,
   from this repository's checkout:

   ```sh
   gh secret set SIGNPATH_API_TOKEN            # paste the token when prompted
   gh variable set SIGNPATH_ORGANIZATION_ID --body '<organization id>'
   gh variable set SIGNPATH_EXPECTED_SUBJECT --body 'SignPath Foundation'
   ```

   Steps 4 and later can be done by an agent if you hand over the values, but the
   token is a secret: setting it yourself keeps it out of any transcript.
5. **Test** with a manual run: Actions → Windows package → Run workflow on `main`
   with `test_signing` ticked. The test certificate is untrusted, so only the
   signature's presence and the byte check are required. Record the run in a handoff.
6. The next version tag is then signed through `release-signing`; approve the
   request in SignPath while the job waits. Record SmartScreen behaviour of the
   signed installer on a clean Windows machine (it feeds `WIN-INSTALL`).

## Limitations

- Only the outer installer is signed. Signing the application executable and the
  NSIS uninstaller would need a two-stage build (sign the app, then bundle and sign
  the installer); that isn't implemented.
- The publisher shown by Windows is "SignPath Foundation", not the project owner.
  SmartScreen reputation still builds up per certificate and download volume, so a
  signed installer can warn for a while.
- SignPath Foundation reviews applications and may refuse or withdraw signing. In
  that case set no variables: prereleases stay unsigned, and ADR 0006 §9 needs a new
  decision before a stable tag can pass.
- Future content packs carry their sources' terms, not BSD-3-Clause. If SignPath
  asks, explain that the signed installer contains the project's own open-source
  code and declared third-party notices.
- Local manual signing through `NAVZEROTH_SIGN_COMMAND` still works for builds on
  your own machine ([install guide](WINDOWS_INSTALL.md#signing)).
