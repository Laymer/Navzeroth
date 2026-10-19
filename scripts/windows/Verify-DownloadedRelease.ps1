# Installs only on an ephemeral GitHub-hosted Windows runner.
[CmdletBinding()]
param(
    [Parameter(Mandatory)][string] $Directory,
    [Parameter(Mandatory)][string] $Version,
    [Parameter(Mandatory)][ValidatePattern('^[a-f0-9]{64}$')][string] $InstallerSha256,
    [Parameter(Mandatory)][string] $Receipt,
    [ValidateSet('prepared','draft','public')][string] $Phase = 'public',
    [string] $Evidence = ''
)
Set-StrictMode -Version 3.0
$ErrorActionPreference = 'Stop'
if ($env:GITHUB_ACTIONS -ne 'true' -or $env:RUNNER_ENVIRONMENT -ne 'github-hosted' -or $env:OS -ne 'Windows_NT') {
    throw 'Download installation verification requires a disposable GitHub-hosted Windows runner'
}
$root = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot '../..'))
$Directory = [IO.Path]::GetFullPath($Directory)
. (Join-Path $PSScriptRoot 'Release-Policy.ps1')
$policy = Get-UnsignedReleasePolicy $root $Version
if ($null -eq $policy) { throw 'No explicit unsigned release policy for this version.' }
. (Join-Path $PSScriptRoot 'Native-Smoke-Helpers.ps1')
$readiness = switch ($Phase) { 'prepared' { 'prebuild' } 'draft' { 'draft' } 'public' { 'public' } }
Assert-ReleaseArtifacts $root $Version $Directory $false $false $readiness $Evidence
$setup = Get-NativeSmokeSetup -Directory $Directory
$setupHash = (Get-FileHash -LiteralPath $setup.FullName -Algorithm SHA256).Hash.ToLowerInvariant()
if ($setupHash -cne $InstallerSha256) { throw 'Downloaded installer differs from the independently recorded packaging hash' }
$setupMetadata = $setup.VersionInfo
if ($setupMetadata.ProductVersion -cne $Version -or $setupMetadata.FileVersion -cne $Version) { throw 'Installer string version metadata differs from release' }
$setupSignature = Get-AuthenticodeSignature -LiteralPath $setup.FullName
if ($setupSignature.Status.ToString() -ne 'NotSigned') { throw 'Expected an explicitly unsigned installer' }

# Exact ZIP entries and bytes; no extraction of untrusted paths.
Add-Type -AssemblyName System.IO.Compression.FileSystem
$expected = [Collections.Generic.Dictionary[string,string]]::new([StringComparer]::Ordinal)
$addon = Join-Path $root 'addons/AzerothNavigator'
foreach ($file in Get-ChildItem -LiteralPath $addon -Recurse -File -Force) {
    $name = 'AzerothNavigator/' + $file.FullName.Substring($addon.Length + 1).Replace('\', '/')
    $expected.Add($name, (Get-FileHash -LiteralPath $file.FullName -Algorithm SHA256).Hash)
}
$archive = [IO.Compression.ZipFile]::OpenRead((Join-Path $Directory "AzerothNavigator-addon-$Version.zip"))
try {
    if ($archive.Entries.Count -ne $expected.Count) { throw 'Addon ZIP entry count differs from source' }
    $seen = [Collections.Generic.HashSet[string]]::new([StringComparer]::Ordinal)
    foreach ($entry in $archive.Entries) {
        if (-not $seen.Add($entry.FullName) -or -not $expected.ContainsKey($entry.FullName) -or $entry.Length -gt 16777216) { throw 'Unexpected or duplicate addon ZIP entry' }
        $stream = $entry.Open(); $sha = [Security.Cryptography.SHA256]::Create()
        try { $actual = [BitConverter]::ToString($sha.ComputeHash($stream)).Replace('-', '') }
        finally { $sha.Dispose(); $stream.Dispose() }
        if ($actual -cne $expected[$entry.FullName]) { throw 'Addon ZIP bytes differ from source' }
    }
} finally { $archive.Dispose() }

$install = Join-Path $env:LOCALAPPDATA 'Azeroth Navigator'
if (Test-Path -LiteralPath $install) { throw 'Verification requires a fresh installation directory' }
try {
    Install-NativeSmokeSetup -SetupPath $setup.FullName
    if ((Get-InstalledDisplayVersion) -cne $Version) { throw 'Installed version differs from release' }
    $exe = Join-Path $install 'azeroth-desktop.exe'
    $signature = Get-AuthenticodeSignature -LiteralPath $exe
    if ($signature.Status.ToString() -ne 'NotSigned') { throw 'Expected an explicitly unsigned application' }
    $inventoryJson = & node (Join-Path $PSScriptRoot 'verify-installed-tree.mjs') $root $install
    if ($LASTEXITCODE -ne 0) { throw 'Installed inventory verification failed' }
    & node (Join-Path $root 'scripts/dependency-notices.mjs') --installed (Join-Path $install 'local-data-notices')
    if ($LASTEXITCODE -ne 0) { throw 'Installed dependency source and notice verification failed' }
    $metadata = (Get-Item -LiteralPath $exe).VersionInfo
    # Tauri preserves SemVer in string metadata; numeric PE versions omit prerelease.
    if ($metadata.ProductVersion -cne $Version -or $metadata.FileVersion -cne $Version) { throw 'Application string version metadata differs from release' }
    $assets = @(Get-ChildItem -LiteralPath $Directory -File | Sort-Object Name | ForEach-Object {
        [ordered]@{ name = $_.Name; bytes = $_.Length; sha256 = (Get-FileHash -LiteralPath $_.FullName -Algorithm SHA256).Hash.ToLowerInvariant() }
    })
    $result = [ordered]@{
        schema_version = 3; phase = $Phase; version = $Version; prerelease = $policy.prerelease; declared_signature = $policy.signature; source_commit = (Get-Content -Raw (Join-Path $Directory 'SOURCE_COMMIT.txt')).Trim()
        checked_at = [DateTime]::UtcNow.ToString('o'); workflow_run_id = $env:GITHUB_RUN_ID; workflow_run_attempt = $env:GITHUB_RUN_ATTEMPT
        assets = $assets; installer_signature = $setupSignature.Status.ToString(); application_signature = $signature.Status.ToString()
        installed_display_version = Get-InstalledDisplayVersion; product_version = $metadata.ProductVersion; file_version = $metadata.FileVersion
        installer_product_version = $setupMetadata.ProductVersion; installer_file_version = $setupMetadata.FileVersion
        installer_numeric_file_version = @($setupMetadata.FileMajorPart, $setupMetadata.FileMinorPart, $setupMetadata.FileBuildPart, $setupMetadata.FilePrivatePart) -join '.'
        application_numeric_file_version = @($metadata.FileMajorPart, $metadata.FileMinorPart, $metadata.FileBuildPart, $metadata.FilePrivatePart) -join '.'
        application_sha256 = (Get-FileHash -LiteralPath $exe -Algorithm SHA256).Hash.ToLowerInvariant()
        installed_files = @($inventoryJson | ConvertFrom-Json); addon_zip_matches_source = $true; dependency_closure_verified = $true
        evidence_scope = 'Hosted synthetic verification; not ordinary-Windows or live acceptance'
    }
    $result | ConvertTo-Json -Depth 8 | Set-Content -LiteralPath $Receipt -Encoding utf8
} finally {
    Stop-NativeSmokeProcesses
    Uninstall-NativeSmokeApp -InstallDirectory $install
}
