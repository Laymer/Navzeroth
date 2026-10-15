# Shared release-material packaging/verification. Dot-source only; no main action.
Set-StrictMode -Version 3.0

function Test-StableReleaseVersion([string] $Version) {
    $parts = $Version -split '-', 2
    if ($parts[0] -notmatch '^(0|[1-9][0-9]{0,8})\.(0|[1-9][0-9]{0,8})\.(0|[1-9][0-9]{0,8})$') { throw 'Invalid release version.' }
    if ($parts.Count -eq 2) {
        if ($parts[1].Length -gt 128) { throw 'Invalid prerelease version.' }
        foreach ($identifier in ($parts[1] -split '\.')) {
            if ($identifier -notmatch '^[0-9A-Za-z-]+$' -or ($identifier -match '^[0-9]+$' -and $identifier.Length -gt 1 -and $identifier.StartsWith('0'))) { throw 'Invalid prerelease version.' }
        }
        return $false
    }
    return -not $parts[0].StartsWith('0.')
}

function Assert-ReleaseDestination([string] $Path) {
    $existing = [IO.Path]::GetFullPath($Path)
    while (-not (Test-Path -LiteralPath $existing)) {
        $parent = [IO.Directory]::GetParent($existing)
        if (-not $parent) { throw 'Release destination has no existing parent.' }
        $existing = $parent.FullName
    }
    Assert-RegularReleasePath $existing -Directory
}

function Assert-RegularReleasePath([string] $Path, [switch] $Directory) {
    $current = [IO.Path]::GetFullPath($Path)
    $item = Get-Item -LiteralPath $current -Force -ErrorAction Stop
    if ([bool]$item.PSIsContainer -ne [bool]$Directory) { throw 'Release path has the wrong file kind.' }
    while ($current) {
        $entry = Get-Item -LiteralPath $current -Force -ErrorAction Stop
        if ($entry.Attributes -band [IO.FileAttributes]::ReparsePoint) { throw 'Release paths cannot traverse reparse points.' }
        $parent = [IO.Directory]::GetParent($current)
        $current = if ($parent) { $parent.FullName } else { $null }
    }
}

function Read-BoundedReleaseText([string] $Path) {
    Assert-RegularReleasePath $Path
    if ((Get-Item -LiteralPath $Path).Length -gt 16777216) { throw 'Release JSON exceeds its size limit.' }
    return [IO.File]::ReadAllText($Path, (New-Object Text.UTF8Encoding($false, $true)))
}

function Test-EmptyMaterialSelection([string] $Root) {
    $inventory = Join-Path $Root 'release/v1/coverage-inventory.json'
    $evidence = Join-Path $Root 'release/v1/acceptance-evidence.json'
    # Historical addon-only preview fixtures have neither release contract.
    if (-not (Test-Path -LiteralPath $inventory) -and -not (Test-Path -LiteralPath $evidence)) { return $true }
    $i = Read-BoundedReleaseText $inventory | ConvertFrom-Json
    $e = Read-BoundedReleaseText $evidence | ConvertFrom-Json
    if ($i.schema_version -ne 1 -or $e.schema_version -ne 1) { throw 'Unsupported release material schema.' }
    return @($i.sources).Count -eq 0 -and @($e.sources).Count -eq 0 -and @($e.packs).Count -eq 0
}

function Invoke-MaterialCli([string] $Root, [string] $Version, [string] $Output) {
    $arguments = @('run', '--locked', '-p', 'azeroth-cli', '--', 'release-materials', '--root', $Root, '--version', $Version)
    if ($Output) { $arguments += @('--output', $Output) }
    $savedEncoding = [Console]::OutputEncoding
    $savedPreference = $ErrorActionPreference
    Push-Location $Root
    try {
        [Console]::OutputEncoding = New-Object Text.UTF8Encoding($false)
        $ErrorActionPreference = 'Continue'
        $lines = & cargo @arguments
        $code = $LASTEXITCODE
    } finally {
        $ErrorActionPreference = $savedPreference
        [Console]::OutputEncoding = $savedEncoding
        Pop-Location
    }
    if ($code -ne 0) { throw 'Verified release material selection failed.' }
    $json = ([string]::Join("`n", [string[]]$lines)) + "`n"
    if ([Text.Encoding]::UTF8.GetByteCount($json) -gt 16777216) { throw 'Material manifest exceeds its size limit.' }
    return [pscustomobject]@{ Json=$json; Manifest=($json | ConvertFrom-Json) }
}

function Get-ReleaseMaterialSelection([string] $Root, [string] $Version) {
    $preview = -not (Test-StableReleaseVersion $Version)
    if ($preview -and (Test-EmptyMaterialSelection $Root)) { return $null }
    return Invoke-MaterialCli $Root $Version
}

