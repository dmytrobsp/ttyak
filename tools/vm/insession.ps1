# Run a command in the desktop session of the user signed in at the console (ssh sessions have no desktop):
#   insession.ps1 -Exe <path> [-Args "<args>"] [-Wait] [-TimeoutSec n] [-User <name>]
# A one-off scheduled task with -LogonType Interactive, removed afterwards. -User defaults to the user running this
# script (over ssh: the test user, who is also the one signed in automatically). -Wait blocks until it exits (for
# captures; at most TimeoutSec); without it, the program keeps running.
param([Parameter(Mandatory)][string]$Exe, [string]$Args = '', [switch]$Wait, [int]$TimeoutSec = 120,
      [string]$User = $env:USERNAME)
$ErrorActionPreference = 'Stop'
$name = 'ttyak-' + [guid]::NewGuid().ToString('N').Substring(0, 8)
$a = if ($Args) { New-ScheduledTaskAction -Execute $Exe -Argument $Args -WorkingDirectory (Split-Path $Exe) } else { New-ScheduledTaskAction -Execute $Exe -WorkingDirectory (Split-Path $Exe) }
$p = New-ScheduledTaskPrincipal -UserId "$env:COMPUTERNAME\$User" -LogonType Interactive -RunLevel Limited
$s = New-ScheduledTaskSettingsSet -AllowStartIfOnBatteries -DontStopIfGoingOnBatteries -ExecutionTimeLimit (New-TimeSpan -Hours 8)
Register-ScheduledTask -TaskName $name -Action $a -Principal $p -Settings $s | Out-Null
Start-ScheduledTask -TaskName $name
if ($Wait) {
    $t0 = Get-Date
    Start-Sleep -Milliseconds 300
    while ((Get-ScheduledTask -TaskName $name).State -eq 'Running' -and ((Get-Date) - $t0).TotalSeconds -lt $TimeoutSec) { Start-Sleep -Milliseconds 200 }
}
Unregister-ScheduledTask -TaskName $name -Confirm:$false
