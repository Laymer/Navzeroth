<#
.SYNOPSIS
    Installs, updates, removes or inspects the Azeroth Navigator in-game addon.

.DESCRIPTION
    Task M9.4. Copies the AzerothNavigator addon folder into a World of Warcraft
    Interface\AddOns directory that the user chooses, and removes it again.

    The AddOns directory of the Forever client is unverified
    (docs/compatibility/ADDON_INSTALL.md), so this script never guesses one: the
    user passes -AddOnsPath or picks the folder in a dialog. It refuses a folder
    that is not named AddOns and never creates one.

    Install is staged and atomic within the AddOns folder: the new copy is
    written next to the old one, verified file by file (SHA-256), then swapped
    in by rename. If the swap fails, the previous addon is restored; a later
    cleanup or location-recording error can occur after the new version is installed.

    Only the AddOns\AzerothNavigator folder is written or removed, and only when
    it is absent or its TOC title is Azeroth Navigator's. SavedVariables are
    removed only with -RemoveSavedVariables. The script needs no administrator
    rights, starts no background process and opens no network connection.

    The AddOns folder used last is recorded in
    %LOCALAPPDATA%\io.github.laymer.azeroth-navigator\addon-location.txt so the
    application's installer can offer to update or remove the addon.

    Runs on Windows PowerShell 5.1 and PowerShell 7.

.PARAMETER Action
    Install (also updates), Uninstall or Status. Without it, an interactive
    session shows a menu and -Quiet runs Install.

.PARAMETER AddOnsPath
    The game's Interface\AddOns folder, or a folder that contains Interface\AddOns.
    Defaults to the recorded folder, then to a folder dialog.

.PARAMETER Source
    The AzerothNavigator addon folder to install. Defaults to the copy next to
    this script (installed layout) or addons\AzerothNavigator (repository layout).

.PARAMETER RemoveSavedVariables
    With Uninstall: also delete AzerothNavigator.lua and AzerothNavigator.lua.bak
    from every WTF\Account\<account>\SavedVariables folder of the same game folder.
    The WTF layout is a convention that is unverified for Forever.

.PARAMETER Quiet
    No menu, dialog or prompts (for the installer and scripts). Fails instead of asking.

.PARAMETER StateDirectory
    Where addon-location.txt is kept. For tests; defaults to the application's
    local data folder.

.EXAMPLE
    .\Install-AzerothNavigatorAddon.ps1 -AddOnsPath 'D:\Games\WoW Forever\Interface\AddOns'

.EXAMPLE
    .\Install-AzerothNavigatorAddon.ps1 -Action Uninstall -RemoveSavedVariables -WhatIf
#>
[CmdletBinding(SupportsShouldProcess = $true)]
param(
    [ValidateSet('Install', 'Uninstall', 'Status')]
    [string] $Action,
    [string] $AddOnsPath,
    [string] $Source,
    [switch] $RemoveSavedVariables,
    [switch] $Quiet,
    [string] $StateDirectory
)

Set-StrictMode -Version 3.0
$ErrorActionPreference = 'Stop'

$AddonName = 'AzerothNavigator'
$AddonTitle = 'Azeroth Navigator Diagnostics'
$BundleId = 'io.github.laymer.azeroth-navigator'
$MarkerName = 'addon-location.txt'
$Separator = [IO.Path]::DirectorySeparatorChar

# Exit codes: 0 done, 1 failed, 2 cancelled by the user.
$ExitFailed = 1
$ExitCancelled = 2

function Write-Step([string] $Message) {
    Write-Host "AzerothNavigator: $Message"
}