function Get-ReleaseTextHash([string] $Text) {
    $sha = [Security.Cryptography.SHA256]::Create()
    try { return ([BitConverter]::ToString($sha.ComputeHash([Text.Encoding]::UTF8.GetBytes($Text)))).Replace('-', '').ToLowerInvariant() }
    finally { $sha.Dispose() }
}

function Test-ReleaseMaterialArchive([string] $ArchivePath, $Selection) {
    Assert-RegularReleasePath $ArchivePath
    if ((Get-Item -LiteralPath $ArchivePath).Length -gt 1073741824) { throw 'Material ZIP exceeds its size limit.' }
    $expected = New-Object 'Collections.Generic.Dictionary[string,object]' ([StringComparer]::Ordinal)
    $total = 0L
    foreach ($file in $Selection.Manifest.files) {
        if ($expected.ContainsKey($file.archive_path)) { throw 'Duplicate material path.' }
        if ($file.input.bytes -le 0 -or $file.input.bytes -gt 268435456) { throw 'Invalid material file size.' }
        $expected.Add($file.archive_path, [pscustomobject]@{Bytes=[long]$file.input.bytes; Hash=[string]$file.input.sha256})
        $total += [long]$file.input.bytes
    }
    if ($expected.Count -gt 16384 -or $total -gt 1073741824) { throw 'Material closure exceeds bounds.' }
    $expected.Add('RELEASE_MATERIALS.json', [pscustomobject]@{Bytes=[Text.Encoding]::UTF8.GetByteCount($Selection.Json); Hash=(Get-ReleaseTextHash $Selection.Json)})
    Add-Type -AssemblyName System.IO.Compression
    $stream = [IO.File]::OpenRead($ArchivePath)
    try {
        $zip = New-Object IO.Compression.ZipArchive($stream, [IO.Compression.ZipArchiveMode]::Read)
        try {
            if ($zip.Entries.Count -ne $expected.Count) { throw 'Material ZIP file set differs from verified selection.' }
            $seen = New-Object 'Collections.Generic.HashSet[string]' ([StringComparer]::Ordinal)
            foreach ($entry in $zip.Entries) {
                if (-not $expected.ContainsKey($entry.FullName) -or -not $seen.Add($entry.FullName)) { throw 'Material ZIP has extra, duplicate or traversal entries.' }
                if (($entry.ExternalAttributes -band 0xF0000000) -eq 0xA0000000 -or ($entry.ExternalAttributes -band 0x400)) { throw 'Material ZIP contains a link.' }
                $record = $expected[$entry.FullName]
                if ($entry.Length -ne $record.Bytes) { throw 'Material ZIP entry size differs.' }
                $input = $entry.Open()
                $sha = [Security.Cryptography.SHA256]::Create()
                try {
                    $buffer = New-Object byte[] 65536
                    $read = 0L
                    while (($count = $input.Read($buffer, 0, $buffer.Length)) -gt 0) {
                        $read += $count
                        if ($read -gt $record.Bytes) { throw 'Material ZIP entry exceeds its size limit.' }
                        $null = $sha.TransformBlock($buffer, 0, $count, $buffer, 0)
                    }
                    $null = $sha.TransformFinalBlock((New-Object byte[] 0), 0, 0)
                    $digest = ([BitConverter]::ToString($sha.Hash)).Replace('-', '').ToLowerInvariant()
                    if ($read -ne $record.Bytes -or $digest -cne $record.Hash) { throw 'Material ZIP entry hash differs.' }
                } finally { $sha.Dispose(); $input.Dispose() }
            }
        } finally { $zip.Dispose() }
    } finally { $stream.Dispose() }
}

function New-ReleaseMaterialArchive([string] $Stage, [string] $ArchivePath, $Selection, [long] $Epoch) {
        Assert-RegularReleasePath $Stage -Directory
        Assert-ReleaseDestination (Split-Path -Parent $ArchivePath)
        if ((Read-BoundedReleaseText (Join-Path $Stage 'RELEASE_MATERIALS.json')) -cne $Selection.Json) { throw 'Staged material manifest differs from verified selection.' }
        foreach ($file in $Selection.Manifest.files) {
            $path = Join-Path $Stage $file.archive_path
            Assert-RegularReleasePath $path
            if ((Get-Item -LiteralPath $path).Length -ne $file.input.bytes -or (Get-FileHash -LiteralPath $path -Algorithm SHA256).Hash.ToLowerInvariant() -cne $file.input.sha256) { throw 'Staged material changed before archive creation.' }
        }
        [string[]] $entries = @('RELEASE_MATERIALS.json') + @($Selection.Manifest.files | ForEach-Object archive_path)
        [Array]::Sort($entries, [StringComparer]::Ordinal)
        Add-Type -AssemblyName System.IO.Compression
        $stream = [IO.File]::Open($archivePath, [IO.FileMode]::CreateNew)
        try {
            $zip = New-Object IO.Compression.ZipArchive($stream, [IO.Compression.ZipArchiveMode]::Create)
            try {
                foreach ($name in $entries) {
                    $path = Join-Path $Stage $name
                    Assert-RegularReleasePath $path
                    $entry = $zip.CreateEntry($name, [IO.Compression.CompressionLevel]::Optimal)
                    $entry.LastWriteTime = [DateTimeOffset]::FromUnixTimeSeconds($Epoch)
                    $destination = $entry.Open()
                    $source = [IO.File]::OpenRead($path)
                    try { $source.CopyTo($destination) } finally { $source.Dispose(); $destination.Dispose() }
                }
            } finally { $zip.Dispose() }
        } finally { $stream.Dispose() }
        Test-ReleaseMaterialArchive $ArchivePath $Selection
}

