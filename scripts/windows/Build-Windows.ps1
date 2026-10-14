<#
.SYNOPSIS
    Builds the Azeroth Navigator Windows installer and addon package from source.

.DESCRIPTION
    Task M9.4. One command from a clean checkout to release files:

      dist\windows\AzerothNavigator-<version>-x64-setup.exe   NSIS installer, per-user, no admin
      dist\windows\AzerothNavigator-addon-<version>.zip       addon folder for manual installation
      dist\windows\SHA256SUMS.txt                             checksums (integrity only, not authenticity)

    Steps: check (or, with -InstallPrerequisites, install) the pinned toolchain;
    check that every version number agrees; install locked dependencies; optionally
    run the tests; build the installer with the Tauri CLI; package the addon; write
    checksums; confirm the build left no tracked file changed.

    The pinned versions come from the repository: rust-toolchain.toml, .node-version
    and the packageManager field of package.json. Corepack is pinned below to the
    same version as .github/workflows/ci.yml.

    Set NAVZEROTH_SIGN_COMMAND to a signing command whose separate word %1 stands
    for the file to sign (Tauri splits it on spaces) to sign the application and
    installer; otherwise they are unsigned and Windows SmartScreen warns.

    -AddonOnly packages the addon and checksums without building the application;
    it also runs on Linux and macOS (PowerShell 7) for checks.

.PARAMETER InstallPrerequisites
    Install missing prerequisites with winget: Visual Studio Build Tools (C++
    workload), rustup and the pinned Node.js. Accepts the package agreements.

.PARAMETER SkipTests
    Skip cargo test and the desktop unit tests (CI runs them separately).

.PARAMETER ExpectedVersion
    Fail unless the source version equals this (a leading "v" is ignored), e.g. the tag.

.PARAMETER OutputDirectory
    Where the release files go. Default: dist\windows under the repository root.

.PARAMETER AllowDirty
    Do not fail when tracked files differ from HEAD before or after the build.

.PARAMETER AddonOnly
    Package only the addon (no toolchain, no application build).

.EXAMPLE
    powershell -ExecutionPolicy Bypass -File scripts\windows\Build-Windows.ps1 -InstallPrerequisites
#>
[CmdletBinding()]
param(
    [switch] $InstallPrerequisites,
    [switch] $SkipTests,
    [string] $ExpectedVersion,
    [string] $OutputDirectory,
    [switch] $AllowDirty,
    [switch] $AddonOnly
)

Set-StrictMode -Version 3.0
$ErrorActionPreference = 'Stop'

# Keep in sync with the "Install isolated Corepack" steps in .github/workflows/ci.yml.
$CorepackVersion = '0.36.0'
$AddonName = 'AzerothNavigator'
$OnWindows = [Environment]::OSVersion.Platform -eq [PlatformID]::Win32NT

$RepoRoot = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot (Join-Path '..' '..')))
if (-not $OutputDirectory) { $OutputDirectory = Join-Path (Join-Path $RepoRoot 'dist') 'windows' }
$OutputDirectory = [IO.Path]::GetFullPath($OutputDirectory)

function Write-Step([string] $Message) { Write-Host "==> $Message" -ForegroundColor Cyan }

function Invoke-Native {
    # Runs a native command; fails on a nonzero exit code. Native stderr (cargo
    # progress, warnings) is not an error, also on Windows PowerShell 5.1. Output
    # goes to the console, never into the pipeline: a function that calls this
    # returns only the values it means to return.
    param([Parameter(Mandatory)] [string] $Command, [string[]] $Arguments = @())
    $saved = $ErrorActionPreference
    $ErrorActionPreference = 'Continue'
    try { & $Command @Arguments | Out-Host }
    finally { $ErrorActionPreference = $saved }
    if ($LASTEXITCODE -ne 0) { throw "$Command $($Arguments -join ' ') failed with exit code $LASTEXITCODE" }
}

function Get-NativeOutput([string] $Command, [string[]] $Arguments = @()) {
    $saved = $ErrorActionPreference
    $ErrorActionPreference = 'Continue'
    try { $output = & $Command @Arguments 2>$null }
    finally { $ErrorActionPreference = $saved }
    if ($LASTEXITCODE -ne 0) { return $null }
    return ($output | Out-String).Trim()
}

function Test-Command([string] $Name) { return [bool](Get-Command $Name -ErrorAction SilentlyContinue) }

function Add-ToPath([string] $Directory) {
    if ((Test-Path -LiteralPath $Directory) -and -not (($env:PATH -split [IO.Path]::PathSeparator) -contains $Directory)) {
        $env:PATH = $Directory + [IO.Path]::PathSeparator + $env:PATH
    }
}

