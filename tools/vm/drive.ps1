# drive.ps1 [-User <name>] <keys.ps1 arguments...>: keys.ps1 in the desktop session, waiting for it (at most 180 s);
# prints its log. -User: whose desktop session (default: the user running this, who must be the one signed in at the
# console). A plain param block, so every other argument stays in $args for keys.ps1.
param([string]$User = $env:USERNAME)
$here = Split-Path -Parent $MyInvocation.MyCommand.Path
$ps = "$env:WINDIR\System32\WindowsPowerShell\v1.0\powershell.exe"
$a = ($args | ForEach-Object { if ($_ -match '\s') { "`"$_`"" } else { $_ } }) -join ' '
& (Join-Path $here 'insession.ps1') -Exe $ps -Args "-NoProfile -ExecutionPolicy Bypass -WindowStyle Hidden -File `"$here\keys.ps1`" $a" -Wait -TimeoutSec 180 -User $User
Get-Content (Join-Path $here 'keys.log')
