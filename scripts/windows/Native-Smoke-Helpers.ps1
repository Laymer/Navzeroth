# Helpers for the hosted native WebView2 smoke and upgrade check (task G2b).
# Regression evidence only; it cannot satisfy WIN-* acceptance.
Set-StrictMode -Version 3.0

$script:NativeSmokeAppId = 'io.github.laymer.azeroth-navigator'

function Get-NativeSmokeSetup {
    [CmdletBinding()]
    param([Parameter(Mandatory)] [string] $Directory)
    $found = @(Get-ChildItem -LiteralPath $Directory -Filter '*-x64-setup.exe' -File -ErrorAction Stop)
    if ($found.Count -ne 1) { throw "Expected exactly one *-x64-setup.exe in $Directory, found $($found.Count)" }
    return $found[0]
}

function Invoke-BoundedProcess {
    # Runs a process with a hard time limit; a hung dialog is killed and reported, never waited on forever.
    [CmdletBinding()]
    param(
        [Parameter(Mandatory)] [string] $FilePath,
        [string[]] $ArgumentList = @(),
        [ValidateRange(1, 1800)] [int] $TimeoutSeconds = 300
    )
    $startArguments = @{ FilePath = $FilePath; PassThru = $true }
    if ($ArgumentList.Count -gt 0) { $startArguments['ArgumentList'] = $ArgumentList }
    $process = Start-Process @startArguments
    try {
        $finished = $process.WaitForExit($TimeoutSeconds * 1000)
        if (-not $finished) {
            Stop-Process -Id $process.Id -Force -ErrorAction SilentlyContinue
            $process.WaitForExit()
            return [pscustomobject]@{ ExitCode = $null; TimedOut = $true }
        }
        return [pscustomobject]@{ ExitCode = $process.ExitCode; TimedOut = $false }
    } finally { $process.Dispose() }
}

function Stop-NativeSmokeProcesses {
    # Stops the application and only the WebView2 processes that belong to its data folder.
    [CmdletBinding()]
    param([ValidateRange(1, 120)] [int] $TimeoutSeconds = 30)
    Get-Process -Name 'azeroth-desktop' -ErrorAction SilentlyContinue | Stop-Process -Force -ErrorAction SilentlyContinue
    if ($env:OS -eq 'Windows_NT') {
        $web = @(Get-CimInstance Win32_Process -Filter "Name = 'msedgewebview2.exe'" -ErrorAction SilentlyContinue |
            Where-Object { $_.CommandLine -and $_.CommandLine.Contains($script:NativeSmokeAppId) })
        foreach ($p in $web) { Stop-Process -Id $p.ProcessId -Force -ErrorAction SilentlyContinue }
    }
    $deadline = (Get-Date).AddSeconds($TimeoutSeconds)
    while ((Get-Date) -lt $deadline) {
        if (@(Get-Process -Name 'azeroth-desktop' -ErrorAction SilentlyContinue).Count -eq 0) { return }
        Start-Sleep -Milliseconds 500
    }
    throw 'azeroth-desktop is still running'
}

function Get-InstalledDisplayVersion {
    [CmdletBinding()]
    param()
    $key = Get-ChildItem 'HKCU:\Software\Microsoft\Windows\CurrentVersion\Uninstall' |
        Where-Object { (Get-ItemProperty $_.PSPath).DisplayName -eq 'Azeroth Navigator' }
    if (-not $key) { return $null }
    return [string](Get-ItemProperty @($key)[0].PSPath).DisplayVersion
}

function Install-NativeSmokeSetup {
    [CmdletBinding()]
    param([Parameter(Mandatory)] [string] $SetupPath, [ValidateRange(1, 1800)] [int] $TimeoutSeconds = 300)
    $result = Invoke-BoundedProcess -FilePath $SetupPath -ArgumentList @('/S') -TimeoutSeconds $TimeoutSeconds
    if ($result.TimedOut) { throw "Installer $SetupPath did not finish within $TimeoutSeconds s" }
    if ($result.ExitCode -ne 0) { throw "Installer $SetupPath exit code $($result.ExitCode)" }
}

function Invoke-NativeSmokeNode {
    [CmdletBinding()]
    param([Parameter(Mandatory)] [string[]] $Arguments)
    & node @Arguments
    if ($LASTEXITCODE -ne 0) { throw "node $($Arguments -join ' ') exited with $LASTEXITCODE" }
}

function Uninstall-NativeSmokeApp {
    # The NSIS uninstaller re-launches itself from TEMP, so wait for its effect rather than its exit.
    [CmdletBinding()]
    param([Parameter(Mandatory)] [string] $InstallDirectory, [ValidateRange(1, 600)] [int] $TimeoutSeconds = 90)
    $uninstaller = Join-Path $InstallDirectory 'uninstall.exe'
    if (-not (Test-Path -LiteralPath $uninstaller)) { return }
    Start-Process -FilePath $uninstaller -ArgumentList '/S' -Wait
    $exe = Join-Path $InstallDirectory 'azeroth-desktop.exe'
    $deadline = (Get-Date).AddSeconds($TimeoutSeconds)
    while ((Test-Path -LiteralPath $exe) -and (Get-Date) -lt $deadline) { Start-Sleep -Seconds 2 }
    if (Test-Path -LiteralPath $exe) { throw 'The application is still installed after uninstall' }
}
