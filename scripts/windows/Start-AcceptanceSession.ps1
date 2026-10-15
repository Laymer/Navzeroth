[CmdletBinding()]
param(
    [Parameter(Mandatory)][string] $Root,
    [Parameter(Mandatory)][string] $Artifacts,
    [Parameter(Mandatory)][string] $InstalledAddon,
    [Parameter(Mandatory)][ValidatePattern('^[0-9a-f]{40}$')][string] $ExpectedSourceCommit,
    [Parameter(Mandatory)][string] $Scenarios,
    [Parameter(Mandatory)][string] $OutputDirectory,
    [string] $Version = '1.0.0'
)
Set-StrictMode -Version 3.0
$ErrorActionPreference = 'Stop'
. (Join-Path $PSScriptRoot 'Acceptance-Kit.ps1')
if ([Environment]::OSVersion.Platform -ne [PlatformID]::Win32NT) { throw 'Operator acceptance requires native Windows.' }
$Root = [IO.Path]::GetFullPath($Root)
$OutputDirectory = [IO.Path]::GetFullPath($OutputDirectory)
if ($OutputDirectory.Equals($Root, [StringComparison]::OrdinalIgnoreCase) -or $OutputDirectory.StartsWith($Root.TrimEnd('\', '/') + [IO.Path]::DirectorySeparatorChar, [StringComparison]::OrdinalIgnoreCase)) { throw 'Evidence must remain outside the source checkout.' }
Assert-ReleaseDestination $OutputDirectory
if (Test-Path -LiteralPath $OutputDirectory) { throw 'Use a new private evidence directory.' }
# This verifier is supplied by the candidate-packaging PR. Older helpers fail closed.
if (-not (Get-Command Assert-ReleaseArtifacts).Parameters.ContainsKey('AcceptanceCandidate')) { throw 'Candidate verifier is unavailable; integrate candidate-packaging first.' }
Assert-ReleaseArtifacts $Root $Version $Artifacts $false $true
$marker = (Read-BoundedReleaseText (Join-Path $Artifacts 'ACCEPTANCE_CANDIDATE.json')) | ConvertFrom-Json
if ($marker.source_commit -cne $ExpectedSourceCommit) { throw 'Candidate differs from expected reviewed source.' }
$materials = (Read-BoundedReleaseText (Join-Path $Artifacts 'RELEASE_MATERIALS.json')) | ConvertFrom-Json
if ($materials.inventory_sha256 -cne $marker.inventory_sha256 -or $materials.production_sha256 -cne $marker.production_sha256 -or $materials.candidate_source_commit -cne $ExpectedSourceCommit) { throw 'Material/candidate binding differs.' }
Assert-InstalledAcceptanceAddon (Join-Path $Artifacts "AzerothNavigator-addon-$Version.zip") $InstalledAddon
$procedures = @(Get-AcceptanceProcedures $Root $Scenarios)
Write-Host 'Evidence is private and unreviewed. Do not enter names, account paths, chat or raw exports. No game action is automated.'
$operator = Read-Host 'Operator pseudonym'
if ($operator -notmatch '^[A-Za-z0-9-]{1,64}$') { throw 'Use a short pseudonym.' }
$os = Get-CimInstance Win32_OperatingSystem
$cpu = Get-CimInstance Win32_Processor
$memory = Get-CimInstance Win32_ComputerSystem
$gpu = @(Get-CimInstance Win32_VideoController | Select-Object Name, DriverVersion)
$environment = [ordered]@{os=$os.Caption; version=$os.Version; build=$os.BuildNumber; architecture=$os.OSArchitecture; cpu=@($cpu | Select-Object Name, NumberOfLogicalProcessors); ram_bytes=$memory.TotalPhysicalMemory; gpu=$gpu; powershell=$PSVersionTable.PSVersion.ToString()}
$environment.operator_details = Read-Host 'Record storage, WebView2, display/scaling/monitor matrix, game mode, locale, combat state and performance workload/sample counts (no personal identifiers)'
$environment.client_tuple = Read-Host 'Observed GetBuildInfo version/build/interface (do not infer it)'
if ($environment.client_tuple -cne '1.60.1/70374/16001') { throw 'Observed target must equal 1.60.1/70374/16001.' }
New-Item -ItemType Directory -Path $OutputDirectory | Out-Null
New-Item -ItemType Directory -Path (Join-Path $OutputDirectory 'reviewed') | Out-Null
$session = [ordered]@{schema_version=1; kind='unreviewed_acceptance_session'; publication_evidence=$false; operator=$operator; started_at=[DateTimeOffset]::UtcNow.ToString('o'); tested_source_commit=$ExpectedSourceCommit; production_sha256=$marker.production_sha256; inventory_sha256=$marker.inventory_sha256; material_manifest_sha256=(Get-AcceptanceHash (Join-Path $Artifacts 'RELEASE_MATERIALS.json')); artifact_manifest_sha256=(Get-AcceptanceHash (Join-Path $Artifacts 'RELEASE_ASSETS.json')); scenario_sha256=(Get-AcceptanceHash $Scenarios); runbook_sha256=(Get-AcceptanceHash (Join-Path $Root 'docs/compatibility/V1_ACCEPTANCE_RUNBOOK.md')); environment=$environment; required_ids=@($procedures | ForEach-Object id); results=@()}
Write-AcceptanceDraft $OutputDirectory $session
$evidenceTotal = 0L
foreach ($procedure in $procedures) {
    Write-Host "`n$($procedure.procedure)"
    $started = [DateTimeOffset]::UtcNow.ToString('o')
    $timer = [Diagnostics.Stopwatch]::StartNew()
    $status = Read-Host 'After completing every subcase: pass / fail / unverified (q saves and stops)'
    if ($status -eq 'q') { break }
    $actual = Read-Host 'Actual results, missing subcases and limitations; identify before/after IDs using pseudonyms'
    $evidence = @()
    while ($true) {
        $path = Read-Host 'Reviewed sanitized evidence file path (empty finishes evidence list); raw exports stay private'
        if ([string]::IsNullOrEmpty($path)) { break }
        Assert-RegularReleasePath $path
        $bytes = (Get-Item -LiteralPath $path).Length
        $evidenceTotal += $bytes
        if ($bytes -gt 268435456 -or $evidenceTotal -gt 1073741824) { throw 'Evidence exceeds 256 MiB per file or 1 GiB per session.' }
        if ((Read-Host 'Confirm you inspected and sanitized this file: type reviewed') -cne 'reviewed') { throw 'Evidence was not reviewed.' }
        $extension = [IO.Path]::GetExtension($path).ToLowerInvariant()
        if ($extension -notin @('.png', '.jpg', '.txt', '.json', '.log', '.pdf', '.mp4')) { $extension = '.bin' }
        $name = '{0}-{1}{2}' -f $procedure.id, $evidence.Count, $extension
        $destination = Join-Path (Join-Path $OutputDirectory 'reviewed') $name
        $hash = Get-AcceptanceHash $path
        Copy-Item -LiteralPath $path -Destination $destination
        if ((Get-AcceptanceHash $destination) -cne $hash) { throw 'Evidence changed during copy.' }
        $evidence += [ordered]@{path=('reviewed/' + $name); bytes=$bytes; sha256=$hash; sanitization='operator-reviewed'}
    }
    $timer.Stop()
    $session.results += New-AcceptanceResult $procedure $status $actual $started $timer.ElapsedMilliseconds $evidence
    Write-AcceptanceDraft $OutputDirectory $session
}
# Check identities again in case files changed during a long operator session.
Assert-ReleaseArtifacts $Root $Version $Artifacts $false $true
Assert-InstalledAcceptanceAddon (Join-Path $Artifacts "AzerothNavigator-addon-$Version.zip") $InstalledAddon
$session.finished_at = [DateTimeOffset]::UtcNow.ToString('o')
$session.identity_rechecked = $true
Write-AcceptanceDraft $OutputDirectory $session
Write-Host 'Draft saved. Missing rows remain unverified. Independent review is required before any release manifest update.'
