# keys.ps1 -Exe <program> [-Open <file> | -ArgLine "<args>"] -Keys "<k1>;<k2>..." [-WaitSec n] [-Shot name.png] [-Leave]
# In the desktop session: start the program (with a file), bring it to the
# front, run each ';'-separated step 1.5 s apart, optionally capture the
# screen, then close it (unless -Leave). A step is SendKeys text, or
# "CLICK x,y" / "RCLICK x,y" / "DCLICK x,y" (screen pixels), "MOVE x,y" (the
# pointer there in five steps, no button), "DRAG x1,y1,x2,y2"
# (a left-button drag in ten steps; "HOLD x1,y1,x2,y2" the same left held,
# then "UP"), "WHEEL x,y,n"
# (n notches, + up), "KEYDOWN ctrl" / "KEYUP ctrl" (shift, alt: a modifier
# held across steps), "SIZE w,h" (the front window restored, w x h at 0,0),
# "CLOSE" (WM_CLOSE to the front window), "SHOT name.png" (a capture now),
# "WINS name.txt" (the program's top-level windows: handle, owner, visible,
# enabled, class, title - into shots/).
# Logs next to itself (keys.log); captures go to shots/ next to itself.
param([Parameter(Mandatory)][string]$Exe, [string]$Open = '', [string]$ArgLine = '', [string]$Keys = '', [int]$WaitSec = 6, [string]$Shot = '', [switch]$Leave)
$here = Split-Path -Parent $MyInvocation.MyCommand.Path
$log = Join-Path $here 'keys.log'
Add-Type @'
using System; using System.Runtime.InteropServices;
public static class In {
  [DllImport("user32.dll")] public static extern bool SetProcessDPIAware();
  [DllImport("user32.dll")] public static extern bool SetCursorPos(int x, int y);
  [DllImport("user32.dll")] public static extern void mouse_event(uint f, uint dx, uint dy, int data, UIntPtr extra);
  [DllImport("kernel32.dll")] public static extern uint SetThreadExecutionState(uint flags);
  [DllImport("user32.dll")] public static extern void keybd_event(byte vk, byte scan, uint flags, UIntPtr extra);
  [DllImport("user32.dll")] public static extern uint MapVirtualKey(uint code, uint type);
  [DllImport("user32.dll")] public static extern IntPtr GetForegroundWindow();
  [DllImport("user32.dll")] public static extern bool PostMessage(IntPtr h, uint msg, IntPtr w, IntPtr l);
  [DllImport("user32.dll")] public static extern bool ShowWindow(IntPtr h, int cmd);
  [DllImport("user32.dll")] public static extern bool MoveWindow(IntPtr h, int x, int y, int w, int ht, bool repaint);
  public delegate bool EnumProc(IntPtr h, IntPtr l);
  [DllImport("user32.dll")] public static extern bool EnumWindows(EnumProc f, IntPtr l);
  [DllImport("user32.dll")] public static extern uint GetWindowThreadProcessId(IntPtr h, out uint pid);
  [DllImport("user32.dll")] public static extern IntPtr GetWindow(IntPtr h, uint cmd);
  [DllImport("user32.dll")] public static extern bool IsWindowVisible(IntPtr h);
  [DllImport("user32.dll")] public static extern bool IsWindowEnabled(IntPtr h);
  [DllImport("user32.dll", CharSet = CharSet.Unicode)] public static extern int GetClassName(IntPtr h, System.Text.StringBuilder s, int n);
  [DllImport("user32.dll", CharSet = CharSet.Unicode)] public static extern int GetWindowText(IntPtr h, System.Text.StringBuilder s, int n);
  public static string List(uint pid) {
    var o = new System.Text.StringBuilder();
    EnumWindows((h, l) => {
      uint p; GetWindowThreadProcessId(h, out p);
      if (p == pid) {
        var c = new System.Text.StringBuilder(256); GetClassName(h, c, 256);
        var t = new System.Text.StringBuilder(256); GetWindowText(h, t, 256);
        o.AppendFormat("{0:X} owner={1:X} vis={2} en={3} class={4} title={5}\n", (long)h, (long)GetWindow(h, 4), IsWindowVisible(h), IsWindowEnabled(h), c, t);
      }
      return true;
    }, IntPtr.Zero);
    return o.ToString();
  }
}
'@
[In]::SetProcessDPIAware() | Out-Null
# Keep the display on while the steps run (ES_CONTINUOUS | ES_DISPLAY_REQUIRED):
# an idle VM turns it off and the captures freeze on its last frame.
[In]::SetThreadExecutionState([uint32]2147483650) | Out-Null
function Click([int]$x, [int]$y, [uint32]$down, [uint32]$up, [int]$times = 1, [int]$gap = 60) {
    [In]::SetCursorPos($x, $y) | Out-Null; Start-Sleep -Milliseconds 120
    # A gap other than 60 ms (DCLICK x,y,ms) also holds each press 40 ms, as a hand does.
    $hold = if ($gap -ne 60) { 40 } else { 0 }
    for ($i = 0; $i -lt $times; $i++) { [In]::mouse_event($down, 0, 0, 0, [UIntPtr]::Zero); if ($hold) { Start-Sleep -Milliseconds $hold }; [In]::mouse_event($up, 0, 0, 0, [UIntPtr]::Zero); Start-Sleep -Milliseconds $gap }
}
"start $(Get-Date -f s) $Exe $Open" | Out-File $log -Encoding utf8
New-Item -ItemType Directory -Force (Join-Path $here 'shots') | Out-Null
$wsh = New-Object -ComObject WScript.Shell
$p = if ($ArgLine) { Start-Process -FilePath $Exe -ArgumentList $ArgLine -PassThru } elseif ($Open) { Start-Process -FilePath $Exe -ArgumentList "`"$Open`"" -PassThru } else { Start-Process -FilePath $Exe -PassThru }
Start-Sleep -Seconds $WaitSec
$ok = $wsh.AppActivate($p.Id)
"activated $ok pid $($p.Id)" | Out-File $log -Append -Encoding utf8
foreach ($k in ($Keys -split ';')) {
    if (-not $k) { Start-Sleep -Milliseconds 1500; continue }
    if ($k -match '^(R|D)?CLICK (\d+),(\d+)(?:,(\d+))?$') {
        $x = [int]$Matches[2]; $y = [int]$Matches[3]
        switch ($Matches[1]) { 'R' { Click $x $y 0x8 0x10 1 $(if ($Matches[4]) { [int]$Matches[4] } else { 60 }) } 'D' { Click $x $y 0x2 0x4 2 $(if ($Matches[4]) { [int]$Matches[4] } else { 60 }) } default { Click $x $y 0x2 0x4 } }
    } elseif ($k -match '^MOVE (\d+),(\d+)$') {
        $x = [int]$Matches[1]; $y = [int]$Matches[2]
        for ($i = 4; $i -ge 0; $i--) { [In]::SetCursorPos($x - 2 * $i, $y) | Out-Null; Start-Sleep -Milliseconds 40 }
    } elseif ($k -match '^(DRAG|HOLD) (\d+),(\d+),(\d+),(\d+)$') {
        $x1 = [int]$Matches[2]; $y1 = [int]$Matches[3]; $x2 = [int]$Matches[4]; $y2 = [int]$Matches[5]
        [In]::SetCursorPos($x1, $y1) | Out-Null; Start-Sleep -Milliseconds 120
        [In]::mouse_event(0x2, 0, 0, 0, [UIntPtr]::Zero); Start-Sleep -Milliseconds 120
        for ($i = 1; $i -le 10; $i++) {
            [In]::SetCursorPos([int]($x1 + ($x2 - $x1) * $i / 10), [int]($y1 + ($y2 - $y1) * $i / 10)) | Out-Null; Start-Sleep -Milliseconds 60
        }
        Start-Sleep -Milliseconds 200
        if ($Matches[1] -eq 'DRAG') { [In]::mouse_event(0x4, 0, 0, 0, [UIntPtr]::Zero) }
    } elseif ($k -match '^KEY(DOWN|UP) (ctrl|shift|alt)$') {
        # A modifier held across the next steps (KEYDOWN ctrl; WHEEL ...; KEYUP ctrl).
        $v = @{ ctrl = 0x11; shift = 0x10; alt = 0x12 }[$Matches[2]]
        [In]::keybd_event([byte]$v, [byte][In]::MapVirtualKey($v, 0), $(if ($Matches[1] -eq 'UP') { 2 } else { 0 }), [UIntPtr]::Zero)
    } elseif ($k -match '^CHORD ((?:\w+\+)*)(\w+)$') {
        # Keys with their scan codes (a real keyboard's; SendKeys' Ctrl+letter
        # may come without one): CHORD ctrl+shift+P, CHORD F4, CHORD ctrl+0x4F.
        $mods = @($Matches[1].TrimEnd('+').Split('+') | Where-Object { $_ })
        $name = $Matches[2]
        $vkOf = @{ ctrl = 0x11; shift = 0x10; alt = 0x12 }
        $vk = if ($name -match '^0x[0-9a-fA-F]+$') { [Convert]::ToInt32($name, 16) } elseif ($name.Length -eq 1) { [int][char]$name.ToUpper() } elseif ($name -match '^F(\d+)$') { 0x6F + [int]$Matches[1] } else { 0 }
        foreach ($m in $mods) { $v = $vkOf[$m.ToLower()]; [In]::keybd_event([byte]$v, [byte][In]::MapVirtualKey($v, 0), 0, [UIntPtr]::Zero) }
        [In]::keybd_event([byte]$vk, [byte][In]::MapVirtualKey($vk, 0), 0, [UIntPtr]::Zero); Start-Sleep -Milliseconds 30
        [In]::keybd_event([byte]$vk, [byte][In]::MapVirtualKey($vk, 0), 2, [UIntPtr]::Zero)
        [array]::Reverse($mods)
        foreach ($m in $mods) { $v = $vkOf[$m.ToLower()]; [In]::keybd_event([byte]$v, [byte][In]::MapVirtualKey($v, 0), 2, [UIntPtr]::Zero) }
    } elseif ($k -match '^SIZE (\d+),(\d+)$') {
        # The front window restored and sized w x h at the screen's corner.
        $h = [In]::GetForegroundWindow()
        [In]::ShowWindow($h, 9) | Out-Null; Start-Sleep -Milliseconds 300
        [In]::MoveWindow($h, 0, 0, [int]$Matches[1], [int]$Matches[2], $true) | Out-Null
    } elseif ($k -eq 'CLOSE') {
        # WM_CLOSE to the front window (the program's own close path; .NET's
        # main-window lookup misses some windows).
        [In]::PostMessage([In]::GetForegroundWindow(), 0x10, [IntPtr]::Zero, [IntPtr]::Zero) | Out-Null
    } elseif ($k -eq 'UP') {
        [In]::mouse_event(0x4, 0, 0, 0, [UIntPtr]::Zero)
    } elseif ($k -match '^WHEEL (\d+),(\d+),(-?\d+)$') {
        [In]::SetCursorPos([int]$Matches[1], [int]$Matches[2]) | Out-Null; Start-Sleep -Milliseconds 120
        $n = [int]$Matches[3]; $step = if ($n -gt 0) { 120 } else { -120 }
        for ($i = 0; $i -lt [Math]::Abs($n); $i++) { [In]::mouse_event(0x800, 0, 0, $step, [UIntPtr]::Zero); Start-Sleep -Milliseconds 80 }
    } elseif ($k -match '^WINS (.+)$') {
        [In]::List([uint32]$p.Id) | Out-File (Join-Path (Join-Path $here 'shots') $Matches[1]) -Encoding utf8
    } elseif ($k -match '^SHOT (.+)$') {
        & (Join-Path $here 'shot.ps1') -Out (Join-Path (Join-Path $here 'shots') $Matches[1])
    } else {
        $wsh.SendKeys($k)
    }
    "step $k" | Out-File $log -Append -Encoding utf8
    Start-Sleep -Milliseconds 1500
}
if ($Shot) { & (Join-Path $here 'shot.ps1') -Out (Join-Path (Join-Path $here 'shots') $Shot) }
if (-not $Leave) { Stop-Process -Id $p.Id -Force -ErrorAction SilentlyContinue }
"done $(Get-Date -f s)" | Out-File $log -Append -Encoding utf8
