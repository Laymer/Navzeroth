# Disposable historical fixture staging only; never records installer acceptance.
[CmdletBinding()]
param([string] $Installer = '', [string] $ExpectedSha256 = '', [string] $OwnershipMarker = '', [string] $OwnershipNonce = '', [switch] $Cleanup)
Set-StrictMode -Version 3.0
$ErrorActionPreference='Stop'
if ($env:GITHUB_ACTIONS -cne 'true' -or $env:RUNNER_ENVIRONMENT -cne 'github-hosted' -or $env:OS -cne 'Windows_NT') { throw 'Historical fixture installation requires disposable hosted Windows.' }
. (Join-Path $PSScriptRoot 'Native-Smoke-Helpers.ps1')
. (Join-Path $PSScriptRoot 'Recovery-Probe-Ownership.ps1')
$install=Join-Path $env:LOCALAPPDATA 'Azeroth Navigator'
if ($Cleanup) {
    if (-not (Test-RecoveryProbeOwnership $OwnershipMarker $OwnershipNonce $env:GITHUB_SHA $ExpectedSha256 $install)) { throw 'Cleanup refuses an unowned fixture installation.' }
    Stop-NativeSmokeProcesses
    Uninstall-NativeSmokeApp -InstallDirectory $install
    $null=Wait-NativeSmokeUninstalled -InstallDirectory $install -TimeoutSeconds 90
    return
}
if (Test-Path -LiteralPath $install) { throw 'Fixture staging requires a fresh installer directory.' }
if ($null -ne (Get-InstalledDisplayVersion)) { throw 'Fixture staging refuses an existing registration.' }
if (@(Get-Process -Name 'azeroth-desktop' -ErrorAction SilentlyContinue).Count -gt 0) { throw 'Fixture staging refuses an unowned application process.' }
$existingWeb=@(Get-CimInstance Win32_Process -Filter "Name = 'msedgewebview2.exe'" -ErrorAction Stop | Where-Object { $_.CommandLine -and $_.CommandLine.Contains($script:NativeSmokeAppId) })
if ($existingWeb.Count -gt 0) { throw 'Fixture staging refuses an unowned application WebView process.' }
if ($ExpectedSha256 -cnotmatch '^[a-f0-9]{64}$' -or (Get-FileHash -LiteralPath $Installer -Algorithm SHA256).Hash.ToLowerInvariant() -cne $ExpectedSha256) { throw 'Historical installer differs from reviewed bytes.' }
if ((Get-AuthenticodeSignature -LiteralPath $Installer).Status.ToString() -cne 'NotSigned') { throw 'Historical fixture must retain its disclosed unsigned state.' }
# Marker is issued only after every fresh-directory/registration/hash/signature guard.
New-RecoveryProbeOwnership $OwnershipMarker $OwnershipNonce $env:GITHUB_SHA $ExpectedSha256 $install
Install-NativeSmokeSetup -SetupPath $Installer -TimeoutSeconds 300
Stop-NativeSmokeProcesses
if (-not (Test-Path -LiteralPath (Join-Path $install 'bundled-content/manifest.json'))) { throw 'Historical full bundle is absent.' }
Write-Host 'Historical fixture staged; old application UI was not used to exercise new recovery code.'