function New-ReleaseMaterials([string] $Root, [string] $Version, [string] $OutputDirectory, $Selection, [long] $Epoch) {
    if (-not $Selection -or @($Selection.Manifest.files).Count -eq 0) { return @() }
    $stage = Join-Path ([IO.Path]::GetTempPath()) ('anav-release-materials-' + [Guid]::NewGuid().ToString('N'))
    try {
        $actual = Invoke-MaterialCli $Root $Version $stage
        if ($actual.Json -cne $Selection.Json) { throw 'Release material selection changed before staging.' }
        $archivePath = Join-Path $OutputDirectory "AzerothNavigator-materials-$Version.zip"
        $manifestPath = Join-Path $OutputDirectory 'RELEASE_MATERIALS.json'
        New-ReleaseMaterialArchive $stage $archivePath $Selection $Epoch
        [IO.File]::WriteAllText($manifestPath, $Selection.Json, (New-Object Text.UTF8Encoding($false)))
        return @($archivePath, $manifestPath)
    } finally {
        if (Test-Path -LiteralPath $stage) { Remove-Item -LiteralPath $stage -Recurse -Force }
    }
}

function Get-ExpectedArtifactNames([string] $Root, [string] $Version, [bool] $AddonOnly, $Selection) {
    [string[]] $names = @("AzerothNavigator-addon-$Version.zip", 'SOURCE_COMMIT.txt')
    if (-not $AddonOnly) { $names += "AzerothNavigator-$Version-x64-setup.exe" }
    if (Test-Path -LiteralPath (Join-Path $Root "docs/releases/$Version.md")) { $names += 'CANDIDATE_NOTES.md' }
    elseif (Test-StableReleaseVersion $Version) { throw 'Stable release notes are missing.' }
    if ($Selection -and @($Selection.Manifest.files).Count -gt 0) {
        $names += @("AzerothNavigator-materials-$Version.zip", 'RELEASE_MATERIALS.json')
    }
    [Array]::Sort($names, [StringComparer]::Ordinal)
    return $names
}

function Get-ReleaseArtifactManifest([string] $Root, [string] $Version, [string] $Directory, [bool] $AddonOnly, $Selection) {
    $files = foreach ($name in (Get-ExpectedArtifactNames $Root $Version $AddonOnly $Selection)) {
        $path = Join-Path $Directory $name
        Assert-RegularReleasePath $path
        [ordered]@{path=$name; bytes=(Get-Item -LiteralPath $path).Length; sha256=(Get-FileHash -LiteralPath $path -Algorithm SHA256).Hash.ToLowerInvariant()}
    }
    # All values are constrained ASCII names, versions, hashes or integers.
    # Compact ordered JSON has identical bytes in Windows PowerShell 5.1 and 7;
    # their indented ConvertTo-Json output does not.
    return (([ordered]@{schema_version=1; version=$Version; source_commit=$Selection.Manifest.candidate_source_commit; files=@($files)} | ConvertTo-Json -Depth 8 -Compress) + "`n")
}

function Write-ReleaseArtifactManifest([string] $Root, [string] $Version, [string] $Directory, [bool] $AddonOnly, $Selection) {
    if (-not $Selection -or @($Selection.Manifest.files).Count -eq 0) { return @() }
    $path = Join-Path $Directory 'RELEASE_ASSETS.json'
    [IO.File]::WriteAllText($path, (Get-ReleaseArtifactManifest $Root $Version $Directory $AddonOnly $Selection), (New-Object Text.UTF8Encoding($false)))
    return @($path)
}

