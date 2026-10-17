<#
.SYNOPSIS
    Replaces the built installer with its externally signed copy (task V0.4).
.DESCRIPTION
    Directory holds the release files Build-Windows.ps1 wrote. SignedDirectory
    holds what the signing service returned. The signed installer must be the
    exact unsigned build plus a signature (Assert-SignedFromUnsigned) and carry
    an Authenticode signature. The asset manifest and SHA256SUMS.txt are then
    regenerated and the whole set is verified against source again.
#>
[CmdletBinding()]
param(
    [Parameter(Mandatory)][string] $Version,
    [Parameter(Mandatory)][string] $Directory,
    [Parameter(Mandatory)][string] $SignedDirectory,
    [switch] $RequireTrusted,
    [string] $ExpectedSubject = ''
)
Set-StrictMode -Version 3.0
$ErrorActionPreference = 'Stop'
. (Join-Path $PSScriptRoot 'Release-Materials.ps1')
. (Join-Path $PSScriptRoot 'Signing.ps1')
$root = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot '../..'))
$Directory = [IO.Path]::GetFullPath($Directory)
$name = "AzerothNavigator-$Version-x64-setup.exe"
$unsigned = Join-Path $Directory $name

$found = @(Get-ChildItem -LiteralPath $SignedDirectory -Recurse -File -Filter $name)
if ($found.Count -ne 1) { throw "Expected one signed $name in $SignedDirectory, found $($found.Count)." }
Assert-SignedFromUnsigned $unsigned $found[0].FullName
$signature = Assert-AuthenticodeSignature $found[0].FullName ([bool]$RequireTrusted) $ExpectedSubject
Copy-Item -LiteralPath $found[0].FullName -Destination $unsigned -Force
Write-Host "Signed installer: $name ($($signature.Status); $($signature.SignerCertificate.Subject))"

# The asset manifest lists the installer hash, and SHA256SUMS.txt covers both.
if (Test-Path -LiteralPath (Join-Path $Directory 'RELEASE_ASSETS.json')) {
    $selection = Get-ReleaseMaterialSelection $root $Version
    $null = Write-ReleaseArtifactManifest $root $Version $Directory $false $selection
}
$sums = Join-Path $Directory 'SHA256SUMS.txt'
$lines = foreach ($line in (Read-BoundedReleaseText $sums).TrimEnd("`n").Split("`n")) {
    $file = $line.Substring(66)
    (Get-FileHash -LiteralPath (Join-Path $Directory $file) -Algorithm SHA256).Hash.ToLowerInvariant() + '  ' + $file
}
[IO.File]::WriteAllText($sums, (($lines -join "`n") + "`n"), (New-Object Text.UTF8Encoding($false)))
Assert-ReleaseArtifacts $root $Version $Directory $false $false
exit 0
