Set-StrictMode -Version 3.0
$ErrorActionPreference='Stop'
. (Join-Path $PSScriptRoot 'Start-SupplementalUpgrade.ps1')
$count=0
function Check([bool]$Condition){if(-not $Condition){throw 'Supplemental launcher regression failed.'};$script:count++}
function Refuses([scriptblock]$Action){$failed=$false;try{&$Action}catch{$failed=$true};Check $failed}
Assert-SupplementalPin ('a'*64) ('b'*40);$count++
Refuses {Assert-SupplementalPin ('A'*64) ('b'*40)}
Refuses {Assert-SupplementalPin ('a'*64) ('b'*39)}
$temp=Join-Path ([IO.Path]::GetTempPath()) ('navzeroth-supplemental-mock-'+[Guid]::NewGuid().ToString('N'))
try {
    $null=New-Item -ItemType Directory -Path $temp
    $file=Join-Path $temp 'snapshots.sqlite3';[IO.File]::WriteAllText($file,'synthetic inert backup fixture')
    $before=@(Get-SupplementalInventory $temp)
    Check ($before.Count -eq 1 -and $before[0].path -ceq 'snapshots.sqlite3')
    Check ($before[0].sha256 -ceq (Get-FileHash -LiteralPath $file -Algorithm SHA256).Hash.ToLowerInvariant())
    Refuses {Assert-SupplementalRegular $file $true}
    Refuses {Assert-SupplementalRegular $temp $false}
    [IO.File]::WriteAllText($file,'different inert bytes')
    Check ((Convert-SupplementalCanonical (Get-SupplementalInventory $temp)) -cne (Convert-SupplementalCanonical $before))
    # The command is dot sourced: no installer, process, policy or private profile access.
    $source=[IO.File]::ReadAllText((Join-Path $PSScriptRoot 'Start-SupplementalUpgrade.ps1'))
    Check ($source.Contains('-Verb RunAs') -and -not $source.Contains('ExecutionPolicy') -and -not $source.Contains("@('/S')"))
} finally {Remove-Item -LiteralPath $temp -Recurse -Force -ErrorAction SilentlyContinue}
Write-Output "Supplemental launcher helpers: $count checks passed. No real installation/profile/elevation exercised."
