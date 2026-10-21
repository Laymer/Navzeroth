# Pure mocks only. No real adapter cmdlet is ever invoked by these tests.
Set-StrictMode -Version 3.0
$ErrorActionPreference='Stop'
. (Join-Path $PSScriptRoot 'Probe-HostedOfflineAdapters.ps1')
$names=@('GITHUB_ACTIONS','RUNNER_ENVIRONMENT','OS','GITHUB_SHA','GITHUB_RUN_ID','GITHUB_RUN_ATTEMPT','RUNNER_TEMP')
$saved=@{};foreach($name in $names){$saved[$name]=[Environment]::GetEnvironmentVariable($name)}
$temp=Join-Path ([IO.Path]::GetTempPath()) ('offline-probe-tests-'+[Guid]::NewGuid().ToString('N'))
$null=New-Item -ItemType Directory -Path $temp
$script:guidA='11111111-1111-1111-1111-111111111111';$script:guidB='22222222-2222-2222-2222-222222222222'
$script:mode='pass';$script:up=$true;$script:calls=0;$count=0
function Get-NetAdapter {[CmdletBinding()]param([switch]$IncludeHidden) @([pscustomobject]@{InterfaceGuid=$script:guidA;Status=$(if($script:up){'Up'}else{'Disabled'})},[pscustomobject]@{InterfaceGuid=$script:guidB;Status='Disabled'})}
function Invoke-StableOffline([scriptblock]$Workload){
 $script:calls++;$script:up=$false
 try{
  if($script:mode -eq 'primary'){throw 'Original controlled failure'}
  $result=&$Workload
 }finally{if($script:mode -ne 'unrestored'){$script:up=$true}}
 $ids=@($script:guidA);if($script:mode -eq 'duplicate'){$ids=@($script:guidA,$script:guidA)}
 if($script:mode -eq 'wrong-guid'){$ids=@($script:guidB)}
 return @{network_disabled=$(if($script:mode -eq 'string-flag'){'true'}else{$true});restored=$true;adapters=$ids;workload=$result}
}
function Check([string]$Name,[scriptblock]$Action){&$Action;$script:count++;Write-Host "ok $Name"}
function Refuse([scriptblock]$Action){$caught=$null;try{&$Action|Out-Null}catch{$caught=$_};if($null -eq $caught){throw 'Expected probe refusal'};return $caught}
function Run-Probe([string]$Name){Invoke-HostedOfflineAdapterProbe (Join-Path $temp $Name)}
try{
 $env:GITHUB_ACTIONS='true';$env:RUNNER_ENVIRONMENT='github-hosted';$env:OS='Windows_NT'
 $env:GITHUB_SHA=(&git -C (Join-Path $PSScriptRoot '../..') rev-parse HEAD).Trim()
 $env:GITHUB_RUN_ID='123';$env:GITHUB_RUN_ATTEMPT='1';$env:RUNNER_TEMP=$temp
 Check 'mocked real boundary requires zero-Up marker and exact post-restore GUID' {
  $r=Run-Probe 'success';if(-not $r.passed -or -not $r.network_disabled -or -not $r.restored -or $r.acceptance_observations_promoted -or $r.controlled_adapters.Count -ne 1){throw 'Bad probe result'}
  $written=Get-Content -Raw (Join-Path $temp 'success/probe.json')|ConvertFrom-Json
  if(-not $written.passed -or $written.source_commit -cne $env:GITHUB_SHA -or $written.kind -cne 'hosted_offline_adapter_engineering_probe'){throw 'Bad report binding'}
 }
 foreach($mode in @('unrestored','duplicate','wrong-guid','string-flag')){
  $script:mode=$mode;$script:up=$true
  Check "strict actual-result refusal $mode" {Refuse {Run-Probe $script:mode}|Out-Null;$d=Get-Content -Raw (Join-Path $temp ($script:mode+'/probe.json'))|ConvertFrom-Json;if($d.passed -or -not $d.error){throw 'Failure promoted'}}
 }
 $script:mode='primary';$script:up=$true
 Check 'original helper failure is retained and never promoted' {$e=Refuse {Run-Probe 'primary'};if($e.Exception.Message -cne 'Original controlled failure'){throw 'Primary failure hidden'}}
 $script:mode='pass';$script:up=$true
 Check 'non-hosted environment refuses before any control call' {$env:RUNNER_ENVIRONMENT='self-hosted';$before=$script:calls;Refuse {Run-Probe 'non-hosted'}|Out-Null;if($script:calls -ne $before){throw 'Control called'};$env:RUNNER_ENVIRONMENT='github-hosted'}
 Check 'wrong issued source refuses before any control call' {$old=$env:GITHUB_SHA;$env:GITHUB_SHA='f'*40;$before=$script:calls;Refuse {Run-Probe 'wrong-source'}|Out-Null;if($script:calls -ne $before){throw 'Control called'};$env:GITHUB_SHA=$old}
 Check 'existing output refuses instead of overwriting evidence' {Refuse {Run-Probe 'success'}|Out-Null}
 Check 'no originally-Up adapter refuses before controls' {$script:up=$false;$before=$script:calls;Refuse {Run-Probe 'none'}|Out-Null;if($script:calls -ne $before){throw 'Control called'};$script:up=$true}
 $originalWriter=${function:Write-OfflineProbeReport}
 function Write-OfflineProbeReport {param($Report,$Path) throw 'Synthetic output failure'}
 $script:mode='primary'
 Check 'report-write failure cannot mask original network-control error' {$e=Refuse {Run-Probe 'write-error'};if($e.Exception.Message -cne 'Original controlled failure'){throw 'Original failure masked'}}
 Set-Item Function:Write-OfflineProbeReport $originalWriter
 Write-Host "PASS $count mocked probe checks; no real adapter controls"
}finally{
 foreach($name in $names){[Environment]::SetEnvironmentVariable($name,$saved[$name])}
 if(Test-Path -LiteralPath $temp){Remove-Item -LiteralPath $temp -Recurse -Force}
}
