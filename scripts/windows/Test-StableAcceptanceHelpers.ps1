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
function Mock-Guid([string]$Id) { return ('00000000-0000-0000-0000-'+([int][char]$Id).ToString('000000000000')) }
$script:guidOverrides=@{};$script:disableFailure='';$script:leaveUp='';$script:queryFailure=$false;$script:failQueryAfterDisable=$false;$script:pollDelay=0;$script:enablePending=$false;$script:failStateQuery=$false;$script:appendMalformed=$false;$script:appearMalformed=$false;$script:malformedStatus='Disabled';$script:allEnableError=$null
function Get-NetAdapter { [CmdletBinding()]param([switch]$IncludeHidden)
 if($script:queryFailure){$script:queryFailure=$false;throw 'Synthetic adapter inventory failure'}
 if($script:enablePending){if($script:pollDelay -gt 0){$script:pollDelay--}else{$script:nics.A='Up';$script:enablePending=$false}}
 foreach($id in $script:nics.Keys){$guid=Mock-Guid $id;if($script:guidOverrides.ContainsKey($id)){$guid=$script:guidOverrides[$id]};[pscustomobject]@{InterfaceGuid=$guid;InterfaceDescription='Duplicate [description]';Name=$id;Status=$script:nics[$id]}}
 if($script:appendMalformed){[pscustomobject]@{InterfaceGuid='malformed-new-unrelated';InterfaceDescription='Other';Name='foreign';Status=$script:malformedStatus}}
}
function Disable-NetAdapter { [CmdletBinding(SupportsShouldProcess)]param($InputObject) $id=$InputObject.Name;$script:nics[$id]='Disabled';$script:disabled+=$id;if($script:appearMalformed){$script:appendMalformed=$true};if($script:leaveUp -eq $id){$script:nics[$id]='Up'};if($script:failQueryAfterDisable){$script:queryFailure=$true;$script:failQueryAfterDisable=$false};if($script:disableFailure -eq $id){throw 'Synthetic partial disable failure'} }
function Enable-NetAdapter { [CmdletBinding(SupportsShouldProcess)]param($InputObject) $id=$InputObject.Name;$script:nics[$id]='Up';$script:enabled+=$id;if($script:failStateQuery){$script:queryFailure=$true;$script:failStateQuery=$false};if($script:pollDelay -gt 0){$script:nics[$id]='Disconnected';$script:enablePending=$true};if($null -ne $script:allEnableError){throw $script:allEnableError};if($script:enableFailure -eq $id){throw 'Synthetic enable failure after state changed'} }
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
 Check 'duplicate descriptions select distinct issued GUID objects and never enable originally-down adapters' {
  $script:nics=@{A='Up';B='Up';C='Disabled'};$script:enabled=@();$script:disabled=@()
  $actual=Invoke-StableOffline {return @{actual='workload'}}
  if($actual.adapters.Count -ne 2 -or @($actual.adapters|Sort-Object -Unique).Count -ne 2 -or ($script:enabled|Sort-Object) -join ',' -ne 'A,B' -or $script:nics.C -ne 'Disabled'){throw 'Ambiguous description affected identity scope'}
 }
 Check 'missing malformed and duplicate GUIDs refuse before any mutation' {
  foreach($bad in @('', 'invalid', '00000000-0000-0000-0000-000000000000',(Mock-Guid 'A'))){
   $script:nics=@{A='Up';B='Up'};$script:guidOverrides=@{B=$bad};$script:disabled=@();$script:enabled=@()
   Refuse {Invoke-StableOffline {throw 'Must not run'}}
   if($script:disabled.Count -ne 0 -or $script:enabled.Count -ne 0){throw 'Invalid GUID mutated adapters'}
  };$script:guidOverrides=@{}
 }
 Check 'partial disable failure restores every attempted GUID and preserves original error' {
  $script:nics=[ordered]@{A='Up';B='Up';C='Disabled'};$script:disableFailure='B';$script:enabled=@()
  $failure=$null;try{Invoke-StableOffline {throw 'Must not run'}|Out-Null}catch{$failure=$_.Exception.Message}
  if($failure -notlike '*Synthetic partial disable failure*' -or ($script:enabled|Sort-Object) -join ',' -ne 'A,B' -or $script:nics.C -ne 'Disabled'){throw 'Partial disable recovery lost scope/cause'}
  $script:disableFailure=''
 }
 Check 'primary workload and cleanup errors are both retained with exact final state' {
  $script:nics=@{A='Up'};$script:enableFailure='A';$failure=$null
  try{Invoke-StableOffline {throw 'Synthetic original workload failure'}|Out-Null}catch{$failure=$_.Exception.Message}
  if($failure -notlike 'Hosted offline control failed: *'){throw 'Diagnostic envelope missing'}
  $diag=$failure.Substring('Hosted offline control failed: '.Length)|ConvertFrom-Json
  if($diag.primary.stage -cne 'workload' -or $diag.primary.error -cne 'Synthetic original workload failure' -or $diag.restoration[0].enable_error -cne 'Synthetic enable failure after state changed' -or $diag.restoration[0].status -cne 'Up'){throw 'One failure masked another'}
  $script:enableFailure=''
 }
 Check 'remaining active interface refuses workload even when restoration succeeds' {
  $script:nics=@{A='Up'};$script:leaveUp='A';$failure=$null
  try{Invoke-StableOffline {throw 'Must not run'}|Out-Null}catch{$failure=$_.Exception.Message}
  if($failure -notlike '*External network adapters remain active*'){throw 'Offline assertion bypassed'};$script:leaveUp=''
 }
 Check 'fresh inventory failure is preserved while exact original adapter is restored' {
  $script:nics=@{A='Up'};$script:failQueryAfterDisable=$true;$script:enabled=@();$failure=$null
  try{Invoke-StableOffline {throw 'Must not run'}|Out-Null}catch{$failure=$_.Exception.Message}
  if($failure -notlike '*Synthetic adapter inventory failure*' -or ($script:enabled -join ',') -cne 'A' -or $script:nics.A -cne 'Up'){throw 'Inventory failure lost cleanup'}
 }
 Check 'originally-down duplicate GUID refuses before mutations and delayed restoration waits for exact Up' {
  $script:nics=@{A='Up';B='Disabled'};$script:guidOverrides=@{B=(Mock-Guid 'A')};$script:disabled=@()
  Refuse {Invoke-StableOffline {throw 'Must not run'}}
  if($script:disabled.Count -ne 0){throw 'Ambiguous down adapter mutated inventory'};$script:guidOverrides=@{}
  $script:nics=@{A='Up'};$script:pollDelay=2
  $actual=Invoke-StableOffline {return @{actual='workload'}}
  if($actual.diagnostics.restoration[0].status -cne 'Up' -or $script:enablePending -or $script:pollDelay -ne 0){throw 'Restoration did not wait for precise Up'}
 }
 Check 'transient restoration query failure remains a failure even after exact Up is observed' {
  $script:nics=@{A='Up'};$script:failStateQuery=$true;$failure=$null
  try{Invoke-StableOffline {return @{actual='workload'}}|Out-Null}catch{$failure=$_.Exception.Message}
  $diag=$failure.Substring('Hosted offline control failed: '.Length)|ConvertFrom-Json
  if($diag.restoration[0].status -cne 'Up' -or $diag.restoration[0].state_error -cne 'Synthetic adapter inventory failure'){throw 'Transient restoration failure was promoted'}
 }
 Check 'new malformed unrelated NIC cannot block cleanup or bypass the zero-Up proof' {
  foreach($state in @('Disabled','Up')){
   $script:nics=@{A='Up';B='Up'};$script:enabled=@();$script:appearMalformed=$true;$script:malformedStatus=$state;$script:workloadRan=$false
   if($state -ceq 'Disabled'){
    $actual=Invoke-StableOffline {$script:workloadRan=$true;return @{actual='workload'}}
    if(-not $script:workloadRan -or @($actual.diagnostics.restoration|Where-Object status -cne 'Up').Count -ne 0){throw 'Controlled restoration failed'}
   }else{
    $failure=$null;try{Invoke-StableOffline {$script:workloadRan=$true;throw 'Must not run'}|Out-Null}catch{$failure=$_.Exception.Message}
    if($script:workloadRan -or $failure -notlike '*External network adapters remain active*'){throw 'New active adapter bypassed offline proof'}
   }
   if(($script:enabled|Sort-Object) -join ',' -ne 'A,B'){throw 'Unrelated identity prevented original restoration'}
   $script:appearMalformed=$false;$script:appendMalformed=$false
  };$script:malformedStatus='Disabled'
 }
 Check '64 adapter escaped errors retain primary and every cleanup failure within the final byte cap' {
  $script:nics=@{};foreach($n in 33..96){$script:nics[[string][char]$n]='Up'}
  $script:allEnableError=([string][char]1)*256;$failure=$null
  try{Invoke-StableOffline {throw ('PRIMARY '+(([string][char]1)*256))}|Out-Null}catch{$failure=$_.Exception.Message}
  if($failure -notlike 'Hosted offline control failed: *' -or [Text.Encoding]::UTF8.GetByteCount($failure) -gt 65536){throw 'Worst-case diagnostics lost bounded envelope'}
  $diag=$failure.Substring('Hosted offline control failed: '.Length)|ConvertFrom-Json
  if($diag.primary.error -notlike 'PRIMARY *' -or $diag.restoration.Count -ne 64 -or @($diag.restoration|Where-Object {$null -eq $_.enable_error -or $_.status -cne 'Up'}).Count -ne 0){throw 'Worst-case diagnostic masked primary or cleanup'}
  $script:allEnableError=$null
 }
 Check 'diagnostic messages are bounded without losing failure status' {
  $script:nics=@{A='Up'};$failure=$null
  try{Invoke-StableOffline {throw ('x'*10000)}|Out-Null}catch{$failure=$_.Exception.Message}
  $diag=$failure.Substring('Hosted offline control failed: '.Length)|ConvertFrom-Json
  if($diag.primary.error.Length -ne 256 -or [Text.Encoding]::UTF8.GetByteCount($failure) -gt 65536){throw 'Failure diagnostics unbounded'}
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
import {writeFileSync} from 'node:fs';
const args=process.argv.slice(2),report=args[args.indexOf('--out')+1];
// Deliberate startup work before the first checkpoint; not a product delay.
await new Promise(resolve=>setTimeout(resolve,5000));
const progress=createWorkloadProgress(report+'.progress.json',{mode:'recover',version:'1.0.0'});
progress.phase('exercise');
try { await progress.track('inspect_database_backup',async()=>{setInterval(()=>{},1000);await new Promise(()=>{});}); }
finally { writeFileSync(report,JSON.stringify({passed:false,completed:false,kind:'synthetic_finally_marker'})); }
'@
   [IO.File]::WriteAllText((Join-Path $scripts 'Stable-Workload.mjs'),$child,(New-Object Text.UTF8Encoding($false)))
   Write-Host ('Synthetic watchdog uses Node '+(& node --version)+' at '+(Get-Command node).Source)
   # A valid child need not reach its first checkpoint inside the former2s test
   # limit. Prove that assumption fails, then allow bounded startup separately
   # in this TEST ONLY. Production MaximumSeconds/20min/60min are unchanged.
   $early=Join-Path $root 'startup-timeout.json';$earlyMessage=''
   try {Invoke-StableWorkload $synthetic 'not-an-installed-executable' 'synthetic-manifest' '1.0.0' 'recover' 'synthetic-state' $early '' 2|Out-Null} catch {$earlyMessage=$_.Exception.Message}
   if($earlyMessage -cne 'Installed engineering workload exceeded its selected finite time bound.' -or (Test-Path -LiteralPath ($early+'.progress.json')) -or (Test-Path -LiteralPath $early)){throw 'Delayed startup did not reproduce the original checkpoint assumption'}
   $earlyWatchdog=Read-BoundedReleaseText ($early+'.watchdog.json')|ConvertFrom-Json
   if($earlyWatchdog.passed -ne $false -or $earlyWatchdog.completed -ne $false -or $earlyWatchdog.maximum_seconds -ne 2){throw 'Startup timeout was lost or promoted'}
   $report=Join-Path $root 'killed-workload.json';$message=''
   try {Invoke-StableWorkload $synthetic 'not-an-installed-executable' 'synthetic-manifest' '1.0.0' 'recover' 'synthetic-state' $report '' 15|Out-Null} catch {$message=$_.Exception.Message}
   if($message -cne 'Installed engineering workload exceeded its selected finite time bound.'){throw ('Wrong timeout failure: '+$message)}
   $progress=Read-BoundedReleaseText ($report+'.progress.json')|ConvertFrom-Json
   $watchdog=Read-BoundedReleaseText ($report+'.watchdog.json')|ConvertFrom-Json
   if((Test-Path -LiteralPath $report) -or $progress.passed -ne $false -or $progress.completed -ne $false -or $progress.active_command -cne 'inspect_database_backup' -or $progress.phase -cne 'exercise'){throw 'Killed child progress was lost or promoted'}
   if($watchdog.passed -ne $false -or $watchdog.completed -ne $false -or $watchdog.maximum_seconds -ne 15 -or $watchdog.elapsed_ms -lt 15000 -or $watchdog.reason -cne 'selected_time_bound_exceeded' -or $watchdog.native_pids.Count -gt 16){throw 'Watchdog metadata differs'}
  }
 } else {Write-Host 'Actual parent timeout regression requires PS7 argument-list API; not executed in PS5.1.'}
 Write-Host "Passed $count hosted producer helper checks."
}finally{$env:GITHUB_ACTIONS=$savedActions;$env:RUNNER_ENVIRONMENT=$savedRunner;$env:OS=$savedOS;Remove-Item -LiteralPath $root -Recurse -Force}
