<#
.SYNOPSIS
    Checks Install-AzerothNavigatorAddon.ps1 against a synthetic game folder.

.DESCRIPTION
    Task M9.4. Runs the addon script in child processes against temporary
    folders that imitate Interface\AddOns and WTF\Account (no real game folder,
    no personal data) and checks install, update, refusal, uninstall and -WhatIf
    behaviour. Exits nonzero when a check fails.

    CI runs it on Windows with Windows PowerShell 5.1 and PowerShell 7. It also
    runs on Linux and macOS with PowerShell 7.

.PARAMETER Shell
    The PowerShell executable that runs the addon script. Default: the current one.
#>
[CmdletBinding()]
param([string] $Shell)

Set-StrictMode -Version 3.0
$ErrorActionPreference = 'Stop'

$Script = Join-Path $PSScriptRoot 'Install-AzerothNavigatorAddon.ps1'
$Source = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot (Join-Path '..' (Join-Path '..' (Join-Path 'addons' 'AzerothNavigator')))))
$OnWindows = [Environment]::OSVersion.Platform -eq [PlatformID]::Win32NT
if (-not $Shell) { $Shell = (Get-Process -Id $PID).Path }

$Root = Join-Path ([IO.Path]::GetTempPath()) ("anav-addon-test-" + [Guid]::NewGuid().ToString('N').Substring(0, 8))
$failures = New-Object System.Collections.Generic.List[string]
$count = 0

function Join-Parts([string[]] $Parts) {
    $path = $Parts[0]
    foreach ($part in $Parts[1..($Parts.Count - 1)]) { $path = Join-Path $path $part }
    return $path
}

function New-Game([string] $Name) {
    $game = Join-Path $Root $Name
    $addOns = Join-Parts @($game, 'Interface', 'AddOns')
    New-Item -ItemType Directory -Path (Join-Path $addOns 'OtherAddon') -Force | Out-Null
    Set-Content -LiteralPath (Join-Parts @($addOns, 'OtherAddon', 'OtherAddon.toc')) -Value '## Title: Other'
    foreach ($account in @('ACCOUNT_A', 'ACCOUNT_B')) {
        $saved = Join-Parts @($game, 'WTF', 'Account', $account, 'SavedVariables')
        New-Item -ItemType Directory -Path $saved -Force | Out-Null
        Set-Content -LiteralPath (Join-Path $saved 'AzerothNavigator.lua') -Value 'AzerothNavigatorDB = {}'
        Set-Content -LiteralPath (Join-Path $saved 'OtherAddon.lua') -Value 'OtherDB = {}'
    }
    return [pscustomobject]@{ Game = $game; AddOns = $addOns; State = (Join-Path $Root "$Name-state") }
}

function Invoke-AddonScript([string[]] $Arguments) {
    $all = @('-NoProfile', '-NonInteractive', '-ExecutionPolicy', 'Bypass', '-File', $Script) + $Arguments
    # Child stderr must not become a terminating error (Windows PowerShell 5.1).
    $saved = $ErrorActionPreference
    $ErrorActionPreference = 'Continue'
    try { $output = & $Shell @all 2>&1 | Out-String }
    finally { $ErrorActionPreference = $saved }
    return [pscustomobject]@{ Code = $LASTEXITCODE; Output = $output }
}

function Check([string] $Name, [bool] $Condition, [string] $Detail = '') {
    $script:count++
    if ($Condition) { Write-Host "ok   $Name" }
    else {
        Write-Host "FAIL $Name $Detail" -ForegroundColor Red
        $script:failures.Add($Name)
    }
}

function Get-Tree([string] $Path) {
    if (-not (Test-Path -LiteralPath $Path)) { return '' }
    $prefix = ([IO.Path]::GetFullPath($Path)).Length
    return ((Get-ChildItem -LiteralPath $Path -Recurse -Force | ForEach-Object {
                $_.FullName.Substring($prefix) + ':' + $(if ($_.PSIsContainer) { 'd' } else { (Get-FileHash -LiteralPath $_.FullName).Hash })
            } | Sort-Object) -join "`n")
}

