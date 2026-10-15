[CmdletBinding()]
param(
    [string] $OutputDirectory = 'dist/source-knowledge',
    [string] $Executable = ''
)
Set-StrictMode -Version 3.0
$ErrorActionPreference = 'Stop'
$root = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot '..'))
Push-Location $root
try {
    if (-not $Executable) {
        # Provision toolchain/crates separately. The ordinary source build must
        # not fall back to a registry, private API or publisher website.
        & cargo build --locked --offline -p azeroth-cli
        if ($LASTEXITCODE -ne 0) { throw 'Offline CLI compilation failed; provision the pinned Rust dependencies separately with cargo fetch --locked.' }
        $suffix = if ($env:OS -eq 'Windows_NT') { '.exe' } else { '' }
        $Executable = Join-Path $root "target/debug/azeroth-cli$suffix"
    }
    & $Executable source-build --registry docs/sources/source-admission-v1.json --plan content/source-build-v1.json --cache content/source-materials/objects --output $OutputDirectory
    if ($LASTEXITCODE -ne 0) { throw 'Offline source build refused; previous outputs and retained inputs remain unchanged.' }
} finally { Pop-Location }