# Candidate mode is opt-in and never a publication permission. Keep all CLI
# integrity/content checks; only the typed pending observations may be deferred.
function Get-AcceptanceCandidateMarker([string] $Root, [string] $Version) {
    $saved = $ErrorActionPreference
    Push-Location $Root
    try {
        $ErrorActionPreference = 'Continue'
        $lines = & cargo run --locked -p azeroth-cli -- candidate-readiness --root $Root --version $Version
        $code = $LASTEXITCODE
    } finally { $ErrorActionPreference = $saved; Pop-Location }
    if ($code -ne 0) { throw 'Acceptance candidate readiness failed.' }
    $report = ($lines -join "`n") | ConvertFrom-Json
    if (-not $report.candidate_ready -or $report.publication_allowed) { throw 'Invalid candidate readiness result.' }
    return (([ordered]@{schema_version=1; kind='acceptance_candidate'; publication_allowed=$false;
        version=$Version; source_commit=$report.candidate_source_commit;
        production_sha256=$report.production_sha256; inventory_sha256=$report.inventory_sha256} |
        ConvertTo-Json -Compress) + "`n")
}

function Assert-ReleaseArtifacts([string] $Root, [string] $Version, [string] $Directory, [bool] $AddonOnly, [bool] $AcceptanceCandidate = $false) {
    Assert-RegularReleasePath $Directory -Directory
    $marker = Join-Path $Directory 'ACCEPTANCE_CANDIDATE.json'
    if ($AcceptanceCandidate) {
        if ((Read-BoundedReleaseText $marker) -cne (Get-AcceptanceCandidateMarker $Root $Version)) {
            throw 'Acceptance candidate marker differs from verified source.'
        }
    } elseif (Test-Path -LiteralPath $marker) {
        throw 'Acceptance candidates cannot pass publication artifact verification.'
    } elseif (Test-StableReleaseVersion $Version) {
        $saved = $ErrorActionPreference
        Push-Location $Root
        try {
            $ErrorActionPreference = 'Continue'
            & cargo run --locked -p azeroth-cli -- release-readiness --root $Root --version $Version | Out-Host
            $code = $LASTEXITCODE
        } finally { $ErrorActionPreference = $saved; Pop-Location }
        if ($code -ne 0) { throw 'Stable publication readiness failed.' }
    }
    $selection = Get-ReleaseMaterialSelection $Root $Version
    [string[]] $names = @(Get-ExpectedArtifactNames $Root $Version $AddonOnly $selection)
    if ($selection -and @($selection.Manifest.files).Count -gt 0) {
        $public = Read-BoundedReleaseText (Join-Path $Directory 'RELEASE_MATERIALS.json')
        if ($public -cne $selection.Json) { throw 'Published material manifest differs from verified source selection.' }
        Test-ReleaseMaterialArchive (Join-Path $Directory "AzerothNavigator-materials-$Version.zip") $selection
        $actual = Read-BoundedReleaseText (Join-Path $Directory 'RELEASE_ASSETS.json')
        if ($actual -cne (Get-ReleaseArtifactManifest $Root $Version $Directory $AddonOnly $selection)) { throw 'Release artifact manifest differs from actual files.' }
        $names += 'RELEASE_ASSETS.json'
    }
    if ($AcceptanceCandidate) { $names += 'ACCEPTANCE_CANDIDATE.json' }
    $actualNames = @(Get-ChildItem -LiteralPath $Directory -Force | ForEach-Object Name)
    if ($actualNames.Count -ne $names.Count + 1 -or @($actualNames | Where-Object { $_ -cnotin ($names + 'SHA256SUMS.txt') }).Count -ne 0) { throw 'Release directory has missing or unexpected artifacts.' }
    $saved = $ErrorActionPreference
    $ErrorActionPreference = 'Continue'
    try { $head = & git -C $Root rev-parse HEAD; $code = $LASTEXITCODE } finally { $ErrorActionPreference = $saved }
    if ($code -ne 0 -or $head -notmatch '^[0-9a-f]{40}$') { throw 'Cannot identify release source.' }
    if ((Read-BoundedReleaseText (Join-Path $Directory 'SOURCE_COMMIT.txt')) -cne ($head + "`n")) { throw 'Artifact source commit differs from checkout.' }
    if ($names -contains 'CANDIDATE_NOTES.md') {
        if ((Get-FileHash (Join-Path $Directory 'CANDIDATE_NOTES.md')).Hash -cne (Get-FileHash (Join-Path $Root "docs/releases/$Version.md")).Hash) { throw 'Candidate notes differ from source.' }
    }
    [Array]::Sort($names, [StringComparer]::Ordinal)
    $lines = foreach ($name in $names) {
        $path = Join-Path $Directory $name
        Assert-RegularReleasePath $path
        ((Get-FileHash -LiteralPath $path -Algorithm SHA256).Hash.ToLowerInvariant()) + '  ' + $name
    }
    if ((Read-BoundedReleaseText (Join-Path $Directory 'SHA256SUMS.txt')) -cne (($lines -join "`n") + "`n")) { throw 'Release checksums differ from exact artifact set.' }
    Write-Host "Verified $($names.Count + 1) release files against source and material selection."
}
