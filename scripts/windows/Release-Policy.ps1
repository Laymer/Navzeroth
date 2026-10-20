# Explicit source-bound exceptions to stable signing. Dot-source only.
Set-StrictMode -Version 3.0
. (Join-Path $PSScriptRoot 'Release-Materials.ps1')

function Get-UnsignedReleasePolicy([string] $Root, [string] $Version) {
    $policy = Read-BoundedReleaseText (Join-Path $Root 'release/v1/artifact-policy.json') | ConvertFrom-Json
    if ($policy.schema_version -ne 1 -or @($policy.PSObject.Properties.Name).Count -ne 2) { throw 'Unsupported artifact policy schema.' }
    $seen = @{}
    $selected = $null
    foreach ($release in $policy.releases) {
        if (@($release.PSObject.Properties.Name).Count -ne 3 -or $release.signature -cne 'unsigned' -or $release.prerelease -isnot [bool]) { throw 'Invalid unsigned artifact policy.' }
        $stable = Test-StableReleaseVersion $release.version
        if ($seen.ContainsKey($release.version) -or $release.prerelease -ne (-not $stable)) { throw 'Duplicate or inconsistent artifact policy.' }
        $seen[$release.version] = $true
        if ($release.version -ceq $Version) { $selected = $release }
    }
    return $selected
}

function Assert-UnsignedReleaseState($Policy, $Release, [string] $Version, [string] $Phase) {
    if ($null -eq $Policy -or $Policy.version -cne $Version) { throw 'No explicit unsigned release policy for this version.' }
    if ($Phase -cnotin @('draft', 'public')) { throw 'Invalid release phase.' }
    if ($Release.tagName -cne ('v' + $Version) -or $Release.isPrerelease -isnot [bool] -or $Release.isDraft -isnot [bool] -or
        $Release.isPrerelease -ne $Policy.prerelease -or $Release.isDraft -ne ($Phase -ceq 'draft')) { throw 'Release identity or state differs from the unsigned policy.' }
}
