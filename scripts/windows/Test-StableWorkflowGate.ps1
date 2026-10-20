# Execute the actual workflow gate body against an owned artifact-verifier stub.
# No installer, registry, adapter, receipt, or release state is touched.
Set-StrictMode -Version 3.0
$ErrorActionPreference = 'Stop'
$workflow = [IO.File]::ReadAllText((Join-Path $PSScriptRoot '../../.github/workflows/package-windows.yml'))
$marker = '      - name: Verify stable hosted evidence and exact materials before accepting source or tag artifacts'
$start = $workflow.IndexOf($marker, [StringComparison]::Ordinal)
if ($start -lt 0) { throw 'Stable workflow gate missing.' }
$lines = $workflow.Substring($start).Split("`n")
$body = @(); $run = $false
foreach ($line in $lines) {
    if ($line.TrimEnd() -ceq '        run: |') { $run = $true; continue }
    if (-not $run) { continue }
    if ($line.Trim().Length -eq 0) { continue }
    if (-not $line.StartsWith('          ', [StringComparison]::Ordinal)) { break }
    $body += $line.Substring(10).TrimEnd("`r")
}
if ($body.Count -lt 3) { throw 'Stable workflow gate body missing.' }
$gate = $body -join "`n"
$producerToken = '${{ steps.stable_acceptance.outcome }}'
$overlayToken = '${{ steps.hosted_evidence.outcome }}'
if (-not $gate.Contains($producerToken) -or -not $gate.Contains($overlayToken)) { throw 'Outcome binding missing.' }
$root = Join-Path ([IO.Path]::GetTempPath()) ('navzeroth-workflow-gate-' + [Guid]::NewGuid().ToString('N'))
$previous = Get-Location
$oldExit = $env:NAVZEROTH_TEST_GATE_EXIT
$script:checks = 0
try {
    New-Item -ItemType Directory -Path (Join-Path $root 'scripts/windows') -Force | Out-Null
    $stub = @'
param([string]$Version,[string]$Directory,[string]$ReadinessPhase,[string]$Evidence)
if ($Version -cne '1.0.0' -or $Directory -cne 'dist/windows' -or $ReadinessPhase -cne 'draft' -or $Evidence -cne 'dist/acceptance-evidence/acceptance-evidence.json') { throw 'Verifier inputs differ.' }
[IO.File]::WriteAllText((Join-Path $PWD 'called.txt'), 'verified')
$global:LASTEXITCODE = [int]$env:NAVZEROTH_TEST_GATE_EXIT
'@
    [IO.File]::WriteAllText((Join-Path $root 'scripts/windows/Verify-ReleaseArtifacts.ps1'), $stub)
    Set-Location $root
    function Run-Gate([string]$Producer,[string]$Overlay,[bool]$Pass,[int]$ExitCode = 0) {
        $called = Join-Path $root 'called.txt'
        if (Test-Path -LiteralPath $called) { Remove-Item -LiteralPath $called }
        $env:NAVZEROTH_TEST_GATE_EXIT = $ExitCode.ToString()
        $actual = $true
        try { & ([ScriptBlock]::Create($gate.Replace($producerToken,$Producer).Replace($overlayToken,$Overlay))) } catch { $actual = $false }
        if ($actual -ne $Pass) { throw "Wrong gate result producer=$Producer overlay=$Overlay exit=$ExitCode" }
        $shouldCall = $Producer -ceq 'success' -and $Overlay -ceq 'success'
        if ((Test-Path -LiteralPath $called) -ne $shouldCall) { throw 'Artifact verifier called before successful actual cases and overlay, or omitted.' }
        $script:checks++
    }
    Run-Gate 'success' 'success' $true
    Run-Gate 'success' 'success' $false 9
    foreach ($missing in @('','skipped','failure','cancelled','unknown')) {
        Run-Gate $missing 'success' $false
        Run-Gate 'success' $missing $false
    }
    Write-Host "PASS $script:checks actual workflow gate checks (synthetic verifier only)."
} finally {
    Set-Location $previous
    $env:NAVZEROTH_TEST_GATE_EXIT = $oldExit
    if (Test-Path -LiteralPath $root) { Remove-Item -LiteralPath $root -Recurse -Force }
}
