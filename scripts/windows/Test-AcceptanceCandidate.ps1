<# Synthetic candidate transport integration. Only Cargo's gate/material responses
   are stubbed; actual packaging, ZIP bytes, Git source and verifier execute.
   This is not content readiness, native installation or live acceptance. #>
[CmdletBinding()]
param()
Set-StrictMode -Version 3.0
$ErrorActionPreference = 'Stop'
. (Join-Path $PSScriptRoot 'Release-Materials.ps1')
$testRoot = Join-Path ([IO.Path]::GetTempPath()) ('anav-candidate-' + [Guid]::NewGuid().ToString('N'))
$count = 0
$oldRefType = $env:REF_TYPE
$oldGitHubRefType = $env:GITHUB_REF_TYPE
function Check([string] $Name, [bool] $Condition) { $script:count++; if (-not $Condition) { throw "FAILED: $Name" }; Write-Host "ok $Name" }
function Refuses([string] $Name, [scriptblock] $Action) { $refused=$false; try { & $Action } catch { $refused=$true }; Check $Name $refused }
function Write-Text([string] $Path, [string] $Text) {
    New-Item -ItemType Directory -Path (Split-Path -Parent $Path) -Force | Out-Null
    [IO.File]::WriteAllText($Path, $Text, (New-Object Text.UTF8Encoding($false)))
}
function Rehash([string] $Directory) {
    [string[]]$names = @(Get-ChildItem -LiteralPath $Directory -File | Where-Object Name -ne SHA256SUMS.txt | ForEach-Object Name)
    [Array]::Sort($names, [StringComparer]::Ordinal)
    $lines = foreach ($name in $names) { (Get-FileHash -LiteralPath (Join-Path $Directory $name)).Hash.ToLowerInvariant() + '  ' + $name }
    Write-Text (Join-Path $Directory 'SHA256SUMS.txt') (($lines -join "`n") + "`n")
}
try {
    New-Item -ItemType Directory -Path $testRoot | Out-Null
    foreach ($file in @('Build-Windows.ps1', 'Release-Materials.ps1')) {
        Write-Text (Join-Path $testRoot "scripts/windows/$file") ([IO.File]::ReadAllText((Join-Path $PSScriptRoot $file)))
    }
    Write-Text (Join-Path $testRoot 'Cargo.toml') "[workspace.package]`nversion = `"1.0.0`"`n"
    foreach ($file in @('package.json', 'apps/desktop/package.json', 'apps/desktop/src-tauri/tauri.conf.json')) {
        Write-Text (Join-Path $testRoot $file) '{"version":"1.0.0"}'
    }
    Write-Text (Join-Path $testRoot 'addons/AzerothNavigator/AzerothNavigator.toc') "## Version: 1.0.0`n"
    Write-Text (Join-Path $testRoot 'addons/AzerothNavigator/Core/Version.lua') "addon = `"1.0.0`",`n"
    Write-Text (Join-Path $testRoot 'addons/AzerothNavigator/LICENSE') 'Synthetic test license only.'
    Write-Text (Join-Path $testRoot 'docs/releases/1.0.0.md') 'Synthetic candidate transport fixture, no game content.'
    Write-Text (Join-Path $testRoot 'source.txt') 'Synthetic source bytes.'
    & git -C $testRoot init -q
    & git -C $testRoot add .
    & git -C $testRoot -c user.name=CandidateTest -c user.email=test@example.invalid -c commit.gpgsign=false commit -q -m 'Synthetic candidate transport'
    if ($LASTEXITCODE -ne 0) { throw 'Cannot initialize synthetic checkout.' }
    $head = & git -C $testRoot rev-parse HEAD
    $report = [ordered]@{candidate_ready=$true; publication_allowed=$false; candidate_source_commit=$head; production_sha256=('1' * 64); inventory_sha256=('2' * 64)}
    $material = [ordered]@{schema_version=1; version='1.0.0'; candidate_source_commit=$head; production_sha256=('1' * 64); inventory_sha256=('2' * 64); target=@{edition='wow_forever'; version='1.60.1'; build='70374'; interface=16001}; packs=@(); sources=@(); files=@(@{archive_path='source-materials/source.txt'; kind='source_material'; input=@{path='source.txt'; bytes=(Get-Item (Join-Path $testRoot 'source.txt')).Length; sha256=(Get-FileHash (Join-Path $testRoot 'source.txt')).Hash.ToLowerInvariant()}})}
    $materialJson = ($material | ConvertTo-Json -Depth 12 -Compress) + "`n"
    $calls = New-Object 'System.Collections.Generic.List[string]'
    # Script-local function shadows native Cargo only for these synthetic calls.
    function cargo {
        $global:LASTEXITCODE = 0
        if ($args -contains 'candidate-readiness') { $calls.Add('candidate'); $report | ConvertTo-Json -Compress; return }
        if ($args -contains 'release-readiness') { $calls.Add('publication'); $global:LASTEXITCODE = 37; '{"publication_allowed":false}'; return }
        if ($args -contains 'release-materials') {
            $calls.Add('materials')
            $index = [Array]::IndexOf($args, '--output')
            if ($index -ge 0) {
                $stage = $args[$index + 1]
                Write-Text (Join-Path $stage 'source-materials/source.txt') ([IO.File]::ReadAllText((Join-Path $testRoot 'source.txt')))
                Write-Text (Join-Path $stage 'RELEASE_MATERIALS.json') $materialJson
            }
            $materialJson.TrimEnd("`n"); return
        }
        throw 'Unexpected synthetic Cargo request.'
    }
    $env:REF_TYPE = 'branch'; $env:GITHUB_REF_TYPE = 'branch'
    $output = Join-Path $testRoot 'candidate-output'
    & (Join-Path $testRoot 'scripts/windows/Build-Windows.ps1') -AddonOnly -AcceptanceCandidate -ExpectedVersion 1.0.0 -OutputDirectory $output
    Check 'positive candidate packaging reaches real ZIP and verifier' (Test-Path (Join-Path $output 'AzerothNavigator-addon-1.0.0.zip'))
    $expectedMarker = '{"schema_version":1,"kind":"acceptance_candidate","publication_allowed":false,"version":"1.0.0","source_commit":"' + $head + '","production_sha256":"' + ('1' * 64) + '","inventory_sha256":"' + ('2' * 64) + "`"}`n"
    $markerPath = Join-Path $output 'ACCEPTANCE_CANDIDATE.json'
    Check 'marker bytes are exact canonical non-publication declaration' ((Read-BoundedReleaseText $markerPath) -ceq $expectedMarker)
    Check 'candidate gate is rechecked before and after packaging' (@($calls | Where-Object { $_ -eq 'candidate' }).Count -ge 3)
    Check 'candidate packaging never invokes publication gate' (-not $calls.Contains('publication'))
    Assert-ReleaseArtifacts $testRoot '1.0.0' $output $true $true
    Check 'independent opt-in verification accepts complete candidate' $true
    Refuses 'default verifier refuses marked candidate' { Assert-ReleaseArtifacts $testRoot '1.0.0' $output $true }
    Write-Text $markerPath ($expectedMarker.Replace('"publication_allowed":false', '"publication_allowed":true'))
    Rehash $output
    Refuses 'tampered marker refuses despite rebuilt checksums' { Assert-ReleaseArtifacts $testRoot '1.0.0' $output $true $true }
    Write-Text $markerPath $expectedMarker
    Rehash $output
    $report.inventory_sha256 = '3' * 64
    Refuses 'candidate inventory mismatch refuses' { Assert-ReleaseArtifacts $testRoot '1.0.0' $output $true $true }
    $report.inventory_sha256 = '2' * 64
    Remove-Item -LiteralPath $markerPath
    Rehash $output
    Refuses 'stripped marker cannot bypass full stable readiness' { Assert-ReleaseArtifacts $testRoot '1.0.0' $output $true }
    Check 'stripped marker invokes actual publication dispatch' ($calls.Contains('publication'))
    foreach ($variable in @('REF_TYPE', 'GITHUB_REF_TYPE')) {
        $env:REF_TYPE='branch'; $env:GITHUB_REF_TYPE='branch'
        Set-Item -LiteralPath ('Env:' + $variable) -Value tag
        $before = $calls.Count
        $sentinel = Join-Path $output 'SHA256SUMS.txt'
        $saved = [IO.File]::ReadAllText($sentinel)
        Refuses "$variable tag refuses candidate packaging" { & (Join-Path $testRoot 'scripts/windows/Build-Windows.ps1') -AddonOnly -AcceptanceCandidate -AllowDirty -OutputDirectory $output }
        Check "$variable tag refuses before Cargo or output mutation" ($calls.Count -eq $before -and [IO.File]::ReadAllText($sentinel) -ceq $saved)
    }
    Write-Host "All $count synthetic candidate transport checks passed. Real content readiness/live acceptance remains unverified."
} finally {
    $env:REF_TYPE=$oldRefType; $env:GITHUB_REF_TYPE=$oldGitHubRefType
    Remove-Item Function:cargo -ErrorAction SilentlyContinue
    Remove-Item -LiteralPath $testRoot -Recurse -Force
}
