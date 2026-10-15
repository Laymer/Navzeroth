# CI process/store observation only; this does not establish interactive readiness.
Set-StrictMode -Version 3.0

function Wait-FirstStartStore {
    [CmdletBinding()]
    param(
        [Parameter(Mandatory)] [System.Diagnostics.Process] $Process,
        [Parameter(Mandatory)] [string] $StorePath,
        [ValidateRange(1, 120)] [int] $TimeoutSeconds = 120,
        [ValidateRange(0, 120)] [int] $MinimumAliveSeconds = 20,
        [ValidateRange(0, 120)] [int] $StoreDwellSeconds = 5
    )
    $timer = [Diagnostics.Stopwatch]::StartNew()
    $firstStoreSeconds = $null
    while ($true) {
        $Process.Refresh()
        if ($Process.HasExited) { throw "Application exited during first start (exit code $($Process.ExitCode), elapsed $($timer.Elapsed.TotalSeconds.ToString('F2')) s)" }
        $elapsed = $timer.Elapsed.TotalSeconds
        if ($elapsed -ge $TimeoutSeconds) { throw "First-start observation timed out after $TimeoutSeconds s: required live process and nonempty store with dwell at $StorePath" }
        $store = Get-Item -LiteralPath $StorePath -ErrorAction SilentlyContinue
        if ($null -ne $store -and -not $store.PSIsContainer -and $store.Length -gt 0) {
            if ($null -eq $firstStoreSeconds) {
                $firstStoreSeconds = $elapsed
                Write-Host "first start: nonempty store first observed at $($elapsed.ToString('F2')) s"
            }
            if ($elapsed -ge $MinimumAliveSeconds -and ($elapsed - $firstStoreSeconds) -ge $StoreDwellSeconds) {
                Write-Host "first start: process survived $($elapsed.ToString('F2')) s; store remained present for at least $StoreDwellSeconds s"
                return
            }
        } else {
            $firstStoreSeconds = $null
        }
        Start-Sleep -Milliseconds 250
    }
}

function Write-FirstStartDiagnostics {
    param([System.Diagnostics.Process] $Process, [string] $DataDirectory, [datetime] $StartedAt)
    # Best effort; diagnostics must not mask the original smoke failure.
    try { $Process.Refresh(); $Process | Select-Object Id, ProcessName, HasExited, Responding, MainWindowHandle, MainWindowTitle, CPU, StartTime | Format-List | Out-Host } catch { Write-Host "Process diagnostics unavailable: $_" }
    try { Get-ChildItem -LiteralPath $DataDirectory -Force -ErrorAction Stop | Select-Object Name, Length, LastWriteTimeUtc | Format-Table | Out-Host } catch { Write-Host "App-data diagnostics unavailable: $_" }
    if ($env:OS -eq 'Windows_NT') {
        try { Get-CimInstance Win32_Process -Filter "Name = 'msedgewebview2.exe' OR Name = 'azeroth-desktop.exe'" | Select-Object ProcessId, ParentProcessId, Name, CreationDate, ExecutablePath | Format-Table | Out-Host } catch { Write-Host "WebView processes unavailable: $_" }
        try { Get-ChildItem 'HKLM:\SOFTWARE\WOW6432Node\Microsoft\EdgeUpdate\Clients', 'HKCU:\Software\Microsoft\EdgeUpdate\Clients' -ErrorAction SilentlyContinue | Get-ItemProperty | Select-Object name, pv | Format-Table | Out-Host } catch { Write-Host "WebView versions unavailable: $_" }
        try { Get-WinEvent -FilterHashtable @{LogName='Application'; StartTime=$StartedAt; Level=2} -MaxEvents 10 -ErrorAction Stop | Select-Object TimeCreated, ProviderName, Id, Message | Format-List | Out-Host } catch { Write-Host "Application error events unavailable: $_" }
    }
}

function Invoke-FirstStartSmoke {
    [CmdletBinding()]
    param([Parameter(Mandatory)] [string] $Executable, [Parameter(Mandatory)] [string] $DataDirectory)
    $store = Join-Path $DataDirectory 'snapshots.sqlite3'
    # Never accept an earlier run's store, or delete data to manufacture freshness.
    foreach ($path in @($store, "$store-wal", "$store-shm")) {
        if (Test-Path -LiteralPath $path) { throw "Unexpected preexisting first-start store: $path" }
    }
    $startedAt = Get-Date
    $app = Start-Process -FilePath $Executable -PassThru
    try {
        Wait-FirstStartStore -Process $app -StorePath $store
    } catch {
        Write-FirstStartDiagnostics -Process $app -DataDirectory $DataDirectory -StartedAt $startedAt
        throw
    } finally {
        $app.Refresh()
        if (-not $app.HasExited) { Stop-Process -Id $app.Id -Force; $app.WaitForExit() }
        $app.Dispose()
    }
}
