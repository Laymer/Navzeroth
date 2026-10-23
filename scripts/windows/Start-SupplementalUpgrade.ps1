# Optional personal-machine operation. Dot sourcing defines helpers only.
[CmdletBinding()]
param(
    [ValidateSet('Backup','Upgrade')][string] $Phase = 'Backup',
    [string] $Installer,
    [string] $InstallerSha256,
    [string] $SourceCommit,
    [string] $SourceCommitFile,
    [string] $BackupDirectory
)
Set-StrictMode -Version 3.0
function Assert-SupplementalRegular([string] $Path, [bool] $Directory) {
    $full=[IO.Path]::GetFullPath($Path)
    $item=Get-Item -LiteralPath $full -Force -ErrorAction Stop
    if ($item.PSIsContainer -ne $Directory) { throw 'Unexpected file/directory type.' }
    $cursor=$item
    while ($null -ne $cursor) {
        if (($cursor.Attributes -band [IO.FileAttributes]::ReparsePoint) -ne 0) { throw 'Reparse paths are refused.' }
        $cursor=if($cursor -is [IO.DirectoryInfo]){$cursor.Parent}else{$cursor.Directory}
    }
    return $item
}
function Get-SupplementalInventory([string] $Root) {
    $null=Assert-SupplementalRegular $Root $true
    $result=@();[long]$total=0
    foreach($item in Get-ChildItem -LiteralPath $Root -Force -Recurse -ErrorAction Stop) {
        if (($item.Attributes -band [IO.FileAttributes]::ReparsePoint) -ne 0) { throw 'Backup refuses reparse entries.' }
        if($item.PSIsContainer){continue}
        $total+=$item.Length
        if($result.Count -ge 100000 -or $total -gt 17179869184 -or $item.Length -gt 8589934592){throw 'Backup exceeds declared bounds.'}
        $result+= [ordered]@{path=$item.FullName.Substring($Root.TrimEnd('\','/').Length+1);bytes=$item.Length;sha256=(Get-FileHash -LiteralPath $item.FullName -Algorithm SHA256).Hash.ToLowerInvariant()}
    }
    return @($result | Sort-Object path)
}
function Convert-SupplementalCanonical($Value) { return ConvertTo-Json -InputObject @($Value) -Depth 8 -Compress }
function Assert-SupplementalPin([string]$Hash,[string]$Commit) {
    if($Hash -cnotmatch '^[a-f0-9]{64}$' -or $Commit -cnotmatch '^[a-f0-9]{40}$'){throw 'Independently selected lowercase SHA256/source commit required.'}
}
function Assert-SupplementalIdle {
    if(@(Get-Process -Name azeroth-desktop -ErrorAction SilentlyContinue).Count){throw 'Close Azeroth Navigator before backup/upgrade.'}
}
function Invoke-SupplementalUpgrade {
    if($env:OS -ne 'Windows_NT'){throw 'This supplemental launcher requires Windows11.'}
    $operatingSystem=Get-CimInstance -ClassName Win32_OperatingSystem -ErrorAction Stop
    if([int]$operatingSystem.BuildNumber -lt 22000 -or $operatingSystem.ProductType -ne 1){throw 'This supplemental launcher requires Windows11 client.'}
    Assert-SupplementalPin $InstallerSha256 $SourceCommit
    $setup=Assert-SupplementalRegular $Installer $false
    if($setup.Length -gt 2147483648 -or $setup.VersionInfo.ProductVersion -cne '1.0.0' -or (Get-FileHash -LiteralPath $setup.FullName -Algorithm SHA256).Hash.ToLowerInvariant() -cne $InstallerSha256){throw 'Draft installer identity mismatch.'}
    if((Get-AuthenticodeSignature -LiteralPath $setup.FullName).Status.ToString() -cne 'NotSigned'){throw 'This exact release contract is explicitly unsigned.'}
    $marker=Assert-SupplementalRegular $SourceCommitFile $false
    if($marker.Length -gt 128 -or [IO.File]::ReadAllText($marker.FullName).Trim() -cne $SourceCommit){throw 'Downloaded source marker differs from independently selected source.'}
    Assert-SupplementalIdle
    $install=Join-Path $env:LOCALAPPDATA 'Azeroth Navigator'
    $exe=Assert-SupplementalRegular (Join-Path $install 'azeroth-desktop.exe') $false
    if($exe.VersionInfo.ProductVersion -cne '0.8.1'){throw 'Only the recorded 0.8.1 baseline is supported.'}
    $roots=[ordered]@{installation=$install;roaming=(Join-Path $env:APPDATA 'io.github.laymer.azeroth-navigator');local=(Join-Path $env:LOCALAPPDATA 'io.github.laymer.azeroth-navigator')}
    $backup=[IO.Path]::GetFullPath($BackupDirectory)
    # Always outside this Git worktree; no arbitrary output under tracked source.
    $repository=[IO.Path]::GetFullPath((Join-Path $PSScriptRoot '../..')).TrimEnd('\')+'\'
    if($backup.StartsWith($repository,[StringComparison]::OrdinalIgnoreCase)){throw 'Private backups must be outside Git.'}
    $ancestor=[IO.Path]::GetDirectoryName($backup)
    while($ancestor){if(Test-Path -LiteralPath (Join-Path $ancestor '.git')){throw 'Private backups cannot be inside any Git worktree.'};$ancestor=[IO.Path]::GetDirectoryName($ancestor)}
    foreach($root in $roots.Values){if($backup.StartsWith($root.TrimEnd('\')+'\',[StringComparison]::OrdinalIgnoreCase)-or $backup -ieq $root){throw 'Backup cannot be inside a protected source directory.'}}
    $manifestPath=Join-Path $backup 'backup-manifest.json'
    if($Phase -ceq 'Backup') {
        if(Test-Path -LiteralPath $backup){throw 'Choose a new backup directory.'}
        $parent=Assert-SupplementalRegular ([IO.Path]::GetDirectoryName($backup)) $true
        $null=New-Item -ItemType Directory -Path $backup -ErrorAction Stop
        $acl=New-Object Security.AccessControl.DirectorySecurity
        $acl.SetAccessRuleProtection($true,$false)
        $sid=[Security.Principal.WindowsIdentity]::GetCurrent().User
        $rule=New-Object Security.AccessControl.FileSystemAccessRule($sid,'FullControl','ContainerInherit,ObjectInherit','None','Allow')
        $acl.AddAccessRule($rule);Set-Acl -LiteralPath $backup -AclObject $acl
        $records=@()
        foreach($name in $roots.Keys){
            $root=$roots[$name];$exists=Test-Path -LiteralPath $root
            $before=if($exists){@(Get-SupplementalInventory $root)}else{@()}
            if($exists){
                $destination=Join-Path $backup $name;$null=New-Item -ItemType Directory -Path $destination
                foreach($file in $before){$to=Join-Path $destination $file.path;$null=New-Item -ItemType Directory -Path ([IO.Path]::GetDirectoryName($to)) -Force;[IO.File]::Copy((Join-Path $root $file.path),$to,$false)}
                if((Convert-SupplementalCanonical (Get-SupplementalInventory $destination)) -cne (Convert-SupplementalCanonical $before) -or (Convert-SupplementalCanonical (Get-SupplementalInventory $root)) -cne (Convert-SupplementalCanonical $before)){throw 'Backup or original changed during copy; partial backup is not usable.'}
            }
            $records+=@{name=$name;source=$root;present=$exists;files=$before}
        }
        Assert-SupplementalIdle
        $document=@{schema_version=1;purpose='supplemental_private_backup_not_acceptance';created_at=[DateTime]::UtcNow.ToString('o');baseline_version='0.8.1';baseline_exe_sha256=(Get-FileHash -LiteralPath $exe.FullName -Algorithm SHA256).Hash.ToLowerInvariant();source_commit=$SourceCommit;installer_sha256=$InstallerSha256;roots=$records}
        [IO.File]::WriteAllText($manifestPath,($document|ConvertTo-Json -Depth 10),(New-Object Text.UTF8Encoding($false)))
        Write-Output 'Private backup complete. Review it before separately invoking Phase Upgrade. No installer was executed.'
        return
    }
    $manifest=Assert-SupplementalRegular $manifestPath $false
    if($manifest.Length -gt 33554432){throw 'Backup manifest exceeds bounds.'}
    $saved=[IO.File]::ReadAllText($manifestPath)|ConvertFrom-Json
    if($saved.schema_version -ne 1 -or $saved.purpose -cne 'supplemental_private_backup_not_acceptance' -or $saved.baseline_version -cne '0.8.1' -or $saved.source_commit -cne $SourceCommit -or $saved.installer_sha256 -cne $InstallerSha256 -or $saved.baseline_exe_sha256 -cne (Get-FileHash -LiteralPath $exe.FullName -Algorithm SHA256).Hash.ToLowerInvariant() -or @($saved.roots).Count -ne 3){throw 'Backup identity mismatch.'}
    foreach($name in $roots.Keys){
        $entry=@($saved.roots|Where-Object name -CEQ $name)
        if($entry.Count -ne 1 -or $entry[0].source -cne $roots[$name] -or $entry[0].present -ne (Test-Path -LiteralPath $roots[$name])){throw 'Backup source identity changed.'}
        if($entry[0].present){foreach($path in @($roots[$name],(Join-Path $backup $name))){if((Convert-SupplementalCanonical (Get-SupplementalInventory $path)) -cne (Convert-SupplementalCanonical $entry[0].files)){throw 'Backup/current files changed since backup; make a new backup.'}}}
    }
    Assert-SupplementalIdle
    if((Get-FileHash -LiteralPath $setup.FullName -Algorithm SHA256).Hash.ToLowerInvariant() -cne $InstallerSha256){throw 'Installer changed during backup verification.'}
    # Ordinary Windows UAC. No /S, policy changes, automatic approval or app launch.
    $process=Start-Process -FilePath $setup.FullName -Verb RunAs -PassThru
    if(-not $process.WaitForExit(1200000)){throw 'Installer remains running after20 minutes; inspect it manually. No process was killed.'}
    $after=Get-Item -LiteralPath $exe.FullName -ErrorAction Stop
    $result=@{schema_version=1;purpose='supplemental_installation_observation_not_smoke';source_commit=$SourceCommit;installer_sha256=$InstallerSha256;finished_at=[DateTime]::UtcNow.ToString('o');installer_exit_code=$process.ExitCode;after_version=$after.VersionInfo.ProductVersion;after_exe_sha256=(Get-FileHash -LiteralPath $after.FullName -Algorithm SHA256).Hash.ToLowerInvariant();human_smoke_performed=$false}
    [IO.File]::WriteAllText((Join-Path $backup 'upgrade-observation.json'),($result|ConvertTo-Json),(New-Object Text.UTF8Encoding($false)))
    if($process.ExitCode -ne 0 -or $result.after_version -cne '1.0.0'){throw 'Upgrade did not establish the expected installed version; retain evidence and investigate.'}
    Write-Output 'Installer completed; no application smoke or acceptance is claimed. Follow the runbook.'
}
if($MyInvocation.InvocationName -ne '.') { $ErrorActionPreference='Stop';Invoke-SupplementalUpgrade }
