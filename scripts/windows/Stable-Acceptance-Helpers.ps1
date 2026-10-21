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
# Match the JSON metadata object shape expected by the strict case validator.
# Do not coerce measurements: missing, nonnumeric and out-of-range values refuse.
function New-StablePerformanceMeasurements($InstallMilliseconds, $Workload) {
    return [pscustomobject][ordered]@{
        install_ms = $InstallMilliseconds
        load_ms = $Workload.raw.startup_measurements.load_ms
        activation_ms = $Workload.raw.startup_measurements.activation_ms
        browse_ms = $Workload.raw.browse_ms
        planning_ms = $Workload.raw.planning_ms
        peak_memory_bytes = $Workload.peak_memory_bytes
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
            if ($timer.Elapsed.TotalSeconds -gt $MaximumSeconds) {
                # Persist before the finally block kills Node: its finally/raw
                # report may never run. Partial diagnostics are never a case pass.
                $pids = @($observations.Keys | Sort-Object | Select-Object -First 16 | ForEach-Object { @{pid=[int]$_;peak_memory_bytes=$observations[$_]} })
                $watchdog = [ordered]@{schema_version=1;kind='workload_watchdog';passed=$false;completed=$false;mode=$Mode;version=$Version;reason='selected_time_bound_exceeded';maximum_seconds=$MaximumSeconds;elapsed_ms=$timer.Elapsed.TotalMilliseconds;samples=$samples;sampling_errors=$samplingErrors;native_pids=$pids;native_pids_truncated=($observations.Count -gt 16);partial_progress_file=([IO.Path]::GetFileName($Report)+'.progress.json')}
                $text=($watchdog | ConvertTo-Json -Depth 10)+"`n"
                if ([Text.Encoding]::UTF8.GetByteCount($text) -gt 16384) { throw 'Watchdog diagnostics exceed bounded capacity.' }
                [IO.File]::WriteAllText($Report+'.watchdog.json',$text,(New-Object Text.UTF8Encoding($false)))
                throw 'Installed engineering workload exceeded its selected finite time bound.'
            }
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
    function Guid-Of($Adapter) {
        $parsed=[Guid]::Empty
        if (-not [Guid]::TryParse([string]$Adapter.InterfaceGuid,[ref]$parsed) -or $parsed -eq [Guid]::Empty) { throw 'Adapter has no valid stable interface GUID.' }
        return $parsed.ToString('D')
    }
    function Inventory {
        $items=@(Get-NetAdapter -IncludeHidden -ErrorAction Stop)
        if ($items.Count -gt 64) { throw 'Adapter inventory exceeds bounded capacity.' }
        return $items
    }
    function Exact-Adapter([string]$Id) {
        $found=@(foreach ($item in @(Inventory)) {
            # A new unrelated malformed identity must not prevent restoration
            # of a known controlled NIC. The target still needs exactly one match.
            $candidate=$null; try { $candidate=Guid-Of $item } catch { }
            if ($candidate -ceq $Id) { $item }
        })
        if ($found.Count -ne 1) { throw 'Stable adapter GUID is absent or ambiguous.' }
        return $found[0]
    }
    function Bound-DiagnosticText([string]$Text,[int]$MaxBytes,[int]$MaxChars) {
        $length=$Text.Length; $truncated=$length -gt $MaxChars
        if ($truncated) { $length=$MaxChars-11 }
        do {
            # Do not end a retained prefix inside a UTF16 surrogate pair.
            if ($length -gt 0 -and [char]::IsHighSurrogate($Text[$length-1])) { $length--; $truncated=$true }
            $value=$Text.Substring(0,$length)
            if ($truncated) { $value+='[truncated]' }
            if ([Text.Encoding]::UTF8.GetByteCount(($value|ConvertTo-Json -Compress)) -le $MaxBytes) { return $value }
            $length--; $truncated=$true
        } while ($length -ge 0)
        return '[truncated]'
    }
    function Short-Error($Failure) {
        return Bound-DiagnosticText ([string]$Failure.Exception.Message) 320 256
    }
    # Descriptions/names can collide or contain wildcard syntax. Resolve each
    # originally-up GUID to one freshly issued CIM instance, including hidden NICs.
    $inventory=@(Inventory)
    $inventoryIds=@(); foreach ($adapter in $inventory) { $inventoryIds+=Guid-Of $adapter }
    if (@($inventoryIds | Sort-Object -Unique).Count -ne $inventoryIds.Count) { throw 'Adapter inventory GUIDs are ambiguous.' }
    $adapters=@($inventory | Where-Object Status -eq 'Up')
    if ($adapters.Count -eq 0) { throw 'No active adapter can be controlled to prove offline behavior.' }
    $ids=@(); foreach ($adapter in $adapters) { $ids+=Guid-Of $adapter }
    if (@($ids | Sort-Object -Unique).Count -ne $ids.Count) { throw 'Originally active adapter GUIDs are ambiguous.' }
    $attempted=@(); $primary=$null; $rows=@(); $result=$null; $stage='disable'; $current=$null
    try {
        foreach ($id in $ids) {
            $current=$id
            $issued=Exact-Adapter $id
            # Append before mutation: even a partially successful cmdlet must be restored.
            $attempted+=$id
            Disable-NetAdapter -InputObject $issued -Confirm:$false -ErrorAction Stop
        }
        $stage='verify_offline'; $current=$null
        if (@(Inventory | Where-Object Status -eq 'Up').Count -ne 0) { throw 'External network adapters remain active.' }
        $stage='workload'; $result=& $Workload
    } catch {
        $primary=[ordered]@{stage=$stage;interface_guid=$current;error=(Short-Error $_)}
    } finally {
        foreach ($id in $attempted) {
            $row=[ordered]@{interface_guid=$id;enable_error=$null;state_error=$null;status=$null}
            try { $issued=Exact-Adapter $id; Enable-NetAdapter -InputObject $issued -Confirm:$false -ErrorAction Stop } catch { $row.enable_error=Short-Error $_ }
            $rows+=,$row
        }
        $restore=[Diagnostics.Stopwatch]::StartNew()
        do {
            $pending=0
            foreach ($row in $rows) {
                try {
                    $issued=Exact-Adapter $row.interface_guid
                    $state=[string]$issued.Status
                    if ([string]::IsNullOrWhiteSpace($state) -or $state.Length -gt 64) { throw 'Adapter restoration status is malformed.' }
                    $row.status=Bound-DiagnosticText $state 128 64
                    if ($state -cne 'Up') { $pending++ }
                } catch { $row.state_error=Short-Error $_; $pending++ }
            }
            if ($pending -gt 0) { Start-Sleep -Milliseconds 200 }
        } while ($pending -gt 0 -and $restore.Elapsed.TotalSeconds -lt 60)
    }
    $cleanupFailed=@($rows | Where-Object { $null -ne $_.enable_error -or $null -ne $_.state_error -or $_.status -cne 'Up' }).Count -gt 0
    $diagnostics=[ordered]@{primary=$primary;restoration=$rows;restore_elapsed_ms=$restore.ElapsedMilliseconds}
    if ($null -ne $primary -or $cleanupFailed) {
        # Preserve both failure paths; cleanup must never replace the original cause.
        $text=$diagnostics|ConvertTo-Json -Depth 6 -Compress
        if ([Text.Encoding]::UTF8.GetByteCount($text) -gt 65536) { throw 'Offline adapter diagnostics exceed bounded capacity.' }
        throw ('Hosted offline control failed: '+$text)
    }
    return @{workload=$result;network_disabled=$true;restored=$true;adapters=$ids;diagnostics=$diagnostics}
}
