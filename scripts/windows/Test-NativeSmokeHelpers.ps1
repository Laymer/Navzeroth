[CmdletBinding()]
param()
Set-StrictMode -Version 3.0
$ErrorActionPreference = 'Stop'
. (Join-Path $PSScriptRoot 'Native-Smoke-Helpers.ps1')
$temp = Join-Path ([IO.Path]::GetTempPath()) ('anav-native-' + [Guid]::NewGuid().ToString('N'))
$count = 0
function Check([string] $Name, [bool] $Condition) { $script:count++; if (-not $Condition) { throw "FAILED: $Name" }; Write-Host "ok $Name" }
function Refuses([string] $Name, [string] $Expected, [scriptblock] $Action) {
    $message = ''
    try { & $Action } catch { $message = $_.Exception.Message }
    Check $Name ($message.Contains($Expected))
}
function Start-Synthetic([string] $Body) {
    $encoded = [Convert]::ToBase64String([Text.Encoding]::Unicode.GetBytes($Body))
    return (Get-Process -Id $PID).Path, @('-NoProfile', '-NonInteractive', '-EncodedCommand', $encoded)
}
try {
    New-Item -ItemType Directory -Path $temp | Out-Null
    Refuses 'no setup executable is refused' 'found 0' { Get-NativeSmokeSetup -Directory $temp }
    [IO.File]::WriteAllText((Join-Path $temp 'A-1.0.0-x64-setup.exe'), 'x')
    Check 'one setup executable is selected' ((Get-NativeSmokeSetup -Directory $temp).Name -eq 'A-1.0.0-x64-setup.exe')
    [IO.File]::WriteAllText((Join-Path $temp 'B-1.0.0-x64-setup.exe'), 'x')
    Refuses 'two setup executables are ambiguous' 'found 2' { Get-NativeSmokeSetup -Directory $temp }

    $exe, $argv = Start-Synthetic 'exit 3'
    $result = Invoke-BoundedProcess -FilePath $exe -ArgumentList $argv -TimeoutSeconds 60
    Check 'exit code is reported' ((-not $result.TimedOut) -and $result.ExitCode -eq 3)

    $exe, $argv = Start-Synthetic 'Start-Sleep -Seconds 60'
    $timer = [Diagnostics.Stopwatch]::StartNew()
    $result = Invoke-BoundedProcess -FilePath $exe -ArgumentList $argv -TimeoutSeconds 2
    Check 'a hung process is killed and reported as timed out' ($result.TimedOut -and $null -eq $result.ExitCode -and $timer.Elapsed.TotalSeconds -lt 30)

    $exe, $argv = Start-Synthetic 'exit 5'
    Refuses 'installer failure is surfaced' 'exit code 5' { Install-NativeSmokeSetup -SetupPath $exe -TimeoutSeconds 60 }

    Stop-NativeSmokeProcesses -TimeoutSeconds 5
    Check 'stopping with nothing running succeeds' $true
    Write-Host "$count native smoke helper checks passed (synthetic processes, not native acceptance)"
} finally { Remove-Item -LiteralPath $temp -Recurse -Force -ErrorAction SilentlyContinue }
