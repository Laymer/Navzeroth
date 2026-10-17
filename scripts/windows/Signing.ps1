# Authenticode helpers for an externally signed installer (task V0.4).
# Dot-source only; no main action. The signing service returns a new file; these
# checks prove it is the exact unsigned build plus an appended signature.
Set-StrictMode -Version 3.0

$script:MaxSignedPeBytes = 512MB

function Read-PeBytes([string] $Path) {
    $item = Get-Item -LiteralPath $Path
    if ($item.PSIsContainer -or $item.Attributes -band [IO.FileAttributes]::ReparsePoint) { throw "Not a regular file: $Path" }
    if ($item.Length -gt $script:MaxSignedPeBytes) { throw "File exceeds the signing size limit: $Path" }
    return [IO.File]::ReadAllBytes($item.FullName)
}

# Offsets of the optional-header CheckSum and the security (certificate table)
# data directory entry, plus that entry's file offset and size.
function Get-PeSignatureLayout([byte[]] $Bytes) {
    if ($Bytes.Length -lt 0x40 -or $Bytes[0] -ne 0x4D -or $Bytes[1] -ne 0x5A) { throw 'Not a PE file (no MZ header).' }
    $pe = [BitConverter]::ToUInt32($Bytes, 0x3C)
    if ($pe -gt $Bytes.Length - 24 -or $Bytes[$pe] -ne 0x50 -or $Bytes[$pe + 1] -ne 0x45 -or $Bytes[$pe + 2] -ne 0 -or $Bytes[$pe + 3] -ne 0) { throw 'Not a PE file (no PE signature).' }
    $optional = $pe + 24
    if ($optional + 2 -gt $Bytes.Length) { throw 'Truncated PE optional header.' }
    $magic = [BitConverter]::ToUInt16($Bytes, $optional)
    if ($magic -eq 0x10B) { $countOffset = $optional + 92; $directories = $optional + 96 }
    elseif ($magic -eq 0x20B) { $countOffset = $optional + 108; $directories = $optional + 112 }
    else { throw ('Unsupported PE optional header magic 0x{0:X}.' -f $magic) }
    if ($directories + 40 -gt $Bytes.Length) { throw 'Truncated PE data directories.' }
    if ([BitConverter]::ToUInt32($Bytes, $countOffset) -lt 5) { throw 'PE file has no security data directory.' }
    $security = $directories + 32
    return [pscustomobject]@{
        ChecksumOffset = $optional + 64
        SecurityEntryOffset = $security
        CertificateOffset = [BitConverter]::ToUInt32($Bytes, $security)
        CertificateSize = [BitConverter]::ToUInt32($Bytes, $security + 4)
    }
}

# Refuses unless Signed equals Unsigned byte for byte, except the CheckSum and
# the security entry, followed only by zero alignment padding and exactly one
# certificate table that runs to the end of the file.
function Assert-SignedFromUnsigned([string] $UnsignedPath, [string] $SignedPath) {
    $unsigned = Read-PeBytes $UnsignedPath
    $signed = Read-PeBytes $SignedPath
    $before = Get-PeSignatureLayout $unsigned
    $after = Get-PeSignatureLayout $signed
    if ($before.CertificateOffset -ne 0 -or $before.CertificateSize -ne 0) { throw 'The unsigned input already carries a certificate table.' }
    if ($after.CertificateOffset -eq 0 -or $after.CertificateSize -eq 0) { throw 'The signed file has no certificate table.' }
    if ($after.ChecksumOffset -ne $before.ChecksumOffset -or $after.SecurityEntryOffset -ne $before.SecurityEntryOffset) { throw 'The signed file has a different PE header layout.' }
    if ($after.CertificateOffset -lt $unsigned.Length -or $after.CertificateOffset % 8 -ne 0) { throw 'The certificate table does not start after the unsigned bytes.' }
    if ([uint64]$after.CertificateOffset + $after.CertificateSize -ne $signed.Length) { throw 'The certificate table does not end the signed file.' }
    for ($i = 0; $i -lt $unsigned.Length; $i++) {
        if ($i -ge $before.ChecksumOffset -and $i -lt $before.ChecksumOffset + 4) { continue }
        if ($i -ge $before.SecurityEntryOffset -and $i -lt $before.SecurityEntryOffset + 8) { continue }
        if ($unsigned[$i] -ne $signed[$i]) { throw ('The signed file differs from the unsigned build at byte {0}.' -f $i) }
    }
    for ($i = $unsigned.Length; $i -lt $after.CertificateOffset; $i++) {
        if ($signed[$i] -ne 0) { throw 'Nonzero bytes between the unsigned build and the certificate table.' }
    }
}

# Trusted mode needs a Valid chain and a timestamp; test-signing certificates are
# untrusted by design, so that mode only requires a signature to be present.
function Assert-AuthenticodeSignature([string] $Path, [bool] $RequireTrusted, [string] $ExpectedSubject) {
    $signature = Get-AuthenticodeSignature -LiteralPath $Path
    if (-not $signature.SignerCertificate) { throw "No Authenticode signature on $Path ($($signature.Status))." }
    if ($RequireTrusted) {
        if ($signature.Status -ne 'Valid') { throw "Authenticode signature is not valid on ${Path}: $($signature.Status) $($signature.StatusMessage)" }
        if (-not $signature.TimeStamperCertificate) { throw "Authenticode signature on $Path has no timestamp." }
    }
    if ($ExpectedSubject -and -not $signature.SignerCertificate.Subject.Contains($ExpectedSubject)) {
        throw "Signer '$($signature.SignerCertificate.Subject)' does not contain '$ExpectedSubject'."
    }
    return $signature
}