function Get-FullPath([string] $Path) {
    $full = [IO.Path]::GetFullPath($Path)
    if ($full.Length -gt 3) { $full = $full.TrimEnd('\', '/') }
    return $full
}

function Get-StateDirectory {
    if ($StateDirectory) { return (Get-FullPath $StateDirectory) }
    if (-not $env:LOCALAPPDATA) { throw 'LOCALAPPDATA is not set; pass -StateDirectory.' }
    return (Join-Path $env:LOCALAPPDATA $BundleId)
}

function Get-RecordedAddOnsPath {
    $marker = Join-Path (Get-StateDirectory) $MarkerName
    if (-not (Test-Path -LiteralPath $marker -PathType Leaf)) { return $null }
    $recorded = [IO.File]::ReadAllText($marker, [Text.UnicodeEncoding]::new($false, $false)).Trim()
    if ($recorded) { return $recorded }
    return $null
}

function Set-RecordedAddOnsPath([string] $Path) {
    $directory = Get-StateDirectory
    if (-not (Test-Path -LiteralPath $directory)) {
        New-Item -ItemType Directory -Path $directory | Out-Null
    }
    # UTF-16LE without BOM or newline: the NSIS uninstaller reads it with FileReadUTF16LE.
    [IO.File]::WriteAllText((Join-Path $directory $MarkerName), $Path, [Text.UnicodeEncoding]::new($false, $false))
}

function Clear-RecordedAddOnsPath([string] $Path) {
    $recorded = Get-RecordedAddOnsPath
    if ($recorded -and [string]::Equals((Get-FullPath $recorded), $Path, [StringComparison]::OrdinalIgnoreCase)) {
        Remove-Item -LiteralPath (Join-Path (Get-StateDirectory) $MarkerName)
    }
}

function Read-Toc([string] $Folder) {
    $toc = Join-Path $Folder "$AddonName.toc"
    if (-not (Test-Path -LiteralPath $toc -PathType Leaf)) { return $null }
    $title = $null
    $version = $null
    $files = New-Object System.Collections.Generic.List[string]
    foreach ($raw in [IO.File]::ReadAllLines($toc)) {
        $line = $raw.Trim()
        if ($line -match '^##\s*Title:\s*(.+)$') { $title = $Matches[1].Trim() }
        elseif ($line -match '^##\s*Version:\s*(.+)$') { $version = $Matches[1].Trim() }
        elseif ($line -and -not $line.StartsWith('#')) { $files.Add($line) }
    }
    return [pscustomobject]@{ Title = $title; Version = $version; Files = $files }
}

function Test-OwnAddon($Toc) {
    return ($null -ne $Toc) -and ($null -ne $Toc.Title) -and [string]::Equals($Toc.Title, $AddonTitle, [StringComparison]::Ordinal)
}

function Resolve-Source {
    $candidates = @()
    if ($Source) { $candidates = @($Source) }
    else {
        $candidates = @(
            (Join-Path $PSScriptRoot $AddonName),
            (Join-Path (Join-Path (Join-Path $PSScriptRoot '..') '..') (Join-Path 'addons' $AddonName))
        )
    }
    foreach ($candidate in $candidates) {
        if (Test-Path -LiteralPath $candidate -PathType Container) {
            $folder = Get-FullPath $candidate
            if ((Split-Path -Leaf $folder) -ne $AddonName) {
                throw "The addon source folder must be named $AddonName`: $folder"
            }
            $toc = Read-Toc $folder
            if (-not (Test-OwnAddon $toc)) { throw "No Azeroth Navigator TOC in $folder" }
            foreach ($file in $toc.Files) {
                $relative = $file.Replace('\', $Separator).Replace('/', $Separator)
                $resolved = Get-FullPath (Join-Path $folder $relative)
                if ([IO.Path]::IsPathRooted($relative) -or
                    -not $resolved.StartsWith(($folder + $Separator), [StringComparison]::OrdinalIgnoreCase)) {
                    throw "The addon TOC contains a path outside its source folder: $file"
                }
                if (-not (Test-Path -LiteralPath (Join-Path $folder $relative) -PathType Leaf)) {
                    throw "The addon source is incomplete: $file is listed in the TOC but missing."
                }
            }
            return [pscustomobject]@{ Path = $folder; Toc = $toc }
        }
    }
    throw "Addon source not found. Looked in: $($candidates -join '; ')"
}

function Select-FolderInteractively([string] $Initial) {
    $canUseDialog = ([Environment]::OSVersion.Platform -eq [PlatformID]::Win32NT) -and
        ([Threading.Thread]::CurrentThread.GetApartmentState() -eq [Threading.ApartmentState]::STA)
    if ($canUseDialog) {
        Add-Type -AssemblyName System.Windows.Forms
        $dialog = New-Object System.Windows.Forms.FolderBrowserDialog
        $dialog.Description = 'Select the game''s Interface\AddOns folder (or the game folder that contains it).'
        $dialog.ShowNewFolderButton = $false
        if ($Initial -and (Test-Path -LiteralPath $Initial)) { $dialog.SelectedPath = $Initial }
        if ($dialog.ShowDialog() -ne [System.Windows.Forms.DialogResult]::OK) {
            throw [OperationCanceledException]::new('No folder selected.')
        }
        return $dialog.SelectedPath
    }
    $typed = Read-Host 'Path of the game''s Interface\AddOns folder (empty to cancel)'
    if (-not $typed) { throw [OperationCanceledException]::new('No folder entered.') }
    return $typed.Trim('"', ' ')
}

function Resolve-AddOnsDirectory([string] $Requested) {
    $chosen = $Requested
    if (-not $chosen) {
        $recorded = Get-RecordedAddOnsPath
        if ($recorded -and $Quiet) { $chosen = $recorded }
        elseif ($recorded) {
            $answer = Read-Host "Use the AddOns folder used last time? $recorded [Y/n]"
            if ($answer -eq '' -or $answer -match '^[Yy]') { $chosen = $recorded }
        }
    }
    if (-not $chosen) {
        if ($Quiet) { throw 'Pass -AddOnsPath: no AddOns folder is recorded and -Quiet forbids asking.' }
        $chosen = Select-FolderInteractively (Get-RecordedAddOnsPath)
    }
    if (-not (Test-Path -LiteralPath $chosen -PathType Container)) {
        throw "Folder not found: $chosen"
    }
    $full = Get-FullPath $chosen
    if ((Split-Path -Leaf $full) -ne 'AddOns') {
        $nested = Join-Path (Join-Path $full 'Interface') 'AddOns'
        if (Test-Path -LiteralPath $nested -PathType Container) { $full = Get-FullPath $nested }
        else {
            throw ("$full is not an AddOns folder and contains no Interface\AddOns. " +
                'Choose the folder the game itself uses for addons; this script does not create it ' +
                '(the Forever AddOns location is unverified, see docs/compatibility/ADDON_INSTALL.md).')
        }
    }
    return $full
}

function Get-GameFolder([string] $AddOns) {
    $interface = Split-Path -Parent $AddOns
    if ((Split-Path -Leaf $interface) -ne 'Interface') { return $null }
    return (Split-Path -Parent $interface)
}

function Assert-GameNotRunning([string] $AddOns) {
    $game = Get-GameFolder $AddOns
    if (-not $game) { return }
    $prefix = $game + $Separator
    $running = @(Get-ProcessPath | Where-Object { $_.Path.StartsWith($prefix, [StringComparison]::OrdinalIgnoreCase) })
    if ($running.Count -gt 0) {
        $names = ($running | ForEach-Object { $_.Name } | Sort-Object -Unique) -join ', '
        throw "Close the game first: $names is running from $game."
    }
}

function Get-ProcessPath {
    # Win32_Process reports executable paths across 32/64-bit boundaries; a 32-bit
    # PowerShell cannot read a 64-bit process's path through Get-Process.
    $found = @()
    if ([Environment]::OSVersion.Platform -eq [PlatformID]::Win32NT) {
        try {
            $found += @(Get-CimInstance -ClassName Win32_Process -ErrorAction Stop |
                    Where-Object { $_.ExecutablePath } |
                    ForEach-Object { [pscustomobject]@{ Name = $_.Name; Path = $_.ExecutablePath } })
        }
        catch { Write-Verbose "Win32_Process unavailable: $($_.Exception.Message)" }
    }
    foreach ($process in Get-Process -ErrorAction SilentlyContinue) {
        $path = $null
        try { $path = $process.Path } catch { $path = $null }
        if ($path) { $found += [pscustomobject]@{ Name = $process.ProcessName; Path = $path } }
    }
    return $found
}

function Get-TreeHashes([string] $Root) {
    $hashes = @{}
    $prefixLength = (Get-FullPath $Root).Length + 1
    foreach ($file in Get-ChildItem -LiteralPath $Root -Recurse -File -Force) {
        $relative = $file.FullName.Substring($prefixLength).Replace('\', '/')
        $hashes[$relative] = (Get-FileHash -LiteralPath $file.FullName -Algorithm SHA256).Hash
    }
    return $hashes
}

function Assert-SameTree([string] $Expected, [string] $Actual) {
    $want = Get-TreeHashes $Expected
    $have = Get-TreeHashes $Actual
    $problems = @()
    foreach ($key in $want.Keys) {
        if (-not $have.ContainsKey($key)) { $problems += "missing $key" }
        elseif ($have[$key] -ne $want[$key]) { $problems += "different $key" }
    }
    foreach ($key in $have.Keys) {
        if (-not $want.ContainsKey($key)) { $problems += "unexpected $key" }
    }
    if ($problems.Count -gt 0) { throw "Copy verification failed: $($problems -join ', ')" }
}

function New-SiblingName([string] $AddOns, [string] $Suffix) {
    $token = [Guid]::NewGuid().ToString('N').Substring(0, 8)
    return (Join-Path $AddOns "$AddonName.$Suffix-$token")
}

function Install-Addon {
    $source = Resolve-Source
    $addOns = Resolve-AddOnsDirectory $AddOnsPath
    $target = Join-Path $addOns $AddonName

    $existing = $null
    if (Test-Path -LiteralPath $target) {
        $existing = Read-Toc $target
        if (-not (Test-OwnAddon $existing)) {
            throw "$target exists but is not Azeroth Navigator; it was left untouched."
        }
    }
    Assert-GameNotRunning $addOns

    $from = if ($existing) { "version $($existing.Version)" } else { 'not installed' }
    if (-not $PSCmdlet.ShouldProcess($target, "Install addon version $($source.Toc.Version) ($from)")) { return }

    $staging = New-SiblingName $addOns 'installing'
    $previous = $null
    try {
        Copy-Item -LiteralPath $source.Path -Destination $staging -Recurse
        Assert-SameTree $source.Path $staging
        if ($existing) {
            $previous = New-SiblingName $addOns 'previous'
            Rename-Item -LiteralPath $target -NewName (Split-Path -Leaf $previous)
        }
        try {
            Rename-Item -LiteralPath $staging -NewName $AddonName
        }
        catch {
            if ($previous) { Rename-Item -LiteralPath $previous -NewName $AddonName }
            $previous = $null
            throw
        }
    }
    finally {
        if (Test-Path -LiteralPath $staging) { Remove-Item -LiteralPath $staging -Recurse -Force }
    }
    if ($previous) { Remove-Item -LiteralPath $previous -Recurse -Force }

    Set-RecordedAddOnsPath $addOns
    Write-Step "installed version $($source.Toc.Version) in $target ($from before)."
    Write-Step 'Start the game and enable "Azeroth Navigator Diagnostics" in the AddOns list.'
}

function Uninstall-Addon {
    $recorded = Get-RecordedAddOnsPath
    if (-not $AddOnsPath -and $recorded -and -not (Test-Path -LiteralPath $recorded -PathType Container)) {
        if ($PSCmdlet.ShouldProcess((Join-Path (Get-StateDirectory) $MarkerName), "Forget missing AddOns folder $recorded")) {
            Remove-Item -LiteralPath (Join-Path (Get-StateDirectory) $MarkerName)
        }
        Write-Step "the recorded AddOns folder no longer exists: $recorded. Nothing removed."
        return
    }
    $addOns = Resolve-AddOnsDirectory $AddOnsPath
    $target = Join-Path $addOns $AddonName
    $removedSomething = $false
    # SavedVariables can exist after the addon itself has been removed.
    if ($RemoveSavedVariables) { Assert-GameNotRunning $addOns }

    if (Test-Path -LiteralPath $target) {
        if (-not (Test-OwnAddon (Read-Toc $target))) {
            throw "$target is not Azeroth Navigator; it was left untouched."
        }
        Assert-GameNotRunning $addOns
        if ($PSCmdlet.ShouldProcess($target, 'Remove addon folder')) {
            Remove-Item -LiteralPath $target -Recurse -Force
            Write-Step "removed $target."
            $removedSomething = $true
        }
    }
    else {
        Write-Step "no addon folder at $target."
    }

    if ($RemoveSavedVariables) {
        $game = Get-GameFolder $addOns
        $accounts = if ($game) { Join-Path (Join-Path $game 'WTF') 'Account' } else { $null }
        if ($accounts -and (Test-Path -LiteralPath $accounts -PathType Container)) {
            foreach ($account in Get-ChildItem -LiteralPath $accounts -Directory) {
                foreach ($name in @("$AddonName.lua", "$AddonName.lua.bak")) {
                    $file = Join-Path (Join-Path $account.FullName 'SavedVariables') $name
                    if ((Test-Path -LiteralPath $file -PathType Leaf) -and
                        $PSCmdlet.ShouldProcess($file, 'Remove SavedVariables file')) {
                        Remove-Item -LiteralPath $file -Force
                        Write-Step "removed SavedVariables file in account folder $($account.Name)."
                        $removedSomething = $true
                    }
                }
            }
        }
        else {
            Write-Step 'no WTF\Account folder next to Interface; no SavedVariables removed.'
        }
    }

    if ($removedSomething -or -not (Test-Path -LiteralPath $target)) {
        if ($PSCmdlet.ShouldProcess((Join-Path (Get-StateDirectory) $MarkerName), 'Forget recorded AddOns folder')) {
            Clear-RecordedAddOnsPath $addOns
        }
    }
}

function Show-Status {
    try {
        $source = Resolve-Source
        Write-Step "bundled version: $($source.Toc.Version) ($($source.Path))"
    }
    catch { Write-Step "bundled version: unavailable ($($_.Exception.Message))" }

    $recorded = Get-RecordedAddOnsPath
    $addOns = if ($AddOnsPath) { $AddOnsPath } else { $recorded }
    if (-not $addOns) {
        Write-Step 'installed: unknown (no AddOns folder recorded; pass -AddOnsPath).'
        return
    }
    $addOns = Resolve-AddOnsDirectory $addOns
    $toc = Read-Toc (Join-Path $addOns $AddonName)
    if (Test-OwnAddon $toc) { Write-Step "installed version: $($toc.Version) in $addOns" }
    elseif ($toc) { Write-Step "installed: a different addon uses the folder name $AddonName in $addOns" }
    else { Write-Step "installed: no in $addOns" }
}

function Select-Action {
    if ($Action) { return $Action }
    if ($Quiet) { return 'Install' }
    Write-Host ''
    Write-Host 'Azeroth Navigator - World of Warcraft addon'
    Write-Host '  [I] Install or update   [U] Uninstall   [S] Status   [Q] Quit'
    switch -Regex (Read-Host 'Choice') {
        '^[Ii]' { return 'Install' }
        '^[Uu]' { return 'Uninstall' }
        '^[Ss]' { return 'Status' }
        default { throw [OperationCanceledException]::new('Nothing done.') }
    }
}

$exitCode = 0
try {
    switch (Select-Action) {
        'Install' { Install-Addon }
        'Uninstall' { Uninstall-Addon }
        'Status' { Show-Status }
    }
}
catch [OperationCanceledException] {
    Write-Step "cancelled: $($_.Exception.Message)"
    $exitCode = $ExitCancelled
}
catch {
    Write-Host "AzerothNavigator: failed: $($_.Exception.Message)" -ForegroundColor Red
    $exitCode = $ExitFailed
}

if (-not $Quiet -and -not $PSBoundParameters.ContainsKey('Action') -and $Host.Name -eq 'ConsoleHost') {
    # Launched from the Start menu: keep the window open so the result can be read.
    [void](Read-Host 'Press Enter to close')
}
exit $exitCode
