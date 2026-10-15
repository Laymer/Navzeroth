[CmdletBinding()]
param([Parameter(Mandatory)][string] $Version, [Parameter(Mandatory)][string] $Directory, [switch] $AddonOnly)
Set-StrictMode -Version 3.0
$ErrorActionPreference = 'Stop'
. (Join-Path $PSScriptRoot 'Release-Materials.ps1')
$root = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot '../..'))
Assert-ReleaseArtifacts $root $Version ([IO.Path]::GetFullPath($Directory)) ([bool]$AddonOnly)
exit 0
