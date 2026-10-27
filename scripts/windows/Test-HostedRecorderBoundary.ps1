[CmdletBinding()]
param()
Set-StrictMode -Version 3.0
$ErrorActionPreference = 'Stop'
. (Join-Path $PSScriptRoot 'Hosted-Acceptance.ps1')
$count=0
function Check([string] $Name,[bool] $Condition){$script:count++;if(-not $Condition){throw "FAILED: $Name"};Write-Host "ok $Name"}
function Clone($Value){ConvertFrom-HostedJsonText ($Value|ConvertTo-Json -Depth 100)}
$fixture=Join-Path $PSScriptRoot 'fixtures/hosted-install-39413135905.json'
$case=ConvertFrom-HostedJsonText ([IO.File]::ReadAllText($fixture))
$rows=@($script:HostedAcceptanceIds|ForEach-Object{[pscustomobject]@{id=$_;method='hosted_windows';status='unverified';operator=$null;recorded_at=$null;environment=$null;artifacts=@()}})
$rows += [pscustomobject]@{id='OWNER-INSTALLER-SMOKE';method='interactive_windows';status='unverified';operator=$null;recorded_at=$null;environment=$null;artifacts=@()}
$base=[pscustomobject]@{schema_version=4;production_sha256=$case.production_sha256;target=$case.target;checks=$rows}
# Execute the real recorder I/O boundary in a temporary synthetic repository.
# Only the prebuild/filesystem contract adapter is stubbed; no release gate or
# installer is executed and this output is never current release evidence.
$temp=Join-Path ([IO.Path]::GetTempPath()) ('anav-timestamp-recorder-'+[Guid]::NewGuid().ToString('N'))
$savedEnvironment=@{};foreach($name in @('GITHUB_ACTIONS','RUNNER_ENVIRONMENT','OS')){$savedEnvironment[$name]=[Environment]::GetEnvironmentVariable($name,'Process')}
try {
    New-Item -ItemType Directory -Path (Join-Path $temp 'scripts/windows'),(Join-Path $temp 'release/v1'),(Join-Path $temp 'results'),(Join-Path $temp 'dist/windows') -Force | Out-Null
    foreach($name in @('Write-HostedAcceptanceEvidence.ps1','Hosted-Acceptance.ps1','Native-Smoke-Helpers.ps1')){Copy-Item -LiteralPath (Join-Path $PSScriptRoot $name) -Destination (Join-Path $temp ('scripts/windows/'+$name))}
    $stub=@'
function Assert-StablePhaseReadiness($Root,$Version,$Phase){if($Version -cne '1.0.0' -or $Phase -cne 'prebuild'){throw 'Unexpected synthetic gate inputs'}}
function Read-BoundedReleaseContractText($Path){return [IO.File]::ReadAllText($Path)}
function Read-BoundedReleaseText($Path){return [IO.File]::ReadAllText($Path)}
function Assert-RegularReleasePath($Path){if(-not (Test-Path -LiteralPath $Path -PathType Leaf)){throw 'Missing synthetic file'}}
function ConvertTo-BoundedReleaseContractText($Value){return ConvertTo-Json -InputObject $Value -Depth 100}
'@
    [IO.File]::WriteAllText((Join-Path $temp 'scripts/windows/Release-Materials.ps1'),$stub)
    [IO.File]::WriteAllText((Join-Path $temp 'tracked.txt'),'Synthetic recorder bookkeeping fixture')
    & git -C $temp init -q;if($LASTEXITCODE -ne 0){throw 'Synthetic Git init failed'}
    & git -C $temp config user.name 'Synthetic test'
    & git -C $temp config user.email 'synthetic@example.invalid'
    & git -C $temp add .
    & git -C $temp commit -qm 'synthetic recorder inputs';if($LASTEXITCODE -ne 0){throw 'Synthetic Git commit failed'}
    $head=& git -C $temp rev-parse HEAD
    $installer=Join-Path $temp 'dist/windows/AzerothNavigator-1.0.0-x64-setup.exe'
    [IO.File]::WriteAllText($installer,'Synthetic bytes, never executed')
    $input=Clone $case;$input.source_commit=$head;$input.installer_sha256=(Get-FileHash -LiteralPath $installer -Algorithm SHA256).Hash.ToLowerInvariant()
    [IO.File]::WriteAllText((Join-Path $temp 'results/hosted-install.json'),($input|ConvertTo-Json -Depth 100))
    [IO.File]::WriteAllText((Join-Path $temp 'release/v1/acceptance-evidence.json'),($base|ConvertTo-Json -Depth 100))
    $env:GITHUB_ACTIONS='true';$env:RUNNER_ENVIRONMENT='github-hosted';$env:OS='Windows_NT'
    & (Join-Path $temp 'scripts/windows/Write-HostedAcceptanceEvidence.ps1') -Root $temp -Version '1.0.0' -InstallerDirectory (Join-Path $temp 'dist/windows') -ResultsDirectory (Join-Path $temp 'results') -OutputDirectory (Join-Path $temp 'dist/acceptance-evidence')
    $written=ConvertFrom-HostedJsonText ([IO.File]::ReadAllText((Join-Path $temp 'dist/acceptance-evidence/acceptance-evidence.json')))
    $row=$written.checks[0];$pin=$row.artifacts[0];$artifactPath=Join-Path $temp $pin.path
    Check 'real recorder boundary preserves literal timestamp and portable artifact path' ($row.recorded_at -is [string] -and $row.recorded_at -ceq $case.recorded_at -and $pin.path -ceq 'dist/acceptance-evidence/evidence/hosted-install.json')
    Check 'real recorder artifact ref matches actual written byte length and SHA256' ($pin.bytes -eq (Get-Item -LiteralPath $artifactPath).Length -and $pin.sha256 -ceq (Get-FileHash -LiteralPath $artifactPath -Algorithm SHA256).Hash.ToLowerInvariant())
    Check 'real recorder does not invent absent observations or owner results' (@($written.checks|Where-Object status -eq 'unverified').Count -eq 8 -and $written.checks[8].status -ceq 'unverified')
} finally {
    foreach($name in $savedEnvironment.Keys){[Environment]::SetEnvironmentVariable($name,$savedEnvironment[$name],'Process')}
    Remove-Item -LiteralPath $temp -Recurse -Force -ErrorAction SilentlyContinue
}
Write-Host "$count recorder boundary checks passed (synthetic I/O; not release acceptance)"
