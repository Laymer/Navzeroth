[CmdletBinding()]
param()
Set-StrictMode -Version 3.0
$ErrorActionPreference = 'Stop'
. (Join-Path $PSScriptRoot 'Release-Policy.ps1')
$root = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot '../..'))
$count = 0
function Check([string] $Name, [bool] $Condition) { $script:count++; if (-not $Condition) { throw "FAILED: $Name" }; Write-Host "ok $Name" }
function Refuses([string] $Name, [scriptblock] $Action) {
    $failed = $false
    try { & $Action } catch { $failed = $true }
    Check $Name $failed
}
$stable = Get-UnsignedReleasePolicy $root '1.0.0'
$rc = Get-UnsignedReleasePolicy $root '1.0.0-rc.1'
Check 'stable explicitly unsigned' ($stable.signature -ceq 'unsigned' -and -not $stable.prerelease)
Check 'historical RC remains prerelease' ($rc.signature -ceq 'unsigned' -and $rc.prerelease)
Check 'another stable version has no exception' ($null -eq (Get-UnsignedReleasePolicy $root '1.0.1'))
$release = [pscustomobject]@{ tagName = 'v1.0.0'; isDraft = $true; isPrerelease = $false }
Assert-UnsignedReleaseState $stable $release '1.0.0' 'draft'
Check 'stable draft accepted' $true
$release.isDraft = $false
Assert-UnsignedReleaseState $stable $release '1.0.0' 'public'
Check 'stable public accepted' $true
Refuses 'wrong draft state refused' { Assert-UnsignedReleaseState $stable $release '1.0.0' 'draft' }
$release.isPrerelease = $true
Refuses 'stable prerelease flag refused' { Assert-UnsignedReleaseState $stable $release '1.0.0' 'public' }
$release.tagName = 'v1.0.0-rc.1'
Assert-UnsignedReleaseState $rc $release '1.0.0-rc.1' 'public'
Check 'historical RC public accepted' $true
Refuses 'wrong tag refused' { Assert-UnsignedReleaseState $stable $release '1.0.0' 'public' }
Refuses 'missing exception refused' { Assert-UnsignedReleaseState $null $release '1.0.1' 'public' }
Refuses 'invalid phase refused' { Assert-UnsignedReleaseState $rc $release '1.0.0-rc.1' 'other' }
$temp = Join-Path ([IO.Path]::GetTempPath()) ('anav-policy-' + [Guid]::NewGuid().ToString('N'))
try {
    New-Item -ItemType Directory -Path (Join-Path $temp 'release/v1') -Force | Out-Null
    $path = Join-Path $temp 'release/v1/artifact-policy.json'
    foreach ($bad in @(
        '{"schema_version":2,"releases":[]}',
        '{"schema_version":1,"releases":[{"version":"1.0.0","prerelease":true,"signature":"unsigned"}]}',
        '{"schema_version":1,"releases":[{"version":"1.0.0","prerelease":false,"signature":"signed"}]}',
        '{"schema_version":1,"releases":[{"version":"1.0.0","prerelease":false,"signature":"unsigned"},{"version":"1.0.0","prerelease":false,"signature":"unsigned"}]}',
        '{"schema_version":1,"releases":[{"version":"1.0.0","prerelease":"false","signature":"unsigned"}]}',
        '{"schema_version":1,"releases":[{"version":"1.0.0;echo","prerelease":false,"signature":"unsigned"}]}'
    )) {
        [IO.File]::WriteAllText($path, $bad)
        Refuses 'malformed/unsupported policy refused' { Get-UnsignedReleasePolicy $temp '1.0.0' }
    }
} finally { Remove-Item -LiteralPath $temp -Recurse -Force }
Write-Host "$count release policy checks passed"
