[CmdletBinding()]
param([string] $ActualRoot)
Set-StrictMode -Version 3.0
$ErrorActionPreference = 'Stop'
. (Join-Path $PSScriptRoot 'Release-Materials.ps1')
$root = Join-Path ([IO.Path]::GetTempPath()) ('anav-contract-bounds-' + [Guid]::NewGuid().ToString('N'))
New-Item -ItemType Directory $root | Out-Null
$count = 0
function Check([string] $Name, [bool] $Condition) { if (-not $Condition) { throw "FAILED: $Name" }; $script:count++; Write-Host "ok $Name" }
function Refuses([string] $Name, [scriptblock] $Action) { $failed=$false; try { & $Action | Out-Null } catch { $failed=$true }; Check $Name $failed }
function Write-Sized([string] $Name, [string] $Prefix, [int] $Bytes) {
    $path = Join-Path $root $Name
    [IO.File]::WriteAllText($path, $Prefix + (' ' * ($Bytes - $Prefix.Length)), (New-Object Text.UTF8Encoding($false)))
    return $path
}
try {
    $legacy = Write-Sized 'legacy.json' '{"schema_version":3}' 16777216
    Check 'generic exact 16 MiB succeeds' ((Read-BoundedReleaseText $legacy).Length -eq 16777216)
    $smallGeneric = Write-Sized 'generic.txt' 'SOURCE_COMMIT' 64
    Check 'generic small artifact succeeds' ((Read-BoundedReleaseText $smallGeneric).StartsWith('SOURCE_COMMIT'))
    Check 'historical exact 16 MiB' ((Read-BoundedReleaseContractText $legacy).Length -eq 16777216)
    $large = Write-Sized 'large.json' '{"schema_version":4}' 16777217
    Check 'schema4 above historical limit' ((Read-BoundedReleaseContractText $large).Length -eq 16777217)
    $overflowError = $null
    try { Read-BoundedReleaseText $large | Out-Null } catch { $overflowError = $_.Exception }
    Check 'generic cap+1 refuses for actual size bound' ($null -ne $overflowError -and $overflowError.Message.Contains('Release JSON exceeds its size limit.'))
    foreach ($version in 1,2,3,6,'"4"','"5"','true','4.0','5.0','null') {
        $bad = Write-Sized 'bad.json' ('{"schema_version":' + $version + '}') 16777217
        Refuses "unsupported/oversized schema $version" { Read-BoundedReleaseContractText $bad }
    }
    $maximum = Write-Sized 'maximum.json' '{"schema_version":4}' 67108864
    Check 'schema4 exact 64 MiB' ((Read-BoundedReleaseContractText $maximum).Length -eq 67108864)
    $overflow = Write-Sized 'overflow.json' '{"schema_version":4}' 67108865
    Refuses 'schema4 cap+1' { Read-BoundedReleaseContractText $overflow }
    $maximum5 = Write-Sized 'maximum5.json' '{"schema_version":5}' 67108864
    Check 'schema5 retains exact 64 MiB bound' ((Read-BoundedReleaseContractText $maximum5).Length -eq 67108864)
    $overflow5 = Write-Sized 'overflow5.json' '{"schema_version":5}' 67108865
    Refuses 'schema5 cap+1' { Read-BoundedReleaseContractText $overflow5 }
    $unicode = [pscustomobject]@{schema_version=3;value=([string][char]233 * 8388608)}
    Refuses 'serialization counts UTF8 bytes' { ConvertTo-BoundedReleaseContractText $unicode }
    $unicode.schema_version=4
    Check 'schema4 compact bounded serialization' ((ConvertTo-BoundedReleaseContractText $unicode | ConvertFrom-Json).value.Length -eq 8388608)
    $unicode.value = [string][char]233 * 33554432
    Refuses 'schema4 serialization over 64 MiB' { ConvertTo-BoundedReleaseContractText $unicode }
    $bom = Write-Sized 'bom.json' '{"schema_version":3}' 16777216
    $original = [IO.File]::ReadAllBytes($bom)
    [IO.File]::WriteAllBytes($bom, ([byte[]]@(239,187,191) + $original))
    Refuses 'historical bound counts UTF8 BOM' { Read-BoundedReleaseContractText $bom }
    $small = Write-Sized 'future.json' '{"schema_version":6}' 20
    Refuses 'future schema refuses even small file' { Read-BoundedReleaseContractText $small }
    $invalid = Join-Path $root 'invalid.json' 
    [IO.File]::WriteAllBytes($invalid, [byte[]]@(255))
    $utf8Error = $null
    try { Read-BoundedReleaseText $invalid | Out-Null } catch { $utf8Error = $_.Exception.GetBaseException() }
    Check 'generic invalid UTF8 reaches strict decoder' ($utf8Error -is [Text.DecoderFallbackException])
    $validBom = Join-Path $root 'valid-bom.txt'
    [IO.File]::WriteAllBytes($validBom, [byte[]]@(239,187,191,97))
    Check 'generic UTF8 BOM succeeds' ((Read-BoundedReleaseText $validBom) -ceq 'a')
    Refuses 'directory refused' { Read-BoundedReleaseContractText $root }
    if ($ActualRoot) {
        foreach ($name in 'coverage-inventory','acceptance-evidence') {
            $path = Join-Path $ActualRoot ("release/v1/$name.json")
            $text = Read-BoundedReleaseContractText $path
            $hash = Get-ReleaseTextHash $text
            $expected = if ($name -eq 'coverage-inventory') { 'f7030627acbc61fbbec32deeacc9bfc4dc1f8adc8fb6a183771559c656ce9eac' } else { '61c3f4ce220b93042dc039d83553bc0ad5073a7807b4ee33d2dca7ab4c2c91a8' }
            Check "actual $name exact hash" ($hash -ceq $expected)
            Check "actual $name over 16 MiB" ((Get-Item -LiteralPath $path).Length -gt 16777216)
            if ($name -eq 'acceptance-evidence') {
                $document = $text | ConvertFrom-Json
                $serialized = ConvertTo-BoundedReleaseContractText $document
                $roundtrip = $serialized | ConvertFrom-Json
                Check 'actual evidence overlay retains checks' (($document.checks | ConvertTo-Json -Depth 100 -Compress) -ceq ($roundtrip.checks | ConvertTo-Json -Depth 100 -Compress))
                Check 'actual evidence overlay retains sources' (($document.sources | ConvertTo-Json -Depth 100 -Compress) -ceq ($roundtrip.sources | ConvertTo-Json -Depth 100 -Compress))
            }
            Write-Host "$name bytes=$((Get-Item -LiteralPath $path).Length) sha256=$hash"
        }
    }
    Write-Host "$count checks passed; PowerShell $($PSVersionTable.PSVersion)"
} finally { Remove-Item -LiteralPath $root -Recurse -Force }
