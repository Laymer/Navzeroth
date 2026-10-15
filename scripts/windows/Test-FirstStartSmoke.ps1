[CmdletBinding()]
param()
Set-StrictMode -Version 3.0
$ErrorActionPreference = 'Stop'
. (Join-Path $PSScriptRoot 'First-StartSmoke.ps1')
$temp = Join-Path ([IO.Path]::GetTempPath()) ('anav-start-' + [Guid]::NewGuid().ToString('N'))
$count = 0
function Check([string] $Name, [bool] $Condition) { $script:count++; if (-not $Condition) { throw "FAILED: $Name" }; Write-Host "ok $Name" }
function Refuses([string] $Name, [string] $Expected, [scriptblock] $Action) {
    $message = ''
    try { & $Action } catch { $message = $_.Exception.Message }
    Check $Name ($message.Contains($Expected))
}
function Start-Synthetic([string] $Body) {
    $encoded = [Convert]::ToBase64String([Text.Encoding]::Unicode.GetBytes($Body))
    Start-Process -FilePath (Get-Process -Id $PID).Path -ArgumentList '-NoProfile', '-NonInteractive', '-EncodedCommand', $encoded -PassThru
}
function Stop-Synthetic([System.Diagnostics.Process] $Process) {
    if (-not $Process.HasExited) { Stop-Process -Id $Process.Id -Force; $Process.WaitForExit() }
    $Process.Dispose()
}
try {
    New-Item -ItemType Directory -Path $temp | Out-Null
    $store = Join-Path $temp 'snapshots.sqlite3'
    [IO.File]::WriteAllText($store, 'stale synthetic data')
    Refuses 'stale store refuses before launching executable' 'Unexpected preexisting' { Invoke-FirstStartSmoke -Executable 'must-not-run' -DataDirectory $temp }
    Remove-Item -LiteralPath $store
    [IO.File]::WriteAllText("$store-wal", 'stale journal')
    Refuses 'orphan journal refuses before launching executable' 'Unexpected preexisting' { Invoke-FirstStartSmoke -Executable 'must-not-run' -DataDirectory $temp }
    Remove-Item -LiteralPath "$store-wal"

    $child = Start-Synthetic 'Start-Sleep -Seconds 30'
    try {
        Refuses 'alive without store cannot pass' 'timed out' { Wait-FirstStartStore $child $store -TimeoutSeconds 1 -MinimumAliveSeconds 0 -StoreDwellSeconds 0 }
        [IO.File]::WriteAllText($store, '')
        Refuses 'empty store cannot pass' 'timed out' { Wait-FirstStartStore $child $store -TimeoutSeconds 1 -MinimumAliveSeconds 0 -StoreDwellSeconds 0 }
        Remove-Item -LiteralPath $store
    } finally { Stop-Synthetic $child }

    # The store arrives after the old synthetic one-second limit. Observe it
    # within a larger bound, retaining minimum survival and post-file dwell.
    $escapedStore = $store.Replace("'", "''")
    $child = Start-Synthetic "Start-Sleep -Seconds 2; [IO.File]::WriteAllText('$escapedStore', 'synthetic store'); Start-Sleep -Seconds 30"
    try {
        $timer = [Diagnostics.Stopwatch]::StartNew()
        Wait-FirstStartStore $child $store -TimeoutSeconds 10 -MinimumAliveSeconds 4 -StoreDwellSeconds 1
        Check 'delayed store passes with retained survival and dwell' ($timer.Elapsed.TotalSeconds -ge 4 -and -not $child.HasExited)
        Refuses 'file alone cannot bypass minimum liveness' 'timed out' { Wait-FirstStartStore $child $store -TimeoutSeconds 1 -MinimumAliveSeconds 2 -StoreDwellSeconds 0 }
        Refuses 'file alone cannot bypass dwell' 'timed out' { Wait-FirstStartStore $child $store -TimeoutSeconds 1 -MinimumAliveSeconds 0 -StoreDwellSeconds 2 }
    } finally { Stop-Synthetic $child }

    $child = Start-Synthetic 'exit 7'
    try {
        $child.WaitForExit()
        Refuses 'exited process fails even with existing file' 'exit code 7' { Wait-FirstStartStore $child $store -TimeoutSeconds 2 -MinimumAliveSeconds 0 -StoreDwellSeconds 0 }
    } finally { Stop-Synthetic $child }
    Write-Host "$count first-start harness checks passed (synthetic processes, not native acceptance)"
} finally { Remove-Item -LiteralPath $temp -Recurse -Force }
