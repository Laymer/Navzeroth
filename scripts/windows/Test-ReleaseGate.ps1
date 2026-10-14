<# Tests stable/prerelease packaging dispatch and preservation before refusal.
   Synthetic temporary checkout only; no release, real installer or game access. #>
[CmdletBinding()]
param([string] $Shell)
Set-StrictMode -Version 3.0
$ErrorActionPreference = 'Stop'
if (-not $Shell) { $Shell = (Get-Process -Id $PID).Path }
$onWindows = [Environment]::OSVersion.Platform -eq [PlatformID]::Win32NT
$testRoot = Join-Path ([IO.Path]::GetTempPath()) ('anav-release-gate-' + [Guid]::NewGuid().ToString('N'))
$oldPath = $env:PATH
$oldMarker = $env:NAVZEROTH_TEST_CARGO_CALLED
$count = 0
function Write-Text([string] $Relative, [string] $Text) {
    $path = Join-Path $testRoot $Relative
    New-Item -ItemType Directory -Path (Split-Path -Parent $path) -Force | Out-Null
    [IO.File]::WriteAllText($path, $Text, (New-Object Text.UTF8Encoding($false)))
}
function Set-Version([string] $Version) {
    Write-Text 'Cargo.toml' ("[workspace.package]`nversion = `"$Version`"`n")
    foreach ($file in @('package.json', 'apps/desktop/package.json', 'apps/desktop/src-tauri/tauri.conf.json')) {
        Write-Text $file (@{version=$Version} | ConvertTo-Json)
    }
    Write-Text 'addons/AzerothNavigator/AzerothNavigator.toc' ("## Version: $Version`n")
    Write-Text 'addons/AzerothNavigator/Core/Version.lua' ("addon = `"$Version`",`n")
}
function Check([string] $Name, [bool] $Condition) {
    $script:count++
    if (-not $Condition) { throw "FAILED: $Name" }
    Write-Host "ok   $Name"
}
try {
    New-Item -ItemType Directory -Path $testRoot | Out-Null
    Write-Text 'scripts/windows/Build-Windows.ps1' ([IO.File]::ReadAllText((Join-Path $PSScriptRoot 'Build-Windows.ps1')))
    Set-Version '0.8.1'
    Write-Text 'addons/AzerothNavigator/LICENSE' 'Synthetic test license; not production content.'
    & git -C $testRoot init -q
    & git -C $testRoot -c user.name=ReleaseGateTest -c user.email=test@example.invalid add .
    & git -C $testRoot -c user.name=ReleaseGateTest -c user.email=test@example.invalid -c commit.gpgsign=false commit -q -m 'Synthetic packaging boundary'
    if ($LASTEXITCODE -ne 0) { throw 'Cannot create synthetic test checkout.' }
    $env:NAVZEROTH_TEST_CARGO_CALLED = Join-Path $testRoot 'cargo-called.txt'
    if ($onWindows) {
        Write-Text 'bin/cargo.cmd' "@echo off`r`necho invoked>`"%NAVZEROTH_TEST_CARGO_CALLED%`"`r`nexit /b 37`r`n"
    } else {
        Write-Text 'bin/cargo' '#!/bin/sh
printf "%s\n" invoked > "$NAVZEROTH_TEST_CARGO_CALLED"
exit 37
'
        & chmod +x (Join-Path $testRoot 'bin/cargo')
    }
    $env:PATH = (Join-Path $testRoot 'bin') + [IO.Path]::PathSeparator + $oldPath
    foreach ($version in @('1.0.0', '2.0.0', '1.0.0-rc.1', '0.8.1', '1.0.0-', '1.0.0-01', '01.0.0')) {
        Set-Version $version
        $output = Join-Path $testRoot ('output-' + $version)
        New-Item -ItemType Directory -Path $output | Out-Null
        $sentinel = Join-Path $output 'SHA256SUMS.txt'
        [IO.File]::WriteAllText($sentinel, 'preserve previous artifact')
        if (Test-Path -LiteralPath $env:NAVZEROTH_TEST_CARGO_CALLED) { Remove-Item -LiteralPath $env:NAVZEROTH_TEST_CARGO_CALLED }
        $saved = $ErrorActionPreference
        $ErrorActionPreference = 'Continue'
        try {
            $result = & $Shell -NoProfile -NonInteractive -ExecutionPolicy Bypass -File (Join-Path $testRoot 'scripts/windows/Build-Windows.ps1') -AddonOnly -AllowDirty -ExpectedVersion $version -OutputDirectory $output 2>&1 | Out-String
            $code = $LASTEXITCODE
        } finally { $ErrorActionPreference = $saved }
        $called = Test-Path -LiteralPath $env:NAVZEROTH_TEST_CARGO_CALLED
        if ($version -in @('1.0.0', '2.0.0')) {
            Check "$version invokes the stable gate even with AddonOnly/AllowDirty" $called
            Check "$version gate refusal is nonzero" ($code -ne 0)
            Check "$version refusal retains previous artifacts" (([IO.File]::ReadAllText($sentinel)) -eq 'preserve previous artifact')
        } elseif ($version -in @('1.0.0-rc.1', '0.8.1')) {
            Check "$version preview addon package succeeds without Cargo" ($code -eq 0)
            Check "$version does not invoke the stable-only packaging gate" (-not $called)
            Check "$version addon ZIP exists" (Test-Path -LiteralPath (Join-Path $output "AzerothNavigator-addon-$version.zip"))
        } else {
            Check "$version malformed version is rejected" ($code -ne 0)
            Check "$version rejection occurs before Cargo" (-not $called)
            Check "$version rejection retains previous artifacts" (([IO.File]::ReadAllText($sentinel)) -eq 'preserve previous artifact')
        }
    }
    Write-Host "All $count release packaging checks passed."
} finally {
    $env:PATH = $oldPath
    $env:NAVZEROTH_TEST_CARGO_CALLED = $oldMarker
    if (Test-Path -LiteralPath $testRoot) { Remove-Item -LiteralPath $testRoot -Recurse -Force }
}
