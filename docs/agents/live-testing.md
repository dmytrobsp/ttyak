# Live testing

How to run ttyak against Slack, on this machine and on test machines, without touching the maintainer's real accounts,
files, screen, clipboard, microphone or time. Read it before any live check. The rules in short are in
[AGENTS.md](../../AGENTS.md#hard-rules); the names, hosts and paths are in `AGENTS.local.md` — without that file, no
live work: ask the maintainer.

The helpers assume Linux with Hyprland, kitty and PipeWire, as on the maintainer's machine; `AGENTS.local.md` says what
this machine has; on any other desktop see [Other desktops](#other-desktops).

## Before every live check

1. `cargo build` (a `cargo test` alone leaves a stale `target/debug/ttyak`).
2. Close your old test windows and wait until none remain (an open one keeps its kitty socket; a new window then fails
   with "Invalid listen_on" and keys go to the wrong window).
3. Claim the Slack accounts you need, so that nobody else — another agent, on this machine or a test machine — uses
   them meanwhile: `python3 "$REPO/tools/accounts/accounts.py" claim A --purpose "<what>"` (the same for B; `$REPO` is
   the main clone holding `.accounts/`, also when you work in a worktree; its absolute path, as in
   [A fake home](#a-fake-home)); keep each claim id. A claim lasts 30 minutes: `renew A ID` during longer checks, or
   claim with `--lease 2h` (a lapsed claim is broken by the next waiter, which also removes the installed copy while
   ttyak still runs). Run `claim` in the background, or with `--report-every 5m` and re-run its `resume:` line, so a
   shell timeout does not kill it while it is queued.
   Waiting for a claim is not a live observation, so the 10-minute cap does not apply (as with box claims below).
   If the check needs a test machine, claim it now (see [Test machines](#test-machines)) and keep the claim id; all
   work on the box goes through the claim tool.
4. Check the account's state before blaming the code: a do-not-disturb schedule or snooze silences notifications
   (`--profile <A> ctl api dnd.info`; see [Do-not-disturb](#do-not-disturb-on-the-test-accounts)).
5. Decide what the check must show and cap its waits (not a wait for a claim) at about
   10 minutes. Only run experiments whose answer changes what gets built; report interim status rather than sit in a
   long wait.
6. Use absolute paths in every command (an agent shell's working directory resets between commands), and keep the
   fake home, recordings and captures in the scratch folder — a reboot wipes it, so record results elsewhere.

## The test workspace and profiles

- **A Slack workspace with two test accounts, A and B**, exists only for developing ttyak. Its name is in
  `AGENTS.local.md`; the accounts, their profile names, roles and rules are in the account ledger (`.accounts/`,
  git-ignored): `python3 "$REPO/tools/accounts/accounts.py" list` shows them (rules such as the DND restore and never
  signing out). B plays the other person: posts, reacts, rings, sends test video
  (`ttyak --profile <B> ctl post …`, `ctl huddle-media …`).
- **Its plan**: the current one is on Slack's **free** plan, but Slack reports `team.plan = "std"` for it, so ttyak's
  paid-plan check — like Slack's own client — treats paid features (VIPs, for one) as available there. A check that
  depends on the plan says so in its record.
- **Every ttyak command names a test profile** with `--profile`. A bare `ttyak` (or a bare `ctl`) uses the machine's
  default profile, which may be the maintainer's real work account. Never use a profile you were not given.
- **The maintainer's own workspaces**: never post, react, upload, sign in or out there. Read-only use only where
  `AGENTS.local.md` allows it, with the test window unfocused (focus marks things read), and their names and ids never
  in a tracked file.
- **Posts in the test workspace** need not be deleted: it is a throwaway workspace. Drafts and files a test leaves on
  disk are still cleaned up.
- **Never sign the test accounts out** of Slack; every email-code sign-in leaves one more session there, harmlessly.
  Signing A and B in on a new machine is in [setup.md](setup.md#5-the-test-workspace).
- One Slack user always gets the same huddle attendee id: a second session of the same account kicks the first, so live
  huddle tests top out at two people with a camera and a share.

### Do-not-disturb on the test accounts

DND settings may be written on the test accounts only, never on another account. Snapshot first
(`ctl api users.prefs.get`, the `dnd_*` keys; `ctl api dnd.info`), then restore exactly: lift with `dnd.endDnd` /
`dnd.endSnooze`, restore a snooze with `dnd.setSnooze num_minutes=<to the old end>`.

## A fake home

Live checks never write the maintainer's real folders (`~/.config/ttyak`, `~/.local/share/ttyak`, `~/.cache/ttyak`).
Run ttyak under a fake home in the scratch folder:

```sh
set -euo pipefail
REPO=<the clone>                                   # its absolute path
FAKE=<scratch>/home                                # a literal path in your scratch folder
mkdir -p "$FAKE"/.config "$FAKE"/.local/share "$FAKE"/.cache
chmod 700 "$FAKE" "$FAKE"/.config "$FAKE"/.local "$FAKE"/.cache
python3 "$REPO/tools/accounts/accounts.py" install A ID --home "$FAKE"   # ID: your claim id
cat > "$FAKE/.config/ttyak/settings.toml" <<'EOF'
[notifications]
sound = "off"
sound_volume = 0
[huddle]
join_sound = "off"
someone_joined = "off"
someone_left = "off"
ring_sound = "off"
alone_sound = "off"
EOF
HOME="$FAKE" XDG_CONFIG_HOME="$FAKE/.config" XDG_DATA_HOME="$FAKE/.local/share" XDG_CACHE_HOME="$FAKE/.cache" \
  "$REPO/target/debug/ttyak" --profile <A>
```

Set `HOME` and `XDG_*` only on the ttyak command line, as above, never exported before running `accounts.py`: the
tool counts `$HOME` and `$XDG_*_HOME` as the real folders and refuses them.

- The claim's `install` copies the session of an account you hold into the fake home (0600), and `release` removes
  the copies it installed (`release --keep` keeps them). Never print a session file — it holds tokens.
  A claim lasts 30 minutes: `renew A ID` during longer checks, or claim with `--lease 2h`.
  Quit ttyak (Ctrl+Q), then end the check with `python3 "$REPO/tools/accounts/accounts.py" release A ID` (then B),
  which also hands the account over to the next claim; never release while ttyak still runs with the copy.
  Delete the fake home after, only once it is checked to lie in your scratch folder:
  `case "$FAKE" in <scratch>/?*) rm -rf -- "${FAKE:?}" ;; *) echo "not in the scratch folder" >&2 ;; esac`.
- A profile with two workspaces = the `workspaces` lists of two `session.json` files combined (in Python, never
  printed) from the installed copies inside the fake home into a temporary profile with a name of its own
  (it goes with the fake home: delete it with it).
- The log is `<cache>/ttyak/<profile>/ttyak.log`; `TTYAK_LOG=ttyak=debug` for detail (exactly that, not `debug`).
- Debug builds read test variables: `TTYAK_FAIL=<method>,…` (those Web API calls fail), `TTYAK_DAYS_LATER=N`,
  `TTYAK_HUDDLE_DROP=N` (see [CONTRIBUTING.md](../../CONTRIBUTING.md)); any build reads `TTYAK_NOTIFY_ROUTE=system`
  (notifications through the system's notifier, see the wiki's [Settings](../wiki/Settings.md)).
- If a test really must touch a real folder (a draft, `settings.toml`), back the file up first and restore it; never
  remove a folder, and never open the maintainer's own drafts. Quit ttyak with Ctrl+Q before closing its window, or a
  cleared composer leaves a stale draft.

## Test windows

Test windows never appear on the maintainer's screen. Start them on a hidden special workspace with a class and a
kitty socket of your own:

```sh
hyprctl dispatch exec "[workspace special:<name> silent] kitty --class <class> -o allow_remote_control=yes \
  --listen-on unix:@<socket> -o window_alert_on_bell=no sh -c '<command>; exec sleep 600'"
```

- Drive it with `kitty @ --to unix:@<socket> send-key …` / `send-text …`; read it with
  `kitty @ --to unix:@<socket> get-text --extent screen` (`--ansi | cat -v` shows colours and bold — a screenshot can
  hide them).
- Sleep ~0.2 s between `send-key` and `send-text` (they can arrive out of order) and ~0.5 s after the last key before
  reading. `send-key G` arrives as `g`: send uppercase letters with `send-text`.
- Mouse: SGR reports as text, 1-based cells: left click `$'\e[<0;COL;ROWM\e[<0;COL;ROWm'`, right click `<2;…`,
  motion `<35;…`.
- Focus: a new window sends no focus-out until it had a focus-in; fake it with `send-text --match all $'\e[O'`
  (unfocused) and `$'\e[I'` (focused).
- Focus traps: Escape in a composer moves focus to the sidebar; from a composer use Escape then Tab to reach the
  messages. Read the screen before destructive keys (`d`, `y`) and before every Enter in F6 (it opens on Availability,
  where Enter edits the real Slack status).
- Another size: `kitty @ set-font-size 6` gives about 170×44 cells (`0` restores); the default is a "narrow" ~95 columns.
- `tools/launch.sh <A> [<B's name>]` opens a *visible* `ttyak-test` window on your A profile (it refuses without a
  profile) and opens the DM with B: use it only while the maintainer is watching.
- **No screenshots of the real screen** (grim, `tools/shot.sh` grab a screen region and have captured the maintainer's
  own windows). Use `get-text`. If a picture is truly needed, focus the test window, check it is the active one, look
  at the picture before relying on it, and delete any capture that shows something else.
- Each agent uses its own class and socket; never close a window you did not open.
- Processes: kill only the PIDs you started (`$!`). Never `pkill -f` (it killed an agent's own shell once), never a
  kill by name: the maintainer may be running their own ttyak. A background process dies with the shell command that
  started it — keep a multi-step test that needs one inside a single command (`… & P=$!; …; wait $P`).
- A shell probe that reads the tty under `timeout` needs `timeout --foreground`.

Other terminals have no input tool here: test what shows on start.

```sh
foot --app-id <class> sh -c '…'
# WezTerm (a flatpak whose Wayland window does not map: X11 only); close it by its PID
flatpak run --socket=x11 org.wezfurlong.wezterm --config enable_wayland=false start --always-new-process \
  --class <class> -- sh -c '…'
xterm -ti vt340 …     # sixel only as a VT340; X resources with -xrm '*res: v'
```

A bare system: run the binary in a podman Debian container with only `libasound2t64`, the fake home (with the sessions
`install` put there) mounted read-only, the cache folder from the scratch folder.

## Other desktops

The recipes above assume Hyprland, kitty and PipeWire. On any other desktop (GNOME, KDE, sway, an X11 session, macOS)
the rules are the same; only the means change.

- **Test windows never appear on the maintainer's screen.** Without a hidden workspace, give the test a display of its
  own:
  - a headless wlroots compositor — `WLR_BACKENDS=headless WLR_LIBINPUT_NO_DEVICES=1 sway -c <config>` (`cage` the
    same way) — with a terminal such as foot inside, its socket in the scratch folder, driven with `swaymsg`;
  - for X11 terminals, Xvfb on a display number of your own (`Xvfb :99 -screen 0 1280x720x24`, then `DISPLAY=:99`),
    keys and clicks with `xdotool` under that `DISPLAY`;
  - either one inside a podman container, which keeps it away from the session altogether.

  kitty inside such a display still answers `kitty @ … get-text`, which is how the screen is read. A capture of the
  virtual display is allowed (it shows only the test); a capture of the real screen never is.
- **No microphone, camera or screen capture; no sound**: the fake home switches sounds off; without PipeWire there is
  no stream to record, so a sound check waits for a machine that has one.
- **Clipboard**: a terminal inside its own display should write that display's clipboard; check that the
  maintainer's clipboard is unchanged after the test, and save and restore it anyway.
- **If none of this is possible** (no compositor or Xvfb can be installed, a macOS desktop without a separate user
  session): do not run the live check. Write what was checked UNIT and NOT, and ask the maintainer.

What was tried: in 0.12.0 (AT10, [history/0.12.md](../history/0.12.md)) a podman Debian container ran a headless sway
with foot, and Xvfb with openbox and xterm, to check that ttyak raises its window; in the C7 follow-up
([history/0.1-0.9.md](../history/0.1-0.9.md)) Xvfb `:99` stood in for the screen in an X11 screen share. A whole live
check against Slack — keys in, screen read back — inside such a display is NOT tried yet; the commands above are a
starting point, not a recipe.

## Clipboard

A test that copies (OSC 52, `y`, a hand-off) overwrites the maintainer's clipboard. Save it first
(`wl-paste --list-types`; text with `wl-paste -n --type text/plain`) and restore it with `wl-copy` after. Clipboard
input for tests is synthetic: `wl-copy` text, `-t image/png < file` (a picture from `ffmpeg -f lavfi -i testsrc`), or
`-t text/uri-list` with `file://` lines. With `kitty -e ttyak` kitty exits with ttyak and drops what it owned: keep the
window alive when testing what happens after quit.

## Sound, microphone, camera, screen

- **Microphone: never captured.** A misrouted capture always falls back to the real microphone (it happened twice:
  once through a routing variable that was ignored, once by a mute toggled while probing keys in a call). Verify the
  capture path with unit tests and the send path with `ctl huddle-media --tone`. A huddle UI test first points the
  microphone at a silent or null device (F6 → Audio & video → Microphone, in the fake home) or avoids every key that
  can reach mute. A live microphone test needs the maintainer's agreement and a device chosen by name that errors
  instead of falling back.
- **Sound only when agreed.** The fake home switches sounds off. To check what ttyak plays, record its own playback
  stream, never a source: `tools/streamrec.py`, then `tools/tonecheck.py` to tell sounds apart by frequency. Once a
  stream ends, `pw-record` falls back to other audio: prove a sound stopped by listing ttyak's sink-inputs
  (`pactl -f json list sink-inputs`, match `ttyak` in `application.name`).
- **Video**: synthetic. B sends with `ctl huddle-media <channel> N --video-test` (camera) and `--share-test` (screen
  share, a second attendee `<id>#content`); A receives with `--record-video DIR` (PNG frames) or the TUI's F8 view.
  Measure CPU on an optimised build in a scratch target dir.
- **Camera**: only a virtual one (v4l2loopback) that the maintainer sets up, fed by `ffmpeg testsrc2`, opened by
  explicit name. One feeder at a time: a second fails with `VIDIOC_G_FMT`.
- **Screen sharing** captures the real screen: only with the maintainer at the picker, told just before, choosing a
  single harmless window; recordings deleted right after. If they are not at the picker, cancel it
  (`hyprctl dispatch closewindow class:hyprland-share-picker`); never choose for them.
- **The real Slack web client** (as the other person): a separate Chrome profile with fake devices, driven by
  `tools/cdp.mjs` ([tools/README.md](../../tools/README.md)); check that only fake devices exist before joining. Never
  press its screen-share button (the fake UI flag would pick the real screen). Delete the profile folder after.
- **Slack's official desktop app** (measuring for the site, comparing behaviour): ask the maintainer before running it.
  It is a real signed-in client on their machine, with its own sounds, notifications and sessions. When agreed: the
  account they name, its audio on a silent null sink, and the method in
  [tools/site/MEASURING.md](../../tools/site/MEASURING.md).

## Notifications

- See what ttyak sends: `dbus-monitor --session "interface='org.freedesktop.Notifications',member='Notify'"` (validate
  the monitor with `notify-send` first). The decision is logged at debug level.
- Simulate a click: `gdbus emit --session --object-path /org/freedesktop/Notifications --signal
  org.freedesktop.Notifications.ActionInvoked "uint32 ID" "default"` (a bare `ID` goes out as int32 and is ignored).
- `TTYAK_NOTIFY_ROUTE=system` forces the system notifier in a terminal that shows notifications itself.
- No prompts on the desktop: anything that can prompt (gpg, ssh-agent, a keyring, sudo, polkit) runs in batch mode
  with a dummy secret (`gpg --batch --pinentry-mode loopback --passphrase-file <scratch file>`) or not at all.

## Checks with the maintainer

Some checks need the maintainer: a real notification click, the Mac's prompts, a decision on what looks right.

- Give one step at a time, with exact commands to paste (environment variables spelled out); wait for their "ready"
  before starting anything timed (a delayed post, a ring). Never start the timer in the same turn as the instructions.
- Script everything they would otherwise retype or read off a screen.
- When they type a code into a test window, read the screen masked (`sed -E 's/[A-Z0-9]/X/g'`) so the code never
  lands in a transcript.
- A character in a fresh window may be theirs: ask before chasing it.
- Stop every subagent that uses a machine before a session with them on it.

## Test machines

A Windows machine and a Mac are used for platform checks; their host aliases, users and paths are in
`AGENTS.local.md`, and how to set one up is in [setup.md](setup.md#7-the-test-machines).

- **Claim first.** Another agent, or the maintainer, may be using a test machine, so every use goes through a claim
  (holder and purpose; a queue in arrival order; the lock lives on the box) made with the claim tool named in
  `AGENTS.local.md`; its README, located through that file, has the commands. Below, `<tool>` stands for that tool's
  command and `BOX` and `ID` for the box name and your claim id. No claim, no use: `run`, `push`, `pull` and `shell`
  refuse without your live claim id, and the maintainer claims too. A claim lasts 30 minutes unless renewed (`run`,
  `push`, `pull` and `shell` renew while they work; `renew` for anything else); exit 4 means the box is withdrawn.
  Waiting for a claim is not a live observation, so the 10-minute cap does not apply: wait as long as needed
  (`--wait forever` is the default); run the `claim` in the background, or use `--report-every 5m` and re-run its
  `resume:` line, so your shell is not blocked. Release when done. Work only in your own folder (`ttyak-test` in the
  user's home). The tool never starts or stops a VM: a stopped VM is started first (from the desktop), then claimed.
  Before claiming, still check whether the box is running and whether someone is on it (its state, logged-in users,
  others' processes), and never shut down, restart or revert it while someone else uses it.
- **Sessions for a box** come from `accounts.py install` into a local fake home under your account claim, then
  `push` or `run` them over; never from `~/.config/ttyak`. The claim does not track copies pushed to a box: delete
  them there (and check) before you release the account.
- **Restore it as found**: leave the box as you found it unless the goal was to change it (if it was running, leave
  it running; if you started it, shut it down after). Delete copied sessions, binaries, recordings and caches there;
  check each path is gone (one path per check). Delete the local cross-build directory
  (`target/x86_64-pc-windows-gnu` grows past 10 GB).
- **Windows**: copy a file with `<tool> push BOX ID …`, run a command with `<tool> run BOX ID -- "<command>"` (the
  arguments are joined and run by the box's shell, cmd.exe; for PowerShell:
  `<tool> run BOX ID -- "powershell -NoProfile -File <script>"`). Run anything multi-step from a `.ps1` file pushed
  over, not through nested quotes. A `run` executes in a services session (no desktop, no notifications, no screen
  capture): to run something on the desktop, register a scheduled task with `-LogonType Interactive` for the logged-on
  user; inject keys and clicks and capture **the machine's own desktop** with PowerShell helpers (SendKeys,
  `mouse_event`, `keybd_event`; the screen capture of that desktop, never the maintainer's):
  [tools/vm/](../../tools/vm/README.md). Run Windows checks this way, without asking the maintainer to sit at the
  console. ttyak's folders there: `%APPDATA%\ttyak\config\<profile>`, `%LOCALAPPDATA%\ttyak\cache\…`. A TUI test, in
  a kitty window: `<tool> run --tty BOX ID -- "<folder>\ttyak.exe --profile <B>"` (the flag goes before the box name).
  **Never switch an app's notifications off through the registry**: Windows copies the setting into its notification
  database and never reads the registry back; undoing it needs the notification service stopped and the database
  edited.
- **macOS**: copy the source with
  `git archive HEAD | <tool> run BOX ID -- "mkdir -p ~/ttyak-test && tar -x -C ~/ttyak-test"` (`run` passes stdin
  through); a non-interactive shell there lacks `/opt/homebrew/bin` (`export PATH=/opt/homebrew/bin:$PATH`; cmake
  comes from Homebrew). ttyak's folder there is `~/Library/Application Support/ttyak/<profile>` (a space: pipe files
  in through `<tool> run BOX ID -- "cat > …"`, and bring files back with `<tool> pull`). ttyak's notifications appear
  as Terminal's (or the host terminal's): that app's notifications must be allowed in System Settings; screen sharing
  mutes banners (Notification Centre still has them); read macOS's view with `log show` for usernoted /
  NotificationCenter. Real speakers, microphone, camera and screen there: ask the maintainer first; their prompts and
  clicks need the maintainer at the Mac.

## After the check

- Cleanup: the fake home, temporary profiles and their cache folders, drafts the test wrote, files written into a
  real config (list the folder after: only what was there before), test windows, processes you started, recordings
  and captures, remote leftovers.
- Record what was checked LIVE, UNIT or NOT (in the task's ledger entry), with what was not tried.
