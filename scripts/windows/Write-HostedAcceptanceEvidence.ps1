[CmdletBinding()]
param([Parameter(Mandatory)][string] $Root, [Parameter(Mandatory)][string] $Version, [Parameter(Mandatory)][string] $InstallerDirectory, [Parameter(Mandatory)][string] $ResultsDirectory, [Parameter(Mandatory)][string] $OutputDirectory)
Set-StrictMode -Version 3.0
$ErrorActionPreference = 'Stop'
if ($env:GITHUB_ACTIONS -ne 'true' -or $env:RUNNER_ENVIRONMENT -ne 'github-hosted' -or $env:OS -ne 'Windows_NT') { throw 'Hosted acceptance recording requires a disposable hosted Windows runner.' }
. (Join-Path $PSScriptRoot 'Release-Materials.ps1')
. (Join-Path $PSScriptRoot 'Native-Smoke-Helpers.ps1')
. (Join-Path $PSScriptRoot 'Hosted-Acceptance.ps1')
$Root = [IO.Path]::GetFullPath($Root)
Assert-StablePhaseReadiness $Root $Version 'prebuild'
$base = Read-BoundedReleaseText (Join-Path $Root 'release/v1/acceptance-evidence.json') | ConvertFrom-Json
$source = & git -C $Root rev-parse HEAD
if ($LASTEXITCODE -ne 0) { throw 'Cannot identify hosted source commit.' }
$setup = Get-NativeSmokeSetup $InstallerDirectory
Assert-RegularReleasePath $setup.FullName
$hash = (Get-FileHash -LiteralPath $setup.FullName -Algorithm SHA256).Hash.ToLowerInvariant()
$output = [IO.Path]::GetFullPath($OutputDirectory)
if (-not $output.StartsWith($Root + [IO.Path]::DirectorySeparatorChar, [StringComparison]::OrdinalIgnoreCase) -or (Test-Path -LiteralPath $output)) { throw 'Use a new evidence output directory inside the checkout.' }
New-Item -ItemType Directory -Path (Join-Path $output 'evidence') -Force | Out-Null
$cases = @{}
foreach ($id in $script:HostedAcceptanceIds) {
    $file = Join-Path $ResultsDirectory ($id.ToLowerInvariant() + '.json')
    if (-not (Test-Path -LiteralPath $file)) { continue }
    $text = Read-BoundedReleaseText $file
    $destination = Join-Path (Join-Path $output 'evidence') ($id.ToLowerInvariant() + '.json')
    [IO.File]::WriteAllText($destination, $text, (New-Object Text.UTF8Encoding($false)))
    $bytes = (Get-Item -LiteralPath $destination).Length
    $cases[$id] = @{ Result = ($text | ConvertFrom-Json); Artifact = [ordered]@{path=$destination.Substring($Root.Length+1).Replace('\','/');bytes=$bytes;sha256=(Get-FileHash -LiteralPath $destination -Algorithm SHA256).Hash.ToLowerInvariant()} }
}
$evidence = New-HostedAcceptanceEvidence $base $Version $source $base.production_sha256 $hash $cases
[IO.File]::WriteAllText((Join-Path $output 'acceptance-evidence.json'), (($evidence | ConvertTo-Json -Depth 100) + "`n"), (New-Object Text.UTF8Encoding($false)))
Write-Host "Recorded $($cases.Count) actual hosted case records; missing cases remain unchanged/unverified. Owner smoke is never created here."
