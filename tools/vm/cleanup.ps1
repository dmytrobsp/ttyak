# cleanup.ps1 -Dir <folder>: stop the ttyak processes started from the work folder -Dir (mandatory, no default: the
# literal unit folder, never the shared %USERPROFILE%\ttyak-test that holds every unit's folder) and stuck keys.ps1
# runs of this copy of the scripts. Others' programs and others' keys.ps1 runs (the machine may be shared) are left
# alone. Prints how many of ours are left.
param([Parameter(Mandatory)][string]$Dir)
$here = Split-Path -Parent $MyInvocation.MyCommand.Path
$ours = { Get-Process ttyak -ErrorAction SilentlyContinue | Where-Object { $_.Path -and $_.Path.StartsWith($Dir.TrimEnd('\') + '\', [StringComparison]::OrdinalIgnoreCase) } }
& $ours | Stop-Process -Force
Get-CimInstance Win32_Process -Filter "Name='powershell.exe'" | Where-Object { $_.CommandLine -like "*$here\keys.ps1*" } | ForEach-Object { Stop-Process -Id $_.ProcessId -Force }
"ttyak left in ${Dir}: " + (& $ours | Measure-Object).Count
