[CmdletBinding()]
param()
Set-StrictMode -Version 3.0
$ErrorActionPreference = 'Stop'
. (Join-Path $PSScriptRoot 'Acceptance-Kit.ps1')
$root = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot '../..'))
$temp = Join-Path ([IO.Path]::GetTempPath()) ('anav-acceptance-' + [Guid]::NewGuid().ToString('N'))
$count = 0
function Check([string] $Name, [bool] $Condition) { $script:count++; if (-not $Condition) { throw "FAILED: $Name" }; Write-Host "ok $Name" }
function Refuses([string] $Name, [scriptblock] $Action) { $refused=$false; try { & $Action } catch { $refused=$true }; Check $Name $refused }
function Json([string] $Path, $Value) { [IO.File]::WriteAllText($Path, ($Value | ConvertTo-Json -Depth 20)) }
try {
    New-Item -ItemType Directory -Path $temp | Out-Null
    Assert-AcceptanceToolRoot $root $PSScriptRoot
    Check 'tooling belongs to candidate checkout' $true
    Refuses 'foreign checkout tooling cannot claim candidate identity' { Assert-AcceptanceToolRoot $temp $PSScriptRoot }
    # Legacy contracts are owned bounded fixtures, never the mutable schema4 release.
    $fixtureRoot = Join-Path $temp 'legacy-candidate'
    New-Item -ItemType Directory -Path (Join-Path $fixtureRoot 'release/v1'), (Join-Path $fixtureRoot 'docs/compatibility') -Force | Out-Null
    Copy-Item -LiteralPath (Join-Path $root 'docs/compatibility/V1_ACCEPTANCE_RUNBOOK.md') -Destination (Join-Path $fixtureRoot 'docs/compatibility/V1_ACCEPTANCE_RUNBOOK.md')
    $windows = @('WIN-INSTALL','WIN-UPGRADE','WIN-UNINSTALL','WIN-RECOVERY','WIN-OFFLINE','WIN-FOCUS','WIN-REFERENCE','WIN-DISPLAY','WIN-ACCESSIBILITY','WIN-PERFORMANCE')
    $live = @('LIVE-BUILD','LIVE-CAPTURE','LIVE-TRANSFER','LIVE-RELOAD','LIVE-RESTRICTIONS','LIVE-HUD','LIVE-PLAN','LIVE-ADVANCE','LIVE-DEVIATION','LIVE-DUO','LIVE-GEAR','LIVE-DUNGEON','LIVE-REMINDERS','LIVE-SESSION','LIVE-NEXT-PLAN')
    $domains = @('acquisition','class_roles','dungeons','equipment','geography','party','quests','reminders','travel')
    $inventory = [ordered]@{schema_version=3; domains=@($domains | ForEach-Object { @{domain=$_; units=@(@{id=('synthetic-' + $_)})} })}
    $manifest = [ordered]@{schema_version=3; checks=@($windows | ForEach-Object { @{id=$_; method='interactive_windows'} })}
    $inventoryPath = Join-Path $fixtureRoot 'release/v1/coverage-inventory.json'
    $manifestPath = Join-Path $fixtureRoot 'release/v1/acceptance-evidence.json'
    Json $inventoryPath $inventory
    Json $manifestPath $manifest
    $scenarios = [ordered]@{schema_version=1; inventory_sha256=(Get-AcceptanceHash $inventoryPath); scenarios=@()}
    foreach ($domain in $inventory.domains) { foreach ($unit in $domain.units) {
        $scenarios.scenarios += @{id=('SCENARIO-' + $domain.domain.Replace('_', '-')); domain=$domain.domain; unit_id=$unit.id; method='interactive_windows'; procedure='Synthetic procedure'; expected='Synthetic expectation'}
    } }
    $scenarioPath = Join-Path $temp 'scenarios.json'
    Json $scenarioPath $scenarios
    foreach ($schema in 1, 2, 3) {
        $inventory.schema_version = $schema
        $manifest.schema_version = $schema
        $manifest.checks = @($windows | ForEach-Object { @{id=$_; method='interactive_windows'} })
        if ($schema -ne 3) { $manifest.checks += @($live | ForEach-Object { @{id=$_; method='live_game'} }) }
        Json $inventoryPath $inventory
        Json $manifestPath $manifest
        $scenarios.inventory_sha256 = Get-AcceptanceHash $inventoryPath
        Json $scenarioPath $scenarios
        $rows = @(Get-AcceptanceProcedures $fixtureRoot $scenarioPath)
        $liveCount = if ($schema -eq 3) { 0 } else { 15 }
        Check "legacy schema $schema retains exact runbook and unit accounting" ($rows.Count -eq (10 + $liveCount + 9) -and @($rows | Where-Object method -eq 'live_game').Count -eq $liveCount -and @($rows | Where-Object inventory_unit -ne $null).Count -eq 9)
    }
    $inventory.schema_version = 4; $manifest.schema_version = 4
    Json $inventoryPath $inventory; Json $manifestPath $manifest
    $message = ''
    try { Get-AcceptanceProcedures $fixtureRoot $scenarioPath | Out-Null } catch { $message = $_.Exception.Message }
    Check 'schema4 refuses explicitly rather than becoming legacy acceptance' ($message -ceq 'Unsupported acceptance or inventory schema.')
    $inventory.schema_version = 3; $manifest.schema_version = 3
    Json $inventoryPath $inventory; Json $manifestPath $manifest
    Check 'schema3 queues no live-client procedure' (@($rows | Where-Object method -eq 'live_game').Count -eq 0)
    $scenarios.inventory_sha256 = '0' * 64
    Json $scenarioPath $scenarios
    Refuses 'stale inventory scenarios refuse' { Get-AcceptanceProcedures $fixtureRoot $scenarioPath }
    $scenarios.inventory_sha256 = Get-AcceptanceHash $inventoryPath
    $scenarios.scenarios = @($scenarios.scenarios | Select-Object -Skip 1)
    Json $scenarioPath $scenarios
    Refuses 'missing unit scenarios refuse' { Get-AcceptanceProcedures $fixtureRoot $scenarioPath }
    $row = $rows[0]
    Refuses 'pass without evidence refuses' { New-AcceptanceResult $row pass observed now 0 @() }
    Refuses 'unknown result vocabulary refuses' { New-AcceptanceResult $row skipped reason now 0 @() }
    $result = New-AcceptanceResult $row unverified 'No native environment' now 0 @()
    Check 'unverified retains explicit reason and zero duration' ($result.status -eq 'unverified' -and $result.duration_ms -eq 0 -and $result.evidence.Count -eq 0)
    Write-AcceptanceDraft $temp @{results=@($result)}
    Check 'checkpoint is valid JSON' (((Get-Content -Raw (Join-Path $temp 'session.json')) | ConvertFrom-Json).results[0].status -eq 'unverified')
    $installed = Join-Path $temp 'installed'
    New-Item -ItemType Directory -Path $installed | Out-Null
    [IO.File]::WriteAllText((Join-Path $installed 'Core.lua'), 'inert synthetic data')
    Add-Type -AssemblyName System.IO.Compression
    $archive = Join-Path $temp 'addon.zip'
    $stream = [IO.File]::Open($archive, [IO.FileMode]::CreateNew)
    $zip = New-Object IO.Compression.ZipArchive($stream, [IO.Compression.ZipArchiveMode]::Create)
    $entry = $zip.CreateEntry('AzerothNavigator/Core.lua')
    $writer = New-Object IO.StreamWriter($entry.Open())
    $writer.Write('inert synthetic data'); $writer.Dispose(); $zip.Dispose(); $stream.Dispose()
    Assert-InstalledAcceptanceAddon $archive $installed
    Check 'installed addon exact files verify' $true
    [IO.File]::WriteAllText((Join-Path $installed 'Extra.lua'), 'extra')
    Refuses 'extra installed file refuses' { Assert-InstalledAcceptanceAddon $archive $installed }
    Remove-Item -LiteralPath (Join-Path $installed 'Extra.lua')
    [IO.File]::WriteAllText((Join-Path $installed 'Core.lua'), 'tampered synthetic!')
    Refuses 'changed installed file refuses' { Assert-InstalledAcceptanceAddon $archive $installed }
    Remove-Item -LiteralPath (Join-Path $installed 'Core.lua')
    Refuses 'missing installed file refuses' { Assert-InstalledAcceptanceAddon $archive $installed }
    foreach ($unsafeName in @('../Core.lua', 'AzerothNavigator/../Core.lua', 'AzerothNavigator/Core.lua')) {
        $stream = [IO.File]::Open($archive, [IO.FileMode]::Open)
        $zip = New-Object IO.Compression.ZipArchive($stream, [IO.Compression.ZipArchiveMode]::Update)
        $entry = $zip.CreateEntry($unsafeName)
        $writer = New-Object IO.StreamWriter($entry.Open())
        $writer.Write('inert synthetic data'); $writer.Dispose(); $zip.Dispose(); $stream.Dispose()
        [IO.File]::WriteAllText((Join-Path $installed 'Core.lua'), 'inert synthetic data')
        Refuses 'unsafe or duplicate ZIP entry refuses' { Assert-InstalledAcceptanceAddon $archive $installed }
        $stream = [IO.File]::Open($archive, [IO.FileMode]::Open)
        $zip = New-Object IO.Compression.ZipArchive($stream, [IO.Compression.ZipArchiveMode]::Update)
        $zip.Entries[$zip.Entries.Count - 1].Delete(); $zip.Dispose(); $stream.Dispose()
    }
    $tokens=$null; $errors=$null
    $null = [Management.Automation.Language.Parser]::ParseFile((Join-Path $PSScriptRoot 'Start-AcceptanceSession.ps1'), [ref]$tokens, [ref]$errors)
    Check 'Windows runner parses' ($errors.Count -eq 0)
    Write-Host "All $count synthetic acceptance-kit checks passed. Native/live acceptance was not run."
} finally { Remove-Item -LiteralPath $temp -Recurse -Force }
