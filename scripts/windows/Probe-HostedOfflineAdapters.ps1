# Engineering network-control probe only; never an acceptance observation.
[CmdletBinding()]
param([string] $OutputDirectory = ([IO.Path]::Combine([IO.Path]::GetTempPath(),'navzeroth-offline-adapter-probe')))
Set-StrictMode -Version 3.0
$ErrorActionPreference = 'Stop'
. (Join-Path $PSScriptRoot 'Stable-Acceptance-Helpers.ps1')

function ConvertTo-ProbeAdapterGuid($Value) {
    $parsed = [Guid]::Empty
    if (($Value -isnot [string] -and $Value -isnot [Guid]) -or
        -not [Guid]::TryParse([string]$Value,[ref]$parsed) -or $parsed -eq [Guid]::Empty) { throw 'Probe adapter identity is invalid.' }
    return $parsed.ToString('D')
}
function Write-OfflineProbeReport($Report,[string] $Path) {
    $text = ($Report | ConvertTo-Json -Depth 24) + "`n"
    if ([Text.Encoding]::UTF8.GetByteCount($text) -gt 65536) { throw 'Probe report exceeds 64 KiB.' }
    [IO.File]::WriteAllText($Path,$text,(New-Object Text.UTF8Encoding($false)))
}
function Invoke-HostedOfflineAdapterProbe([string] $Directory) {
    if ($env:GITHUB_ACTIONS -ne 'true' -or $env:RUNNER_ENVIRONMENT -ne 'github-hosted' -or $env:OS -ne 'Windows_NT') { throw 'Probe requires a disposable GitHub-hosted Windows runner.' }
    if ($env:GITHUB_SHA -notmatch '^[0-9a-f]{40}$' -or $env:GITHUB_RUN_ID -notmatch '^[0-9]{1,20}$' -or $env:GITHUB_RUN_ATTEMPT -notmatch '^[0-9]{1,6}$') { throw 'Probe run identity is invalid.' }
    $root = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot '../..'))
    $source = (& git -C $root rev-parse HEAD).Trim()
    if ($LASTEXITCODE -ne 0 -or $source -cne $env:GITHUB_SHA) { throw 'Probe source differs from the issued checkout.' }
    $dirty = @(& git -C $root status --porcelain --untracked-files=normal)
    if ($LASTEXITCODE -ne 0 -or $dirty.Count -ne 0) { throw 'Probe requires a clean source checkout.' }
    $parent = [IO.Path]::GetFullPath($env:RUNNER_TEMP).TrimEnd([IO.Path]::DirectorySeparatorChar,[IO.Path]::AltDirectorySeparatorChar) + [IO.Path]::DirectorySeparatorChar
    $directoryPath = [IO.Path]::GetFullPath($Directory)
    if (-not $directoryPath.StartsWith($parent,[StringComparison]::OrdinalIgnoreCase) -or (Test-Path -LiteralPath $directoryPath)) { throw 'Probe output must be a new directory under RUNNER_TEMP.' }
    $null = New-Item -ItemType Directory -Path $directoryPath -ErrorAction Stop
    $reportPath = Join-Path $directoryPath 'probe.json'
    $marker = Join-Path $directoryPath 'offline-marker.txt'
    $timer = [Diagnostics.Stopwatch]::StartNew()
    $failure = $null
    $report = [ordered]@{schema_version=1;kind='hosted_offline_adapter_engineering_probe';acceptance_observations_promoted=$false;passed=$false;source_commit=$source;run_id=$env:GITHUB_RUN_ID;run_attempt=$env:GITHUB_RUN_ATTEMPT;runner_environment=$env:RUNNER_ENVIRONMENT;runner_os=$env:OS;powershell_version=$PSVersionTable.PSVersion.ToString();phase='initial_inventory';controlled_adapters=@();network_disabled=$null;restored=$null;elapsed_ms=0;error=$null}
    try {
        $original = @(Get-NetAdapter -IncludeHidden -ErrorAction Stop | Where-Object Status -eq 'Up')
        if ($original.Count -lt 1 -or $original.Count -gt 64) { throw 'Probe requires 1..64 originally-up adapters.' }
        $ids = @($original | ForEach-Object { ConvertTo-ProbeAdapterGuid $_.InterfaceGuid })
        if (@($ids | Sort-Object -Unique).Count -ne $ids.Count) { throw 'Probe initial adapter identities repeat.' }
        $report.controlled_adapters = $ids
        $report.phase = 'offline_workload'
        $offline = Invoke-StableOffline {
            if (@(Get-NetAdapter -IncludeHidden -ErrorAction Stop | Where-Object Status -eq 'Up').Count -ne 0) { throw 'Probe observed an active adapter during offline work.' }
            [IO.File]::WriteAllText($marker,"network-disabled-local-marker`n",(New-Object Text.UTF8Encoding($false)))
            return @{marker_written=$true;up_count=0}
        }
        if ($offline.network_disabled -isnot [bool] -or -not $offline.network_disabled -or $offline.restored -isnot [bool] -or -not $offline.restored) { throw 'Probe helper did not establish disablement and restoration.' }
        $actual = @($offline.adapters)
        if ($actual.Count -ne $ids.Count -or $actual.Count -gt 64) { throw 'Probe helper adapter identities differ.' }
        $actual = @($actual | ForEach-Object { ConvertTo-ProbeAdapterGuid $_ })
        if (@($actual | Sort-Object -Unique).Count -ne $actual.Count -or (@($actual | Sort-Object) -join ',') -cne (@($ids | Sort-Object) -join ',')) { throw 'Probe helper adapter identities differ.' }
        if ($offline.workload.marker_written -isnot [bool] -or -not $offline.workload.marker_written -or $offline.workload.up_count -ne 0 -or -not (Test-Path -LiteralPath $marker) -or (Get-Item -LiteralPath $marker).Length -gt 128 -or [IO.File]::ReadAllText($marker) -cne "network-disabled-local-marker`n") { throw 'Probe local offline workload was not completed.' }
        $report.phase = 'post_restore'
        $after = @(Get-NetAdapter -IncludeHidden -ErrorAction Stop)
        foreach ($id in $ids) {
            $matches = @($after | Where-Object { (ConvertTo-ProbeAdapterGuid $_.InterfaceGuid) -ceq $id })
            if ($matches.Count -ne 1 -or $matches[0].Status -ne 'Up') { throw 'Probe independently observed an unrestored adapter.' }
        }
        $report.network_disabled = $true
        $report.restored = $true
        $report.passed = $true
        $report.phase = 'complete'
    } catch {
        $message = [string]$_.Exception.Message
        $bytes = [Text.Encoding]::UTF8.GetBytes($message)
        if ($bytes.Length -gt 8192) { $message = [Text.Encoding]::UTF8.GetString($bytes,0,8192) + ' [truncated]' }
        $report.error = $message
        $failure = $_
    } finally {
        $report.elapsed_ms = $timer.Elapsed.TotalMilliseconds
        try { Write-OfflineProbeReport $report $reportPath } catch {
            # An output error must not hide the network-control failure.
            if ($null -eq $failure) { $failure = $_ }
        }
    }
    if ($null -ne $failure) { throw $failure }
    return $report
}
if ($MyInvocation.InvocationName -ne '.') {
    Invoke-HostedOfflineAdapterProbe $OutputDirectory | ConvertTo-Json -Depth 24
}
