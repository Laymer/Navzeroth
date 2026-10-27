[CmdletBinding()]
param([Parameter(Mandatory)][string] $Version, [Parameter(Mandatory)][string] $Directory, [switch] $AddonOnly, [switch] $AcceptanceCandidate, [ValidateSet('public','prebuild','draft')][string] $ReadinessPhase = 'public', [string] $Evidence = '')
Set-StrictMode -Version 3.0
$ErrorActionPreference = 'Stop'
. (Join-Path $PSScriptRoot 'Release-Materials.ps1')
$root = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot '../..'))
Assert-ReleaseArtifacts $root $Version ([IO.Path]::GetFullPath($Directory)) ([bool]$AddonOnly) ([bool]$AcceptanceCandidate) $ReadinessPhase $Evidence
exit 0
