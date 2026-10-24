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
    $root = 'HKCU:\Software\Microsoft\Windows\CurrentVersion\Uninstall'
    # A fresh account may have no uninstall root. Only absence is benign;
    # unreadable roots and failures after this lookup must remain failures.
    try { $null = Get-Item -LiteralPath $root -ErrorAction Stop }
    catch [System.Management.Automation.ItemNotFoundException] { return $null }
    $key = Get-ChildItem -LiteralPath $root -ErrorAction Stop |
        Where-Object {
            $properties = Get-ItemProperty -LiteralPath $_.PSPath -ErrorAction Stop
            $name = $properties.PSObject.Properties['DisplayName']
            $null -ne $name -and $name.Value -eq 'Azeroth Navigator'
        }
    if (-not $key) { return $null }
    $registration = Get-ItemProperty -LiteralPath @($key)[0].PSPath -ErrorAction Stop
    $version = $registration.PSObject.Properties['DisplayVersion']
    if ($null -eq $version -or $version.Value -isnot [string] -or [string]::IsNullOrWhiteSpace($version.Value)) {
        throw 'Installed registration has missing or invalid DisplayVersion.'
    }
    return $version.Value
}

function Install-NativeSmokeSetup {
    [CmdletBinding()]
    param([Parameter(Mandatory)] [string] $SetupPath, [ValidateRange(1, 1800)] [int] $TimeoutSeconds = 300)
    $result = Invoke-BoundedProcess -FilePath $SetupPath -ArgumentList @('/S') -TimeoutSeconds $TimeoutSeconds
    if ($result.TimedOut) { throw "Installer $SetupPath did not finish within $TimeoutSeconds s" }
    if ($result.ExitCode -ne 0) {
        # A silent installer refuses to replace a newer registered version with exit 2, so name it.
        $registered = 'unavailable'
        try { $found = Get-InstalledDisplayVersion; $registered = if ($null -eq $found) { 'none' } else { $found } } catch { }
        throw "Installer $SetupPath exit code $($result.ExitCode) (registered version: $registered)"
    }
}

function Invoke-NativeSmokeNode {
    [CmdletBinding()]
    param([Parameter(Mandatory)] [string[]] $Arguments)
    & node @Arguments
    if ($LASTEXITCODE -ne 0) { throw "node $($Arguments -join ' ') exited with $LASTEXITCODE" }
}

function Wait-NativeSmokeUninstalled {
    # The NSIS uninstaller re-launches itself from TEMP and removes the Uninstall registry entry after the
    # files. A silent installer started in between still sees the old version and refuses to replace it
    # (exit 2), so completion means the executable AND the registered version are both gone.
    [CmdletBinding()]
    param(
        [Parameter(Mandatory)] [string] $InstallDirectory,
        [ValidateRange(1, 600)] [int] $TimeoutSeconds = 120,
        [scriptblock] $RegisteredVersion = { Get-InstalledDisplayVersion },
        [ValidateRange(1, 5000)] [int] $PollMilliseconds = 250
    )
    $exe = Join-Path $InstallDirectory 'azeroth-desktop.exe'
    $timer = [Diagnostics.Stopwatch]::StartNew()
    do {
        $registered = $null; $probeFailed = $false
        # The key can vanish while it is enumerated; an unreadable registry is "not yet", never "gone".
        try { $registered = & $RegisteredVersion } catch { $probeFailed = $true }
        $exeGone = -not (Test-Path -LiteralPath $exe)
        if ($exeGone -and -not $probeFailed -and $null -eq $registered) { return [int]$timer.Elapsed.TotalMilliseconds }
        Start-Sleep -Milliseconds $PollMilliseconds
    } while ($timer.Elapsed.TotalSeconds -lt $TimeoutSeconds)
    $state = if ($probeFailed) { 'registry unreadable' } elseif ($null -eq $registered) { 'none' } else { [string]$registered }
    throw "The application is still installed after uninstall (executable present: $(-not $exeGone), registered version: $state)"
}

function Uninstall-NativeSmokeApp {
    # The NSIS uninstaller re-launches itself from TEMP, so wait for its effect rather than its exit.
    [CmdletBinding()]
    param([Parameter(Mandatory)] [string] $InstallDirectory, [ValidateRange(1, 600)] [int] $TimeoutSeconds = 90)
    $uninstaller = Join-Path $InstallDirectory 'uninstall.exe'
    if (-not (Test-Path -LiteralPath $uninstaller)) { return }
    Start-Process -FilePath $uninstaller -ArgumentList '/S' -Wait
    $null = Wait-NativeSmokeUninstalled -InstallDirectory $InstallDirectory -TimeoutSeconds $TimeoutSeconds
}
