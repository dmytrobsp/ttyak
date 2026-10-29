# Driving the Windows test machine

PowerShell helpers that run on the Windows test machine and let an agent use its **desktop** from an ssh session: start
a program in the signed-in user's desktop session, press keys and click, and capture that machine's screen. An ssh
session on Windows runs in a services session with no desktop: no windows to see, no notifications, no screen to
capture. These scripts get around that with a one-off scheduled task that runs in the console user's session.

What the Windows machine needs, and the rules for using it (it may be shared, restore it as found, never capture
the maintainer's own screen), are in [docs/agents/setup.md](../../docs/agents/setup.md#a-windows-machine) and
[docs/agents/live-testing.md](../../docs/agents/live-testing.md#test-machines). The host alias, the user and the work
folder come from `AGENTS.local.md`; nothing here names them.

- `autologon.ps1 [-User <name>]` turns on automatic sign-in of the test user, so the machine has a desktop session
  after each start. Run it once, by hand, in an elevated PowerShell at the machine: it asks for the password at a
  hidden prompt and checks it first; the undo commands are at its top. Windows keeps that password in the registry in
  plain text: a test machine and a password used nowhere else only.
- `insession.ps1 -Exe <path> [-Args "<args>"] [-Wait] [-TimeoutSec n] [-User <name>]` runs a program in the desktop
  session through a one-off scheduled task (`-LogonType Interactive`), removed afterwards. `-User` defaults to the
  user running it; `-Wait` blocks until the program exits (at most `-TimeoutSec`, default 120).
- `shot.ps1 -Out <png>` captures the whole desktop (DPI-aware). It works only inside the desktop session: run it
  through `insession.ps1 -Wait`, or use `capture.ps1`.
- `capture.ps1 -Name <png> [-User <name>]`: `shot.ps1` through `insession.ps1`, into `shots\` next to the scripts.
- `wakeshot.ps1 -Out <png>`: `shot.ps1` after waking the display (an idle machine turns it off and captures freeze on
  its last frame); run it through `insession.ps1 -Wait`.
- `drive.ps1 [-User <name>] <keys.ps1 arguments>`: `keys.ps1` in the desktop session, waiting for it (at most 180 s);
  prints its log.
- **Whose desktop**: `insession.ps1`, `capture.ps1` and `drive.ps1` run in the session of `-User`, by default the user
  the ssh session runs as. That assumes the ssh user is the one signed in at the console (the test user with automatic
  sign-in); otherwise pass `-User <the signed-in user>`, or nothing shows.
- `keys.ps1 -Exe <program> [-Open <file> | -ArgLine "<args>"] [-Keys "<s1>;<s2>;..."] [-WaitSec n] [-Shot <png>]
  [-Leave]` starts the program, brings it to the front, runs the steps 1.5 s apart, optionally captures, then stops it
  (unless `-Leave`). Logs go to `keys.log`, captures and lists to `shots\`. A step is SendKeys text or one of:
  - `CLICK x,y`, `RCLICK x,y[,ms]`, `DCLICK x,y[,ms]` (screen pixels; ms: the gap between the clicks), `MOVE x,y`;
  - `DRAG x1,y1,x2,y2` (or `HOLD x1,y1,x2,y2`, later `UP`), `WHEEL x,y,n` (n notches, + up);
  - `KEYDOWN ctrl` / `KEYUP ctrl` (also shift, alt), `CHORD ctrl+shift+P` (keys with their scan codes);
  - `SIZE w,h` (the front window restored and sized), `CLOSE` (WM_CLOSE to the front window);
  - `SHOT name.png` (a capture now), `WINS name.txt` (the program's top-level windows).
- `cleanup.ps1 -Dir <folder>` stops the ttyak processes started from the folder `-Dir` (mandatory, no default: the
  literal unit folder, never `%USERPROFILE%\ttyak-test`, which holds every unit's folder and so another unit's ttyak)
  and stuck `keys.ps1` runs of this copy; other people's programs are left alone.
- `test_vm_scripts.py` (run here, not there): every script is plain ASCII and parses with PowerShell's parser.

## Using them

On the shared Windows VM, only under a box claim and only through the claim tool named in `AGENTS.local.md`
(`<tool>`, with `BOX` and your claim id `ID`: live-testing's
[Test machines](../../docs/agents/live-testing.md#test-machines)); never a raw `ssh` or `scp` to the machine. One
command per call, every path written out: `<unit>` is your unit's folder under `ttyak-test` (outside a milestone,
your task name), `<user>` is in `AGENTS.local.md`, `<checkout>` is the absolute path of the tree you work in (in a
milestone, your worktree) and `<evidence>` an absolute folder of yours (in a milestone, the unit's evidence folder).

```sh
# your unit folder, then the scripts into its vm folder (`push` places a folder as scp -r would)
<tool> run BOX ID -- "if not exist C:\Users\<user>\ttyak-test\<unit> mkdir C:\Users\<user>\ttyak-test\<unit>"
<tool> push BOX ID <checkout>/tools/vm 'C:\Users\<user>\ttyak-test\<unit>\vm'
# a capture of the machine's desktop, copied back
<tool> run BOX ID -- "powershell -NoProfile -ExecutionPolicy Bypass \
    -File C:\Users\<user>\ttyak-test\<unit>\vm\capture.ps1 -Name desk.png"
<tool> pull BOX ID 'C:\Users\<user>\ttyak-test\<unit>\vm\shots\desk.png' <evidence>/desk.png
# at the end: stop what ran from the unit folder, remove the folder, check it is gone
<tool> run BOX ID -- "powershell -NoProfile -ExecutionPolicy Bypass \
    -File C:\Users\<user>\ttyak-test\<unit>\vm\cleanup.ps1 -Dir C:\Users\<user>\ttyak-test\<unit>"
<tool> run BOX ID -- "rmdir /s /q C:\Users\<user>\ttyak-test\<unit>"
<tool> run BOX ID -- "if exist C:\Users\<user>\ttyak-test\<unit> echo left"
```

- `run` hands its line to the box's shell, cmd; anything with several steps goes into a `.ps1` file that is pushed
  over and run with `-File`, not through nested quotes.
- Look at every capture before relying on it; it shows the Windows machine's desktop only.
- `cleanup.ps1` always gets `-Dir` with the literal unit folder (it has no default), before the folder is removed.
- The unit folder holds everything pushed (the `vm` folder, its `shots`): removing it is live-testing's "Restore it as
  found". A Windows clone has commands of its own for this (`<tool> instance push`, `exec --desktop`, `shot`:
  live-testing's [Windows in a clone](../../docs/agents/live-testing.md#windows-in-a-clone)), and is thrown away.

## Where they came from

Copied on 2026-10-26 from the maintainer's public `fastview-rea` repository on GitHub (`tools/vm/`, commit `a204dd2`;
same author, same licences), where they drove the same Windows test machine for another project. Changed here: the test
user is a parameter (default: the user running the script) instead of a fixed name; the scheduled tasks are named
`ttyak-…`; `cleanup.ps1` stops only ttyak processes from the work folder and only this copy's `keys.ps1` runs (the
machine is shared); `wakeshot.ps1` nudges the cursor at the screen's right edge instead of fixed pixels; that project's
own options (`-Soft`, `-SoftLog`) and its program's name are gone; the files are plain ASCII (Windows PowerShell 5.1
reads a file without a byte-order mark in the system's code page).

**Verified**: the originals were used LIVE on the Windows test machine in that project, and the same technique (an
interactive scheduled task, injected input, captures of the machine's desktop) for ttyak's Windows notification checks
in 0.12.1 ([history](../../docs/history/0.12.md)). These copies are UNIT only:
`python3 -m unittest tools/vm/test_vm_scripts.py` checks that every script is plain ASCII and parses with PowerShell's
own parser (pwsh, or pwsh in podman). They have NOT been run on Windows since the changes above, nor have the commands
under "Using them" in this form; `keys.ps1` with ttyak itself (a console program, whose window belongs to the console
host) is NOT tried.
