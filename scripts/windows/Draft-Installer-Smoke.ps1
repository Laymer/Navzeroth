# Exact downloaded installer UI evidence. Dot-source only; never records a final gate.
Set-StrictMode -Version 3.0
function Get-DraftReleaseApi([string] $Repository,[string] $Tag,$Release) {
    # REST's by-tag endpoint only exposes published releases. The release view
    # supplies the issued database ID even while the release remains a draft.
    if ($Repository -cnotmatch '^[A-Za-z0-9][A-Za-z0-9_.-]*/[A-Za-z0-9][A-Za-z0-9_.-]*$' -or [string]::IsNullOrWhiteSpace($Tag)) { throw 'Draft release repository or tag is invalid.' }
    $releaseId=$Release.databaseId
    if (($releaseId -isnot [int] -and $releaseId -isnot [long]) -or $releaseId -lt 1 -or $Release.tagName -cne $Tag -or $Release.isDraft -isnot [bool] -or -not $Release.isDraft -or $Release.isPrerelease -isnot [bool] -or $Release.isPrerelease) { throw 'Release view does not identify the expected stable draft.' }
    $json=gh api "repos/$Repository/releases/$releaseId"
    if ($LASTEXITCODE -ne 0) { throw 'Cannot obtain draft release API identity by ID.' }
    $api=$json | ConvertFrom-Json
    if (($api.id -isnot [int] -and $api.id -isnot [long]) -or $api.id -ne $releaseId -or $api.tag_name -cne $Tag -or $api.draft -isnot [bool] -or -not $api.draft -or $api.prerelease -isnot [bool] -or $api.prerelease) { throw 'Release API identity or draft state differs from the release view.' }
    return $api
}
function Get-DraftEvidenceRef([string] $Root,[string] $Path,[long] $MaximumBytes = 8388608) {
    Assert-RegularReleasePath $Path
    $rootPath=[IO.Path]::GetFullPath($Root).TrimEnd([IO.Path]::DirectorySeparatorChar,[IO.Path]::AltDirectorySeparatorChar)+[IO.Path]::DirectorySeparatorChar
    $full=[IO.Path]::GetFullPath($Path)
    if (-not $full.StartsWith($rootPath,[StringComparison]::OrdinalIgnoreCase)) { throw 'Draft evidence must be retained under its source checkout.' }
    $bytes=(Get-Item -LiteralPath $full).Length
    if ($bytes -lt 1 -or $bytes -gt $MaximumBytes) { throw 'Draft evidence exceeds its finite size limit.' }
    return [ordered]@{path=$full.Substring($rootPath.Length).Replace('\','/');bytes=$bytes;sha256=(Get-FileHash -LiteralPath $full -Algorithm SHA256).Hash.ToLowerInvariant()}
}
function Assert-DraftUiChecks($Fresh,$Restart) {
    foreach ($item in @($Fresh,$Restart)) {
        if ($item.passed -isnot [bool] -or -not $item.passed -or $item.completed -isnot [bool] -or -not $item.completed -or $item.ui_interaction_verified -isnot [bool] -or -not $item.ui_interaction_verified -or $item.human_performed -isnot [bool] -or $item.human_performed) { throw 'Draft native UI work is incomplete.' }
    }
    if ($Fresh.mode -cne 'fresh' -or $Restart.mode -cne 'restart') { throw 'Draft UI result modes differ.' }
    foreach ($name in @('first_start','bundled_activation','source_backed_planning')) {
        if ($Fresh.checks.$name -isnot [bool] -or -not $Fresh.checks.$name) { throw 'Draft UI fresh-install check did not pass.' }
    }
    if ($Restart.checks.restart_persistence -isnot [bool] -or -not $Restart.checks.restart_persistence) { throw 'Draft UI restart persistence did not pass.' }
}
function New-DraftDownloadResult([string] $Root,[string] $ReceiptDirectory,$Release,$Job,[string] $SourceCommit,[string] $InstallerSha256,[long] $RunId,[long] $RunAttempt) {
    if ($SourceCommit -cnotmatch '^[a-f0-9]{40}$' -or $InstallerSha256 -cnotmatch '^[a-f0-9]{64}$' -or $RunId -lt 1 -or $RunAttempt -lt 1) { throw 'Draft download requires exact independently selected identities.' }
    $downloadPath=Join-Path $ReceiptDirectory 'verification.json'
    $nativePath=Join-Path $ReceiptDirectory 'native-result.json'
    $bytesRead=0L
    $download=Read-ReleaseTextWithCap $downloadPath 1048576 ([ref]$bytesRead) | ConvertFrom-Json
    $native=Read-ReleaseTextWithCap $nativePath 1048576 ([ref]$bytesRead) | ConvertFrom-Json
    $build=Read-BoundedBundleIdentity (Join-Path $Root 'release/v1/bundled-content-build-identity.json') | ConvertFrom-Json
    if ($build.production_sha256 -cnotmatch '^[a-f0-9]{64}$' -or $build.app_version -cne $download.version -or $download.schema_version -ne 3 -or $download.phase -cne 'draft' -or $download.source_commit -cne $SourceCommit -or $download.installer_signature -cne 'NotSigned' -or $download.application_signature -cne 'NotSigned' -or $download.workflow_run_id -isnot [string] -or $download.workflow_run_id -cne [string]$RunId -or $download.workflow_run_attempt -isnot [string] -or $download.workflow_run_attempt -cne [string]$RunAttempt) { throw 'Downloaded draft verification identity differs.' }
    foreach ($name in @('dependency_closure_verified','addon_zip_matches_source')) { if ($download.$name -isnot [bool] -or -not $download.$name) { throw 'Downloaded resources were not verified.' } }
    if ($native.schema_version -ne 1 -or $native.phase -cne 'draft' -or $native.source_commit -cne $SourceCommit -or $native.installer_sha256 -cne $InstallerSha256 -or $native.human_performed -isnot [bool] -or $native.human_performed) { throw 'Native draft identity differs.' }
    foreach ($name in @('completed','passed','ui_interaction_verified')) { if ($native.$name -isnot [bool] -or -not $native.$name) { throw 'Native draft UI work did not complete.' } }
    foreach ($name in @('first_start','bundled_activation','source_backed_planning','restart_persistence')) { if ($native.checks.$name -isnot [bool] -or -not $native.checks.$name) { throw 'Native draft UI check did not pass.' } }
    $assets=@($download.assets | Where-Object { $_.name.EndsWith('.exe',[StringComparison]::Ordinal) })
    if ($assets.Count -ne 1 -or $assets[0].sha256 -cne $InstallerSha256 -or $assets[0].bytes -lt 1) { throw 'Downloaded installer identity is absent or ambiguous.' }
    $asset=$assets[0]
    $released=@($Release.assets | Where-Object { $_.name -ceq $asset.name })
    if ($Release.id -lt 1 -or $Release.tag_name -cne ('v'+$download.version) -or $Release.draft -isnot [bool] -or -not $Release.draft -or $Release.prerelease -isnot [bool] -or $Release.prerelease -or $released.Count -ne 1 -or $released[0].id -lt 1 -or $released[0].size -ne $asset.bytes) { throw 'Release API draft installer identity differs.' }
    if ($Job.id -lt 1 -or $Job.run_id -ne $RunId -or $Job.run_attempt -ne $RunAttempt -or $Job.head_sha -cne $SourceCommit -or $Job.name -cne 'Verify downloaded unsigned release') { throw 'Current workflow job identity differs.' }
    # This running job cannot attest its own eventual success. The coordinator
    # later retrieves completed REST snapshots and an independent review.
    $identity=[ordered]@{source_commit=$SourceCommit;production_sha256=$build.production_sha256;installer_sha256=$InstallerSha256;release_tag=$Release.tag_name;release_id=$Release.id;asset_id=$released[0].id;asset_name=$asset.name;asset_bytes=$asset.bytes;run_id=$RunId;run_attempt=$RunAttempt;job_id=$Job.id;workflow_sha256=(Get-FileHash -LiteralPath (Join-Path $Root '.github/workflows/verify-release-downloads.yml') -Algorithm SHA256).Hash.ToLowerInvariant()}
    return [ordered]@{schema_version=1;identity=$identity;phase='draft';human_performed=$false;downloaded=$true;unsigned_verified=$true;installation_verified=$true;first_start_verified=$true;bundled_resources_verified=$true;planning_verified=$true;restart_persistence_verified=$true;ui_interaction_verified=$true;download_receipt=(Get-DraftEvidenceRef $Root $downloadPath 1048576);native_result=(Get-DraftEvidenceRef $Root $nativePath 1048576)}
}
function Invoke-DownloadedDraftUiSmoke([string] $Root,[string] $Exe,[string] $Manifest,[string] $Version,[string] $InstallerSha256,[string] $OutputDirectory) {
    if ($env:GITHUB_ACTIONS -cne 'true' -or $env:RUNNER_ENVIRONMENT -cne 'github-hosted' -or $env:OS -cne 'Windows_NT') { throw 'Draft UI acceptance requires a disposable hosted Windows runner.' }
    if ($PSVersionTable.PSVersion.Major -lt 7 -or $InstallerSha256 -cnotmatch '^[a-f0-9]{64}$') { throw 'Draft UI acceptance requires PowerShell7 and an exact installer hash.' }
    $source=(& git -C $Root rev-parse HEAD).Trim()
    if ($LASTEXITCODE -ne 0 -or $source -cnotmatch '^[a-f0-9]{40}$') { throw 'Draft UI source identity is unavailable.' }
    if (Test-Path -LiteralPath $OutputDirectory) { throw 'Draft UI evidence directory must be new.' }
    $rootPrefix=[IO.Path]::GetFullPath($Root).TrimEnd([IO.Path]::DirectorySeparatorChar,[IO.Path]::AltDirectorySeparatorChar)+[IO.Path]::DirectorySeparatorChar
    if (-not ([IO.Path]::GetFullPath($OutputDirectory)).StartsWith($rootPrefix,[StringComparison]::OrdinalIgnoreCase)) { throw 'Draft evidence must be retained under its source checkout.' }
    Assert-ReleaseDestination $OutputDirectory
    $null=New-Item -ItemType Directory -Path $OutputDirectory -ErrorAction Stop
    $state=Join-Path $OutputDirectory 'state.json'
    $clock=[Diagnostics.Stopwatch]::StartNew()
    $results=@{}
    foreach ($mode in @('fresh','restart')) {
        $remaining=[Math]::Floor(3600-$clock.Elapsed.TotalSeconds)
        if ($remaining -lt 1) { throw 'Downloaded draft UI suite exceeded its 60 minute bound.' }
        $report=Join-Path $OutputDirectory ($mode+'.json')
        $results[$mode]=Invoke-StableWorkload -Root $Root -Exe $Exe -Manifest $Manifest -Version $Version -Mode $mode -State $state -Report $report -MaximumSeconds ([int][Math]::Min(1200,$remaining)) -Driver 'draft-ui' -SourceCommit $source -InstallerSha256 $InstallerSha256
    }
    Assert-DraftUiChecks $results.fresh.raw $results.restart.raw
    # Retain only bounded actual diagnostic files, not the isolated profile DB.
    $details=@(Get-ChildItem -LiteralPath $OutputDirectory -File -Recurse | Where-Object { $_.FullName -cne $state } | Sort-Object FullName)
    if ($details.Count -lt 2 -or $details.Count -gt 16) { throw 'Draft native detail inventory exceeds its finite bounds.' }
    $pins=@($details | ForEach-Object { Get-DraftEvidenceRef $Root $_.FullName })
    return [ordered]@{schema_version=1;phase='draft';source_commit=$source;installer_sha256=$InstallerSha256;completed=$true;passed=$true;human_performed=$false;ui_interaction_verified=$true;checks=[ordered]@{first_start=$true;bundled_activation=$true;source_backed_planning=$true;restart_persistence=$true};detail_files=$pins;elapsed_ms=$clock.Elapsed.TotalMilliseconds;limitations='Actual downloaded installed application on disposable hosted Windows. Source-backed engineering scenario with manual location/time/utility; no human, live-client or ordinary-PC acceptance.'}
}
