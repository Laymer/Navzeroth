# Spike for task G2b: which launch variants make the installed app's WebView2 open a
# remote debugging port on a hosted runner. Prints facts only; never used to pass a check.
[CmdletBinding()]
param([Parameter(Mandatory)] [string] $Executable, [Parameter(Mandatory)] [string] $DataDirectory)
Set-StrictMode -Version 3.0
$ErrorActionPreference = 'Continue'
. (Join-Path $PSScriptRoot 'Native-Smoke-Helpers.ps1')
$udf = Join-Path $env:LOCALAPPDATA 'io.github.laymer.azeroth-navigator\EBWebView'

function Probe([string] $Name, [hashtable] $Environment, [int] $Port) {
    Write-Host "=== variant: $Name"
    Stop-NativeSmokeProcesses
    Start-Sleep -Seconds 3
    if (Test-Path -LiteralPath $udf) { Remove-Item -LiteralPath $udf -Recurse -Force -ErrorAction SilentlyContinue }
    foreach ($k in $Environment.Keys) { Set-Item -Path "Env:$k" -Value $Environment[$k] }
    $p = Start-Process -FilePath $Executable -PassThru
    $answered = $false
    for ($i = 0; $i -lt 20 -and -not $answered; $i++) {
        Start-Sleep -Seconds 1
        try { $r = Invoke-WebRequest -UseBasicParsing -Uri "http://127.0.0.1:$Port/json/version" -TimeoutSec 2; $answered = $true; Write-Host "ANSWERED: $($r.Content -replace '\s+', ' ')" } catch { }
    }
    if (-not $answered) { Write-Host "no answer on port $Port" }
    Get-CimInstance Win32_Process -Filter "Name = 'msedgewebview2.exe'" | Where-Object { $_.CommandLine -match 'embedded-browser-webview=1' -and $_.CommandLine -notmatch '--type=' } |
        ForEach-Object { Write-Host "browser cmdline: $($_.CommandLine)" }
    foreach ($k in $Environment.Keys) { Remove-Item -Path "Env:$k" -ErrorAction SilentlyContinue }
    Stop-NativeSmokeProcesses
}

Probe 'env var, port only (Start-Process)' @{ WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS = '--remote-debugging-port=9301' } 9301
Probe 'env var, port + allow-origins' @{ WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS = '--remote-debugging-port=9302 --remote-allow-origins=*' } 9302
Probe 'env var plus explicit user data folder' @{ WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS = '--remote-debugging-port=9303'; WEBVIEW2_USER_DATA_FOLDER = (Join-Path $env:RUNNER_TEMP 'probe-udf') } 9303
Probe 'env var, port + disable-features duplicate' @{ WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS = '--remote-debugging-port=9304 --disable-features=msWebOOUI,msPdfOOUI,msSmartScreenProtection' } 9304
Probe 'remote-debugging-port=0 (DevToolsActivePort)' @{ WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS = '--remote-debugging-port=0' } 9305
Get-ChildItem -Recurse -Filter DevToolsActivePort -Force -ErrorAction SilentlyContinue (Join-Path $env:LOCALAPPDATA 'io.github.laymer.azeroth-navigator') | ForEach-Object { Write-Host "DevToolsActivePort: $($_.FullName)"; Get-Content $_.FullName }
