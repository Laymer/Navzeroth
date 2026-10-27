# Public ownership marker for one disposable fixture attempt; not a credential.
Set-StrictMode -Version 3.0
function New-RecoveryProbeOwnership([string] $Path,[string] $Nonce,[string] $Source,[string] $InstallerHash,[string] $InstallDirectory) {
    if ($Nonce -cnotmatch '^[a-f0-9]{32}$' -or $Source -cnotmatch '^[a-f0-9]{40}$' -or $InstallerHash -cnotmatch '^[a-f0-9]{64}$') { throw 'Invalid fixture ownership identity.' }
    $record=[ordered]@{schema_version=1;ownership_nonce=$Nonce;source_commit=$Source;installer_sha256=$InstallerHash;install_directory=[IO.Path]::GetFullPath($InstallDirectory)}
    $bytes=(New-Object Text.UTF8Encoding($false)).GetBytes(($record|ConvertTo-Json -Compress))
    if ($bytes.Length -gt 4096) { throw 'Fixture ownership marker exceeds bound.' }
    $file=[IO.File]::Open($Path,[IO.FileMode]::CreateNew,[IO.FileAccess]::Write,[IO.FileShare]::None)
    try { $file.Write($bytes,0,$bytes.Length) } finally { $file.Dispose() }
}
function Test-RecoveryProbeOwnership([string] $Path,[string] $Nonce,[string] $Source,[string] $InstallerHash,[string] $InstallDirectory) {
    if ($Nonce -cnotmatch '^[a-f0-9]{32}$' -or $Source -cnotmatch '^[a-f0-9]{40}$' -or $InstallerHash -cnotmatch '^[a-f0-9]{64}$') { return $false }
    if (-not (Test-Path -LiteralPath $Path -PathType Leaf)) { return $false }
    $item=Get-Item -LiteralPath $Path
    if ($item.Length -lt 1 -or $item.Length -gt 4096 -or ($item.Attributes -band [IO.FileAttributes]::ReparsePoint)) { return $false }
    try {
        # Owned marker, not an adversarial filesystem sandbox. Cap the opened
        # descriptor as well as preflight metadata; never ReadAllText after stat.
        $stream=[IO.File]::Open($Path,[IO.FileMode]::Open,[IO.FileAccess]::Read,[IO.FileShare]::Read)
        try {
            if ($stream.Length -gt 4096) { return $false }
            $bytes=New-Object byte[] 4097;$count=0
            do { $n=$stream.Read($bytes,$count,$bytes.Length-$count);$count+=$n } while($n -gt 0 -and $count -lt $bytes.Length)
            if ($count -gt 4096 -or $count -ne $stream.Length) { return $false }
            $record=(New-Object Text.UTF8Encoding($false,$true)).GetString($bytes,0,$count)|ConvertFrom-Json
        } finally { $stream.Dispose() }
        return (@($record.PSObject.Properties).Count -eq 5 -and ($record.schema_version -is [int] -or $record.schema_version -is [long]) -and $record.schema_version -eq 1 -and $record.ownership_nonce -is [string] -and $record.source_commit -is [string] -and $record.installer_sha256 -is [string] -and $record.install_directory -is [string] -and $record.ownership_nonce -ceq $Nonce -and $record.source_commit -ceq $Source -and $record.installer_sha256 -ceq $InstallerHash -and $record.install_directory -ieq [IO.Path]::GetFullPath($InstallDirectory))
    } catch { return $false }
}
