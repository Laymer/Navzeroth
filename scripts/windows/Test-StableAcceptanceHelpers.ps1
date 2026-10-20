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
 Write-Host "Passed $count hosted producer helper checks."
}finally{$env:GITHUB_ACTIONS=$savedActions;$env:RUNNER_ENVIRONMENT=$savedRunner;$env:OS=$savedOS;Remove-Item -LiteralPath $root -Recurse -Force}
