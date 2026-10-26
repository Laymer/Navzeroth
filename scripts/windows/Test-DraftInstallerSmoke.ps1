# Synthetic producer and process-boundary regressions only; never installs an app.
Set-StrictMode -Version 3.0
$ErrorActionPreference='Stop'
. (Join-Path $PSScriptRoot 'Release-Materials.ps1')
. (Join-Path $PSScriptRoot 'Stable-Acceptance-Helpers.ps1')
. (Join-Path $PSScriptRoot 'Draft-Installer-Smoke.ps1')
$root=Join-Path ([IO.Path]::GetTempPath()) ('anav-draft-tests-'+[Guid]::NewGuid().ToString('N'))
$script:count=0
function Check($Name,[scriptblock]$Action) { &$Action; $script:count++; Write-Host "ok $Name" }
function Refuse([scriptblock]$Action) { $failed=$false;try{&$Action|Out-Null}catch{$failed=$true};if(-not $failed){throw 'Expected refusal'} }
function WriteJson($Path,$Value) { [IO.File]::WriteAllText($Path,(($Value|ConvertTo-Json -Depth 30)+"`n"),(New-Object Text.UTF8Encoding($false))) }
function Clone($Value) { return ($Value|ConvertTo-Json -Depth 30|ConvertFrom-Json) }
$source='a'*40;$hash='b'*64
try {
 $view=Clone @{databaseId=11;tagName='v1.0.0';isDraft=$true;isPrerelease=$false}
 $script:apiReply=Clone @{id=11;tag_name='v1.0.0';draft=$true;prerelease=$false;assets=@()}
 $script:apiCalls=@();$script:apiExit=0;$script:malformedApi=$false
 function gh {
  param($Command,$Endpoint)
  $script:apiCalls+= $Endpoint
  # This models the observed provider boundary: a draft's by-tag request is404.
  $global:LASTEXITCODE=if($Endpoint -match '/tags/') {1} else {$script:apiExit}
  if($Command -cne 'api'){throw 'Unexpected GitHub command'}
  if($script:malformedApi){return 'not JSON'}
  return ($script:apiReply|ConvertTo-Json -Depth 10)
 }
 try {
  Check 'draft lookup uses issued numeric ID despite unpublished by-tag404' {
   $actual=Get-DraftReleaseApi 'Laymer/Navzeroth' 'v1.0.0' $view
   if($actual.id -ne 11 -or $script:apiCalls.Count -ne 1 -or $script:apiCalls[0] -cne 'repos/Laymer/Navzeroth/releases/11'){throw 'Draft lookup did not preserve its issued ID'}
  }
  Check '64-bit release IDs are preserved without truncation' {
   $changed=Clone $view;$changed.databaseId=[long]2147483648
   $saved=Clone $script:apiReply;$script:apiReply.id=[long]2147483648
   try {
    $actual=Get-DraftReleaseApi 'Laymer/Navzeroth' 'v1.0.0' $changed
    if($actual.id -ne [long]2147483648 -or $script:apiCalls[-1] -cne 'repos/Laymer/Navzeroth/releases/2147483648'){throw 'Release ID was truncated'}
   } finally {$script:apiReply=$saved}
  }
  Check 'malformed repository and mismatched release view refuse before network' {
   $before=$script:apiCalls.Count
   foreach($repo in @('other/../Navzeroth','https://elsewhere.invalid/repo','Laymer/Navzeroth?x=1')) { Refuse {Get-DraftReleaseApi $repo 'v1.0.0' $view} }
   foreach($bad in @($null,0,-1,'11',$true,11.5)) { $changed=Clone $view;$changed.databaseId=$bad;Refuse {Get-DraftReleaseApi 'Laymer/Navzeroth' 'v1.0.0' $changed} }
   foreach($field in @('tagName','isDraft','isPrerelease')) { $changed=Clone $view;$changed.$field='wrong';Refuse {Get-DraftReleaseApi 'Laymer/Navzeroth' 'v1.0.0' $changed} }
   if($script:apiCalls.Count -ne $before){throw 'Invalid view reached the network'}
  }
  Check 'REST ID tag and exact draft state must match the initial view' {
   $saved=Clone $script:apiReply
   foreach($field in @('id','tag_name','draft','prerelease')) {
    $script:apiReply=Clone $saved;$script:apiReply.$field='wrong';Refuse {Get-DraftReleaseApi 'Laymer/Navzeroth' 'v1.0.0' $view}
   }
   $script:apiReply=Clone $saved;$script:apiReply.id=12;Refuse {Get-DraftReleaseApi 'Laymer/Navzeroth' 'v1.0.0' $view}
   $script:apiReply=Clone $saved;$script:apiReply.draft=$false;Refuse {Get-DraftReleaseApi 'Laymer/Navzeroth' 'v1.0.0' $view}
   $script:apiReply=Clone $saved;$script:apiReply.prerelease=$true;Refuse {Get-DraftReleaseApi 'Laymer/Navzeroth' 'v1.0.0' $view}
   $script:apiReply=$saved
  }
  Check 'failed or malformed REST response cannot create draft identity' {
   $script:apiExit=1;Refuse {Get-DraftReleaseApi 'Laymer/Navzeroth' 'v1.0.0' $view};$script:apiExit=0
   $script:malformedApi=$true;Refuse {Get-DraftReleaseApi 'Laymer/Navzeroth' 'v1.0.0' $view};$script:malformedApi=$false
  }
 } finally {Remove-Item Function:\gh}
 $null=New-Item -ItemType Directory -Path (Join-Path $root 'release/v1'),(Join-Path $root '.github/workflows'),(Join-Path $root 'dist/draft-evidence'),(Join-Path $root 'scripts/windows') -Force
 $receiptDir=Join-Path $root 'dist/draft-evidence'
 $downloadPath=Join-Path $receiptDir 'verification.json';$nativePath=Join-Path $receiptDir 'native-result.json'
 [IO.File]::WriteAllText((Join-Path $root 'release/v1/bundled-content-build-identity.json'),(@{schema_version=1;app_version='1.0.0';production_sha256=('c'*64)}|ConvertTo-Json -Compress),(New-Object Text.UTF8Encoding($false)))
 [IO.File]::WriteAllText((Join-Path $root '.github/workflows/verify-release-downloads.yml'),'synthetic workflow')
 $download=Clone @{schema_version=3;version='1.0.0';phase='draft';source_commit=$source;installer_signature='NotSigned';application_signature='NotSigned';dependency_closure_verified=$true;addon_zip_matches_source=$true;workflow_run_id='13';workflow_run_attempt='2';assets=@(@{name='setup.exe';bytes=50;sha256=$hash})}
 $native=Clone @{schema_version=1;phase='draft';source_commit=$source;installer_sha256=$hash;completed=$true;passed=$true;human_performed=$false;ui_interaction_verified=$true;checks=@{first_start=$true;bundled_activation=$true;source_backed_planning=$true;restart_persistence=$true}}
 $release=Clone @{id=11;tag_name='v1.0.0';draft=$true;prerelease=$false;assets=@(@{id=12;name='setup.exe';size=50})}
 $job=Clone @{id=14;run_id=13;run_attempt=2;head_sha=$source;name='Verify downloaded unsigned release'}
 function Result { New-DraftDownloadResult $root $receiptDir $release $job $source $hash 13 2 }
 WriteJson $downloadPath $download;WriteJson $nativePath $native
 Check 'actual raw producer pins exact bytes without claiming final workflow or review' {
  $r=Result
  if($r.identity.source_commit -cne $source -or $r.identity.asset_id -ne 12 -or $r.identity.run_attempt -ne 2 -or $r.human_performed -or $r.native_result.path -cne 'dist/draft-evidence/native-result.json' -or $r.native_result.sha256 -cne (Get-FileHash $nativePath).Hash.ToLowerInvariant() -or $r.Contains('provenance') -or $r.Contains('independent_review')) { throw 'Identity or authority differs' }
 }
 foreach($field in @('completed','passed','ui_interaction_verified','human_performed')) {
  Check "native $field must retain the exact truth value" {
   foreach($bad in @('true',$null,1,(-not $native.$field))) { $changed=Clone $native;$changed.$field=$bad;WriteJson $nativePath $changed;Refuse {Result} }
   WriteJson $nativePath $native
  }
 }
 foreach($field in @('first_start','bundled_activation','source_backed_planning','restart_persistence')) {
  Check "missing or false $field refuses raw success" {
   $changed=Clone $native;$changed.checks.$field=$false;WriteJson $nativePath $changed;Refuse {Result}
   $changed.checks.PSObject.Properties.Remove($field);WriteJson $nativePath $changed;Refuse {Result};WriteJson $nativePath $native
  }
 }
 Check 'wrong source installer phase and native version refuse' {
  foreach($field in @('source_commit','installer_sha256','phase','schema_version')) { $changed=Clone $native;$changed.$field='wrong';WriteJson $nativePath $changed;Refuse {Result} };WriteJson $nativePath $native
 }
 Check 'download source signatures run attempt and verification remain bound' {
  foreach($field in @('source_commit','installer_signature','application_signature','phase','workflow_run_id','workflow_run_attempt','dependency_closure_verified','addon_zip_matches_source')) { $changed=Clone $download;$changed.$field='wrong';WriteJson $downloadPath $changed;Refuse {Result} };WriteJson $downloadPath $download
 }
 Check 'missing duplicate and changed installer refuse' {
  foreach($kind in @('missing','duplicate','hash','size')) {
   $changed=Clone $download
   switch($kind) {'missing'{$changed.assets=@()} 'duplicate'{$changed.assets+=Clone $changed.assets[0]} 'hash'{$changed.assets[0].sha256='f'*64} 'size'{$changed.assets[0].bytes=49}}
   WriteJson $downloadPath $changed;Refuse {Result}
  };WriteJson $downloadPath $download
 }
 Check 'public prerelease or ambiguous release cannot create draft proof' {
  $saved=Clone $release
  foreach($kind in @('public','prerelease','duplicate','assetid','tag')) {
   $release=Clone $saved
   switch($kind) {'public'{$release.draft=$false} 'prerelease'{$release.prerelease=$true} 'duplicate'{$release.assets+=Clone $release.assets[0]} 'assetid'{$release.assets[0].id=0} 'tag'{$release.tag_name='v0.8.1'}}
   Refuse {Result}
  };$release=$saved
 }
 Check 'wrong job run attempt source and name refuse' {
  $saved=Clone $job
  foreach($field in @('id','run_id','run_attempt','head_sha','name')) { $job=Clone $saved;$job.$field=0;Refuse {Result} };$job=$saved
 }
 Check 'evidence file refs enforce boundaries and byte limits' {
  Refuse {Get-DraftEvidenceRef (Join-Path $root 'other') $nativePath}
  Refuse {Get-DraftEvidenceRef $root $nativePath 1}
  $empty=Join-Path $root 'empty';[IO.File]::WriteAllText($empty,'');Refuse {Get-DraftEvidenceRef $root $empty}
 }
 Check 'producer applies the one MiB role bound before decoding' {
  [IO.File]::WriteAllText($nativePath,(' '*1048577));Refuse {Result};WriteJson $nativePath $native
 }
 $fresh=Clone $native;$fresh|Add-Member mode 'fresh';$restart=Clone $native;$restart|Add-Member mode 'restart'
 Check 'fresh and restart require separate successful raw modes' { Assert-DraftUiChecks $fresh $restart;$bad=Clone $restart;$bad.mode='fresh';Refuse {Assert-DraftUiChecks $fresh $bad};$bad=Clone $restart;$bad.checks.restart_persistence=$false;Refuse {Assert-DraftUiChecks $fresh $bad} }
 Check 'UI orchestrator refuses ordinary machines before any mutation' {
  $saved=$env:RUNNER_ENVIRONMENT
  try{$env:RUNNER_ENVIRONMENT='self-hosted';Refuse {Invoke-DownloadedDraftUiSmoke $root 'none' 'none' '1.0.0' $hash (Join-Path $root 'must-not-exist')};if(Test-Path (Join-Path $root 'must-not-exist')){throw 'Mutated local environment'}}finally{$env:RUNNER_ENVIRONMENT=$saved}
 }
 if($PSVersionTable.PSVersion.Major -ge 7) {
  $child=@'
import {writeFileSync} from 'node:fs';
const args=process.argv.slice(2),arg=k=>args[args.indexOf(k)+1];
const result={pid:process.pid,passed:true,completed:true,version:arg('--version'),mode:arg('--mode'),phase:'draft',source_commit:arg('--source-commit'),installer_sha256:arg('--installer-sha256'),human_performed:false,ui_interaction_verified:true};
if(arg('--exe')==='wrong-source')result.source_commit='f'.repeat(40);
if(arg('--exe')==='incomplete')result.completed=false;
writeFileSync(arg('--out'),JSON.stringify(result));
'@
  [IO.File]::WriteAllText((Join-Path $root 'scripts/windows/Draft-Installer-Workload.mjs'),$child,(New-Object Text.UTF8Encoding($false)))
  $report=Join-Path $root 'child.json'
  Check 'actual child argument transport and typed raw identity roundtrip' {
   $actual=Invoke-StableWorkload -Root $root -Exe 'synthetic no native app' -Manifest 'synthetic path with spaces' -Version '1.0.0' -Mode 'fresh' -State 'synthetic state' -Report $report -MaximumSeconds 10 -Driver 'draft-ui' -SourceCommit $source -InstallerSha256 $hash
   if($actual.raw.source_commit -cne $source -or $actual.raw.installer_sha256 -cne $hash){throw 'Arguments lost'}
  }
  foreach($exe in @('wrong-source','incomplete')) {Check "actual child $exe report cannot pass" {Refuse {Invoke-StableWorkload -Root $root -Exe $exe -Manifest 'none' -Version '1.0.0' -Mode 'restart' -State 'none' -Report $report -MaximumSeconds 10 -Driver 'draft-ui' -SourceCommit $source -InstallerSha256 $hash}}}
  Check 'unsupported draft mode and malformed expected source refuse before launch' {Refuse {Invoke-StableWorkload -Driver 'draft-ui' -Mode 'recover' -SourceCommit $source -InstallerSha256 $hash};Refuse {Invoke-StableWorkload -Driver 'draft-ui' -Mode 'fresh' -SourceCommit 'short' -InstallerSha256 $hash}}
 } else {Write-Host 'PS7 process ArgumentList checks not executed in PS5.1.'}
 if($PSVersionTable.PSVersion.Major -ge 7) {
  $savedActions=$env:GITHUB_ACTIONS;$savedRunner=$env:RUNNER_ENVIRONMENT;$savedOS=$env:OS
  try {
   $env:GITHUB_ACTIONS='true';$env:RUNNER_ENVIRONMENT='github-hosted';$env:OS='Windows_NT'
   function git { $global:LASTEXITCODE=0;return ('a'*40) }
   $script:modes=@();$script:failRestart=$false
   function Invoke-StableWorkload { param($Root,$Exe,$Manifest,$Version,$Mode,$State,$Report,$MaximumSeconds,$Driver,$SourceCommit,$InstallerSha256)
    if($Driver -cne 'draft-ui' -or $MaximumSeconds -gt 1200 -or $MaximumSeconds -lt 1 -or $SourceCommit -cne ('a'*40) -or $InstallerSha256 -cne ('b'*64)){throw 'Orchestrator lost identity or bounds'}
    $script:modes+= $Mode
    $result=Clone $native;$result|Add-Member mode $Mode
    if($Mode -ceq 'restart' -and $script:failRestart){$result.checks.restart_persistence=$false}
    WriteJson $Report $result
    $screens=Join-Path ([IO.Path]::GetDirectoryName($Report)) ($Mode+'.screenshots')
    $null=New-Item -ItemType Directory -Path $screens
    [IO.File]::WriteAllText((Join-Path $screens 'screen.png'),'synthetic screenshot bytes')
    [IO.File]::WriteAllText($State,'synthetic private state must be excluded')
    return @{raw=$result}
   }
   Check 'orchestrator executes both modes and pins actual reports excluding state' {
    $result=Invoke-DownloadedDraftUiSmoke $root 'synthetic' 'synthetic' '1.0.0' $hash (Join-Path $root 'native-success')
    if(($script:modes -join ',') -cne 'fresh,restart' -or $result.detail_files.Count -ne 4 -or @($result.detail_files|Where-Object path -match 'state').Count -ne 0 -or @($result.detail_files|Where-Object path -match 'screenshots/screen.png$').Count -ne 2 -or -not $result.checks.restart_persistence){throw 'Incomplete native orchestration'}
   }
   Check 'orchestrator refuses failed restart and existing or outside output' {
    $script:failRestart=$true;Refuse {Invoke-DownloadedDraftUiSmoke $root 'synthetic' 'synthetic' '1.0.0' $hash (Join-Path $root 'native-failed')}
    Refuse {Invoke-DownloadedDraftUiSmoke $root 'synthetic' 'synthetic' '1.0.0' $hash (Join-Path $root 'native-success')}
    Refuse {Invoke-DownloadedDraftUiSmoke $root 'synthetic' 'synthetic' '1.0.0' $hash ($root+'-outside')}
   }
  } finally {$env:GITHUB_ACTIONS=$savedActions;$env:RUNNER_ENVIRONMENT=$savedRunner;$env:OS=$savedOS}
 }
 Write-Host "Passed $script:count downloaded-draft synthetic checks."
} finally {Remove-Item -LiteralPath $root -Recurse -Force}