function Read-Text([string] $RelativePath) {
    return [IO.File]::ReadAllText((Join-Path $RepoRoot $RelativePath))
}

# ---------------------------------------------------------------------------
# Versions
# ---------------------------------------------------------------------------

function Get-PinnedVersions {
    $toolchain = Read-Text 'rust-toolchain.toml'
    if ($toolchain -notmatch '(?m)^channel\s*=\s*"([^"]+)"') { throw 'No channel in rust-toolchain.toml' }
    $rust = $Matches[1]
    $node = (Read-Text '.node-version').Trim()
    $manager = (Read-Text 'package.json' | ConvertFrom-Json).packageManager
    if ($manager -notmatch '^pnpm@([0-9][^+]*)') { throw "Unexpected packageManager: $manager" }
    return [pscustomobject]@{ Rust = $rust; Node = $node; Pnpm = $Matches[1] }
}

function Get-SourceVersion {
    $found = [ordered]@{}
    $cargo = Read-Text 'Cargo.toml'
    if ($cargo -match '(?ms)^\[workspace\.package\][^\[]*?^version\s*=\s*"([^"]+)"') { $found['Cargo.toml [workspace.package]'] = $Matches[1] }
    $found['apps/desktop/src-tauri/tauri.conf.json'] = (Read-Text 'apps/desktop/src-tauri/tauri.conf.json' | ConvertFrom-Json).version
    $found['package.json'] = (Read-Text 'package.json' | ConvertFrom-Json).version
    $found['apps/desktop/package.json'] = (Read-Text 'apps/desktop/package.json' | ConvertFrom-Json).version
    $toc = Read-Text "addons/$AddonName/$AddonName.toc"
    if ($toc -match '(?m)^##\s*Version:\s*(\S+)') { $found["addons/$AddonName/$AddonName.toc"] = $Matches[1] }

    $lua = Read-Text "addons/$AddonName/Core/Version.lua"
    if ($lua -match '(?m)^\s*addon\s*=\s*"([^"]+)"') { $found["addons/$AddonName/Core/Version.lua"] = $Matches[1] }

    $distinct = @($found.Values | Sort-Object -Unique)
    if ($found.Count -ne 6 -or $distinct.Count -ne 1) {
        $report = ($found.GetEnumerator() | ForEach-Object { "  $($_.Key): $($_.Value)" }) -join [Environment]::NewLine
        throw "Version numbers disagree or are missing (expected 6 equal values):$([Environment]::NewLine)$report"
    }
    $version = $distinct[0]
    if ($ExpectedVersion -and $ExpectedVersion.TrimStart('v') -ne $version) {
        throw "Source version $version does not match the expected version $ExpectedVersion."
    }
    return $version
}

# ---------------------------------------------------------------------------
# Prerequisites (Windows)
# ---------------------------------------------------------------------------

function Install-WithWinget([string] $Id, [string[]] $Extra = @()) {
    if (-not (Test-Command 'winget')) {
        throw "winget is not available; install $Id manually (see docs/compatibility/WINDOWS_INSTALL.md)."
    }
    Write-Step "winget install $Id $($Extra -join ' ')"
    Invoke-Native 'winget' (@('install', '--id', $Id, '--exact', '--silent', '--accept-package-agreements', '--accept-source-agreements') + $Extra)
}

function Assert-MsvcBuildTools {
    $vswhere = Join-Path ${env:ProgramFiles(x86)} 'Microsoft Visual Studio\Installer\vswhere.exe'
    $query = @('-products', '*', '-latest', '-requires', 'Microsoft.VisualStudio.Component.VC.Tools.x86.x64', '-property', 'installationPath')
    $found = $null
    if (Test-Path -LiteralPath $vswhere) { $found = Get-NativeOutput $vswhere $query }
    if (-not $found) {
        if (-not $InstallPrerequisites) { throw 'Visual Studio C++ build tools not found. Re-run with -InstallPrerequisites or install the "Desktop development with C++" workload.' }
        Install-WithWinget 'Microsoft.VisualStudio.2022.BuildTools' @('--override', '--wait --passive --add Microsoft.VisualStudio.Workload.VCTools --includeRecommended')
        $found = Get-NativeOutput $vswhere $query
        if (-not $found) { throw 'The C++ build tools are still missing after installation.' }
    }
    Write-Step "C++ build tools: $found"
}

