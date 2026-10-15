# Synthetic archive and artifact transport checks; no upstream source or game data.
[CmdletBinding()]
param()
Set-StrictMode -Version 3.0
$ErrorActionPreference = 'Stop'
. (Join-Path $PSScriptRoot 'Release-Materials.ps1')
$root = Join-Path ([IO.Path]::GetTempPath()) ('anav-material-tests-' + [Guid]::NewGuid().ToString('N'))
$count = 0
function Check([string] $Name, [bool] $Condition) {
    $script:count++
    if (-not $Condition) { throw "FAILED: $Name" }
    Write-Host "ok   $Name"
}
function Refuses([string] $Name, [scriptblock] $Action) {
    $refused = $false
    try { & $Action } catch { $refused = $true }
    Check $Name $refused
}
function Write-Text([string] $Path, [string] $Text) {
    New-Item -ItemType Directory -Path (Split-Path -Parent $Path) -Force | Out-Null
    [IO.File]::WriteAllText($Path, $Text, (New-Object Text.UTF8Encoding($false)))
}
function Set-Checksums([string] $Directory) {
    [string[]] $names = @(Get-ChildItem -LiteralPath $Directory -File | Where-Object Name -ne 'SHA256SUMS.txt' | ForEach-Object Name)
    [Array]::Sort($names, [StringComparer]::Ordinal)
    $lines = foreach ($name in $names) { (Get-FileHash -LiteralPath (Join-Path $Directory $name) -Algorithm SHA256).Hash.ToLowerInvariant() + '  ' + $name }
    Write-Text (Join-Path $Directory 'SHA256SUMS.txt') (($lines -join "`n") + "`n")
}
try {
    $stage = Join-Path $root 'stage'
    $output = Join-Path $root 'output'
    New-Item -ItemType Directory -Path $stage, $output -Force | Out-Null
    $name = 'source-materials/vendor/raw.lua'
    $raw = Join-Path $stage $name
    Write-Text $raw '-- synthetic inert source'
    [byte[]]$bytes = @(0, 255, 13, 10, 97, 98, 99)
    [IO.File]::WriteAllBytes($raw, $bytes)
    Write-Text (Join-Path $stage 'unselected-private-proof.txt') 'Do not publish this file.'
    & git -C $root init -q
    & git -C $root -c user.name=MaterialTest -c user.email=test@example.invalid add .
    & git -C $root -c user.name=MaterialTest -c user.email=test@example.invalid -c commit.gpgsign=false commit -q -m 'Synthetic material transport'
    if ($LASTEXITCODE -ne 0) { throw 'Cannot initialize synthetic repository.' }
    $head = & git -C $root rev-parse HEAD
    $manifest = [ordered]@{schema_version=1; version='0.8.1'; candidate_source_commit=$head; production_sha256=('1' * 64); inventory_sha256=('2' * 64); target=@{edition='wow_forever';version='1.60.1';build='70374';interface=16001}; packs=@(); sources=@(); files=@(@{archive_path=$name;kind='source_material';input=@{path='vendor/raw.lua';bytes=$bytes.Length;sha256=(Get-FileHash $raw -Algorithm SHA256).Hash.ToLowerInvariant()}})}
    $selection = [pscustomobject]@{Json=(($manifest | ConvertTo-Json -Depth 12).Replace("`r`n", "`n") + "`n");Manifest=$manifest}
    Write-Text (Join-Path $stage 'RELEASE_MATERIALS.json') $selection.Json
    $archive = Join-Path $output 'AzerothNavigator-materials-0.8.1.zip'
    New-ReleaseMaterialArchive $stage $archive $selection 315532800
    Check 'bounded material archive validates' $true
    $again = Join-Path $root 'again.zip'
    New-ReleaseMaterialArchive $stage $again $selection 315532800
    Check 'material ZIP bytes are deterministic' ((Get-FileHash $archive).Hash -ceq (Get-FileHash $again).Hash)
    [IO.File]::WriteAllBytes($raw, [byte[]]@(1, 2, 3))
    $refusedArchive = Join-Path $root 'must-not-exist.zip'
    Refuses 'changed staged source refuses before archive mutation' { New-ReleaseMaterialArchive $stage $refusedArchive $selection 315532800 }
    Check 'refused archive was not created' (-not (Test-Path -LiteralPath $refusedArchive))
    [IO.File]::WriteAllBytes($raw, $bytes)
    Add-Type -AssemblyName System.IO.Compression
    $stream = [IO.File]::OpenRead($archive)
    $zip = New-Object IO.Compression.ZipArchive($stream, [IO.Compression.ZipArchiveMode]::Read)
    try {
        Check 'only selected material and public manifest are included' ($zip.Entries.Count -eq 2 -and $null -eq $zip.GetEntry('unselected-private-proof.txt'))
        $input = $zip.GetEntry($name).Open()
        $memory = New-Object IO.MemoryStream
        try { $input.CopyTo($memory); Check 'raw non-UTF8 and CRLF bytes remain exact' ([Convert]::ToBase64String($memory.ToArray()) -ceq [Convert]::ToBase64String($bytes)) }
        finally { $input.Dispose(); $memory.Dispose() }
    } finally { $zip.Dispose(); $stream.Dispose() }
    foreach ($change in @('missing', 'changed', 'extra', 'traversal', 'duplicate', 'link', 'manifest')) {
        $bad = Join-Path $root ($change + '.zip')
        Copy-Item -LiteralPath $archive -Destination $bad
        $stream = [IO.File]::Open($bad, [IO.FileMode]::Open)
        $zip = New-Object IO.Compression.ZipArchive($stream, [IO.Compression.ZipArchiveMode]::Update)
        try {
            if ($change -eq 'missing') { $zip.GetEntry($name).Delete() }
            elseif ($change -eq 'link') { $zip.GetEntry($name).ExternalAttributes = -1610612736 }
            else {
                $entryName = switch ($change) { 'traversal' { '../private.txt' } 'extra' { 'extra.lua' } 'manifest' { 'RELEASE_MATERIALS.json' } default { $name } }
                if ($change -in @('changed', 'manifest')) { $zip.GetEntry($entryName).Delete() }
                $entry = $zip.CreateEntry($entryName)
                $writer = New-Object IO.StreamWriter($entry.Open())
                try { $writer.Write('tampered') } finally { $writer.Dispose() }
            }
        } finally { $zip.Dispose(); $stream.Dispose() }
        Refuses "$change ZIP material is refused" { Test-ReleaseMaterialArchive $bad $selection }
    }
    # Substitute only the selection adapter for isolated artifact checks. The
    # production entrypoint recomputes this with the tested Rust selector.
    function Get-ReleaseMaterialSelection([string] $Root, [string] $Version) { return $selection }
    Write-Text (Join-Path $output 'RELEASE_MATERIALS.json') $selection.Json
    Write-Text (Join-Path $output 'SOURCE_COMMIT.txt') ($head + "`n")
    Write-Text (Join-Path $output 'AzerothNavigator-addon-0.8.1.zip') 'synthetic addon artifact'
    $null = Write-ReleaseArtifactManifest $root '0.8.1' $output $true $selection
    $expectedFiles = foreach ($fileName in @('AzerothNavigator-addon-0.8.1.zip', 'AzerothNavigator-materials-0.8.1.zip', 'RELEASE_MATERIALS.json', 'SOURCE_COMMIT.txt')) {
        $filePath = Join-Path $output $fileName
        '{"path":"' + $fileName + '","bytes":' + (Get-Item -LiteralPath $filePath).Length + ',"sha256":"' + (Get-FileHash -LiteralPath $filePath).Hash.ToLowerInvariant() + '"}'
    }
    $expectedManifest = '{"schema_version":1,"version":"0.8.1","source_commit":"' + $head + '","files":[' + ($expectedFiles -join ',') + "]}`n"
    Check 'asset manifest has fixed canonical bytes across PowerShell versions' ((Read-BoundedReleaseText (Join-Path $output 'RELEASE_ASSETS.json')) -ceq $expectedManifest)
    Set-Checksums $output
    Assert-ReleaseArtifacts $root '0.8.1' $output $true
    Check 'complete artifact set validates independently' $true
    Write-Text (Join-Path $output 'SOURCE_COMMIT.txt') (('3' * 40) + "`n")
    Refuses 'wrong source commit is refused even with rebuilt checksums' { Set-Checksums $output; Assert-ReleaseArtifacts $root '0.8.1' $output $true }
    Write-Text (Join-Path $output 'SOURCE_COMMIT.txt') ($head + "`n")
    Remove-Item -LiteralPath (Join-Path $output 'RELEASE_ASSETS.json')
    Refuses 'missing artifact manifest is refused' { Assert-ReleaseArtifacts $root '0.8.1' $output $true }
    $null = Write-ReleaseArtifactManifest $root '0.8.1' $output $true $selection
    Copy-Item -LiteralPath (Join-Path $root 'changed.zip') -Destination $archive -Force
    $null = Write-ReleaseArtifactManifest $root '0.8.1' $output $true $selection
    Set-Checksums $output
    Refuses 'tampered material fails despite rebuilt outer manifests' { Assert-ReleaseArtifacts $root '0.8.1' $output $true }
    Write-Host "All $count material delivery checks passed."
} finally {
    if (Test-Path -LiteralPath $root) { Remove-Item -LiteralPath $root -Recurse -Force }
}
exit 0