function Get-Marker([string] $State) {
    $file = Join-Path $State 'addon-location.txt'
    if (-not (Test-Path -LiteralPath $file)) { return $null }
    return [IO.File]::ReadAllBytes($file)
}

try {
    New-Item -ItemType Directory -Path $Root | Out-Null
    $sourceTree = Get-Tree $Source
    Write-Host "Shell: $Shell"

    # Install into a fresh AddOns folder.
    $g = New-Game 'fresh'
    $r = Invoke-AddonScript @('-Action', 'Install', '-AddOnsPath', $g.AddOns, '-Quiet', '-StateDirectory', $g.State)
    $target = Join-Path $g.AddOns 'AzerothNavigator'
    Check 'install exits 0' ($r.Code -eq 0) $r.Output
    Check 'install copies the addon exactly' ((Get-Tree $target) -eq $sourceTree)
    Check 'install leaves no staging folder' (@(Get-ChildItem -LiteralPath $g.AddOns -Directory).Count -eq 2)
    Check 'install keeps the other addon' (Test-Path -LiteralPath (Join-Parts @($g.AddOns, 'OtherAddon', 'OtherAddon.toc')))
    $bytes = Get-Marker $g.State
    $expectedBytes = [Text.UnicodeEncoding]::new($false, $false).GetBytes([IO.Path]::GetFullPath($g.AddOns))
    Check 'marker is UTF-16LE without BOM or newline' (($null -ne $bytes) -and ([Convert]::ToBase64String($bytes) -eq [Convert]::ToBase64String($expectedBytes)))

    # Update over an older copy; a stray file from the old version disappears.
    Set-Content -LiteralPath (Join-Path $target 'Stale.lua') -Value '-- old'
    $r = Invoke-AddonScript @('-Action', 'Install', '-Quiet', '-StateDirectory', $g.State)
    Check 'update via the recorded folder exits 0' ($r.Code -eq 0) $r.Output
    Check 'update replaces the whole folder' ((Get-Tree $target) -eq $sourceTree)
    Check 'update leaves no previous folder' (@(Get-ChildItem -LiteralPath $g.AddOns -Directory).Count -eq 2)

    # Status.
    $r = Invoke-AddonScript @('-Action', 'Status', '-StateDirectory', $g.State)
    Check 'status reports the installed version' (($r.Code -eq 0) -and ($r.Output -match 'installed version: ')) $r.Output

    # -WhatIf changes nothing.
    $before = Get-Tree $g.Game
    $r = Invoke-AddonScript @('-Action', 'Uninstall', '-AddOnsPath', $g.AddOns, '-RemoveSavedVariables', '-Quiet', '-StateDirectory', $g.State, '-WhatIf')
    Check 'uninstall -WhatIf exits 0' ($r.Code -eq 0) $r.Output
    Check 'uninstall -WhatIf changes nothing' ((Get-Tree $g.Game) -eq $before)
    Check 'uninstall -WhatIf keeps the marker' ($null -ne (Get-Marker $g.State))

    # Uninstall with SavedVariables.
    $r = Invoke-AddonScript @('-Action', 'Uninstall', '-AddOnsPath', $g.AddOns, '-RemoveSavedVariables', '-Quiet', '-StateDirectory', $g.State)
    Check 'uninstall exits 0' ($r.Code -eq 0) $r.Output
    Check 'uninstall removes the addon folder' (-not (Test-Path -LiteralPath $target))
    Check 'uninstall keeps the other addon' (Test-Path -LiteralPath (Join-Parts @($g.AddOns, 'OtherAddon', 'OtherAddon.toc')))
    $remaining = @(Get-ChildItem -LiteralPath (Join-Parts @($g.Game, 'WTF')) -Recurse -File | ForEach-Object { $_.Name } | Sort-Object)
    Check 'uninstall removes only this addon''s SavedVariables' (($remaining -join ',') -eq 'OtherAddon.lua,OtherAddon.lua') ($remaining -join ',')
    Check 'uninstall forgets the recorded folder' ($null -eq (Get-Marker $g.State))

    # A recorded folder that has disappeared is forgotten on uninstall.
    $g = New-Game 'vanished'
    $r = Invoke-AddonScript @('-Action', 'Install', '-AddOnsPath', $g.AddOns, '-Quiet', '-StateDirectory', $g.State)
    Remove-Item -LiteralPath $g.Game -Recurse -Force
    $r = Invoke-AddonScript @('-Action', 'Uninstall', '-Quiet', '-StateDirectory', $g.State)
    Check 'a vanished recorded folder is forgotten' (($r.Code -eq 0) -and ($null -eq (Get-Marker $g.State))) $r.Output

    # The game folder is accepted in place of AddOns when it contains Interface\AddOns.
    $g = New-Game 'gamefolder'
    $r = Invoke-AddonScript @('-Action', 'Install', '-AddOnsPath', $g.Game, '-Quiet', '-StateDirectory', $g.State)
    Check 'game folder resolves to Interface\AddOns' (($r.Code -eq 0) -and (Test-Path -LiteralPath (Join-Parts @($g.AddOns, 'AzerothNavigator', 'AzerothNavigator.toc')))) $r.Output

    $r = Invoke-AddonScript @('-Action', 'Status', '-AddOnsPath', $g.Game, '-Quiet', '-StateDirectory', $g.State)
    Check 'status resolves a game folder like install' (($r.Code -eq 0) -and ($r.Output -match 'installed version: ')) $r.Output

    # Refusals.
    $plain = Join-Path $Root 'not-a-game'
    New-Item -ItemType Directory -Path $plain | Out-Null
    $r = Invoke-AddonScript @('-Action', 'Install', '-AddOnsPath', $plain, '-Quiet', '-StateDirectory', (Join-Path $Root 'plain-state'))
    Check 'a folder without AddOns is refused' (($r.Code -eq 1) -and (@(Get-ChildItem -LiteralPath $plain -Force).Count -eq 0)) $r.Output

    $r = Invoke-AddonScript @('-Action', 'Install', '-Quiet', '-StateDirectory', (Join-Path $Root 'empty-state'))
    Check '-Quiet without a folder is refused' ($r.Code -eq 1) $r.Output

    $g = New-Game 'foreign'
    $foreign = Join-Path $g.AddOns 'AzerothNavigator'
    New-Item -ItemType Directory -Path $foreign | Out-Null
    Set-Content -LiteralPath (Join-Path $foreign 'AzerothNavigator.toc') -Value '## Title: Somebody Else'
    $before = Get-Tree $g.Game
    $r = Invoke-AddonScript @('-Action', 'Install', '-AddOnsPath', $g.AddOns, '-Quiet', '-StateDirectory', $g.State)
    Check 'a different addon with the same folder name is refused on install' (($r.Code -eq 1) -and ((Get-Tree $g.Game) -eq $before)) $r.Output
    $r = Invoke-AddonScript @('-Action', 'Uninstall', '-AddOnsPath', $g.AddOns, '-Quiet', '-StateDirectory', $g.State)
    Check 'a different addon with the same folder name is refused on uninstall' (($r.Code -eq 1) -and ((Get-Tree $g.Game) -eq $before)) $r.Output

    Set-Content -LiteralPath (Join-Path $foreign 'AzerothNavigator.toc') -Value '## Title: Azeroth Navigator Unrelated'
    $before = Get-Tree $g.Game
    foreach ($action in @('Install', 'Uninstall')) {
        $r = Invoke-AddonScript @('-Action', $action, '-AddOnsPath', $g.AddOns, '-Quiet', '-StateDirectory', $g.State)
        Check "a title prefix is insufficient ownership for $action" (($r.Code -eq 1) -and ((Get-Tree $g.Game) -eq $before)) $r.Output
    }

    $broken = Join-Path (Join-Path $Root 'broken-source') 'AzerothNavigator'
    Copy-Item -LiteralPath $Source -Destination $broken -Recurse
    Remove-Item -LiteralPath (Join-Parts @($broken, 'Core', 'Json.lua'))
    $g = New-Game 'broken'
    $r = Invoke-AddonScript @('-Action', 'Install', '-AddOnsPath', $g.AddOns, '-Source', $broken, '-Quiet', '-StateDirectory', $g.State)
    Check 'an incomplete source is refused' (($r.Code -eq 1) -and -not (Test-Path -LiteralPath (Join-Path $g.AddOns 'AzerothNavigator'))) $r.Output

    $escape = Join-Path (Join-Path $Root 'escaping-source') 'AzerothNavigator'
    Copy-Item -LiteralPath $Source -Destination $escape -Recurse
    Set-Content -LiteralPath (Join-Path (Split-Path -Parent $escape) 'outside.lua') -Value '-- outside'
    Add-Content -LiteralPath (Join-Path $escape 'AzerothNavigator.toc') -Value '../outside.lua'
    $r = Invoke-AddonScript @('-Action', 'Install', '-AddOnsPath', $g.AddOns, '-Source', $escape, '-Quiet', '-StateDirectory', $g.State)
    Check 'TOC paths escaping the source are refused' (($r.Code -eq 1) -and ($r.Output -match 'outside its source folder')) $r.Output

    # A process running from the game folder blocks changes.
    $g = New-Game 'running'
    if ($OnWindows) {
        # A self-contained sleeper compiled with the .NET Framework compiler present on
        # every Windows; copied system tools can fail without their language resources.
        $copy = Join-Path $g.Game 'GameStub.exe'
        $code = Join-Path $Root 'GameStub.cs'
        Set-Content -LiteralPath $code -Value 'class GameStub { static void Main() { System.Threading.Thread.Sleep(60000); } }'
        $csc = Join-Path $env:WINDIR 'Microsoft.NET\Framework64\v4.0.30319\csc.exe'
        & $csc /nologo /target:winexe "/out:$copy" $code | Out-Null
        if ($LASTEXITCODE -ne 0) { throw 'csc failed to build the game stub' }
        $arguments = @()
    }
    else {
        $copy = Join-Path $g.Game 'sleep'
        Copy-Item -LiteralPath (Get-Command sleep).Source -Destination $copy
        $arguments = @('60')
    }
    $start = @{ FilePath = $copy; PassThru = $true }
    if ($arguments.Count -gt 0) { $start['ArgumentList'] = $arguments }
    if ($OnWindows) { $start['WindowStyle'] = 'Hidden' }
    $process = Start-Process @start
    try {
        Start-Sleep -Milliseconds 500
        $r = Invoke-AddonScript @('-Action', 'Install', '-AddOnsPath', $g.AddOns, '-Quiet', '-StateDirectory', $g.State)
        Check 'a running game blocks installation' (($r.Code -eq 1) -and ($r.Output -match 'Close the game first') -and -not (Test-Path -LiteralPath (Join-Path $g.AddOns 'AzerothNavigator'))) $r.Output
        $before = Get-Tree $g.Game
        $r = Invoke-AddonScript @('-Action', 'Uninstall', '-AddOnsPath', $g.AddOns, '-RemoveSavedVariables', '-Quiet', '-StateDirectory', $g.State)
        Check 'a running game protects SavedVariables even without an addon folder' (($r.Code -eq 1) -and ($r.Output -match 'Close the game first') -and ((Get-Tree $g.Game) -eq $before)) $r.Output
    }
    finally {
        Stop-Process -Id $process.Id -Force -ErrorAction SilentlyContinue
        $process.WaitForExit()
    }
}
finally {
    Remove-Item -LiteralPath $Root -Recurse -Force -ErrorAction SilentlyContinue
}

Write-Host ''
if ($failures.Count -gt 0) {
    Write-Host "$($failures.Count) of $count checks failed: $($failures -join '; ')" -ForegroundColor Red
    exit 1
}
Write-Host "All $count checks passed."
exit 0
