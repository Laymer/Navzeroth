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

    # The installer is always run as `setup /S`; a .cmd stand-in exits with a chosen code.
    $fakeSetup = Join-Path $temp 'fake-setup.cmd'
    [IO.File]::WriteAllText($fakeSetup, "@exit /b 5`r`n")
    Refuses 'installer failure is surfaced' 'exit code 5' { Install-NativeSmokeSetup -SetupPath $fakeSetup -TimeoutSeconds 60 }
    [IO.File]::WriteAllText($fakeSetup, "@exit /b 0`r`n")
    Install-NativeSmokeSetup -SetupPath $fakeSetup -TimeoutSeconds 60
    Check 'installer success is accepted' $true

    # Uninstall completion needs the executable AND the registered version gone (silent downgrade refusal).
    $install = Join-Path $temp 'install'
    New-Item -ItemType Directory -Path $install | Out-Null
    $exePath = Join-Path $install 'azeroth-desktop.exe'
    $script:probes = 0
    $registered = { $script:probes++; if ($script:probes -lt 4) { '1.0.0' } else { $null } }
    $elapsed = Wait-NativeSmokeUninstalled -InstallDirectory $install -TimeoutSeconds 30 -RegisteredVersion $registered -PollMilliseconds 20
    Check 'wait outlasts a registered version after the files are gone' ($script:probes -eq 4 -and $elapsed -ge 40)
    [IO.File]::WriteAllText($exePath, 'x')
    $remover = Start-Job { param($Path) Start-Sleep -Milliseconds 400; Remove-Item -LiteralPath $Path -Force } -ArgumentList $exePath
    $elapsed = Wait-NativeSmokeUninstalled -InstallDirectory $install -TimeoutSeconds 30 -RegisteredVersion { $null } -PollMilliseconds 20
    $null = Wait-Job $remover; Remove-Job $remover -Force
    Check 'wait outlasts a present executable' ($elapsed -ge 300 -and -not (Test-Path -LiteralPath $exePath))
    $script:probes = 0
    $flaky = { $script:probes++; if ($script:probes -lt 3) { throw 'key vanished during enumeration' }; $null }
    $null = Wait-NativeSmokeUninstalled -InstallDirectory $install -TimeoutSeconds 30 -RegisteredVersion $flaky -PollMilliseconds 20
    Check 'an unreadable registry is retried, not treated as uninstalled' ($script:probes -eq 3)
    Refuses 'a persistently unreadable registry times out' 'registry unreadable' {
        Wait-NativeSmokeUninstalled -InstallDirectory $install -TimeoutSeconds 1 -RegisteredVersion { throw 'denied' } -PollMilliseconds 20
    }
    Refuses 'a lingering registered version times out and names it' 'registered version: 1.0.0' {
        Wait-NativeSmokeUninstalled -InstallDirectory $install -TimeoutSeconds 1 -RegisteredVersion { '1.0.0' } -PollMilliseconds 20
    }
    Refuses 'a lingering executable times out and is reported' 'executable present: True' {
        [IO.File]::WriteAllText($exePath, 'x')
        try { Wait-NativeSmokeUninstalled -InstallDirectory $install -TimeoutSeconds 1 -RegisteredVersion { $null } -PollMilliseconds 20 }
        finally { Remove-Item -LiteralPath $exePath -Force }
    }
    Uninstall-NativeSmokeApp -InstallDirectory $install
    Check 'uninstall without an uninstaller is a no-op' $true

    Refuses 'dialog cancellation refuses a different executable before looking for windows' 'does not match' {
        & (Join-Path $PSScriptRoot 'Cancel-OwnedDialog.ps1') -TargetProcessId $PID -ExpectedExecutable (Join-Path $temp 'unrelated.exe') -TimeoutSeconds 1
    }
    Refuses 'dialog cancellation has a bounded missing-dialog refusal' 'No visible dialog owned' {
        & (Join-Path $PSScriptRoot 'Cancel-OwnedDialog.ps1') -TargetProcessId $PID -ExpectedExecutable (Get-Process -Id $PID).Path -TimeoutSeconds 1
    }
    Stop-NativeSmokeProcesses -TimeoutSeconds 5
    Check 'stopping with nothing running succeeds' $true
    Write-Host "$count native smoke helper checks passed (synthetic processes, not native acceptance)"
} finally { Remove-Item -LiteralPath $temp -Recurse -Force -ErrorAction SilentlyContinue }