function Assert-Rust([string] $Channel) {
    Add-ToPath (Join-Path $env:USERPROFILE '.cargo\bin')
    if (-not (Test-Command 'rustup')) {
        if (-not $InstallPrerequisites) { throw 'rustup not found. Re-run with -InstallPrerequisites or install it from https://rustup.rs.' }
        Install-WithWinget 'Rustlang.Rustup'
        Add-ToPath (Join-Path $env:USERPROFILE '.cargo\bin')
    }
    Push-Location $RepoRoot
    try {
        # Installs the toolchain pinned by rust-toolchain.toml when missing.
        Invoke-Native 'rustup' @('show')
        $rustInfo = Get-NativeOutput 'rustc' @('-vV')
    }
    finally { Pop-Location }
    if ($rustInfo -notmatch "(?m)^release:\s*$([regex]::Escape($Channel))") { throw "rustc is not the pinned $Channel`:$([Environment]::NewLine)$rustInfo" }
    if ($rustInfo -notmatch '(?m)^host:\s*x86_64-pc-windows-msvc') { throw "The Rust host must be x86_64-pc-windows-msvc (rustup set default-host x86_64-pc-windows-msvc).$([Environment]::NewLine)$rustInfo" }
    Write-Step "Rust $Channel (x86_64-pc-windows-msvc)"
}

function Assert-Node([string] $Version) {
    $current = if (Test-Command 'node') { Get-NativeOutput 'node' @('--version') } else { $null }
    if ($current -ne "v$Version") {
        if (-not $InstallPrerequisites) { throw "Node.js $Version is required (found: $current). Re-run with -InstallPrerequisites or use a version manager." }
        Install-WithWinget 'OpenJS.NodeJS' @('--version', $Version)
        Add-ToPath (Join-Path $env:ProgramFiles 'nodejs')
        $current = Get-NativeOutput 'node' @('--version')
        if ($current -ne "v$Version") { throw "Node.js $Version is still not first on PATH (found: $current). Open a new terminal and retry." }
    }
    Write-Step "Node.js $current"
}

function Assert-Pnpm([string] $Version) {
    # Same arrangement as CI: an isolated Corepack whose pnpm shim honours packageManager.
    $current = if (Test-Command 'corepack') { Get-NativeOutput 'corepack' @('--version') } else { $null }
    if ($current -ne $CorepackVersion) {
        $base = if ($env:LOCALAPPDATA) { $env:LOCALAPPDATA } else { [IO.Path]::GetTempPath() }
        $prefix = Join-Path $base "navzeroth-build\corepack-$CorepackVersion"
        $bin = Join-Path $prefix 'node_modules\.bin'
        if (-not (Test-Path -LiteralPath (Join-Path $bin 'corepack.cmd'))) {
            Write-Step "Installing Corepack $CorepackVersion in $prefix"
            Invoke-Native 'npm' @('install', '--prefix', $prefix, '--no-package-lock', '--no-audit', '--no-fund', "corepack@$CorepackVersion")
        }
        Add-ToPath $bin
    }
    $env:COREPACK_ENABLE_DOWNLOAD_PROMPT = '0'
    Push-Location $RepoRoot
    try { $pnpm = Get-NativeOutput 'pnpm' @('--version') }
    finally { Pop-Location }
    if ($pnpm -ne $Version) { throw "pnpm $Version is required (found: $pnpm)." }
    Write-Step "pnpm $pnpm (Corepack $CorepackVersion)"
}

# ---------------------------------------------------------------------------
# Packaging
# ---------------------------------------------------------------------------

function Get-SourceDateEpoch {
    if ($env:SOURCE_DATE_EPOCH) { return [long] $env:SOURCE_DATE_EPOCH }
    $epoch = Get-NativeOutput 'git' @('-C', $RepoRoot, 'log', '-1', '--format=%ct')
    if ($epoch) { return [long] $epoch }
    return [long] 315532800 # 1980-01-01, the earliest ZIP timestamp
}

