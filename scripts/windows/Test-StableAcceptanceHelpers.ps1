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
 # ADR 0010 firewall isolation: mocked profiles/rules and connectivity only.
 function New-MockFirewall {
  $script:fwProfiles=[ordered]@{Domain=@{e='True';i='NotConfigured';o='NotConfigured'};Private=@{e='True';i='Block';o='Allow'};Public=@{e='False';i='NotConfigured';o='NotConfigured'}}
  $script:fwRules=[ordered]@{};foreach($n in @('core-dns','edge-mdns','ssh-in')){$script:fwRules[$n]=@{Enabled=$true;Action='Allow';Group=''}};$script:fwRules['wsh-block']=@{Enabled=$true;Action='Block';Group=''}
  $script:fwOnline=$true;$script:fwLeak=$false;$script:fwDisableFail=$false;$script:fwEnableFail='';$script:fwStickyRule='';$script:fwNoReturn=$false;$script:fwAgents=@('C:\actions\runner\bin\Runner.Listener.exe','C:\actions\runner\bin\Runner.Worker.exe');$script:fwSeen=$null;$script:fwSequence=0;$script:fwWasIsolated=$false;$script:fwCalls=@();$script:fwRestoreProfileFail=$false;$script:fwCreateFail=$false;$script:fwActive=@();$script:fwInfra=@('C:\Program Files\hosted-compute-agent\hosted-compute-agent.exe')
 }
 function Get-StableRunnerAgentPrograms { if($script:fwAgents.Count -ne 2){throw 'Synthetic missing runner ancestry'};return $script:fwAgents }
 $script:realInfrastructurePrograms=${function:Get-StableHostedInfrastructurePrograms}
 function Get-StableHostedInfrastructurePrograms { return $script:fwInfra }
 function Resolve-StableProbeAddress([string]$HostName) { return '192.0.2.1' }
 function Test-StableExternalTcp([string]$Address,[int]$Port,[int]$TimeoutMs=5000) {
  if(-not $script:fwOnline){return $false}
  $blocked=@($script:fwProfiles.Values|Where-Object {$_.e -eq 'True' -and $_.o -eq 'Block'}).Count -eq 3
  if($blocked){$script:fwWasIsolated=$true;return $script:fwLeak};if($script:fwNoReturn -and $script:fwWasIsolated){return $false};return $true
 }
 function Get-NetFirewallProfile { [CmdletBinding()]param([string]$PolicyStore)
  foreach($n in $script:fwProfiles.Keys){$p=$script:fwProfiles[$n];[pscustomobject]@{Name=$n;Enabled=$p.e;DefaultInboundAction=$p.i;DefaultOutboundAction=$p.o}}
 }
 function Set-NetFirewallProfile { [CmdletBinding()]param([string[]]$Name,[string]$Enabled,[string]$DefaultInboundAction,[string]$DefaultOutboundAction,[string]$PolicyStore)
  if($DefaultOutboundAction -ne 'Block'){$script:fwCalls+='restore-profile';if($script:fwRestoreProfileFail){throw 'Synthetic profile restore failure'}}
  foreach($n in $Name){$script:fwProfiles[$n]=@{e=$Enabled;i=$DefaultInboundAction;o=$DefaultOutboundAction}}
 }
 function Get-NetFirewallRule { [CmdletBinding()]param([string]$PolicyStore,[string]$Enabled,[string]$Action,[string]$Group,[string]$Direction)
  foreach($n in @($script:fwRules.Keys)){$r=$script:fwRules[$n]
   if(($Enabled -and ([string]$r.Enabled) -ne $Enabled) -or ($Action -and $r.Action -ne $Action) -or ($Group -and $r.Group -ne $Group)){continue}
   [pscustomobject]@{Name=$n}}
 }
 function New-NetFirewallRule { [CmdletBinding()]param([string]$DisplayName,[string]$Group,[string]$Direction,[string]$Action,[string]$Program,[string]$Service,[string]$Protocol,[string]$RemotePort,[string]$PolicyStore)
  if($script:fwCreateFail){throw 'Synthetic rule creation failure'};$script:fwSequence++;$script:fwRules[('{iso-'+$script:fwSequence+'}')]=@{Enabled=$true;Action=$Action;Group=$Group;Program=$Program}
 }
 function Remove-NetFirewallRule { [CmdletBinding()]param([string]$Group,[string]$PolicyStore)
  $script:fwCalls+='remove-group';$matched=@($script:fwRules.Keys|Where-Object {$script:fwRules[$_].Group -eq $Group})
  if($matched.Count -eq 0){throw 'No MSFT_NetFirewallRule objects found with property Group'}
  foreach($n in $matched){$script:fwRules.Remove($n)}
 }
 function Disable-NetFirewallRule { [CmdletBinding()]param([string[]]$Name,[string]$PolicyStore)
  foreach($n in $Name){if($script:fwDisableFail -and $n -eq 'edge-mdns'){throw 'Synthetic partial disable failure'};if($n -ne $script:fwStickyRule){$script:fwRules[$n].Enabled=$false}}
 }
 function Enable-NetFirewallRule { [CmdletBinding()]param([string[]]$Name,[string]$PolicyStore)
  $script:fwCalls+='enable-rules';foreach($n in $Name){if($n -eq $script:fwEnableFail){throw 'Synthetic enable failure'}}
  foreach($n in $Name){$script:fwRules[$n].Enabled=$true}
 }
 function Get-StableActiveAllowInventory([string]$Group,[int]$OwnRules) { if($OwnRules -ne 4+$script:fwInfra.Count){throw 'Synthetic wrong own-rule count'};return $script:fwActive }
 function Snapshot-MockFirewall { return (@{p=$script:fwProfiles;r=@($script:fwRules.Keys|ForEach-Object {$_+'='+$script:fwRules[$_].Enabled})}|ConvertTo-Json -Depth 5 -Compress) }
 function FirewallFailure([scriptblock]$Workload,[int]$RestoreSeconds=60) {
  $failure=$null;try{Invoke-StableFirewallOffline $Workload -RestoreSeconds $RestoreSeconds|Out-Null}catch{$failure=$_.Exception.Message};return $failure
 }
 Check 'firewall isolation blocks external access during the workload and restores exactly' {
  New-MockFirewall;$before=Snapshot-MockFirewall
  $result=Invoke-StableFirewallOffline {
   $script:fwSeen=@{blocked=@($script:fwProfiles.Values|Where-Object {$_.e -eq 'True' -and $_.i -eq 'Block' -and $_.o -eq 'Block'}).Count;enabled=@($script:fwRules.Keys|Where-Object {$script:fwRules[$_].Enabled -and $script:fwRules[$_].Action -eq 'Allow'});programs=@($script:fwRules.Values|Where-Object {$_.Group}|ForEach-Object {$_.Program})}
   return @{actual='workload'}}
  if($script:fwSeen.blocked -ne 3 -or $script:fwSeen.enabled.Count -ne 5 -or @($script:fwSeen.programs|Where-Object {$_ -like '*hosted-compute-agent.exe'}).Count -ne 1 -or @($script:fwSeen.enabled|Where-Object {$_ -notlike '{iso-*'}).Count -ne 0 -or @($script:fwSeen.programs|Where-Object {$_ -like '*Runner.*'}).Count -ne 2){throw 'Isolation scope differs from runner, host agents and DNS only'}
  if((Snapshot-MockFirewall) -cne $before){throw 'Firewall state was not restored exactly'}
  if(-not $result.network_isolated -or -not $result.external_connection_refused -or -not $result.restored -or $result.isolation -cne 'windows_firewall_runner_and_host_agents_only' -or $result.workload.actual -ne 'workload' -or $result.allowed_programs.Count -ne 2 -or @($result.allowed_infrastructure_programs).Count -ne 1 -or $result.diagnostics.disabled_allow_rules -ne 3){throw 'Isolation result differs'}
 }
 Check 'isolation works when no hosted infrastructure agent is present' {
  New-MockFirewall;$script:fwInfra=@();$before=Snapshot-MockFirewall
  $result=Invoke-StableFirewallOffline {return @{actual='workload'}}
  if(@($result.allowed_infrastructure_programs).Count -ne 0 -or (Snapshot-MockFirewall) -cne $before){throw 'Optional infrastructure agents became required'}
 }
 if ([IO.Path]::IsPathRooted('C:\x')) {
 Check 'hosted infrastructure selection keeps only real, bounded, non-workload agents' {
  # Real selection logic with mocked processes and files; never inspects the host.
  function Get-Process { [CmdletBinding()]param() foreach($p in $script:procs){[pscustomobject]$p} }
  function ConvertTo-StableLongPath([string]$Path) { return $Path.Replace('RUNNER~1','runneradmin') }
  function Test-Path { [CmdletBinding()]param([string]$LiteralPath,[string]$PathType) return $LiteralPath -notlike '*missing*' }
  $script:procs=@(@{ProcessName='hosted-compute-agent';Path='C:\ProgramData\GitHub\HostedComputeAgent\hosted-compute-agent'},@{ProcessName='provjobd.exe1';Path='C:\Users\RUNNER~1\AppData\Local\Temp\provjobd.exe1'},@{ProcessName='WaAppAgent';Path='C:\WindowsAzure\G\WaAppAgent.exe'},@{ProcessName='pwsh';Path='C:\Program Files\PowerShell\7\pwsh.exe'},@{ProcessName='WaAppAgent';Path=$null})
  $paths=@(& $script:realInfrastructurePrograms)
  if(($paths -join '|') -cne 'C:\ProgramData\GitHub\HostedComputeAgent\hosted-compute-agent|C:\Users\runneradmin\AppData\Local\Temp\provjobd.exe1|C:\WindowsAzure\G\WaAppAgent.exe'){throw ('Wrong agent selection: '+($paths -join '|'))}
  $script:procs=@(@{ProcessName='provjobd.exe2';Path='C:\Temp\node.exe'});Refuse {& $script:realInfrastructurePrograms}
  $script:procs=@(@{ProcessName='WaAppAgent';Path='C:\missing\WaAppAgent.exe'});Refuse {& $script:realInfrastructurePrograms}
  $script:procs=@(@{ProcessName='WaAppAgent';Path='relative\WaAppAgent.exe'});Refuse {& $script:realInfrastructurePrograms}
  $script:procs=@(1..9|ForEach-Object {@{ProcessName='provjobd.exe'+$_;Path='C:\Temp\provjobd.exe'+$_}});Refuse {& $script:realInfrastructurePrograms}
  $script:procs=@();if(@(& $script:realInfrastructurePrograms).Count -ne 0){throw 'Absent agents were invented'}
 }
 } else {Write-Host 'Hosted infrastructure path selection uses Windows drive paths; not executed on this platform.'}
 Check 'firewall workload failure restores state and keeps the original cause' {
  New-MockFirewall;$before=Snapshot-MockFirewall
  $failure=FirewallFailure {throw 'Synthetic native failure'}
  if($failure -notlike 'Hosted offline firewall control failed: *'){throw 'Diagnostic envelope missing'}
  $diag=$failure.Substring('Hosted offline firewall control failed: '.Length)|ConvertFrom-Json
  if($diag.primary.stage -cne 'workload' -or $diag.primary.error -cne 'Synthetic native failure' -or (Snapshot-MockFirewall) -cne $before){throw 'Workload failure lost cause or state'}
 }
 Check 'external access under isolation refuses before the workload' {
  New-MockFirewall;$before=Snapshot-MockFirewall;$script:fwLeak=$true;$script:workloadRan=$false
  $failure=FirewallFailure {$script:workloadRan=$true;return @{}}
  if($script:workloadRan -or $failure -notlike '*External connection still succeeds under isolation*' -or (Snapshot-MockFirewall) -cne $before){throw 'Leaking isolation was accepted'}
 }
 Check 'unprovable offline state changes nothing' {
  New-MockFirewall;$before=Snapshot-MockFirewall;$script:fwOnline=$false
  Refuse {Invoke-StableFirewallOffline {throw 'Must not run'}}
  New-MockFirewall;$script:fwAgents=@('only-one')
  Refuse {Invoke-StableFirewallOffline {throw 'Must not run'}}
  if((Snapshot-MockFirewall) -cne $before -or @($script:fwRules.Values|Where-Object {$_.Group}).Count -ne 0){throw 'Refusal changed firewall state'}
 }
 Check 'partial rule disable is restored with its cause' {
  New-MockFirewall;$before=Snapshot-MockFirewall;$script:fwDisableFail=$true
  $failure=FirewallFailure {throw 'Must not run'}
  if($failure -notlike '*Synthetic partial disable failure*' -or (Snapshot-MockFirewall) -cne $before){throw 'Partial disable recovery lost scope/cause'}
 }
 Check 'an allow rule that stays enabled refuses isolation' {
  New-MockFirewall;$script:fwStickyRule='ssh-in'
  $failure=FirewallFailure {throw 'Must not run'}
  if($failure -notlike '*Allow rules other than the runner agent, hosted infrastructure and DNS remain enabled*'){throw 'Unclean isolation accepted'}
 }
 Check 'restore failures are reported without replacing a pass into success' {
  New-MockFirewall;$script:fwEnableFail='core-dns'
  $failure=FirewallFailure {return @{actual='workload'}}
  $diag=$failure.Substring('Hosted offline firewall control failed: '.Length)|ConvertFrom-Json
  if($null -ne $diag.primary -or $diag.restoration.rules_enabled -notlike '*not*enabled*' -or $script:fwRules['edge-mdns'].Enabled -ne $true){throw 'Restore failure hidden or other rules left disabled'}
  New-MockFirewall;$script:fwNoReturn=$true
  $failure=FirewallFailure {return @{actual='workload'}} 1
  $diag=$failure.Substring('Hosted offline firewall control failed: '.Length)|ConvertFrom-Json
  if($diag.restoration.connectivity -notlike 'external connection did not return*'){throw 'Unrestored connectivity accepted'}
 }
 Check 'cleanup lifts default-deny before removing the runner rules' {
  New-MockFirewall;Invoke-StableFirewallOffline {return @{}}|Out-Null
  if(($script:fwCalls -join ',') -cne 'restore-profile,restore-profile,restore-profile,enable-rules,remove-group'){throw ('Unsafe cleanup order: '+($script:fwCalls -join ','))}
 }
 Check 'failed profile restore keeps the runner rules and reports both causes' {
  New-MockFirewall;$script:fwRestoreProfileFail=$true
  $failure=FirewallFailure {throw 'Synthetic native failure'} 1
  $diag=$failure.Substring('Hosted offline firewall control failed: '.Length)|ConvertFrom-Json
  if($diag.primary.error -cne 'Synthetic native failure' -or $diag.restoration.profiles -notlike '*Synthetic profile restore failure*' -or $diag.restoration.rules_removed -cne 'kept because profiles were not restored' -or @($script:fwRules.Values|Where-Object {$_.Group -and $_.Program -like '*Runner.*'}).Count -ne 2){throw 'Runner rules removed while still default-deny'}
 }
 Check 'failed first rule creation reports only its own cause' {
  New-MockFirewall;$before=Snapshot-MockFirewall;$script:fwCreateFail=$true
  $failure=FirewallFailure {throw 'Must not run'}
  $diag=$failure.Substring('Hosted offline firewall control failed: '.Length)|ConvertFrom-Json
  if($diag.primary.error -cne 'Synthetic rule creation failure' -or $diag.restoration.rules_removed -cne 'ok' -or $diag.restoration.verification -cne 'ok' -or (Snapshot-MockFirewall) -cne $before){throw 'Empty group removal hid the cause'}
 }
 Check 'effective-store allow rules that admit the workload refuse isolation' {
  foreach($item in @(@{program='Any';service='Any'},@{program='';service=''},@{program='C:\Program Files (x86)\Microsoft\EdgeWebView\Application\1\msedgewebview2.exe';service='Any'},@{program='C:\a\node.exe';service='Any'})){
   New-MockFirewall;$before=Snapshot-MockFirewall;$script:fwActive=@([ordered]@{program=$item.program;service=$item.service});$script:workloadRan=$false
   $failure=FirewallFailure {$script:workloadRan=$true;return @{}}
   if($script:workloadRan -or $failure -notlike '*admits the offline workload processes*' -or (Snapshot-MockFirewall) -cne $before){throw ('Admitting rule accepted: '+$item.program)}
  }
  New-MockFirewall;$script:fwActive=@([ordered]@{program='Any';service='Dnscache'},[ordered]@{program='%SystemRoot%\system32\svchost.exe';service='Any'})
  $result=Invoke-StableFirewallOffline {return @{}}
  if($result.diagnostics.effective_outbound_allow.count -ne 2){throw 'Effective inventory not recorded'}
 }
 Check 'offline validator refuses mixed shapes parsed from JSON' {
  $base=[ordered]@{schema_version=1;id='HOSTED-OFFLINE';method='hosted_windows';status='pass';version='1.0.0';source_commit=$identity.source_commit;production_sha256=$identity.production_sha256;installer_sha256=$identity.installer_sha256;target=$identity.target;operator='Synthetic';environment='Synthetic';recorded_at='2026-10-23T00:00:00Z';completed=$true}
  function ParseCase($Extra){$c=[ordered]@{};foreach($k in $base.Keys){$c[$k]=$base[$k]};foreach($k in $Extra.Keys){$c[$k]=$Extra[$k]};return ConvertFrom-HostedJsonText ($c|ConvertTo-Json -Depth 10)}
  function AssertCase($Case){Assert-HostedCase $Case 'HOSTED-OFFLINE' '1.0.0' $identity.source_commit $identity.production_sha256 $identity.installer_sha256 $identity.target}
  AssertCase (ParseCase @{network_disabled=$true})
  AssertCase (ParseCase @{network_isolation='windows_firewall_runner_and_host_agents_only';network_isolated=$true;external_connection_refused=$true;network_restored=$true;allowed_programs=@('a','b');allowed_infrastructure_programs=@('c');dns_allowed=$true})
  Refuse {AssertCase (ParseCase @{network_disabled=$true;network_isolated=$true;external_connection_refused=$true})}
  Refuse {AssertCase (ParseCase @{network_disabled=$true;dns_allowed=$true})}
  Refuse {AssertCase (ParseCase @{network_disabled=$true;allowed_infrastructure_programs=@('c')})}
  Refuse {AssertCase (ParseCase @{network_isolation='windows_firewall_runner_and_host_agents_only';network_isolated=$true;external_connection_refused=$true;network_restored='true'})}
 }
 Check 'offline case accepts exactly one offline evidence shape' {
  $firewall=@{network_isolation='windows_firewall_runner_and_host_agents_only';network_isolated=$true;external_connection_refused=$true;network_restored=$true;allowed_programs=@('a','b');dns_allowed=$true}
  Invoke-StableCase 'HOSTED-OFFLINE' $identity $root {return $firewall.Clone()}
  Invoke-StableCase 'HOSTED-OFFLINE' $identity $root {return @{network_disabled=$true;network_restored=$true}}
  $mixed=$firewall.Clone();$mixed.network_disabled=$true;Refuse {Invoke-StableCase 'HOSTED-OFFLINE' $identity $root {return $mixed}}
  foreach($field in @('network_isolated','external_connection_refused','network_restored')){$bad=$firewall.Clone();$bad[$field]=$false;Refuse {Invoke-StableCase 'HOSTED-OFFLINE' $identity $root {return $bad}}}
  $bad=$firewall.Clone();$bad.network_isolation='adapter';Refuse {Invoke-StableCase 'HOSTED-OFFLINE' $identity $root {return $bad}}
 }
 Check 'firewall controls refuse non-hosted environments' {New-MockFirewall;$env:RUNNER_ENVIRONMENT='self-hosted';try{Refuse {Invoke-StableFirewallOffline {throw 'Should not run'}}}finally{$env:RUNNER_ENVIRONMENT='github-hosted'}}
 Check 'offline controls refuse non-hosted environments' {$env:RUNNER_ENVIRONMENT='self-hosted';Refuse {Invoke-StableOffline {throw 'Should not run'}}}
 Check 'application data resolves the known folder and ignores an APPDATA override' {
  $saved=$env:APPDATA;$override=Join-Path $root 'override-appdata'
  try{$env:APPDATA=$override;$resolved=Get-StableAppDataDirectory 'io.github.laymer.azeroth-navigator'}finally{$env:APPDATA=$saved}
  $expected=Join-Path ([Environment]::GetFolderPath([Environment+SpecialFolder]::ApplicationData,[Environment+SpecialFolderOption]::DoNotVerify)) 'io.github.laymer.azeroth-navigator'
  if($resolved -cne $expected -or $resolved.StartsWith($override)){throw 'Application data followed the environment override'}
 }
 Check 'invalid application identifiers are refused' {foreach($bad in @('','..','a/b','io.github..x','IO.Github.x')){Refuse {Get-StableAppDataDirectory $bad}}}
 Check 'application data is moved aside intact and never deleted' {
  $data=Join-Path $root 'appdata-source';$aside=Join-Path $root 'appdata-aside'
  New-Item -ItemType Directory -Path (Join-Path $data 'nested')|Out-Null;[IO.File]::WriteAllText((Join-Path $data 'nested/snapshots.sqlite3'),'synthetic')
  if((Move-StableAppDataAside $data $aside) -ne $true -or (Test-Path -LiteralPath $data)){throw 'Data was not moved aside'}
  if([IO.File]::ReadAllText((Join-Path $aside 'nested/snapshots.sqlite3')) -cne 'synthetic'){throw 'Moved data differs'}
 }
 Check 'absent application data reports nothing moved' {if((Move-StableAppDataAside (Join-Path $root 'appdata-absent') (Join-Path $root 'appdata-absent-aside')) -ne $false -or (Test-Path -LiteralPath (Join-Path $root 'appdata-absent-aside'))){throw 'Absent data was reported or created'}}
 Check 'an existing destination or a file refuses moving and keeps the data' {
  $data=Join-Path $root 'appdata-kept';$taken=Join-Path $root 'appdata-taken';$file=Join-Path $root 'appdata-file'
  New-Item -ItemType Directory -Path $data,$taken|Out-Null;[IO.File]::WriteAllText($file,'synthetic')
  Refuse {Move-StableAppDataAside $data $taken};if(-not (Test-Path -LiteralPath $data -PathType Container)){throw 'Refused move lost the data'}
  Refuse {Move-StableAppDataAside $file (Join-Path $root 'appdata-file-aside')};if(-not (Test-Path -LiteralPath $file -PathType Leaf)){throw 'Refused move lost the file'}
 }
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
