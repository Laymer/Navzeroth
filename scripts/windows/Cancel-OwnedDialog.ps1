[CmdletBinding()]
param(
    [Parameter(Mandatory = $true)][ValidateRange(1, 2147483647)][int] $TargetProcessId,
    [Parameter(Mandatory = $true)][string] $ExpectedExecutable,
    [ValidateRange(1, 30)][int] $TimeoutSeconds = 20
)
Set-StrictMode -Version 3.0
$ErrorActionPreference = 'Stop'
$expected = [IO.Path]::GetFullPath($ExpectedExecutable)
function Assert-OwnedProcess {
    $process = [Diagnostics.Process]::GetProcessById($TargetProcessId)
    try {
        if (-not [StringComparer]::OrdinalIgnoreCase.Equals($process.MainModule.FileName, $expected)) {
            throw 'Dialog process executable does not match the launched application'
        }
    } finally { $process.Dispose() }
}
Assert-OwnedProcess
if (-not ('NavOwnedDialog' -as [type])) {
    Add-Type -TypeDefinition @'
using System;
using System.Runtime.InteropServices;
using System.Text;
public static class NavOwnedDialog {
    public delegate bool WindowCallback(IntPtr window, IntPtr parameter);
    [DllImport("user32.dll")] public static extern bool EnumWindows(WindowCallback callback, IntPtr parameter);
    [DllImport("user32.dll")] public static extern uint GetWindowThreadProcessId(IntPtr window, out uint processId);
    [DllImport("user32.dll", CharSet = CharSet.Unicode)] public static extern int GetClassName(IntPtr window, StringBuilder name, int maximum);
    [DllImport("user32.dll")] public static extern bool IsWindowVisible(IntPtr window);
    [DllImport("user32.dll", SetLastError = true)] public static extern bool PostMessage(IntPtr window, uint message, IntPtr wparam, IntPtr lparam);
    public static IntPtr Find(uint processId) {
        IntPtr found = IntPtr.Zero;
        EnumWindows((window, parameter) => {
            uint owner;
            GetWindowThreadProcessId(window, out owner);
            if (owner != processId || !IsWindowVisible(window)) return true;
            var name = new StringBuilder(256);
            GetClassName(window, name, name.Capacity);
            if (name.ToString() != "#32770") return true;
            found = window;
            return false;
        }, IntPtr.Zero);
        return found;
    }
}
'@
}
$timer = [Diagnostics.Stopwatch]::StartNew()
while ($timer.Elapsed.TotalSeconds -lt $TimeoutSeconds) {
    Assert-OwnedProcess
    $window = [NavOwnedDialog]::Find([uint32]$TargetProcessId)
    if ($window -ne [IntPtr]::Zero) {
        Assert-OwnedProcess
        [uint32]$ownerId = 0
        [void][NavOwnedDialog]::GetWindowThreadProcessId($window, [ref]$ownerId)
        if ($ownerId -ne $TargetProcessId) { throw 'Dialog ownership changed before cancellation' }
        if (-not [NavOwnedDialog]::PostMessage($window, 0x0010, [IntPtr]::Zero, [IntPtr]::Zero)) {
            throw 'Owned dialog cancellation could not be dispatched'
        }
        Write-Output 'Dispatched cancellation to the launched application dialog'
        return
    }
    Start-Sleep -Milliseconds 50
}
throw 'No visible dialog owned by the launched application appeared before timeout'