function New-AddonZip([string] $Version) {
    $source = Join-Path (Join-Path $RepoRoot 'addons') $AddonName
    $zipPath = Join-Path $OutputDirectory "$AddonName-addon-$Version.zip"
    Add-Type -AssemblyName System.IO.Compression
    # Same bytes for the same commit and runtime: sorted entries, '/' separators,
    # one timestamp (the commit time), no directory entries, no OS metadata.
    $stamp = [DateTimeOffset]::FromUnixTimeSeconds((Get-SourceDateEpoch))
    $prefixLength = $source.Length + 1
    $byEntry = @{}
    foreach ($file in Get-ChildItem -LiteralPath $source -Recurse -File -Force) {
        $byEntry["$AddonName/" + $file.FullName.Substring($prefixLength).Replace('\', '/')] = $file.FullName
    }
    [string[]] $entries = @($byEntry.Keys)
    [Array]::Sort($entries, [StringComparer]::Ordinal)

    if (Test-Path -LiteralPath $zipPath) { Remove-Item -LiteralPath $zipPath }
    $stream = [IO.File]::Open($zipPath, [IO.FileMode]::CreateNew)
    try {
        $archive = New-Object IO.Compression.ZipArchive($stream, [IO.Compression.ZipArchiveMode]::Create)
        try {
            foreach ($name in $entries) {
                $entry = $archive.CreateEntry($name, [IO.Compression.CompressionLevel]::Optimal)
                $entry.LastWriteTime = $stamp
                $out = $entry.Open()
                try {
                    $in = [IO.File]::OpenRead($byEntry[$name])
                    try { $in.CopyTo($out) } finally { $in.Dispose() }
                }
                finally { $out.Dispose() }
            }
        }
        finally { $archive.Dispose() }
    }
    finally { $stream.Dispose() }
    Write-Step "Addon package: $zipPath ($($entries.Count) files)"
    return $zipPath
}

function Write-Checksums([string[]] $Paths) {
    foreach ($path in $Paths) {
        if (-not $path -or -not (Test-Path -LiteralPath $path -PathType Leaf)) { throw "Not a release file: '$path'" }
    }
    $lines = foreach ($path in ($Paths | Sort-Object)) {
        $hash = (Get-FileHash -LiteralPath $path -Algorithm SHA256).Hash.ToLowerInvariant()
        "$hash  $(Split-Path -Leaf $path)"
    }
    $sums = Join-Path $OutputDirectory 'SHA256SUMS.txt'
    # LF line endings and no BOM so `sha256sum -c` accepts the file.
    [IO.File]::WriteAllText($sums, (($lines -join "`n") + "`n"), (New-Object Text.UTF8Encoding($false)))
    Write-Step "Checksums: $sums"
    $lines | ForEach-Object { Write-Host "    $_" }
}

function Assert-CleanTree([string] $When) {
    if ($AllowDirty) { return }
    $status = Get-NativeOutput 'git' @('-C', $RepoRoot, 'status', '--porcelain', '--untracked-files=no')
    if ($status) { throw "Tracked files differ from HEAD $When (use -AllowDirty to override):$([Environment]::NewLine)$status" }
}

function Assert-ReleaseReadiness([string] $Version) {
    # Match the CLI's strict version subset. Malformed input must never turn a
    # stable release into an exempt preview. AddonOnly/AllowDirty do not bypass it.
    $parts = $Version -split '-', 2
    if ($parts[0] -notmatch '^(0|[1-9][0-9]{0,8})\.(0|[1-9][0-9]{0,8})\.(0|[1-9][0-9]{0,8})$') {
        throw 'Invalid release version.'
    }
    if ($parts.Count -eq 2) {
        if ($parts[1].Length -gt 128) { throw 'Invalid prerelease version.' }
        foreach ($identifier in ($parts[1] -split '\.')) {
            if ($identifier -notmatch '^[0-9A-Za-z-]+$' -or
                ($identifier -match '^[0-9]+$' -and $identifier.Length -gt 1 -and $identifier.StartsWith('0'))) {
                throw 'Invalid prerelease version.'
            }
        }
        return
    }
    if ($parts[0].StartsWith('0.')) { return }
    if (-not (Test-Command 'cargo')) { throw 'Stable packaging requires the locked Rust release-readiness gate.' }
    Push-Location $RepoRoot
    try {
        Invoke-Native 'cargo' @('run', '--locked', '-p', 'azeroth-cli', '--', 'release-readiness',
            '--root', $RepoRoot, '--version', $Version)
    }
    finally { Pop-Location }
}

function Build-Installer([string] $Version) {
    Push-Location $RepoRoot
    try {
        Invoke-Native 'pnpm' @('install', '--frozen-lockfile')
        Invoke-Native 'cargo' @('fetch', '--locked')
        if (-not $SkipTests) {
            Write-Step 'Tests'
            Invoke-Native 'cargo' @('test', '--locked', '--workspace')
            Invoke-Native 'pnpm' @('--dir', 'apps/desktop', 'test:unit')
        }

        $tauriArguments = @('--dir', 'apps/desktop', 'exec', 'tauri', 'build', '--ci', '--bundles', 'nsis')
        $signConfig = $null
        if ($env:NAVZEROTH_SIGN_COMMAND) {
            # Tauri splits the string on ' ' and substitutes only an argument equal to %1.
            if (@($env:NAVZEROTH_SIGN_COMMAND.Split(' ')) -notcontains '%1') {
                throw 'NAVZEROTH_SIGN_COMMAND must contain %1 as a separate, unquoted word (the file to sign).'
            }
            $signConfig = Join-Path ([IO.Path]::GetTempPath()) "navzeroth-sign-$([Guid]::NewGuid().ToString('N')).json"
            $json = @{ bundle = @{ windows = @{ signCommand = $env:NAVZEROTH_SIGN_COMMAND } } } | ConvertTo-Json -Depth 4
            [IO.File]::WriteAllText($signConfig, $json, (New-Object Text.UTF8Encoding($false)))
            $tauriArguments += @('--config', $signConfig)
            Write-Step 'Signing enabled (NAVZEROTH_SIGN_COMMAND)'
        }
        else {
            Write-Warning 'NAVZEROTH_SIGN_COMMAND is not set: the installer is unsigned and SmartScreen will warn users.'
        }

        Write-Step 'tauri build --bundles nsis'
        try { Invoke-Native 'pnpm' $tauriArguments }
        finally { if ($signConfig) { Remove-Item -LiteralPath $signConfig -ErrorAction SilentlyContinue } }
    }
    finally { Pop-Location }

    $bundleDirectory = Join-Path $RepoRoot 'target\release\bundle\nsis'
    $built = @(Get-ChildItem -LiteralPath $bundleDirectory -Filter "*_$($Version)_x64-setup.exe" -File)
    if ($built.Count -ne 1) { throw "Expected one installer for $Version in $bundleDirectory, found $($built.Count)." }
    $installer = Join-Path $OutputDirectory "$AddonName-$Version-x64-setup.exe"
    Copy-Item -LiteralPath $built[0].FullName -Destination $installer -Force
    Write-Step "Installer: $installer"
    return $installer
}

# ---------------------------------------------------------------------------

$started = Get-Date
$version = Get-SourceVersion
Write-Step "Azeroth Navigator $version from $RepoRoot"
Assert-CleanTree 'before the build'

if (-not $AddonOnly) {
    if (-not $OnWindows) { throw 'The installer is built on Windows only (plan section 5.2). Use -AddonOnly elsewhere.' }
    $pins = Get-PinnedVersions
    Assert-MsvcBuildTools
    Assert-Rust $pins.Rust
    Assert-Node $pins.Node
    Assert-Pnpm $pins.Pnpm
}
# Refuse an unready stable version before removing or writing any output files.
Assert-ReleaseReadiness $version

# Remove only files an earlier run of this script wrote; never the directory itself.
if (Test-Path -LiteralPath $OutputDirectory) {
    Get-ChildItem -LiteralPath $OutputDirectory -File |
        Where-Object { $_.Name -like "$AddonName-*-x64-setup.exe" -or $_.Name -like "$AddonName-addon-*.zip" -or @('SHA256SUMS.txt', 'SOURCE_COMMIT.txt', 'CANDIDATE_NOTES.md') -contains $_.Name } |
        Remove-Item
}
else { New-Item -ItemType Directory -Path $OutputDirectory | Out-Null }

$artifacts = @()
if (-not $AddonOnly) {
    $artifacts += Build-Installer $version
}
$artifacts += New-AddonZip $version
# Candidate downloads identify their exact source without creating a tag/release.
$sourceCommit = Get-NativeOutput 'git' @('-C', $RepoRoot, 'rev-parse', 'HEAD')
if (-not $sourceCommit -or $sourceCommit -notmatch '^[0-9a-f]{40}$') { throw 'Cannot identify candidate source commit.' }
$sourcePath = Join-Path $OutputDirectory 'SOURCE_COMMIT.txt'
[IO.File]::WriteAllText($sourcePath, ($sourceCommit + "`n"), (New-Object Text.UTF8Encoding($false)))
$artifacts += $sourcePath
$notesPath = Join-Path $RepoRoot ("docs/releases/$version.md")
if (Test-Path -LiteralPath $notesPath -PathType Leaf) {
    $candidateNotes = Join-Path $OutputDirectory 'CANDIDATE_NOTES.md'
    Copy-Item -LiteralPath $notesPath -Destination $candidateNotes -Force
    $artifacts += $candidateNotes
}
Write-Checksums $artifacts
Assert-CleanTree 'after the build'

Write-Step ("Done in {0:N0} s. Release files are in {1}" -f ((Get-Date) - $started).TotalSeconds, $OutputDirectory)
