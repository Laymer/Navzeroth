[CmdletBinding()]
param()
Set-StrictMode -Version 3.0
$ErrorActionPreference = 'Stop'
. (Join-Path $PSScriptRoot 'Hosted-Acceptance.ps1')
$count = 0
function Check([string] $Name, [bool] $Condition) { $script:count++; if (-not $Condition) { throw "FAILED: $Name" }; Write-Host "ok $Name" }
function Refuses([string] $Name, [scriptblock] $Action) { $failed=$false; try { & $Action } catch { $failed=$true }; Check $Name $failed }
function Validate($Case) { Assert-HostedCase $Case 'HOSTED-INSTALL' $case.version $case.source_commit $case.production_sha256 $case.installer_sha256 $case.target }
function Clone($Value) { ConvertFrom-HostedJsonText ($Value | ConvertTo-Json -Depth 100) }
# Historical actual hosted installation report; no current acceptance is created.
$fixture = Join-Path $PSScriptRoot 'fixtures/hosted-install-39413135905.json'
$text = [IO.File]::ReadAllText($fixture)
$default = $text | ConvertFrom-Json
if ($PSVersionTable.PSVersion.Major -ge 6) {
    Check 'actual PS7 default parsing reproduces DateTime conversion' ($default.recorded_at -is [datetime])
    Refuses 'old date object is refused by strict metadata checks' { Validate $default }
} else { Check 'Windows PowerShell5.1 default already retains timestamp text' ($default.recorded_at -is [string]) }
$case = ConvertFrom-HostedJsonText $text
Check 'actual literal UTC timestamp survives raw-case parsing exactly' ($case.recorded_at -is [string] -and $case.recorded_at -ceq '2026-10-20T11:59:10Z')
Validate $case; Check 'actual historical operator/date/environment validates' $true
$nested = [pscustomobject]@{checks=@([pscustomobject]@{recorded_at=$case.recorded_at},[pscustomobject]@{recorded_at=$null})}
$roundtrip = Clone $nested
Check 'base evidence clone retains exact existing timestamp strings' ($roundtrip.checks[0].recorded_at -is [string] -and $roundtrip.checks[0].recorded_at -ceq $case.recorded_at -and $null -eq $roundtrip.checks[1].recorded_at)
if ($PSVersionTable.PSVersion.Major -ge 6) {
    $legacy = $text | ConvertFrom-Json
    Restore-HostedTimestampStrings $text $legacy
    Check 'older PS7 raw-token fallback preserves literal case timestamp' ($legacy.recorded_at -is [string] -and $legacy.recorded_at -ceq $case.recorded_at)
    $rawNested = $nested | ConvertTo-Json -Depth 100
    $legacy = $rawNested | ConvertFrom-Json
    Restore-HostedTimestampStrings $rawNested $legacy
    Check 'older PS7 raw-token fallback preserves nested base rows and null' ($legacy.checks[0].recorded_at -is [string] -and $legacy.checks[0].recorded_at -ceq $case.recorded_at -and $null -eq $legacy.checks[1].recorded_at)
}
foreach ($bad in @('', '2026-10-20', '2026-10-20T11:59:10', '2026-10-20T11:59:10+00:00', '2026-02-30T11:59:10Z', 123, $null)) {
    $changed=Clone $case; $changed.recorded_at=$bad
    $parsed=Clone $changed
    Refuses 'missing malformed non-UTC or nonstring timestamp remains refused' { Validate $parsed }
}
foreach ($field in @('operator','environment')) {
    foreach ($bad in @('', "bad`nmetadata", 123, $null)) {
        $changed=Clone $case; $changed.$field=$bad
        Refuses 'operator/environment metadata checks remain strict' { Validate $changed }
    }
}
$failed=Clone $case; $failed.status='fail'; $failed.completed=$false
Validate $failed
Check 'recorded failure remains a failure after parsing' ($failed.status -ceq 'fail' -and -not $failed.completed)
# Verify New-HostedAcceptanceEvidence uses the same literal-preserving clone,
# including an existing observation when no new case overwrites that row.
$rows = @($script:HostedAcceptanceIds | ForEach-Object { [pscustomobject]@{id=$_;method='hosted_windows';status='unverified';operator=$null;recorded_at=$null;environment=$null;artifacts=@()} })
$rows += [pscustomobject]@{id='OWNER-INSTALLER-SMOKE';method='interactive_windows';status='unverified';operator=$null;recorded_at=$null;environment=$null;artifacts=@()}
$base=[pscustomobject]@{schema_version=4;production_sha256=$case.production_sha256;target=$case.target;checks=$rows}
$base.checks[0].status='pass';$base.checks[0].recorded_at=$case.recorded_at;$base.checks[0].operator=$case.operator;$base.checks[0].environment=$case.environment
$result=New-HostedAcceptanceEvidence $base $case.version $case.source_commit $case.production_sha256 $case.installer_sha256 @{}
Check 'evidence roundtrip preserves existing literal observation date' ($result.checks[0].recorded_at -is [string] -and $result.checks[0].recorded_at -ceq $case.recorded_at)
Check 'unobserved hosted rows and owner are not promoted' (@($result.checks | Where-Object status -eq 'unverified').Count -eq 8 -and $result.checks[8].status -ceq 'unverified' -and $result.checks[8].method -ceq 'interactive_windows')
$artifact=[ordered]@{path=(ConvertTo-HostedArtifactPath 'dist\acceptance-evidence\evidence\hosted-install.json');bytes=(Get-Item -LiteralPath $fixture).Length;sha256=(Get-FileHash -LiteralPath $fixture -Algorithm SHA256).Hash.ToLowerInvariant()}
Check 'single Windows separators become portable artifact paths' ($artifact.path -ceq 'dist/acceptance-evidence/evidence/hosted-install.json')
$cases=@{'HOSTED-INSTALL'=@{Result=$case;Artifact=$artifact}}
$recorded=New-HostedAcceptanceEvidence $base $case.version $case.source_commit $case.production_sha256 $case.installer_sha256 $cases
Check 'portable actual historical case artifact is recorded without promotion of missing rows' ($recorded.checks[0].status -ceq 'pass' -and $recorded.checks[0].artifacts[0].path -ceq $artifact.path -and @($recorded.checks|Where-Object status -eq 'unverified').Count -eq 8)
foreach ($bad in @('..\outside.json','/absolute.json','C:\absolute.json','\\server\share\input.json','unsafe;name.json')) {
    $badArtifact=[ordered]@{path=(ConvertTo-HostedArtifactPath $bad);bytes=$artifact.bytes;sha256=$artifact.sha256}
    Refuses 'canonicalization does not waive unsafe artifact refusal' { New-HostedAcceptanceEvidence $base $case.version $case.source_commit $case.production_sha256 $case.installer_sha256 @{'HOSTED-INSTALL'=@{Result=$case;Artifact=$badArtifact}} }
}
Write-Host "$count hosted timestamp parsing checks passed (historical fixture and synthetic clones; not current acceptance)"
