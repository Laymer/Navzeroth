[CmdletBinding()]
param()
Set-StrictMode -Version 3.0
$ErrorActionPreference = 'Stop'
. (Join-Path $PSScriptRoot 'Signing.ps1')
$temp = Join-Path ([IO.Path]::GetTempPath()) ('anav-signing-' + [Guid]::NewGuid().ToString('N'))
$count = 0
function Check([string] $Name, [bool] $Condition) { $script:count++; if (-not $Condition) { throw "FAILED: $Name" }; Write-Host "ok $Name" }
function Refuses([string] $Name, [string] $Expected, [scriptblock] $Action) {
    $message = ''
    try { & $Action } catch { $message = $_.Exception.Message }
    Check $Name ($message.Contains($Expected))
}
function Write-UInt32([byte[]] $Bytes, [int] $Offset, [uint32] $Value) {
    [Array]::Copy([BitConverter]::GetBytes($Value), 0, $Bytes, $Offset, 4)
}
# A synthetic PE32+ header with odd-length overlay data, like an NSIS installer.
function New-SyntheticPe([int] $Length) {
    $bytes = New-Object byte[] $Length
    $bytes[0] = 0x4D; $bytes[1] = 0x5A
    Write-UInt32 $bytes 0x3C 0x80
    $bytes[0x80] = 0x50; $bytes[0x81] = 0x45
    [Array]::Copy([BitConverter]::GetBytes([uint16]0x20B), 0, $bytes, 0x98, 2)
    Write-UInt32 $bytes (0x98 + 108) 16
    for ($i = 0x200; $i -lt $Length; $i++) { $bytes[$i] = [byte](($i * 7) % 251 + 1) }
    return $bytes
}
# What a signer does: new CheckSum, padding to 8 bytes, certificate table, security entry.
function New-SyntheticSigned([byte[]] $Unsigned, [int] $Padding, [byte] $PadValue) {
    $offset = $Unsigned.Length + $Padding
    $signed = New-Object byte[] ($offset + 64)
    [Array]::Copy($Unsigned, $signed, $Unsigned.Length)
    for ($i = $Unsigned.Length; $i -lt $offset; $i++) { $signed[$i] = $PadValue }
    for ($i = $offset; $i -lt $signed.Length; $i++) { $signed[$i] = 0xAB }
    Write-UInt32 $signed (0x98 + 64) 0x1234ABCD
    Write-UInt32 $signed (0x98 + 112 + 32) $offset
    Write-UInt32 $signed (0x98 + 112 + 36) 64
    return $signed
}
try {
    New-Item -ItemType Directory -Path $temp | Out-Null
    $unsignedPath = Join-Path $temp 'unsigned.exe'
    $signedPath = Join-Path $temp 'signed.exe'
    $unsigned = New-SyntheticPe 1021
    [IO.File]::WriteAllBytes($unsignedPath, $unsigned)

    $layout = Get-PeSignatureLayout $unsigned
    Check 'PE32+ layout offsets are located' ($layout.ChecksumOffset -eq 0xD8 -and $layout.SecurityEntryOffset -eq 0x128 -and $layout.CertificateOffset -eq 0)

    [IO.File]::WriteAllBytes($signedPath, (New-SyntheticSigned $unsigned 3 0))
    Assert-SignedFromUnsigned $unsignedPath $signedPath
    Check 'unsigned bytes plus padding and certificate are accepted' $true

    $tampered = New-SyntheticSigned $unsigned 3 0
    $tampered[0x300] = $tampered[0x300] -bxor 1
    [IO.File]::WriteAllBytes($signedPath, $tampered)
    Refuses 'a changed byte in the build is refused' 'differs from the unsigned build at byte 768' { Assert-SignedFromUnsigned $unsignedPath $signedPath }

    [IO.File]::WriteAllBytes($signedPath, (New-SyntheticSigned $unsigned 3 0xFF))
    Refuses 'nonzero padding is refused' 'Nonzero bytes' { Assert-SignedFromUnsigned $unsignedPath $signedPath }

    [IO.File]::WriteAllBytes($signedPath, (New-SyntheticSigned $unsigned 2 0))
    Refuses 'a misaligned certificate table is refused' 'does not start after' { Assert-SignedFromUnsigned $unsignedPath $signedPath }

    $extra = New-SyntheticSigned $unsigned 3 0
    [IO.File]::WriteAllBytes($signedPath, ($extra + [byte[]](1, 2, 3)))
    Refuses 'data after the certificate table is refused' 'does not end the signed file' { Assert-SignedFromUnsigned $unsignedPath $signedPath }

    [IO.File]::WriteAllBytes($signedPath, $unsigned)
    Refuses 'a file without a certificate table is refused' 'no certificate table' { Assert-SignedFromUnsigned $unsignedPath $signedPath }

    [IO.File]::WriteAllBytes($unsignedPath, (New-SyntheticSigned $unsigned 3 0))
    Refuses 'an already signed input is refused' 'already carries' { Assert-SignedFromUnsigned $unsignedPath $signedPath }

    [IO.File]::WriteAllBytes($unsignedPath, [byte[]](0x4D, 0x5A, 0, 0))
    Refuses 'a truncated header is refused' 'Not a PE file' { Assert-SignedFromUnsigned $unsignedPath $signedPath }
    [IO.File]::WriteAllText($unsignedPath, 'plain text, not an executable')
    Refuses 'a non-PE file is refused' 'Not a PE file' { Assert-SignedFromUnsigned $unsignedPath $signedPath }

    Refuses 'an unsigned file has no Authenticode signature' 'No Authenticode signature' { Assert-AuthenticodeSignature $signedPath $false '' }
    Write-Host "$count signing checks passed (synthetic PE files, not a real signature)"
} finally { Remove-Item -LiteralPath $temp -Recurse -Force -ErrorAction SilentlyContinue }
