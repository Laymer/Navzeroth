# Actual schema4 acceptance producers. Only the disposable hosted runner is touched.
[CmdletBinding()]
param(
    [Parameter(Mandatory)][string] $Root,
    [Parameter(Mandatory)][string] $Directory,
    [Parameter(Mandatory)][ValidatePattern('^[a-f0-9]{64}$')][string] $InstallerSha256,
    [Parameter(Mandatory)][string] $BaselineInstaller,
    [Parameter(Mandatory)][ValidatePattern('^[a-f0-9]{64}$')][string] $BaselineSha256,
    [Parameter(Mandatory)][string] $BuildIdentity,
    [Parameter(Mandatory)][string] $OutputDirectory,
    [string] $Version = '1.0.0'
)
Set-StrictMode -Version 3.0
$ErrorActionPreference = 'Stop'
if ($PSVersionTable.PSVersion.Major -lt 7 -or $env:GITHUB_ACTIONS -ne 'true' -or $env:RUNNER_ENVIRONMENT -ne 'github-hosted' -or $env:OS -ne 'Windows_NT') { throw 'Stable acceptance requires PowerShell 7 on a disposable hosted Windows runner.' }
. (Join-Path $PSScriptRoot 'Release-Materials.ps1')
. (Join-Path $PSScriptRoot 'Native-Smoke-Helpers.ps1')
. (Join-Path $PSScriptRoot 'Hosted-Acceptance.ps1')
. (Join-Path $PSScriptRoot 'Stable-Acceptance-Helpers.ps1')
$Root = [IO.Path]::GetFullPath($Root); $OutputDirectory = [IO.Path]::GetFullPath($OutputDirectory)
if (Test-Path -LiteralPath $OutputDirectory) { throw 'Use a new hosted acceptance output directory.' }
$setup = Get-NativeSmokeSetup $Directory
foreach ($path in @($setup.FullName,$BaselineInstaller,$BuildIdentity)) { Assert-RegularReleasePath $path }
if ((Get-FileHash $setup.FullName -Algorithm SHA256).Hash.ToLowerInvariant() -cne $InstallerSha256 -or (Get-FileHash $BaselineInstaller -Algorithm SHA256).Hash.ToLowerInvariant() -cne $BaselineSha256) { throw 'Installer bytes differ from independently selected expected hashes.' }
if ($setup.VersionInfo.ProductVersion -cne $Version -or (Get-Item $BaselineInstaller).VersionInfo.ProductVersion -cne '1.0.0-rc.1') { throw 'Candidate/baseline versions differ from the explicit acceptance contract.' }
foreach ($path in @($setup.FullName,$BaselineInstaller)) { if ((Get-AuthenticodeSignature $path).Status.ToString() -cne 'NotSigned') { throw 'Expected explicitly unsigned candidate and historical RC installers.' } }
$policyPath='HKLM:\SOFTWARE\Policies\Microsoft\Edge\WebView2\AdditionalBrowserArguments'
if ($null -ne (Get-ItemProperty -LiteralPath $policyPath -Name 'azeroth-desktop.exe' -ErrorAction SilentlyContinue)) { throw 'Existing native debugging policy value must be preserved.' }
$published=Read-BoundedReleaseText (Join-Path $Root 'docs/releases/1.0.0-rc.1-publication.json') | ConvertFrom-Json
$baselinePin=@($published.assets | Where-Object name -CEQ 'AzerothNavigator-1.0.0-rc.1-x64-setup.exe')
if ($published.tag -cne 'v1.0.0-rc.1' -or $published.public_verified -ne $true -or $baselinePin.Count -ne 1 -or $baselinePin[0].sha256 -cne $BaselineSha256 -or (Get-Item $BaselineInstaller).Length -ne $baselinePin[0].bytes) { throw 'Baseline differs from the independently retained public RC receipt.' }
Assert-ReleaseArtifacts $Root $Version ([IO.Path]::GetFullPath($Directory)) $false $false 'prebuild'
$identityDocument = Read-BoundedBundleIdentity $BuildIdentity | ConvertFrom-Json
$source = & git -C $Root rev-parse HEAD
if ($LASTEXITCODE -ne 0 -or $source -cnotmatch '^[a-f0-9]{40}$' -or $identityDocument.app_version -cne $Version) { throw 'Hosted source/build identity is unavailable.' }
$release = Read-BoundedReleaseContractText (Join-Path $Root 'release/v1/acceptance-evidence.json') | ConvertFrom-Json
if ($release.schema_version -ne 4 -or $release.production_sha256 -cne $identityDocument.production_sha256) { throw 'Build identity differs from source-bound release production/target.' }
Assert-StableTargetIdentity $release.target $identityDocument.target
$identity = @{version=$Version;source_commit=$source;production_sha256=$release.production_sha256;installer_sha256=$InstallerSha256;target=$release.target;operator=('GitHub Actions '+$env:GITHUB_RUN_ID+'/'+$env:GITHUB_RUN_ATTEMPT);environment=('GitHub-hosted Windows '+[Environment]::OSVersion.VersionString)}
$install = Join-Path $env:LOCALAPPDATA 'Azeroth Navigator'
if (Test-Path -LiteralPath $install) { throw 'Acceptance requires an initially absent installation.' }
if (@(Get-Process -Name azeroth-desktop -ErrorAction SilentlyContinue).Count -ne 0) { throw 'Acceptance requires no other native application process.' }
New-Item -ItemType Directory -Path $OutputDirectory -Force | Out-Null
$reports = Join-Path $OutputDirectory 'raw'; New-Item -ItemType Directory -Path $reports | Out-Null
$previousAppData = $env:APPDATA
$env:APPDATA = Join-Path $OutputDirectory 'isolated-appdata'
New-Item -ItemType Directory -Path $env:APPDATA | Out-Null
$exe = Join-Path $install 'azeroth-desktop.exe'; $manifest = Join-Path $install 'bundled-content/manifest.json'
$state = Join-Path $reports 'state.json'; $suite = [Diagnostics.Stopwatch]::StartNew()
function Remaining([int] $Maximum) {
    $seconds=[Math]::Floor(3600-$suite.Elapsed.TotalSeconds)
    if ($seconds -lt 1) { throw 'Hosted acceptance suite exceeded its 60 minute bound.' }
    return [int][Math]::Min($Maximum,$seconds)
}
function Install-Owned([string] $Path) { Install-NativeSmokeSetup $Path (Remaining 1200) }
function Workload([string] $Mode,[string] $AppVersion = $Version) {
    $report = Join-Path $reports ($Mode+'-'+[Guid]::NewGuid().ToString('N')+'.json')
    return Invoke-StableWorkload $Root $exe $manifest $AppVersion $Mode $state $report (Join-Path $reports ('backup-'+[Guid]::NewGuid().ToString('N')+'.sqlite3')) (Remaining 1200)
}
function Verify-OwnedResources {
        Assert-VerifiedBundleInputs $Root $Version (Join-Path $install 'bundled-content') $BuildIdentity
        $inventory = & node (Join-Path $PSScriptRoot 'verify-installed-tree.mjs') $Root $install
        if ($LASTEXITCODE -ne 0) { throw 'Exact installed resource inventory differs from source.' }
        & node (Join-Path $Root 'scripts/dependency-notices.mjs') --installed (Join-Path $install 'local-data-notices') | Out-Null
        if ($LASTEXITCODE -ne 0) { throw 'Installed dependency notices/source differ.' }
        return @{installed_resources_verified=$true;source_notices_verified=$true;installed_files=@($inventory | ConvertFrom-Json)}
}
function Remove-OwnedInstallation {
    Stop-NativeSmokeProcesses
    $uninstaller = Join-Path $install 'uninstall.exe'
    if (-not (Test-Path -LiteralPath $uninstaller)) { return }
    $result = Invoke-BoundedProcess $uninstaller @('/S') 300
    if ($result.TimedOut -or $result.ExitCode -ne 0) { throw 'Owned uninstaller did not complete within bounds.' }
    $timer=[Diagnostics.Stopwatch]::StartNew()
    while ((Test-Path -LiteralPath $exe) -and $timer.Elapsed.TotalSeconds -lt 120) { Start-Sleep -Milliseconds 250 }
    if (Test-Path -LiteralPath $exe) { throw 'Owned installation was not removed.' }
}
try {
    $script:installMs = $null; $script:fresh = $null
    Invoke-StableCase 'HOSTED-INSTALL' $identity $OutputDirectory {
        $timer=[Diagnostics.Stopwatch]::StartNew(); Install-Owned $setup.FullName; $script:installMs=$timer.Elapsed.TotalMilliseconds
        if ((Get-InstalledDisplayVersion) -cne $Version) { throw 'Installed registry version differs.' }
        Assert-RegularReleasePath $exe
        if ((Get-Item $exe).VersionInfo.ProductVersion -cne $Version -or (Get-AuthenticodeSignature $exe).Status.ToString() -cne 'NotSigned') { throw 'Installed native app version/signature differs.' }
        return @{install_ms=$script:installMs;installed_version=$Version}
    }
    Invoke-StableCase 'HOSTED-RESOURCES' $identity $OutputDirectory {
        return Verify-OwnedResources
    }
    Invoke-StableCase 'HOSTED-FIRST-START' $identity $OutputDirectory {
        $script:fresh=Workload 'fresh'
        return @{raw_report=$script:fresh.raw}
    }
    Invoke-StableCase 'HOSTED-PERFORMANCE' $identity $OutputDirectory {
        if ($null -eq $script:fresh -or $null -eq $script:installMs -or $script:fresh.peak_memory_bytes -le 0) { throw 'Actual first-load measurements/peak memory are unavailable.' }
        $machine=Get-CimInstance Win32_ComputerSystem; $cpu=@(Get-CimInstance Win32_Processor)[0]
        return @{workload=$script:fresh.raw.workload;hardware=($machine.Manufacturer+' '+$machine.Model+'; '+$cpu.Name+'; RAM bytes '+$machine.TotalPhysicalMemory);limitations=('Disposable hosted runner; fresh database after resource verification; OS file cache uncontrolled/possibly warm. '+$script:fresh.memory_scope+' load_ms covers content verify/parse/stage/compose; activation_ms covers defaults only, excluding database open/WebView2. Manual estimates are not gameplay facts or ordinary-PC performance.');measurements=(New-StablePerformanceMeasurements $script:installMs $script:fresh);raw_report=$script:fresh.raw;memory_observation=$script:fresh.memory_observation}
    }
    Invoke-StableCase 'HOSTED-PERSISTENCE-RECOVERY' $identity $OutputDirectory {
        $restart=Workload 'restart'; $recovery=Workload 'recover'
        return @{restart_verified=$restart.raw.restart_verified;recovery_verified=$recovery.raw.recovery_verified;restart=$restart.raw;recovery=$recovery.raw}
    }
    Invoke-StableCase 'HOSTED-OFFLINE' $identity $OutputDirectory {
        $offline=Invoke-StableOffline { Workload 'offline' }
        return @{network_disabled=$offline.network_disabled;network_restored=$offline.restored;controlled_adapters=$offline.adapters;raw_report=$offline.workload.raw}
    }
    Invoke-StableCase 'HOSTED-UPGRADE-RC1' $identity $OutputDirectory {
        # Baseline needs the same explicit manual scope, supplied by the verified
        # candidate manifest outside the installation while the RC is installed.
        $savedManifest=Join-Path $reports 'candidate-manifest.json'; Copy-Item -LiteralPath $manifest -Destination $savedManifest
        Remove-OwnedInstallation
        $env:APPDATA=Join-Path $OutputDirectory 'upgrade-appdata';New-Item -ItemType Directory -Path $env:APPDATA | Out-Null
        Install-Owned $BaselineInstaller
        $previousManifest=$manifest; $manifest=$savedManifest
        try { $baseline=Workload 'baseline' '1.0.0-rc.1' } finally { $manifest=$previousManifest }
        Install-Owned $setup.FullName
        if ((Get-InstalledDisplayVersion) -cne $Version) { throw 'Candidate did not upgrade the RC installation.' }
        $resources=Verify-OwnedResources
        $upgraded=Workload 'restart'
        return @{baseline_version='1.0.0-rc.1';baseline_installer_sha256=$BaselineSha256;baseline=$baseline.raw;upgraded=$upgraded.raw;installed_resources_verified=$resources.installed_resources_verified;source_notices_verified=$resources.source_notices_verified;installed_files=$resources.installed_files}
    }
    Invoke-StableCase 'HOSTED-DOWNGRADE-REFUSAL' $identity $OutputDirectory {
        $before=(Get-FileHash $exe -Algorithm SHA256).Hash
        $attempt=Invoke-BoundedProcess $BaselineInstaller @('/S') (Remaining 300)
        if ($attempt.TimedOut -or (Get-InstalledDisplayVersion) -cne $Version -or (Get-FileHash $exe -Algorithm SHA256).Hash -cne $before) { throw 'Older installer timed out or replaced the newer installation.' }
        $after=Workload 'restart'
        return @{baseline_version='1.0.0-rc.1';baseline_installer_sha256=$BaselineSha256;attempted=$true;timed_out=$false;installed_version_after=$Version;exit_code=$attempt.ExitCode;raw_report=$after.raw}
    }
} finally {
    try { Remove-OwnedInstallation } finally {
        $env:APPDATA=$previousAppData
        Remove-ItemProperty -LiteralPath $policyPath -Name 'azeroth-desktop.exe' -ErrorAction SilentlyContinue
    }
}
