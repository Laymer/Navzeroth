# Dot-source only. Operator records are drafts, never release-gate evidence.
Set-StrictMode -Version 3.0
. (Join-Path $PSScriptRoot 'Release-Materials.ps1')

function Assert-AcceptanceToolRoot([string] $Root, [string] $ToolsDirectory) {
    Assert-RegularReleasePath $Root -Directory
    Assert-RegularReleasePath $ToolsDirectory -Directory
    $expected = [IO.Path]::GetFullPath((Join-Path $Root 'scripts/windows')).TrimEnd('\', '/')
    $actual = [IO.Path]::GetFullPath($ToolsDirectory).TrimEnd('\', '/')
    $comparison = if ([Environment]::OSVersion.Platform -eq [PlatformID]::Win32NT) { [StringComparison]::OrdinalIgnoreCase } else { [StringComparison]::Ordinal }
    if (-not $actual.Equals($expected, $comparison)) { throw 'Run acceptance tooling from the reviewed candidate checkout.' }
}

function Get-AcceptanceHash([string] $Path) {
    Assert-RegularReleasePath $Path
    return (Get-FileHash -LiteralPath $Path -Algorithm SHA256).Hash.ToLowerInvariant()
}

function Assert-InstalledAcceptanceAddon([string] $Archive, [string] $Installed) {
    Assert-RegularReleasePath $Archive
    Assert-RegularReleasePath $Installed -Directory
    if ((Get-Item -LiteralPath $Archive).Length -gt 268435456) { throw 'Addon ZIP exceeds bound.' }
    Add-Type -AssemblyName System.IO.Compression
    $stream = [IO.File]::OpenRead($Archive)
    $zip = New-Object IO.Compression.ZipArchive($stream, [IO.Compression.ZipArchiveMode]::Read)
    $seen = New-Object 'System.Collections.Generic.HashSet[string]' ([StringComparer]::OrdinalIgnoreCase)
    $total = 0L
    try {
        if ($zip.Entries.Count -gt 4096) { throw 'Too many addon ZIP entries.' }
        foreach ($entry in $zip.Entries) {
            $name = $entry.FullName
            if ($name -notmatch '^AzerothNavigator/(?:[A-Za-z0-9_.-]+/)*[A-Za-z0-9_.-]+/?$' -or $name -match '(^|/)\.{1,2}(/|$)' -or (($entry.ExternalAttributes -shr 16) -band 61440) -eq 40960) { throw 'Unsafe addon ZIP entry.' }
            if ($name.EndsWith('/')) { continue }
            $relative = $name.Substring('AzerothNavigator/'.Length)
            if (-not $seen.Add($relative)) { throw 'Duplicate addon ZIP entry.' }
            $total += $entry.Length
            if ($entry.Length -gt 16777216 -or $total -gt 268435456) { throw 'Decoded addon ZIP exceeds bound.' }
            $path = Join-Path $Installed $relative
            Assert-RegularReleasePath $path
            if ((Get-Item -LiteralPath $path).Length -ne $entry.Length) { throw 'Installed addon file size differs.' }
            $input = $entry.Open()
            $sha = [Security.Cryptography.SHA256]::Create()
            try { $hash = [BitConverter]::ToString($sha.ComputeHash($input)).Replace('-', '').ToLowerInvariant() }
            finally { $input.Dispose(); $sha.Dispose() }
            if ($hash -cne (Get-AcceptanceHash $path)) { throw 'Installed addon file hash differs.' }
        }
        if ($seen.Count -eq 0) { throw 'Addon ZIP is empty.' }
        $files = @(Get-ChildItem -LiteralPath $Installed -Recurse -Force)
        foreach ($file in $files) { Assert-RegularReleasePath $file.FullName -Directory:($file.PSIsContainer) }
        if (@($files | Where-Object { -not $_.PSIsContainer }).Count -ne $seen.Count) { throw 'Installed addon has extra files.' }
    } finally { $zip.Dispose(); $stream.Dispose() }
}

function Get-AcceptanceProcedures([string] $Root, [string] $ScenarioPath) {
    $manifest = (Read-BoundedReleaseText (Join-Path $Root 'release/v1/acceptance-evidence.json')) | ConvertFrom-Json
    $inventory = (Read-BoundedReleaseText (Join-Path $Root 'release/v1/coverage-inventory.json')) | ConvertFrom-Json
    $runbook = Read-BoundedReleaseText (Join-Path $Root 'docs/compatibility/V1_ACCEPTANCE_RUNBOOK.md')
    if ($inventory.schema_version -notin 1, 2, 3 -or $manifest.schema_version -ne $inventory.schema_version) { throw 'Unsupported acceptance or inventory schema.' }
    $seen = New-Object 'System.Collections.Generic.HashSet[string]' ([StringComparer]::Ordinal)
    $rows = @()
    foreach ($check in $manifest.checks) {
        if (-not $seen.Add($check.id) -or $check.id -notmatch '^(LIVE|WIN)-[A-Z-]+$') { throw 'Invalid acceptance check identity.' }
        $method = if ($check.id.StartsWith('LIVE-')) { 'live_game' } else { 'interactive_windows' }
        if ($check.method -cne $method) { throw 'Acceptance method mismatch.' }
        $pattern = '(?ms)^## ' + [Regex]::Escape($check.id) + ' [^\r\n]+\r?\n(.*?)(?=^## |\z)'
        $match = [Regex]::Match($runbook, $pattern)
        if (-not $match.Success) { throw "Missing procedure for $($check.id)." }
        $rows += [pscustomobject]@{id=$check.id; method=$method; procedure=$match.Value.Trim(); inventory_unit=$null}
    }
    # Schema 3 (ADR 0007 section 5) has no LIVE-* rows; the 10 WIN-* rows are unchanged.
    $expectedLive = if ($inventory.schema_version -eq 3) { 0 } else { 15 }
    if (@($rows | Where-Object method -eq 'live_game').Count -ne $expectedLive -or @($rows | Where-Object method -eq 'interactive_windows').Count -ne 10) { throw "Expected $expectedLive live and 10 Windows checks." }
    $scenarios = (Read-BoundedReleaseText $ScenarioPath) | ConvertFrom-Json
    if ($scenarios.schema_version -ne 1 -or $scenarios.inventory_sha256 -cne (Get-AcceptanceHash (Join-Path $Root 'release/v1/coverage-inventory.json'))) { throw 'Scenario inventory binding mismatch.' }
    if (@($scenarios.scenarios).Count -gt 10000) { throw 'Too many acceptance scenarios.' }
    $covered = New-Object 'System.Collections.Generic.HashSet[string]' ([StringComparer]::Ordinal)
    $units = @{}
    foreach ($domain in $inventory.domains) {
        foreach ($unit in $domain.units) { $units[($domain.domain + '/' + $unit.id)] = $unit }
    }
    foreach ($scenario in $scenarios.scenarios) {
        $key = $scenario.domain + '/' + $scenario.unit_id
        if (-not $units.ContainsKey($key) -or -not $seen.Add($scenario.id) -or $scenario.id -notmatch '^SCENARIO-[A-Za-z0-9-]+$' -or $scenario.method -notin @('live_game', 'interactive_windows') -or [string]::IsNullOrWhiteSpace($scenario.procedure) -or [string]::IsNullOrWhiteSpace($scenario.expected)) { throw 'Invalid inventory scenario.' }
        $null = $covered.Add($key)
        $rows += [pscustomobject]@{id=$scenario.id; method=$scenario.method; procedure=($scenario.procedure + "`nExpected: " + $scenario.expected); inventory_unit=$key}
    }
    foreach ($key in $units.Keys) { if (-not $covered.Contains($key)) { throw "Missing inventory scenario: $key" } }
    return $rows
}

function New-AcceptanceResult($Procedure, [string] $Status, [string] $Actual, [string] $Started, [long] $Duration, $Evidence) {
    if ($Status -notin @('pass', 'fail', 'unverified') -or [string]::IsNullOrWhiteSpace($Actual) -or $Actual.Length -gt 16000 -or $Duration -lt 0) { throw 'Invalid operator result.' }
    if ($Status -in @('pass', 'fail') -and @($Evidence).Count -eq 0) { throw 'Executed results require reviewed evidence.' }
    return [ordered]@{id=$Procedure.id; method=$Procedure.method; inventory_unit=$Procedure.inventory_unit; status=$Status; started_at=$Started; duration_ms=$Duration; actual=$Actual; evidence=@($Evidence)}
}

function Write-AcceptanceDraft([string] $Directory, $Session) {
    $json = $Session | ConvertTo-Json -Depth 30
    $temp = Join-Path $Directory 'session.tmp'
    [IO.File]::WriteAllText($temp, $json + "`n", (New-Object Text.UTF8Encoding($false)))
    Move-Item -LiteralPath $temp -Destination (Join-Path $Directory 'session.json') -Force
}
