# Pure case construction plus bounded owned-node execution; no action on import.
Set-StrictMode -Version 3.0
# JSON object member order is not identity. Admit only the exact typed target shape.
function Assert-StableTargetIdentity($Expected, $Actual) {
    foreach ($target in @($Expected, $Actual)) {
        if ($target -isnot [pscustomobject]) { throw 'Stable target must be a JSON object.' }
        $names = @($target.PSObject.Properties.Name | Sort-Object)
        if (($names -join ',') -cne 'build,edition,interface,version') { throw 'Stable target fields differ from the exact contract.' }
        foreach ($name in @('edition','version','build')) {
            if ($target.$name -isnot [string] -or [string]::IsNullOrWhiteSpace($target.$name)) { throw 'Stable target string field is malformed.' }
        }
        if (($target.interface -isnot [int] -and $target.interface -isnot [long]) -or $target.interface -lt 1 -or $target.interface -gt 4294967295) { throw 'Stable target interface must be a bounded JSON integer.' }
    }
    foreach ($name in @('edition','version','build','interface')) {
        if ($Expected.$name -cne $Actual.$name) { throw 'Build identity differs from source-bound release target.' }
    }
}
function Invoke-StableCase([string] $Id, [hashtable] $Identity, [string] $Output, [scriptblock] $Action) {
    if ($Id -cnotin $script:HostedAcceptanceIds) { throw 'Hosted producer cannot create owner or unknown cases.' }
    $case = [ordered]@{schema_version=1;id=$Id;method='hosted_windows';status='fail';version=$Identity.version;source_commit=$Identity.source_commit;production_sha256=$Identity.production_sha256;installer_sha256=$Identity.installer_sha256;target=$Identity.target;operator=$Identity.operator;environment=$Identity.environment;recorded_at=[DateTime]::UtcNow.ToString('yyyy-MM-ddTHH:mm:ssZ')}
    try {
        $details = & $Action
        if ($details -isnot [hashtable]) { throw 'Hosted case did not return actual detailed results.' }
        foreach ($name in $details.Keys) {
            if ($case.Contains($name) -or $name -eq 'completed') { throw 'Hosted details cannot replace release identity or completion.' }
            $case[$name] = $details[$name]
        }
        $case.status = 'pass'; $case['completed'] = $true
        Assert-HostedCase ([pscustomobject]$case) $Id $Identity.version $Identity.source_commit $Identity.production_sha256 $Identity.installer_sha256 $Identity.target
    } catch { $case.status = 'fail'; $case['error'] = $_.Exception.Message; throw }
    finally {
        $text = ($case | ConvertTo-Json -Depth 100) + "`n"
        if ([Text.Encoding]::UTF8.GetByteCount($text) -gt 16777216) { throw 'Actual hosted case exceeds evidence capacity.' }
        [IO.File]::WriteAllText((Join-Path $Output ($Id.ToLowerInvariant()+'.json')), $text, (New-Object Text.UTF8Encoding($false)))
    }
}
function Invoke-StableWorkload([string] $Root,[string] $Exe,[string] $Manifest,[string] $Version,[string] $Mode,[string] $State,[string] $Report,[string] $Backup = '',[ValidateRange(1,1200)][int] $MaximumSeconds = 1200) {
    if ($PSVersionTable.PSVersion.Major -lt 7) { throw 'Bounded hosted workload requires PowerShell 7 argument-list support.' }
    $start = New-Object Diagnostics.ProcessStartInfo
    $start.FileName = (Get-Command node -ErrorAction Stop).Source; $start.UseShellExecute = $false
    foreach ($value in @((Join-Path $Root 'scripts/windows/Stable-Workload.mjs'),'--mode',$Mode,'--exe',$Exe,'--manifest',$Manifest,'--version',$Version,'--state',$State,'--out',$Report)) { $start.ArgumentList.Add($value) }
    if ($Backup) { $start.ArgumentList.Add('--backup'); $start.ArgumentList.Add($Backup) }
    $process = New-Object Diagnostics.Process
    $process.StartInfo = $start
    $started = $false
    $observations = @{}; $samples = 0; $samplingErrors = 0
    $peak = 0L; $timer = [Diagnostics.Stopwatch]::StartNew()
    try {
        if (-not $process.Start()) { throw 'Cannot start installed engineering workload.' }; $started = $true
        while (-not $process.WaitForExit(200)) {
            if ($timer.Elapsed.TotalSeconds -gt $MaximumSeconds) { throw 'Installed engineering workload exceeded its selected finite time bound.' }
            foreach ($native in @(Get-Process -Name azeroth-desktop -ErrorAction SilentlyContinue)) {
                try {
                    if ([string]::Equals($native.Path,$Exe,[StringComparison]::OrdinalIgnoreCase)) {
                        $native.Refresh(); $samples++
                        $previous = if ($observations.ContainsKey($native.Id)) { $observations[$native.Id] } else { 0L }
                        $observations[$native.Id] = [Math]::Max($previous, $native.PeakWorkingSet64)
                    }
                } catch { $samplingErrors++ } finally { $native.Dispose() }
            }
        }
        if ($process.ExitCode -ne 0) { throw 'Installed native engineering workload failed.' }
        $raw = Read-BoundedReleaseText $Report | ConvertFrom-Json
        if ($raw.passed -isnot [bool] -or -not $raw.passed -or $raw.version -cne $Version -or $raw.mode -cne $Mode) { throw 'Installed native workload report is incomplete.' }
        if ($observations.ContainsKey([int]$raw.pid)) { $peak = $observations[[int]$raw.pid] }
        return @{raw=$raw;peak_memory_bytes=$peak;memory_observation=@{pid=$raw.pid;samples=$samples;sampling_errors=$samplingErrors;target_interval_ms=200};memory_scope='Maximum observed PeakWorkingSet64 of the exact native app PID; WebView2 descendants and installer excluded. Samples target 200 ms; allocations after the last successful sample may be missed.'}
    } finally {
        if ($started -and -not $process.HasExited) { $process.Kill($true); $null = $process.WaitForExit(15000) }
        $process.Dispose()
    }
}
function Invoke-StableOffline([scriptblock] $Workload) {
    if ($env:GITHUB_ACTIONS -ne 'true' -or $env:RUNNER_ENVIRONMENT -ne 'github-hosted' -or $env:OS -ne 'Windows_NT') { throw 'Offline workload requires a disposable hosted Windows runner.' }
    # Restore the exact originally-up adapters; never enable others.
    $adapters = @(Get-NetAdapter -IncludeHidden -ErrorAction Stop | Where-Object Status -eq 'Up')
    if ($adapters.Count -eq 0) { throw 'No active adapter can be controlled to prove offline behavior.' }
    $disabled = @()
    try {
        foreach ($adapter in $adapters) {
            $disabled += $adapter.InterfaceDescription
            Disable-NetAdapter -InterfaceDescription $adapter.InterfaceDescription -Confirm:$false -ErrorAction Stop
        }
        if (@(Get-NetAdapter -IncludeHidden -ErrorAction Stop | Where-Object Status -eq 'Up').Count -ne 0) { throw 'External network adapters remain active.' }
        $result = & $Workload
    } finally {
        $failed = $false
        foreach ($description in $disabled) {
            try { Enable-NetAdapter -InterfaceDescription $description -Confirm:$false -ErrorAction Stop } catch { $failed = $true }
        }
        $restore = [Diagnostics.Stopwatch]::StartNew()
        do {
            $pending = @()
            foreach ($description in $disabled) {
                try { if (@(Get-NetAdapter -InterfaceDescription $description -IncludeHidden -ErrorAction Stop | Where-Object Status -eq 'Up').Count -ne 1) { $pending += $description } } catch { $failed = $true }
            }
            if ($pending.Count -gt 0) { Start-Sleep -Milliseconds 200 }
        } while ($pending.Count -gt 0 -and $restore.Elapsed.TotalSeconds -lt 60)
        if ($failed -or $pending.Count -gt 0) { throw 'Controlled adapters could not all be restored to their original active states.' }
    }
    return @{workload=$result;network_disabled=$true;restored=$true;adapters=$disabled}
}
