# Real wrapper/marker control flow with inert native operation stubs only.
Set-StrictMode -Version 3.0
$ErrorActionPreference='Stop'
$root=Join-Path ([IO.Path]::GetTempPath()) ('nav-recovery-owner-'+[Guid]::NewGuid().ToString('N'))
$null=New-Item -ItemType Directory -Path $root
$previous=@{};foreach($name in @('GITHUB_ACTIONS','RUNNER_ENVIRONMENT','OS','GITHUB_SHA','LOCALAPPDATA','PROBE_TEST_MODE','PROBE_TEST_LOG')){$previous[$name]=[Environment]::GetEnvironmentVariable($name)}
$script:count=0
function Check($Name,[scriptblock]$Action){&$Action;$script:count++;Write-Host "ok $Name"}
function Refuse([scriptblock]$Action){$failed=$false;try{&$Action|Out-Null}catch{$failed=$true};if(-not $failed){throw 'Expected refusal'}}
try {
 foreach($file in @('Recovery-Probe-Install.ps1','Recovery-Probe-Ownership.ps1')){Copy-Item -LiteralPath (Join-Path $PSScriptRoot $file) -Destination (Join-Path $root $file)}
 $stub=@'
$script:NativeSmokeAppId='io.github.laymer.azeroth-navigator'
function Get-InstalledDisplayVersion {if($env:PROBE_TEST_MODE -ceq 'registered'){return 'existing'};return $null}
function Get-Process {if($env:PROBE_TEST_MODE -ceq 'portable'){return [pscustomobject]@{Name='inert portable'}}}
function Get-CimInstance {if($env:PROBE_TEST_MODE -ceq 'web'){return [pscustomobject]@{CommandLine=$script:NativeSmokeAppId}}}
function Get-AuthenticodeSignature {return [pscustomobject]@{Status='NotSigned'}}
function Install-NativeSmokeSetup {Add-Content -LiteralPath $env:PROBE_TEST_LOG -Value 'install';throw 'Synthetic partial install; never executes an installer'}
function Stop-NativeSmokeProcesses {Add-Content -LiteralPath $env:PROBE_TEST_LOG -Value 'stop'}
function Uninstall-NativeSmokeApp {Add-Content -LiteralPath $env:PROBE_TEST_LOG -Value 'uninstall'}
function Wait-NativeSmokeUninstalled {Add-Content -LiteralPath $env:PROBE_TEST_LOG -Value 'wait';return $true}
'@
 [IO.File]::WriteAllText((Join-Path $root 'Native-Smoke-Helpers.ps1'),$stub)
 . (Join-Path $root 'Recovery-Probe-Ownership.ps1')
 $env:GITHUB_ACTIONS='true';$env:RUNNER_ENVIRONMENT='github-hosted';$env:OS='Windows_NT';$env:GITHUB_SHA='a'*40;$env:LOCALAPPDATA=$root
 $env:PROBE_TEST_LOG=Join-Path $root 'operations.txt';$marker=Join-Path $root 'owner.json';$nonce='1'*32
 $installer=Join-Path $root 'inert.exe';[IO.File]::WriteAllText($installer,'inert data, never executable');$hash=(Get-FileHash $installer -Algorithm SHA256).Hash.ToLowerInvariant()
 $wrapper=Join-Path $root 'Recovery-Probe-Install.ps1';$install=Join-Path $root 'Azeroth Navigator'
 function InvokeWrapper([switch]$Cleanup){& $wrapper -Installer $installer -ExpectedSha256 $hash -OwnershipMarker $marker -OwnershipNonce $nonce -Cleanup:$Cleanup}
 function Owned {Test-RecoveryProbeOwnership $marker $nonce $env:GITHUB_SHA $hash $install}
 Check 'pre-existing directory remains unowned and cleanup refuses' {
  $null=New-Item -ItemType Directory -Path $install
  Refuse {InvokeWrapper};Refuse {InvokeWrapper -Cleanup}
  if(Test-Path $marker){throw 'Issued before fresh-directory guard'}
  if(Test-Path $env:PROBE_TEST_LOG){throw 'Touched an unowned application'}
  Remove-Item -LiteralPath $install
 }
 foreach($mode in @('registered','portable','web')) {
  Check "pre-existing $mode state cannot issue ownership or trigger cleanup" {
   $env:PROBE_TEST_MODE=$mode;Refuse {InvokeWrapper};Refuse {InvokeWrapper -Cleanup}
   if((Test-Path $marker) -or (Test-Path $env:PROBE_TEST_LOG)){throw 'Mutated unowned state'}
  }
 }
 $env:PROBE_TEST_MODE='partial'
 Check 'partial installation receives marker and only then permits owned cleanup' {
  Refuse {InvokeWrapper}
  if(-not (Owned)){throw 'Owned partial installation lost its cleanup marker'}
  InvokeWrapper -Cleanup
  $operations=@(Get-Content $env:PROBE_TEST_LOG)
  if(($operations -join ',') -cne 'install,stop,uninstall,wait'){throw 'Wrong ownership ordering'}
 }
 Check 'marker refuses replay tampering oversized bytes and overwrite' {
  Refuse {New-RecoveryProbeOwnership $marker $nonce $env:GITHUB_SHA $hash $install}
  if(Test-RecoveryProbeOwnership $marker ('2'*32) $env:GITHUB_SHA $hash $install){throw 'Accepted foreign nonce'}
  if(Test-RecoveryProbeOwnership $marker $nonce ('b'*40) $hash $install){throw 'Accepted foreign source'}
  [IO.File]::WriteAllText($marker,('x'*4097));if(Owned){throw 'Accepted oversized marker'}
  [IO.File]::WriteAllText($marker,'{}');if(Owned){throw 'Accepted partial marker'}
 }
 Write-Host "Passed $script:count inert ownership checks; no real installer or process control executed."
} finally {
 foreach($name in $previous.Keys){[Environment]::SetEnvironmentVariable($name,$previous[$name])}
 Remove-Item -LiteralPath $root -Recurse -Force
}
