# Stable hosted observations only. This recorder cannot create either final smoke gate.
Set-StrictMode -Version 3.0
$script:HostedAcceptanceIds = @('HOSTED-INSTALL','HOSTED-FIRST-START','HOSTED-UPGRADE-RC1','HOSTED-DOWNGRADE-REFUSAL','HOSTED-PERSISTENCE-RECOVERY','HOSTED-RESOURCES','HOSTED-OFFLINE','HOSTED-PERFORMANCE')
# Preserve literal evidence timestamps before validation. PS7 normally converts
# ISO JSON strings to DateTime, whose culture formatting is not the source value.
function Restore-HostedTimestampStrings([string] $Text, $Value) {
    $document = [System.Text.Json.JsonDocument]::Parse($Text)
    try {
        $root = $document.RootElement
        if ($Value.PSObject.Properties['recorded_at']) {
            $timestamp = $root.GetProperty('recorded_at')
            if ($timestamp.ValueKind -eq [System.Text.Json.JsonValueKind]::String) { $Value.recorded_at = $timestamp.GetString() }
        }
        if ($Value.PSObject.Properties['checks'] -and $root.GetProperty('checks').ValueKind -eq [System.Text.Json.JsonValueKind]::Array) {
            $checks = @($Value.checks)
            $tokens = $root.GetProperty('checks')
            if ($checks.Count -ne $tokens.GetArrayLength()) { throw 'Hosted JSON check count differs.' }
            for ($i = 0; $i -lt $checks.Count; $i++) {
                if ($checks[$i].PSObject.Properties['recorded_at']) {
                    $timestamp = $tokens[$i].GetProperty('recorded_at')
                    if ($timestamp.ValueKind -eq [System.Text.Json.JsonValueKind]::String) { $checks[$i].recorded_at = $timestamp.GetString() }
                }
            }
        }
    } finally { $document.Dispose() }
}
function ConvertFrom-HostedJsonText([string] $Text) {
    if ((Get-Command ConvertFrom-Json).Parameters.ContainsKey('DateKind')) { return ConvertFrom-Json -InputObject $Text -DateKind String }
    $value = ConvertFrom-Json -InputObject $Text
    # Windows PowerShell5.1 already keeps strings. Older PS7 uses the raw JSON
    # token for only the two supported shapes: a case or evidence.checks rows.
    if ($PSVersionTable.PSVersion.Major -ge 6) { Restore-HostedTimestampStrings $Text $value }
    return $value
}
# Relative Windows file paths become portable FileRef paths before validation.
function ConvertTo-HostedArtifactPath([string] $Relative) { return $Relative.Replace('\', '/') }
function Test-HostedText($Value) { return $Value -is [string] -and $Value.Trim().Length -gt 0 -and $Value.Length -le 1024 -and $Value -notmatch '[\x00-\x1f\x7f]' }
function Assert-HostedCase($Case, [string] $Id, [string] $Version, [string] $SourceCommit, [string] $Production, [string] $Installer, $Target) {
    if ($Case.schema_version -ne 1 -or $Case.id -cne $Id -or $Case.method -cne 'hosted_windows' -or $Case.status -cnotin @('pass','fail') -or $Case.version -cne $Version -or $Case.source_commit -cne $SourceCommit -or $Case.production_sha256 -cne $Production -or $Case.installer_sha256 -cne $Installer -or ($Case.target | ConvertTo-Json -Depth 20 -Compress) -cne ($Target | ConvertTo-Json -Depth 20 -Compress)) { throw 'Hosted case release/installer/source identity differs.' }
    if (-not (Test-HostedText $Case.operator) -or -not (Test-HostedText $Case.environment) -or $Case.recorded_at -isnot [string] -or $Case.recorded_at -cnotmatch '^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}(?:\.\d+)?Z$') { throw 'Hosted case lacks actual operator/date/environment.' }
    $date = [DateTimeOffset]::MinValue
    if (-not [DateTimeOffset]::TryParse($Case.recorded_at, [ref]$date)) { throw 'Hosted date is invalid.' }
    if ($Case.status -ceq 'fail') { return }
    if ($Case.completed -isnot [bool] -or -not $Case.completed) { throw 'Passing hosted case was not completed.' }
    switch ($Id) {
        'HOSTED-UPGRADE-RC1' { if ($Case.baseline_version -cne '1.0.0-rc.1' -or $Case.baseline_installer_sha256 -cnotmatch '^[a-f0-9]{64}$') { throw 'Upgrade must identify the exact rc.1 baseline installer.' } }
        'HOSTED-DOWNGRADE-REFUSAL' { if ($Case.baseline_version -cne '1.0.0-rc.1' -or $Case.baseline_installer_sha256 -cnotmatch '^[a-f0-9]{64}$' -or $Case.attempted -isnot [bool] -or -not $Case.attempted -or $Case.timed_out -isnot [bool] -or $Case.timed_out -or $Case.installed_version_after -cne $Version) { throw 'Downgrade refusal was not completed against rc.1.' } }
        'HOSTED-PERSISTENCE-RECOVERY' { if ($Case.restart_verified -isnot [bool] -or -not $Case.restart_verified -or $Case.recovery_verified -isnot [bool] -or -not $Case.recovery_verified) { throw 'Persistence and recovery must both be verified.' } }
        'HOSTED-RESOURCES' { if ($Case.installed_resources_verified -isnot [bool] -or -not $Case.installed_resources_verified -or $Case.source_notices_verified -isnot [bool] -or -not $Case.source_notices_verified) { throw 'Exact resources and source/notices must both be verified.' } }
        'HOSTED-OFFLINE' {
            # Historical cases disabled adapters; ADR 0010 cases isolate by firewall
            # with only the runner agent and DNS allowed. Never mix the two shapes.
            $names = @($Case.PSObject.Properties.Name)
            if ($names -contains 'network_isolation') {
                if ($names -contains 'network_disabled' -or $Case.network_isolation -cne 'windows_firewall_runner_and_host_agents_only' -or $Case.network_isolated -isnot [bool] -or -not $Case.network_isolated -or $Case.external_connection_refused -isnot [bool] -or -not $Case.external_connection_refused -or $Case.network_restored -isnot [bool] -or -not $Case.network_restored) { throw 'Offline case requires verified firewall isolation and restoration.' }
            } elseif (@($names | Where-Object { $_ -cin @('network_isolated','external_connection_refused','allowed_programs','allowed_infrastructure_programs','dns_allowed') }).Count -ne 0) { throw 'Offline case mixes adapter and firewall evidence.'
            } elseif ($Case.network_disabled -isnot [bool] -or -not $Case.network_disabled) { throw 'Offline case requires an actually disabled network.' }
        }
        'HOSTED-PERFORMANCE' {
            if (-not (Test-HostedText $Case.workload) -or -not (Test-HostedText $Case.hardware) -or -not (Test-HostedText $Case.limitations)) { throw 'Performance requires workload/hardware/limitations.' }
            $metrics = @('install_ms','load_ms','activation_ms','browse_ms','planning_ms','peak_memory_bytes')
            $names = @($Case.measurements.PSObject.Properties.Name)
            if ($names.Count -ne $metrics.Count -or @($names | Where-Object { $_ -cnotin $metrics }).Count -ne 0) { throw 'Performance metric set differs.' }
            foreach ($name in $metrics) {
                $value = $Case.measurements.$name
                if (($value -isnot [int] -and $value -isnot [long] -and $value -isnot [double] -and $value -isnot [decimal]) -or [double]::IsNaN([double]$value) -or [double]::IsInfinity([double]$value) -or $value -lt 0 -or $value -gt 9007199254740991) { throw 'Performance measurement is missing or invalid.' }
            }
        }
    }
}
function New-HostedAcceptanceEvidence($Base, [string] $Version, [string] $SourceCommit, [string] $Production, [string] $Installer, [hashtable] $Cases = @{}) {
    if ($Base.schema_version -notin 4, 5 -or $SourceCommit -cnotmatch '^[a-f0-9]{40}$' -or $Production -cnotmatch '^[a-f0-9]{64}$' -or $Installer -cnotmatch '^[a-f0-9]{64}$' -or $Base.production_sha256 -cne $Production) { throw 'Hosted evidence requires bound stable production.' }
    $result = ConvertFrom-HostedJsonText ($Base | ConvertTo-Json -Depth 100)
    $finalId = if ($Base.schema_version -eq 5) { 'DRAFT-INSTALLER-SMOKE' } else { 'OWNER-INSTALLER-SMOKE' }
    $finalMethod = if ($Base.schema_version -eq 5) { 'automated_draft_windows' } else { 'interactive_windows' }
    $seen = New-Object 'Collections.Generic.HashSet[string]' ([StringComparer]::Ordinal)
    foreach ($row in $result.checks) {
        if (-not $seen.Add($row.id) -or ($row.id -cnotin $script:HostedAcceptanceIds -and $row.id -cne $finalId)) { throw 'Unexpected or repeated acceptance row.' }
        $expectedMethod = if ($row.id -ceq $finalId) { $finalMethod } else { 'hosted_windows' }
        if ($row.method -cne $expectedMethod) { throw 'Acceptance method differs from its stable schema.' }
    }
    if ($seen.Count -ne 9) { throw 'Stable acceptance rows are missing.' }
    foreach ($id in $Cases.Keys) { if ($id -cnotin $script:HostedAcceptanceIds) { throw 'Hosted recorder cannot create owner or unknown observations.' } }
    $previousHash = $result.PSObject.Properties['candidate_installer_sha256']
    if ($null -ne $previousHash -and $previousHash.Value -and $previousHash.Value -cne $Installer -and @($result.checks | Where-Object { $_.status -cne 'unverified' -or $_.operator -or $_.recorded_at -or $_.environment -or @($_.artifacts).Count -gt 0 }).Count -gt 0) { throw 'New installer invalidates prior observations; retain/reset them explicitly before preparing another candidate.' }
    $result | Add-Member -NotePropertyName candidate_installer_sha256 -NotePropertyValue $Installer -Force
    foreach ($row in $result.checks) {
        if (-not $Cases.ContainsKey($row.id)) { continue }
        $entry = $Cases[$row.id]
        Assert-HostedCase $entry.Result $row.id $Version $SourceCommit $Production $Installer $Base.target
        if ($entry.Artifact.bytes -le 0 -or $entry.Artifact.bytes -gt 16777216 -or $entry.Artifact.sha256 -cnotmatch '^[a-f0-9]{64}$' -or $entry.Artifact.path -notmatch '^[A-Za-z0-9_.\-/]+$' -or $entry.Artifact.path -match '(^|/)\.\.(/|$)' -or [IO.Path]::IsPathRooted($entry.Artifact.path)) { throw 'Hosted artifact reference is invalid.' }
        $row.status = $entry.Result.status; $row.operator = $entry.Result.operator; $row.environment = $entry.Result.environment; $row.recorded_at = $entry.Result.recorded_at
        $row.artifacts = @($entry.Artifact)
        $row | Add-Member -NotePropertyName installer_sha256 -NotePropertyValue $Installer -Force
    }
    return $result
}
