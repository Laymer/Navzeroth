# Synthetic contracts and mocked adapters only: never execute installers/network controls.
Set-StrictMode -Version 3.0
$ErrorActionPreference='Stop'
. (Join-Path $PSScriptRoot 'Hosted-Acceptance.ps1')
. (Join-Path $PSScriptRoot 'Release-Materials.ps1')
. (Join-Path $PSScriptRoot 'Stable-Acceptance-Helpers.ps1')
$root=Join-Path ([IO.Path]::GetTempPath()) ('anav-hosted-helper-'+[Guid]::NewGuid().ToString('N'))
$count=0
function Check($Name,[scriptblock]$Action){&$Action;$script:count++;Write-Host "ok $Name"}
function Refuse([scriptblock]$Action){$failed=$false;try{&$Action|Out-Null}catch{$failed=$true};if(-not $failed){throw 'Expected refusal'}}
$identity=@{version='1.0.0';source_commit=('a'*40);production_sha256=('b'*64);installer_sha256=('c'*64);target=@{edition='wow_forever';version='1.60.1';build='70450';interface=16001};operator='Synthetic hosted helper test';environment='Synthetic mock only'}
$savedActions=$env:GITHUB_ACTIONS;$savedRunner=$env:RUNNER_ENVIRONMENT;$savedOS=$env:OS
function Get-NetAdapter { [CmdletBinding()]param([string]$InterfaceDescription,[switch]$IncludeHidden) foreach($id in $script:nics.Keys){if(-not $InterfaceDescription -or $InterfaceDescription -eq $id){[pscustomobject]@{InterfaceDescription=$id;Status=$script:nics[$id]}}} }
function Disable-NetAdapter { [CmdletBinding(SupportsShouldProcess)]param([string]$InterfaceDescription) $script:nics[$InterfaceDescription]='Disabled';$script:disabled+=$InterfaceDescription }
function Enable-NetAdapter { [CmdletBinding(SupportsShouldProcess)]param([string]$InterfaceDescription) $script:nics[$InterfaceDescription]='Up';$script:enabled+=$InterfaceDescription;if($script:enableFailure -eq $InterfaceDescription){throw 'Synthetic enable failure after state changed'} }
try{
 New-Item -ItemType Directory -Path $root|Out-Null
 $expectedTarget='{"build":"70450","edition":"wow_forever","interface":16001,"version":"1.60.1"}' | ConvertFrom-Json
 $actualTarget='{"edition":"wow_forever","version":"1.60.1","build":"70450","interface":16001}' | ConvertFrom-Json
 Check 'actual current target identity accepts different JSON field order' {
  if (($expectedTarget|ConvertTo-Json -Compress) -ceq ($actualTarget|ConvertTo-Json -Compress)) {throw 'Regression fixture does not reproduce original order mismatch'}
  Assert-StableTargetIdentity $expectedTarget $actualTarget
 }
 Check 'current source-bound release and committed build identity targets match' {
  $repository=[IO.Path]::GetFullPath((Join-Path $PSScriptRoot '../..'))
  $release=Read-BoundedReleaseContractText (Join-Path $repository 'release/v1/acceptance-evidence.json')|ConvertFrom-Json
  $build=Read-BoundedBundleIdentity (Join-Path $repository 'release/v1/bundled-content-build-identity.json')|ConvertFrom-Json
  Assert-StableTargetIdentity $release.target $build.target
 }
 Check 'integer widths from supported PowerShell JSON parsers retain identity' {$actualTarget.interface=[long]16001;Assert-StableTargetIdentity $expectedTarget $actualTarget}
 foreach($field in @('edition','version','build','interface')) {
  Check "wrong target $field is refused" {
   $bad=$actualTarget|ConvertTo-Json|ConvertFrom-Json
   if($field -eq 'interface'){$bad.$field=16002}else{$bad.$field='different'}
   Refuse {Assert-StableTargetIdentity $expectedTarget $bad}
  }
 }
 Check 'missing and additional target fields are refused' {
  $bad=$actualTarget|ConvertTo-Json|ConvertFrom-Json;$bad.PSObject.Properties.Remove('build');Refuse {Assert-StableTargetIdentity $expectedTarget $bad}
  $bad=$actualTarget|ConvertTo-Json|ConvertFrom-Json;$bad|Add-Member extra 'unbound';Refuse {Assert-StableTargetIdentity $expectedTarget $bad}
 }
 Check 'malformed interface types and bounds are refused' {
  foreach($value in @('16001',$true,16001.0,0,-1,4294967296,$null)) {
   $bad=$actualTarget|ConvertTo-Json|ConvertFrom-Json;$bad.interface=$value;Refuse {Assert-StableTargetIdentity $expectedTarget $bad}
  }
 }
 Check 'non-object targets and malformed string fields are refused on either side' {
  foreach($bad in @($null,'target',@($actualTarget),@{edition='wow_forever'})) {Refuse {Assert-StableTargetIdentity $expectedTarget $bad};Refuse {Assert-StableTargetIdentity $bad $actualTarget}}
  foreach($name in @('edition','version','build')) {$bad=$actualTarget|ConvertTo-Json|ConvertFrom-Json;$bad.$name=16001;Refuse {Assert-StableTargetIdentity $expectedTarget $bad}}
 }
 # Same pure constructor used by the real producer, through real case validation.
 $workload=[pscustomobject]@{raw=[pscustomobject]@{startup_measurements=[pscustomobject]@{load_ms=1;activation_ms=0};browse_ms=2.5;planning_ms=3};peak_memory_bytes=4L}
 function PerformanceCase($Measurements) { Invoke-StableCase 'HOSTED-PERFORMANCE' $identity $root {return @{workload='Synthetic measured workload';hardware='Synthetic hardware';limitations='Synthetic helper only, not installed acceptance';measurements=$Measurements}} }
 function ReadPerformance { ConvertFrom-HostedJsonText ([IO.File]::ReadAllText((Join-Path $root 'hosted-performance.json'))) }
 Check 'old six-key ordered dictionary reproduces actual performance admission failure' {
  $old=[ordered]@{install_ms=0;load_ms=1;activation_ms=0;browse_ms=2.5;planning_ms=3;peak_memory_bytes=4L}
  if($old.Keys.Count -ne 6 -or @($old.PSObject.Properties.Name).Count -eq 6){throw 'Regression fixture does not reproduce dictionary adapter mismatch'}
  Refuse {PerformanceCase $old}
  if((ReadPerformance).status -cne 'fail'){throw 'Original failure was not retained'}
 }
 Check 'actual performance constructor passes live case validation and JSON roundtrip' {
  $measurements=New-StablePerformanceMeasurements 0 $workload
  PerformanceCase $measurements
  $case=ReadPerformance
  Assert-HostedCase $case 'HOSTED-PERFORMANCE' $identity.version $identity.source_commit $identity.production_sha256 $identity.installer_sha256 $identity.target
  if($case.status -cne 'pass' -or @($case.measurements.PSObject.Properties.Name).Count -ne 6 -or $case.measurements.activation_ms -ne 0 -or $case.measurements.browse_ms -ne 2.5){throw 'Exact actual measurements were lost'}
 }
 Check 'extra missing and renamed performance metrics still refuse live admission' {
  foreach($kind in @('extra','missing','renamed')) {
   $measurements=New-StablePerformanceMeasurements 0 $workload
   if($kind -eq 'extra'){$measurements|Add-Member extra 1}else{$measurements.PSObject.Properties.Remove('load_ms');if($kind -eq 'renamed'){$measurements|Add-Member LOAD_MS 1}}
   Refuse {PerformanceCase $measurements}
   if((ReadPerformance).status -cne 'fail'){throw 'Invalid metric keys were promoted'}
  }
 }
 Check 'all six metrics retain strict numeric type finite and range validation' {
  foreach($field in @('install_ms','load_ms','activation_ms','browse_ms','planning_ms','peak_memory_bytes')) {
   foreach($bad in @($null,'1',$true,-1,[double]::NaN,[double]::PositiveInfinity,[double]::NegativeInfinity,9007199254740992L)) {
    $measurements=New-StablePerformanceMeasurements 0 $workload;$measurements.$field=$bad
    Refuse {PerformanceCase $measurements}
    if((ReadPerformance).status -cne 'fail'){throw 'Invalid metric value was promoted'}
   }
  }
 }
 Check 'performance constructor never coerces raw string values to numbers' {
  $changed=ConvertFrom-HostedJsonText ($workload|ConvertTo-Json -Depth 100);$changed.raw.startup_measurements.load_ms='1'
  $measurements=New-StablePerformanceMeasurements 0 $changed
  if($measurements.load_ms -isnot [string]){throw 'Measurement was coerced'}
  Refuse {PerformanceCase $measurements}
 }
 Check 'completed actual details create a pass' {Invoke-StableCase 'HOSTED-INSTALL' $identity $root {return @{install_ms=123}};$case=Get-Content -Raw (Join-Path $root 'hosted-install.json')|ConvertFrom-Json;if($case.status -ne 'pass' -or $case.install_ms -ne 123){throw 'Missing actual measurement'}}
 Check 'actual failure is retained as fail and thrown' {Refuse {Invoke-StableCase 'HOSTED-INSTALL' $identity $root {throw 'Synthetic failure'}};$case=Get-Content -Raw (Join-Path $root 'hosted-install.json')|ConvertFrom-Json;if($case.status -ne 'fail'){throw 'Failure promoted'}}
 Check 'hosted producer cannot record owner smoke' {Refuse {Invoke-StableCase 'OWNER-INSTALLER-SMOKE' $identity $root {return @{}}}}
 Check 'action cannot replace release identity' {Refuse {Invoke-StableCase 'HOSTED-INSTALL' $identity $root {return @{installer_sha256=('f'*64)}}}}
 Check 'unsupported or missing results cannot pass' {Refuse {Invoke-StableCase 'HOSTED-INSTALL' $identity $root {return $null}};Refuse {Invoke-StableCase 'HOSTED-OFFLINE' $identity $root {return @{network_disabled=$false}}}}
 $env:GITHUB_ACTIONS='true';$env:RUNNER_ENVIRONMENT='github-hosted';$env:OS='Windows_NT'
 $script:enableFailure='';$script:nics=@{A='Up';B='Disabled'};$script:enabled=@();$script:disabled=@()
 Check 'offline workload runs with external adapters disabled and restores only originally-up ones' {
  $result=Invoke-StableOffline {if(@(Get-NetAdapter|Where-Object Status -eq 'Up').Count -ne 0){throw 'Network active'};return @{actual='workload'}}
  if(-not $result.network_disabled -or -not $result.restored -or $result.workload.actual -ne 'workload' -or $script:nics.A -ne 'Up' -or $script:nics.B -ne 'Disabled' -or ($script:enabled -join ',') -ne 'A'){throw 'Adapter scope/result mismatch'}
 }
 Check 'workload failure restores adapters before propagating error' {Refuse {Invoke-StableOffline {throw 'Synthetic native failure'}};if($script:nics.A -ne 'Up'){throw 'Adapter not restored'}}
 Check 'all original adapters get a restoration attempt after one failure' {
  $script:nics=@{A='Up';B='Up'};$script:enableFailure='A';$script:enabled=@()
  Refuse {Invoke-StableOffline {return @{actual='workload'}}}
  if($script:enabled.Count -ne 2 -or $script:nics.A -ne 'Up' -or $script:nics.B -ne 'Up'){throw 'Restore skipped adapter'};$script:enableFailure=''
 }
 Check 'lack of controllable adapter is unverified/failure' {$script:nics=@{A='Disabled'};Refuse {Invoke-StableOffline {throw 'Should not run'}}}
 Check 'offline controls refuse non-hosted environments' {$env:RUNNER_ENVIRONMENT='self-hosted';Refuse {Invoke-StableOffline {throw 'Should not run'}}}
 if ($PSVersionTable.PSVersion.Major -ge 7) {
  Check 'actual parent timeout retains pending child phase and watchdog without a pass' {
   $synthetic=Join-Path $root 'timeout-root';$scripts=Join-Path $synthetic 'scripts/windows'
   New-Item -ItemType Directory -Path $scripts -Force|Out-Null
   Copy-Item -LiteralPath (Join-Path $PSScriptRoot 'stable-workload-progress.mjs') -Destination $scripts
   $child=@'
import {createWorkloadProgress} from './stable-workload-progress.mjs';
const args=process.argv.slice(2),report=args[args.indexOf('--out')+1];
const progress=createWorkloadProgress(report+'.progress.json',{mode:'recover',version:'1.0.0'});
progress.phase('exercise');
await progress.track('inspect_database_backup',async()=>{setInterval(()=>{},1000);await new Promise(()=>{});});
'@
   [IO.File]::WriteAllText((Join-Path $scripts 'Stable-Workload.mjs'),$child,(New-Object Text.UTF8Encoding($false)))
   $report=Join-Path $root 'killed-workload.json';$message=''
   try {Invoke-StableWorkload $synthetic 'not-an-installed-executable' 'synthetic-manifest' '1.0.0' 'recover' 'synthetic-state' $report '' 2|Out-Null} catch {$message=$_.Exception.Message}
   if($message -cne 'Installed engineering workload exceeded its selected finite time bound.'){throw ('Wrong timeout failure: '+$message)}
   $progress=Read-BoundedReleaseText ($report+'.progress.json')|ConvertFrom-Json
   $watchdog=Read-BoundedReleaseText ($report+'.watchdog.json')|ConvertFrom-Json
   if((Test-Path -LiteralPath $report) -or $progress.passed -ne $false -or $progress.completed -ne $false -or $progress.active_command -cne 'inspect_database_backup' -or $progress.phase -cne 'exercise'){throw 'Killed child progress was lost or promoted'}
   if($watchdog.passed -ne $false -or $watchdog.completed -ne $false -or $watchdog.maximum_seconds -ne 2 -or $watchdog.elapsed_ms -lt 2000 -or $watchdog.reason -cne 'selected_time_bound_exceeded' -or $watchdog.native_pids.Count -gt 16){throw 'Watchdog metadata differs'}
  }
 } else {Write-Host 'Actual parent timeout regression requires PS7 argument-list API; not executed in PS5.1.'}
 Write-Host "Passed $count hosted producer helper checks."
}finally{$env:GITHUB_ACTIONS=$savedActions;$env:RUNNER_ENVIRONMENT=$savedRunner;$env:OS=$savedOS;Remove-Item -LiteralPath $root -Recurse -Force}
