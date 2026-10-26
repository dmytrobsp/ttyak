# wakeshot.ps1 -Out <png>: wake the display (it turns off when the machine idles and the captures then freeze on its
# last frame), nudge the cursor near the screen's right edge, capture. Run it through insession.ps1 -Wait.
param([Parameter(Mandatory)][string]$Out)
Add-Type -AssemblyName System.Windows.Forms
Add-Type @'
using System; using System.Runtime.InteropServices;
public static class W {
  [DllImport("kernel32.dll")] public static extern uint SetThreadExecutionState(uint flags);
  [DllImport("user32.dll")] public static extern bool SetCursorPos(int x, int y);
  [DllImport("user32.dll")] public static extern bool SetProcessDPIAware();
}
'@
[W]::SetProcessDPIAware() | Out-Null
[W]::SetThreadExecutionState([uint32]2147483650) | Out-Null
$b = [System.Windows.Forms.SystemInformation]::VirtualScreen
$x = $b.Right - 10; $y = $b.Top + [int]($b.Height / 2)
[W]::SetCursorPos($x - 5, $y) | Out-Null; Start-Sleep -Milliseconds 300; [W]::SetCursorPos($x, $y + 5) | Out-Null
Start-Sleep -Seconds 3
& (Join-Path (Split-Path -Parent $MyInvocation.MyCommand.Path) 'shot.ps1') -Out $Out
