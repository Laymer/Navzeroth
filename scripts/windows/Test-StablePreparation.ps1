# Synthetic contracts only: no build, network, installer execution or release.
Set-StrictMode -Version 3.0
$ErrorActionPreference = 'Stop'
. (Join-Path $PSScriptRoot 'Release-Materials.ps1')
. (Join-Path $PSScriptRoot 'Hosted-Acceptance.ps1')
$count = 0
function Check([string]$Name, [scriptblock]$Action) { & $Action; $script:count++; Write-Host "ok $Name" }
function Refuse([scriptblock]$Action) { $failed=$false; try { & $Action | Out-Null } catch { $failed=$true }; if(-not $failed){throw 'Expected refusal.'} }
function Clone($Value) { return ($Value | ConvertTo-Json -Depth 100 | ConvertFrom-Json) }
function cargo { $script:command = $args[5]; $global:LASTEXITCODE=$script:exitCode; return ($script:report | ConvertTo-Json -Depth 100 -Compress) }
$root=Join-Path ([IO.Path]::GetTempPath()) ('anav-preparation-'+[Guid]::NewGuid().ToString('N'))
$sha='a'*64; $source='b'*40; $installer='c'*64
$target=[pscustomobject]@{edition='retail';build='70245'}
$base=[pscustomobject]@{schema_version=4;production_sha256=$sha;target=$target;checks=@()}
foreach($id in ($script:HostedAcceptanceIds+@('OWNER-INSTALLER-SMOKE'))){
    $method=if($id -eq 'OWNER-INSTALLER-SMOKE'){'interactive_windows'}else{'hosted_windows'}
    $base.checks+= [pscustomobject]@{id=$id;method=$method;status='unverified';operator=$null;recorded_at=$null;environment=$null;artifacts=@()}
}
function Case([string]$Id) {
    $case=[pscustomobject]@{schema_version=1;id=$Id;method='hosted_windows';status='pass';version='1.0.0';source_commit=$source;production_sha256=$sha;installer_sha256=$installer;target=$target;operator='hosted runner';environment='Synthetic test environment';recorded_at='2026-10-19T12:00:00Z';completed=$true;baseline_version='1.0.0-rc.1';baseline_installer_sha256=('d'*64);attempted=$true;timed_out=$false;installed_version_after='1.0.0';restart_verified=$true;recovery_verified=$true;installed_resources_verified=$true;source_notices_verified=$true;network_disabled=$true;workload='full pack load activate browse plan';hardware='Synthetic test hardware';limitations='Synthetic metrics only';measurements=[pscustomobject]@{install_ms=0;load_ms=1;activation_ms=2;browse_ms=3;planning_ms=4;peak_memory_bytes=5}}
    return @{Result=$case;Artifact=[pscustomobject]@{path='dist/evidence/'+$Id+'.json';bytes=123;sha256=$sha}}
}
try {
    New-Item -ItemType Directory -Path (Join-Path $root 'release/v1') -Force | Out-Null
    $basePath=Join-Path $root 'release/v1/acceptance-evidence.json'
    [IO.File]::WriteAllText($basePath, ($base|ConvertTo-Json -Depth 100))
    $inventoryPath=Join-Path $root 'release/v1/coverage-inventory.json'
    [IO.File]::WriteAllText($inventoryPath, '{"schema_version":4}')
    $script:exitCode=0
    $script:report=@{stable=$true;stable_ready=$true;publication_allowed=$true}
    Check 'public default uses full release gate' { Assert-StablePhaseReadiness $root '1.0.0'; if($script:command -ne 'release-readiness'){throw 'Wrong CLI command.'} }
    foreach($phase in @('prebuild','draft')){
        $script:report=@{phase=$phase;build_ready=$true;publication_allowed=$false;report=@{report_version=4;stable=$true;publication_allowed=$false}}
        Check "$phase explicitly cannot publish" { Assert-StablePhaseReadiness $root '1.0.0' $phase; $expected=if($phase -eq 'prebuild'){'release-build-readiness'}else{'release-draft-readiness'}; if($script:command -ne $expected){throw 'Wrong phase command.'} }
        Check "$phase publication promotion refused" { $script:report.publication_allowed=$true; Refuse { Assert-StablePhaseReadiness $root '1.0.0' $phase }; $script:report.publication_allowed=$false }
        Check "$phase incomplete readiness refused" { $script:report.build_ready=$false; Refuse { Assert-StablePhaseReadiness $root '1.0.0' $phase }; $script:report.build_ready=$true }
        Check "$phase nested publication permission refused" { $script:report.report.publication_allowed=$true; Refuse { Assert-StablePhaseReadiness $root '1.0.0' $phase }; $script:report.report.publication_allowed=$false }
    }
    Check 'failed CLI exit refused' { $script:exitCode=9; Refuse { Assert-StablePhaseReadiness $root '1.0.0' }; $script:exitCode=0 }
    Check 'public cannot accept preparation report' { Refuse { Assert-StablePhaseReadiness $root '1.0.0' } }
    Check 'historical prerelease bypass unchanged' { $script:exitCode=9; Assert-StablePhaseReadiness $root '1.0.0-rc.1'; $script:exitCode=0 }
    $overlay=Clone $base; $overlay|Add-Member candidate_installer_sha256 $installer
    $overlayPath=Join-Path $root 'observed.json'
    [IO.File]::WriteAllText($overlayPath, ($overlay|ConvertTo-Json -Depth 100))
    Check 'observation only overlay allowed' { Assert-StableEvidenceOverlay $root 'observed.json' }
    Check 'overlay cannot change production' { $overlay.production_sha256='f'*64; [IO.File]::WriteAllText($overlayPath, ($overlay|ConvertTo-Json -Depth 100)); Refuse { Assert-StableEvidenceOverlay $root 'observed.json' }; $overlay.production_sha256=$sha }
    Check 'overlay traversal refused' { Refuse { Assert-StableEvidenceOverlay $root '../observed.json' } }
    [IO.File]::WriteAllText((Join-Path $root 'AzerothNavigator-1.0.0-x64-setup.exe'), 'synthetic installer bytes')
    $overlay.candidate_installer_sha256=(Get-FileHash (Join-Path $root 'AzerothNavigator-1.0.0-x64-setup.exe')).Hash.ToLowerInvariant()
    [IO.File]::WriteAllText($overlayPath, ($overlay|ConvertTo-Json -Depth 100))
    Check 'draft binds actual installer bytes' { Assert-StableInstallerBinding $root '1.0.0' $root 'observed.json' }
    Check 'replacement installer refused' { [IO.File]::AppendAllText((Join-Path $root 'AzerothNavigator-1.0.0-x64-setup.exe'),'altered'); Refuse { Assert-StableInstallerBinding $root '1.0.0' $root 'observed.json' } }
    Check 'missing hosted observations remain unverified' { $result=New-HostedAcceptanceEvidence $base '1.0.0' $source $sha $installer; if(@($result.checks|Where-Object status -ne 'unverified').Count -ne 0){throw 'Fabricated result.'} }
    $cases=@{}; foreach($id in $script:HostedAcceptanceIds){$cases[$id]=Case $id}
    Check 'all actual hosted cases recorded without owner promotion' { $result=New-HostedAcceptanceEvidence $base '1.0.0' $source $sha $installer $cases; if(@($result.checks|Where-Object status -eq 'pass').Count -ne 8 -or $result.checks[-1].status -ne 'unverified'){throw 'Incorrect case recording.'} }
    foreach($property in @('source_commit','production_sha256','installer_sha256','method')){
        Check "wrong $property refused" { $entry=Case 'HOSTED-INSTALL'; $entry.Result.$property='wrong'; Refuse { New-HostedAcceptanceEvidence $base '1.0.0' $source $sha $installer @{'HOSTED-INSTALL'=$entry} } }
    }
    Check 'owner cannot be recorded by hosted helper' { Refuse { New-HostedAcceptanceEvidence $base '1.0.0' $source $sha $installer @{'OWNER-INSTALLER-SMOKE'=(Case 'OWNER-INSTALLER-SMOKE')} } }
    $automated=Clone $base; $automated.schema_version=5
    $automated.checks[-1].id='DRAFT-INSTALLER-SMOKE'; $automated.checks[-1].method='automated_draft_windows'
    Check 'schema5 hosted recording preserves the separate draft gate' {
        $result=New-HostedAcceptanceEvidence $automated '1.0.0' $source $sha $installer $cases
        if(@($result.checks|Where-Object status -eq 'pass').Count -ne 8 -or $result.checks[-1].status -cne 'unverified' -or $result.checks[-1].method -cne 'automated_draft_windows'){throw 'Draft gate was changed or promoted.'}
    }
    Check 'hosted producer cannot manufacture automated draft acceptance' { Refuse { New-HostedAcceptanceEvidence $automated '1.0.0' $source $sha $installer @{'DRAFT-INSTALLER-SMOKE'=(Case 'DRAFT-INSTALLER-SMOKE')} } }
    Check 'schema5 refuses historical owner row and wrong method' {
        $wrong=Clone $automated; $wrong.checks[-1].id='OWNER-INSTALLER-SMOKE'
        Refuse { New-HostedAcceptanceEvidence $wrong '1.0.0' $source $sha $installer }
        $wrong=Clone $automated; $wrong.checks[-1].method='hosted_windows'
        Refuse { New-HostedAcceptanceEvidence $wrong '1.0.0' $source $sha $installer }
    }
    Check 'historical schema4 cannot accept the new final gate' {
        $wrong=Clone $automated; $wrong.schema_version=4
        Refuse { New-HostedAcceptanceEvidence $wrong '1.0.0' $source $sha $installer }
    }
    Check 'schema5 overlays add only final proof binding, never immutable source fields' {
        [IO.File]::WriteAllText($basePath, ($automated|ConvertTo-Json -Depth 100))
        $newOverlay=Clone $automated
        $newOverlay|Add-Member draft_installer_smoke ([pscustomobject]@{raw_result=@{path='dist/raw.json';bytes=1;sha256=$sha}})
        [IO.File]::WriteAllText($overlayPath, ($newOverlay|ConvertTo-Json -Depth 100))
        # Shape/hash/content of the final proof is checked by the typed CLI gate.
        Assert-StableEvidenceOverlay $root 'observed.json'
        $newOverlay.production_sha256='f'*64
        [IO.File]::WriteAllText($overlayPath, ($newOverlay|ConvertTo-Json -Depth 100))
        Refuse { Assert-StableEvidenceOverlay $root 'observed.json' }
        $newOverlay=Clone $automated; $newOverlay.schema_version=4
        [IO.File]::WriteAllText($overlayPath, ($newOverlay|ConvertTo-Json -Depth 100))
        Refuse { Assert-StableEvidenceOverlay $root 'observed.json' }
        [IO.File]::WriteAllText($basePath, ($base|ConvertTo-Json -Depth 100))
    }
    Check 'schema4 overlay cannot add automated draft proof' {
        $newOverlay=Clone $base; $newOverlay|Add-Member draft_installer_smoke @{}
        [IO.File]::WriteAllText($overlayPath, ($newOverlay|ConvertTo-Json -Depth 100))
        Refuse { Assert-StableEvidenceOverlay $root 'observed.json' }
    }
    Check 'preparation reports must match the actual stable contract version' {
        $script:report=@{phase='draft';build_ready=$true;publication_allowed=$false;report=@{report_version=5;stable=$true;publication_allowed=$false}}
        Refuse { Assert-StablePhaseReadiness $root '1.0.0' 'draft' }
        [IO.File]::WriteAllText($inventoryPath, '{"schema_version":5}')
        Assert-StablePhaseReadiness $root '1.0.0' 'draft'
        $script:report.report.report_version=4
        Refuse { Assert-StablePhaseReadiness $root '1.0.0' 'draft' }
        [IO.File]::WriteAllText($inventoryPath, '{"schema_version":4}')
    }
    Check 'timed out downgrade not passed' { $entry=Case 'HOSTED-DOWNGRADE-REFUSAL'; $entry.Result.timed_out=$true; Refuse { New-HostedAcceptanceEvidence $base '1.0.0' $source $sha $installer @{'HOSTED-DOWNGRADE-REFUSAL'=$entry} } }
    Check 'wrong upgrade baseline refused' { $entry=Case 'HOSTED-UPGRADE-RC1'; $entry.Result.baseline_version='0.9.2'; Refuse { New-HostedAcceptanceEvidence $base '1.0.0' $source $sha $installer @{'HOSTED-UPGRADE-RC1'=$entry} } }
    Check 'unverified offline assertion refused' { $entry=Case 'HOSTED-OFFLINE'; $entry.Result.network_disabled=$false; Refuse { New-HostedAcceptanceEvidence $base '1.0.0' $source $sha $installer @{'HOSTED-OFFLINE'=$entry} } }
    Check 'missing metric not treated as zero' { $entry=Case 'HOSTED-PERFORMANCE'; $entry.Result.measurements.load_ms=$null; Refuse { New-HostedAcceptanceEvidence $base '1.0.0' $source $sha $installer @{'HOSTED-PERFORMANCE'=$entry} } }
    Check 'failure remains a failure' { $entry=Case 'HOSTED-INSTALL'; $entry.Result.status='fail'; $result=New-HostedAcceptanceEvidence $base '1.0.0' $source $sha $installer @{'HOSTED-INSTALL'=$entry}; if($result.checks[0].status -ne 'fail'){throw 'Failure was promoted.'} }
    Check 'changed installer cannot inherit observations' { $old=New-HostedAcceptanceEvidence $base '1.0.0' $source $sha $installer @{'HOSTED-INSTALL'=(Case 'HOSTED-INSTALL')}; Refuse { New-HostedAcceptanceEvidence $old '1.0.0' $source $sha ('e'*64) } }
    Check 'unsafe artifact path refused' { $entry=Case 'HOSTED-INSTALL'; $entry.Artifact.path='../escape'; Refuse { New-HostedAcceptanceEvidence $base '1.0.0' $source $sha $installer @{'HOSTED-INSTALL'=$entry} } }
    Check 'stable desktop cannot package an absent bundle' { Refuse { Assert-VerifiedBundleInputs $root '1.0.0' '' '' } }
    Check 'historical RC permits absent bundle' { Assert-VerifiedBundleInputs $root '1.0.0-rc.1' '' '' }
    $identityPath=Join-Path $root 'identity.json'
    [IO.File]::WriteAllText($identityPath,'{"schema_version":1}')
    Check 'bundle success requires exact app version and nonempty content' {
        $script:report=@{verified=$true;app_version='1.0.0';pack_count=2;file_count=8}
        Assert-VerifiedBundleInputs $root '1.0.0' $root $identityPath
        $script:report.app_version='1.0.0-rc.1'
        Refuse { Assert-VerifiedBundleInputs $root '1.0.0' $root $identityPath }
        $script:report.app_version='1.0.0'; $script:report.pack_count=0
        Refuse { Assert-VerifiedBundleInputs $root '1.0.0' $root $identityPath }
    }
    Check 'bundle CLI failure cannot pass' { $script:exitCode=2; Refuse { Assert-VerifiedBundleInputs $root '1.0.0' $root $identityPath }; $script:exitCode=0 }
    Check 'identity multiline and overlimit refused' {
        [IO.File]::AppendAllText($identityPath,"`n"); Refuse { Read-BoundedBundleIdentity $identityPath }
        [IO.File]::WriteAllText($identityPath,('a'*65537)); Refuse { Read-BoundedBundleIdentity $identityPath }
    }
    $pin=[pscustomobject]@{path='content/catalog.json';bytes=300000000;sha256=$sha}
    $material=[pscustomobject]@{kind='content_pack';input=$pin}
    $manifest=[pscustomobject]@{schema_version=2;catalog_files=@((Clone $pin))}
    Check 'schema2 exact catalog pin gets scoped 576MiB limit' { if((Get-ReleaseMaterialFileLimit $manifest $material) -ne 603979776){throw 'Wrong catalog limit.'} }
    foreach($field in @('path','sha256','bytes')){
        Check "different catalog $field keeps 256MiB limit" {
            $changed=Clone $material
            if($field -eq 'bytes'){$changed.input.$field++}else{$changed.input.$field='different'}
            if((Get-ReleaseMaterialFileLimit $manifest $changed) -ne 268435456){throw 'Catalog authorization escaped exact pin.'}
        }
    }
    Check 'noncatalog content and historical schemas retain old limit' {
        $changed=Clone $material; $changed.kind='corresponding_source'
        if((Get-ReleaseMaterialFileLimit $manifest $changed) -ne 268435456){throw 'Noncatalog exemption.'}
        $manifest.schema_version=1
        if((Get-ReleaseMaterialFileLimit $manifest $material) -ne 268435456){throw 'Historical schema exemption.'}
    }
    Check 'Tauri bundle identity environment and temporary config restore after failure' {
        [IO.File]::WriteAllText($identityPath,'{"schema_version":1}')
        $tokens=$null; $errors=$null
        $ast=[Management.Automation.Language.Parser]::ParseFile((Join-Path $PSScriptRoot 'Build-Windows.ps1'),[ref]$tokens,[ref]$errors)
        $definition=$ast.Find({param($node) $node -is [Management.Automation.Language.FunctionDefinitionAst] -and $node.Name -eq 'Build-Installer'},$false)
        . ([scriptblock]::Create($definition.Extent.Text))
        function Write-Step($Text) { }
        function Invoke-Native([string]$Command,[string[]]$Arguments) {
            if($Arguments -contains 'tauri'){
                $script:testConfig=$Arguments[-1]
                $config=[IO.File]::ReadAllText($script:testConfig)|ConvertFrom-Json
                if(@($config.bundle.resources.PSObject.Properties.Value)[0] -ne 'bundled-content/' -or $env:NAVZEROTH_BUNDLE_IDENTITY_JSON -cne '{"schema_version":1}') {throw 'Wrong embedded bundle identity/resources.'}
                $script:verifiedConfig=$true
                throw 'Synthetic Tauri build failure.'
            }
        }
        $RepoRoot=$root; $BundledContentDirectory=$root; $BundleBuildIdentityPath=$identityPath; $SkipTests=$true
        $script:report=@{verified=$true;app_version='1.0.0';pack_count=2;file_count=8}
        $savedIdentity=$env:NAVZEROTH_BUNDLE_IDENTITY_JSON; $savedSign=$env:NAVZEROTH_SIGN_COMMAND
        try {
            $env:NAVZEROTH_BUNDLE_IDENTITY_JSON='previous environment'; $env:NAVZEROTH_SIGN_COMMAND=$null
            $script:verifiedConfig=$false
            Refuse { Build-Installer '1.0.0' }
            if(-not $script:verifiedConfig){throw 'Mock Tauri did not receive verified identity/resources.'}
            if($env:NAVZEROTH_BUNDLE_IDENTITY_JSON -cne 'previous environment' -or (Test-Path -LiteralPath $script:testConfig)){throw 'Bundle build leaked environment or temporary config.'}
        } finally { $env:NAVZEROTH_BUNDLE_IDENTITY_JSON=$savedIdentity; $env:NAVZEROTH_SIGN_COMMAND=$savedSign }
    }
    Write-Host "Passed $count stable preparation/hosted evidence checks."
} finally { Remove-Item -LiteralPath $root -Recurse -Force }
