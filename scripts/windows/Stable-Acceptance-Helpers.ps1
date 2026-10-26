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
function Get-StableAppDataDirectory([string] $Identifier) {
    if ($Identifier -cnotmatch '^[a-z0-9]+(\.[a-z0-9-]+)+$') { throw 'Invalid application identifier.' }
    # The app resolves the Roaming known folder (SHGetKnownFolderPath), which
    # ignores $env:APPDATA; overriding that variable never isolated its data.
    $base = [Environment]::GetFolderPath([Environment+SpecialFolder]::ApplicationData, [Environment+SpecialFolderOption]::DoNotVerify)
    if (-not [IO.Path]::IsPathRooted($base)) { throw 'Application data folder is unavailable.' }
    return Join-Path $base $Identifier
}
function Move-StableAppDataAside([string] $Data, [string] $Destination) {
    if (Test-Path -LiteralPath $Destination) { throw 'Application data destination already exists.' }
    if (-not (Test-Path -LiteralPath $Data)) { return $false }
    if (-not (Test-Path -LiteralPath $Data -PathType Container)) { throw 'Application data is not a directory.' }
    # Retained, never deleted: the earlier cases' database stays inspectable.
    Move-Item -LiteralPath $Data -Destination $Destination
    if (Test-Path -LiteralPath $Data) { throw 'Application data remained after moving it aside.' }
    return $true
}
function Invoke-StableWorkload([string] $Root,[string] $Exe,[string] $Manifest,[string] $Version,[string] $Mode,[string] $State,[string] $Report,[string] $Backup = '',[ValidateRange(1,1200)][int] $MaximumSeconds = 1200,[ValidateSet('stable','draft-ui')][string] $Driver = 'stable',[string] $SourceCommit = '',[string] $InstallerSha256 = '') {
    if ($PSVersionTable.PSVersion.Major -lt 7) { throw 'Bounded hosted workload requires PowerShell 7 argument-list support.' }
    if ($Driver -ceq 'draft-ui' -and ($Mode -cnotin @('fresh','restart') -or $SourceCommit -cnotmatch '^[a-f0-9]{40}$' -or $InstallerSha256 -cnotmatch '^[a-f0-9]{64}$' -or $Backup)) { throw 'Draft UI workload requires exact source/installer identity and fresh/restart mode.' }
    $start = New-Object Diagnostics.ProcessStartInfo
    $start.FileName = (Get-Command node -ErrorAction Stop).Source; $start.UseShellExecute = $false
    $scriptPath = if ($Driver -ceq 'draft-ui') { 'scripts/windows/Draft-Installer-Workload.mjs' } else { 'scripts/windows/Stable-Workload.mjs' }
    foreach ($value in @((Join-Path $Root $scriptPath),'--mode',$Mode,'--exe',$Exe,'--manifest',$Manifest,'--version',$Version,'--state',$State,'--out',$Report)) { $start.ArgumentList.Add($value) }
    if ($Driver -ceq 'draft-ui') { foreach ($value in @('--source-commit',$SourceCommit,'--installer-sha256',$InstallerSha256)) { $start.ArgumentList.Add($value) } }
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
        if ($Driver -ceq 'draft-ui' -and ($raw.source_commit -cne $SourceCommit -or $raw.installer_sha256 -cne $InstallerSha256 -or $raw.phase -cne 'draft' -or $raw.completed -isnot [bool] -or -not $raw.completed -or $raw.human_performed -isnot [bool] -or $raw.human_performed -or $raw.ui_interaction_verified -isnot [bool] -or -not $raw.ui_interaction_verified)) { throw 'Draft UI report lacks exact identity or completed native UI evidence.' }
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
# Hosted runners lose their job about 4.5 minutes after every adapter is disabled
# (runs 39641767585/1 and 39931563703/1-2), shorter than the full-pack offline
# workload. ADR 0010: isolate by firewall instead. Both directions default to
# block, every enabled allow rule is disabled, and only the runner that holds
# the job, the hosted VM's own agents and the system DNS client stay allowed.
$script:StableFirewallIsolation = 'windows_firewall_runner_and_host_agents_only'
function Get-StableRunnerAgentPrograms {
    $found = [ordered]@{}; $seen = @{}; $current = $PID
    for ($depth = 0; $depth -lt 32 -and $current -gt 0 -and -not $seen.ContainsKey($current); $depth++) {
        $seen[$current] = $true
        $process = @(Get-CimInstance Win32_Process -Filter ('ProcessId=' + [int]$current) -ErrorAction Stop)
        if ($process.Count -ne 1) { break }
        $name = [string]$process[0].Name
        if ($name -cin @('Runner.Worker.exe','Runner.Listener.exe')) {
            if ($found.Contains($name)) { throw 'Runner agent ancestry repeats.' }
            $found[$name] = ConvertTo-StableLongPath ([string]$process[0].ExecutablePath)
        }
        $current = [int]$process[0].ParentProcessId
    }
    if ($found.Count -ne 2) { throw 'Runner.Listener and Runner.Worker ancestry is unavailable.' }
    foreach ($path in $found.Values) {
        if (-not [IO.Path]::IsPathRooted($path) -or $path.Length -gt 260 -or -not (Test-Path -LiteralPath $path -PathType Leaf)) { throw 'Runner agent executable path is invalid.' }
    }
    return @($found.Values)
}
# Hosted VM infrastructure agents. Allowing only the runner kept the job for
# 600 s (run 39972424100) but lost it during a 1260 s hold (run 39975150461),
# so the VM's own health/provisioning agents need their connections too. They
# are matched by process name and recorded by exact path; none runs test code.
# Windows Firewall refuses 8.3 short program paths such as C:\Users\RUNNER~1
# ("The parameter is incorrect", run 39982567901) and accepts their long form.
function ConvertTo-StableLongPath([string] $Path) {
    if (-not ('NavzerothStable.LongPath' -as [type])) {
        Add-Type -Namespace NavzerothStable -Name LongPath -MemberDefinition '[DllImport("kernel32.dll", CharSet = CharSet.Unicode, SetLastError = true)] public static extern uint GetLongPathName(string shortPath, System.Text.StringBuilder longPath, uint size);'
    }
    $buffer = New-Object Text.StringBuilder 1024
    $length = [NavzerothStable.LongPath]::GetLongPathName($Path, $buffer, 1024)
    if ($length -eq 0 -or $length -ge 1024) { throw 'Agent program path has no bounded long form.' }
    return $buffer.ToString()
}
function Get-StableHostedInfrastructurePrograms {
    $paths = @(Get-Process -ErrorAction SilentlyContinue | Where-Object { $_.ProcessName -in @('hosted-compute-agent','WindowsAzureGuestAgent','WaAppAgent') -or $_.ProcessName -like 'provjobd*' } | ForEach-Object { [string]$_.Path } | Where-Object { $_ } | ForEach-Object { ConvertTo-StableLongPath $_ } | Sort-Object -Unique)
    if ($paths.Count -gt 8) { throw 'Hosted infrastructure agent inventory exceeds bounded capacity.' }
    foreach ($path in $paths) {
        if (-not [IO.Path]::IsPathRooted($path) -or $path.Length -gt 260 -or -not (Test-Path -LiteralPath $path -PathType Leaf)) { throw 'Hosted infrastructure agent path is invalid.' }
        if (Test-StableAllowAdmitsWorkload @{program=$path;service='Any'}) { throw 'Hosted infrastructure agent matches a workload executable.' }
    }
    return $paths
}
function Resolve-StableProbeAddress([string] $HostName) {
    $addresses = @([Net.Dns]::GetHostAddresses($HostName) | Where-Object AddressFamily -eq ([Net.Sockets.AddressFamily]::InterNetwork))
    if ($addresses.Count -eq 0) { throw 'External probe host has no IPv4 address.' }
    return $addresses[0].ToString()
}
function Test-StableExternalTcp([string] $Address, [int] $Port, [int] $TimeoutMs = 5000) {
    $client = New-Object Net.Sockets.TcpClient
    try {
        $pending = $client.ConnectAsync($Address, $Port)
        try { return ($pending.Wait($TimeoutMs) -and $client.Connected) } catch { return $false }
    } finally { $client.Dispose() }
}
function Get-StableFirewallProfiles([string] $Store) {
    $items = @(Get-NetFirewallProfile -PolicyStore $Store -ErrorAction Stop)
    if ($items.Count -ne 3) { throw 'Firewall profile inventory differs from Domain/Private/Public.' }
    return @($items | Sort-Object Name | ForEach-Object { [ordered]@{name=[string]$_.Name;enabled=[string]$_.Enabled;inbound=[string]$_.DefaultInboundAction;outbound=[string]$_.DefaultOutboundAction} })
}
function Get-StableEnabledAllowRules {
    $names = @(Get-NetFirewallRule -PolicyStore PersistentStore -Enabled True -Action Allow -ErrorAction SilentlyContinue | ForEach-Object { [string]$_.Name })
    if ($names.Count -gt 8192) { throw 'Firewall rule inventory exceeds bounded capacity.' }
    return @($names | Sort-Object -Unique)
}
function Limit-StableDiagnostic($Failure) {
    $text = [string]$Failure.Exception.Message
    if ($text.Length -gt 256) { $text = $text.Substring(0, 245) + '[truncated]' }
    return $text
}
# Enabled outbound allow rules in the effective store, outside the isolation group.
# PersistentStore rules are disabled, but other stores (service hardening, GPO)
# can still allow traffic. Each returned item names its program and service.
function Get-StableActiveAllowInventory([string] $Group, [int] $OwnRules) {
    # Fail closed: the isolation group's own rules must be visible, so an empty
    # or failed query cannot pass as "no other allow rules".
    $all = @(Get-NetFirewallRule -PolicyStore ActiveStore -Enabled True -Action Allow -Direction Outbound -ErrorAction Stop)
    if (@($all | Where-Object { [string]$_.Group -ceq $Group }).Count -ne $OwnRules) { throw 'Effective policy does not show exactly the isolation rules.' }
    $rules = @($all | Where-Object { [string]$_.Group -cne $Group })
    if ($rules.Count -gt 256) { throw 'Effective outbound allow rule inventory exceeds bounded capacity.' }
    return @(foreach ($rule in $rules) {
        $program = [string](@($rule | Get-NetFirewallApplicationFilter -ErrorAction Stop)[0].Program)
        $service = [string](@($rule | Get-NetFirewallServiceFilter -ErrorAction Stop)[0].Service)
        [ordered]@{program=$program;service=$service}
    })
}
# A rule admits the test processes when it is unscoped (any program, any service)
# or names an executable the offline workload runs.
function Test-StableAllowAdmitsWorkload($Item) {
    $program = [string]$Item.program; $service = [string]$Item.service
    $unscopedProgram = [string]::IsNullOrWhiteSpace($program) -or $program -ceq 'Any'
    $unscopedService = [string]::IsNullOrWhiteSpace($service) -or $service -ceq 'Any'
    if ($unscopedProgram) { return $unscopedService }
    $leaf = $program.Replace('/','\').Split('\')[-1]
    return $leaf -in @('azeroth-desktop.exe','msedgewebview2.exe','node.exe','pwsh.exe','powershell.exe')
}
function Invoke-StableFirewallOffline([scriptblock] $Workload, [string] $ProbeHost = 'github.com', [int] $ProbePort = 443, [ValidateRange(1,60)][int] $RestoreSeconds = 60) {
    if ($env:GITHUB_ACTIONS -ne 'true' -or $env:RUNNER_ENVIRONMENT -ne 'github-hosted' -or $env:OS -ne 'Windows_NT') { throw 'Offline workload requires a disposable hosted Windows runner.' }
    # Everything below is read-only until the first firewall change.
    $agents = @(Get-StableRunnerAgentPrograms)
    $infrastructure = @(Get-StableHostedInfrastructurePrograms)
    $address = Resolve-StableProbeAddress $ProbeHost
    if (-not (Test-StableExternalTcp $address $ProbePort)) { throw 'External network is unavailable before isolation; offline behavior cannot be proven.' }
    $profiles = @(Get-StableFirewallProfiles 'PersistentStore')
    $allow = @(Get-StableEnabledAllowRules)
    $group = 'Navzeroth hosted offline isolation ' + [Guid]::NewGuid().ToString('N')
    $stage = 'isolate'; $primary = $null; $result = $null; $disabled = @(); $changed = $false; $remainingAllow = $null; $activeAllow = $null
    try {
        $changed = $true
        foreach ($program in @($agents + $infrastructure)) {
            New-NetFirewallRule -DisplayName ($group + ' agent') -Group $group -Direction Outbound -Action Allow -Program $program -PolicyStore PersistentStore -ErrorAction Stop | Out-Null
        }
        foreach ($protocol in @('UDP','TCP')) {
            New-NetFirewallRule -DisplayName ($group + ' dns') -Group $group -Direction Outbound -Action Allow -Program '%SystemRoot%\System32\svchost.exe' -Service Dnscache -Protocol $protocol -RemotePort 53 -PolicyStore PersistentStore -ErrorAction Stop | Out-Null
        }
        # Record before mutation: a partial batch must still be restored, and
        # re-enabling a rule that stayed enabled is harmless.
        $disabled = $allow
        if ($allow.Count -gt 0) { Disable-NetFirewallRule -Name $allow -PolicyStore PersistentStore -ErrorAction Stop }
        Set-NetFirewallProfile -Name Domain,Private,Public -Enabled True -DefaultInboundAction Block -DefaultOutboundAction Block -PolicyStore PersistentStore -ErrorAction Stop
        $stage = 'verify_offline'
        foreach ($active in @(Get-StableFirewallProfiles 'ActiveStore')) {
            if ($active.enabled -cne 'True' -or $active.inbound -cne 'Block' -or $active.outbound -cne 'Block') { throw 'Effective firewall profile is not default-deny.' }
        }
        # Only the isolation group's own agent and DNS rules may remain enabled.
        $remainingAllow = @(Get-StableEnabledAllowRules).Count
        if ($remainingAllow -ne $agents.Count + $infrastructure.Count + 2) { throw 'Allow rules other than the runner agent, hosted infrastructure and DNS remain enabled.' }
        $inventory = @(Get-StableActiveAllowInventory $group ($agents.Count + $infrastructure.Count + 2))
        $activeAllow = [ordered]@{count=$inventory.Count;programs=@($inventory | ForEach-Object { ([string]$_.program).Replace('/','\').Split('\')[-1] } | Sort-Object -Unique | Select-Object -First 32)}
        if (@($inventory | Where-Object { Test-StableAllowAdmitsWorkload $_ }).Count -ne 0) { throw 'An effective outbound allow rule admits the offline workload processes.' }
        if (Test-StableExternalTcp $address $ProbePort) { throw 'External connection still succeeds under isolation.' }
        $stage = 'workload'; $result = & $Workload
        $stage = 'verify_after_workload'
        if (Test-StableExternalTcp $address $ProbePort) { throw 'External connection succeeded after the workload under isolation.' }
    } catch {
        $primary = [ordered]@{stage=$stage;error=(Limit-StableDiagnostic $_)}
    } finally {
        $restoration = [ordered]@{profiles=$null;rules_enabled=$null;rules_removed=$null;verification=$null;connectivity=$null}
        if ($changed) {
            # Lift default-deny first: until then the runner relies on the
            # isolation group, which therefore goes last.
            try {
                foreach ($saved in $profiles) { Set-NetFirewallProfile -Name $saved.name -Enabled $saved.enabled -DefaultInboundAction $saved.inbound -DefaultOutboundAction $saved.outbound -PolicyStore PersistentStore -ErrorAction Stop }
                $restoration.profiles = 'ok'
            } catch { $restoration.profiles = Limit-StableDiagnostic $_ }
            $enableErrors = 0
            if ($disabled.Count -gt 0) {
                try { Enable-NetFirewallRule -Name $disabled -PolicyStore PersistentStore -ErrorAction Stop } catch {
                    # Retry individually so one vanished rule cannot leave the rest disabled.
                    foreach ($name in $disabled) { try { Enable-NetFirewallRule -Name $name -PolicyStore PersistentStore -ErrorAction Stop } catch { $enableErrors++ } }
                }
            }
            $restoration.rules_enabled = if ($enableErrors -eq 0) { 'ok' } else { [string]$enableErrors + ' rule(s) not re-enabled' }
            if ($restoration.profiles -cne 'ok') {
                # Still default-deny: keep the runner's own rules so the job can
                # report this failure instead of losing its connection.
                $restoration.rules_removed = 'kept because profiles were not restored'
            } else {
                try {
                    # Removing an empty group throws; a failed first rule creation leaves none.
                    if (@(Get-NetFirewallRule -Group $group -PolicyStore PersistentStore -ErrorAction SilentlyContinue).Count -gt 0) { Remove-NetFirewallRule -Group $group -PolicyStore PersistentStore -ErrorAction Stop }
                    $restoration.rules_removed = 'ok'
                } catch { $restoration.rules_removed = Limit-StableDiagnostic $_ }
            }
        }
    }
    $verify = [Diagnostics.Stopwatch]::StartNew()
    if ($changed) {
        # Independently re-read the restored state instead of trusting cmdlet success.
        try {
            # Keep an earlier, more specific cause when one was already recorded.
            if ($restoration.profiles -ceq 'ok' -and (@(Get-StableFirewallProfiles 'PersistentStore') | ConvertTo-Json -Compress) -cne ($profiles | ConvertTo-Json -Compress)) { $restoration.profiles = 'restored profiles differ from the snapshot' }
            $enabledNow = @(Get-StableEnabledAllowRules)
            $missing = @($allow | Where-Object { $_ -cnotin $enabledNow }).Count
            if ($missing -ne 0 -and $restoration.rules_enabled -ceq 'ok') { $restoration.rules_enabled = [string]$missing + ' snapshot rule(s) not enabled' }
            if ($restoration.rules_removed -ceq 'ok' -and @(Get-NetFirewallRule -Group $group -PolicyStore PersistentStore -ErrorAction SilentlyContinue).Count -ne 0) { $restoration.rules_removed = 'isolation rules remain' }
            $restoration.verification = 'ok'
        } catch { $restoration.verification = Limit-StableDiagnostic $_ }
    }
    $verify.Stop()
    $restore = [Diagnostics.Stopwatch]::StartNew()
    if ($changed) {
        $online = $false
        do { $online = Test-StableExternalTcp $address $ProbePort; if (-not $online) { Start-Sleep -Milliseconds 500 } } while (-not $online -and $restore.Elapsed.TotalSeconds -lt $RestoreSeconds)
        $restoration.connectivity = if ($online) { 'ok' } else { ('external connection did not return within ' + $RestoreSeconds + ' s') }
    }
    $cleanupFailed = $changed -and @($restoration.Values | Where-Object { $_ -cne 'ok' }).Count -gt 0
    $diagnostics = [ordered]@{primary=$primary;restoration=$restoration;verify_elapsed_ms=$verify.ElapsedMilliseconds;restore_elapsed_ms=$restore.ElapsedMilliseconds;disabled_allow_rules=$disabled.Count;remaining_enabled_allow_rules=$remainingAllow;effective_outbound_allow=$activeAllow}
    if ($null -ne $primary -or $cleanupFailed) {
        # Preserve both failure paths; cleanup must never replace the original cause.
        throw ('Hosted offline firewall control failed: ' + ($diagnostics | ConvertTo-Json -Depth 6 -Compress))
    }
    return @{workload=$result;network_isolated=$true;isolation=$script:StableFirewallIsolation;allowed_programs=$agents;allowed_infrastructure_programs=$infrastructure;dns_allowed=$true;external_connection_refused=$true;restored=$true;diagnostics=$diagnostics}
}
