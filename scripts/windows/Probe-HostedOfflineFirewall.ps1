# Engineering firewall-isolation probe only; never an acceptance observation.
# Proves on a real hosted runner that the job survives isolation longer than the
# full-pack offline workload while external connections stay refused (ADR 0010).
[CmdletBinding()]
param(
    [string] $OutputDirectory = ([IO.Path]::Combine([IO.Path]::GetTempPath(),'navzeroth-offline-firewall-probe')),
    [ValidateRange(60,1500)][int] $HoldSeconds = 1260
)
Set-StrictMode -Version 3.0
$ErrorActionPreference = 'Stop'
. (Join-Path $PSScriptRoot 'Stable-Acceptance-Helpers.ps1')

function Write-FirewallProbeReport($Report,[string] $Path) {
    $text = ($Report | ConvertTo-Json -Depth 24) + "`n"
    if ([Text.Encoding]::UTF8.GetByteCount($text) -gt 65536) { throw 'Probe report exceeds 64 KiB.' }
    [IO.File]::WriteAllText($Path,$text,(New-Object Text.UTF8Encoding($false)))
}
# Which processes hold external connections before isolation. Names only, bounded;
# this tells a failed probe which agent the allowlist missed.
function Get-ExternalConnectionOwners {
    $owners = @{}
    foreach ($connection in @(Get-NetTCPConnection -State Established -ErrorAction SilentlyContinue | Select-Object -First 512)) {
        $remote = [string]$connection.RemoteAddress
        if ($remote -in @('127.0.0.1','::1') -or $remote.StartsWith('127.')) { continue }
        $name = try { (Get-Process -Id $connection.OwningProcess -ErrorAction Stop).ProcessName } catch { 'unknown' }
        if ($name.Length -gt 64) { $name = $name.Substring(0,64) }
        $owners[$name] = 1 + $(if ($owners.ContainsKey($name)) { $owners[$name] } else { 0 })
    }
    return @($owners.Keys | Sort-Object | Select-Object -First 32 | ForEach-Object { [ordered]@{process=$_;connections=$owners[$_]} })
}
function Test-LoopbackTcp {
    $listener = New-Object Net.Sockets.TcpListener ([Net.IPAddress]::Loopback),0
    $listener.Start()
    try { return (Test-StableExternalTcp '127.0.0.1' ([Net.IPEndPoint]$listener.LocalEndpoint).Port 3000) } finally { $listener.Stop() }
}
function Invoke-HostedOfflineFirewallProbe([string] $Directory,[int] $Hold) {
    if ($env:GITHUB_ACTIONS -ne 'true' -or $env:RUNNER_ENVIRONMENT -ne 'github-hosted' -or $env:OS -ne 'Windows_NT') { throw 'Probe requires a disposable GitHub-hosted Windows runner.' }
    if ($env:GITHUB_SHA -notmatch '^[0-9a-f]{40}$' -or $env:GITHUB_RUN_ID -notmatch '^[0-9]{1,20}$' -or $env:GITHUB_RUN_ATTEMPT -notmatch '^[0-9]{1,6}$') { throw 'Probe run identity is invalid.' }
    $root = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot '../..'))
    $source = (& git -C $root rev-parse HEAD).Trim()
    if ($LASTEXITCODE -ne 0 -or $source -cne $env:GITHUB_SHA) { throw 'Probe source differs from the issued checkout.' }
    $parent = [IO.Path]::GetFullPath($env:RUNNER_TEMP).TrimEnd([IO.Path]::DirectorySeparatorChar,[IO.Path]::AltDirectorySeparatorChar) + [IO.Path]::DirectorySeparatorChar
    $directoryPath = [IO.Path]::GetFullPath($Directory)
    if (-not $directoryPath.StartsWith($parent,[StringComparison]::OrdinalIgnoreCase) -or (Test-Path -LiteralPath $directoryPath)) { throw 'Probe output must be a new directory under RUNNER_TEMP.' }
    $null = New-Item -ItemType Directory -Path $directoryPath -ErrorAction Stop
    $reportPath = Join-Path $directoryPath 'probe.json'
    $timer = [Diagnostics.Stopwatch]::StartNew()
    $report = [ordered]@{schema_version=1;kind='hosted_offline_firewall_engineering_probe';acceptance_observations_promoted=$false;passed=$false;source_commit=$source;run_id=$env:GITHUB_RUN_ID;run_attempt=$env:GITHUB_RUN_ATTEMPT;runner_os=$env:OS;powershell_version=$PSVersionTable.PSVersion.ToString();hold_seconds=$Hold;phase='inventory';external_connection_owners=@();checks=0;isolation=$null;elapsed_ms=0;error=$null}
    $failure = $null
    try {
        $report.external_connection_owners = @(Get-ExternalConnectionOwners)
        # Written before isolation, so a lost job still leaves this inventory.
        Write-FirewallProbeReport $report $reportPath
        $report.phase = 'isolated_hold'
        $offline = Invoke-StableFirewallOffline {
            $holdTimer = [Diagnostics.Stopwatch]::StartNew()
            $address = Resolve-StableProbeAddress 'github.com'
            while ($holdTimer.Elapsed.TotalSeconds -lt $Hold) {
                if (Test-StableExternalTcp $address 443 3000) { throw 'External connection succeeded during the isolated hold.' }
                if (-not (Test-LoopbackTcp)) { throw 'Loopback connection failed during isolation.' }
                $report.checks++
                Start-Sleep -Seconds 15
            }
            return @{held_ms=$holdTimer.ElapsedMilliseconds}
        }
        $report.isolation = [ordered]@{isolation=$offline.isolation;allowed_programs=$offline.allowed_programs;allowed_infrastructure_programs=$offline.allowed_infrastructure_programs;held_ms=$offline.workload.held_ms;diagnostics=$offline.diagnostics}
        $report.passed = $true
        $report.phase = 'complete'
    } catch {
        $message = [string]$_.Exception.Message
        if ($message.Length -gt 8192) { $message = $message.Substring(0,8192) + ' [truncated]' }
        $report.error = $message
        $failure = $_
    } finally {
        $report.elapsed_ms = $timer.Elapsed.TotalMilliseconds
        try { Write-FirewallProbeReport $report $reportPath } catch { if ($null -eq $failure) { $failure = $_ } }
    }
    if ($null -ne $failure) { throw $failure }
    return $report
}
if ($MyInvocation.InvocationName -ne '.') {
    Invoke-HostedOfflineFirewallProbe $OutputDirectory $HoldSeconds | ConvertTo-Json -Depth 24
}
