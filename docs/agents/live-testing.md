# Live testing

How to run ttyak against Slack, on this machine and on test machines, without touching the maintainer's real accounts,
files, screen, clipboard, microphone or time. Read it before any live check. The rules in short are in
[AGENTS.md](../../AGENTS.md#hard-rules); the names, hosts and paths are in `AGENTS.local.md` — without that file, no
live work: ask the maintainer.

The helpers assume Linux with Hyprland, kitty and PipeWire, as on the maintainer's machine; `AGENTS.local.md` says what
this machine has.

## Before every live check

1. `cargo build` (a `cargo test` alone leaves a stale `target/debug/ttyak`).
2. Close your old test windows and wait until none remain (an open one keeps its kitty socket; a new window then fails
   with "Invalid listen_on" and keys go to the wrong window).
3. Make sure nobody else — another agent, on this machine or a test machine — is using the Slack account you need.
4. Check the account's state before blaming the code: a do-not-disturb schedule or snooze silences notifications
   (`--profile <A> ctl api dnd.info`; see [Do-not-disturb](#do-not-disturb-on-the-test-accounts)).
5. Decide what the check must show and cap its waits at about 10 minutes. Only run experiments whose answer changes
   what gets built; report interim status rather than sit in a long wait.
6. Use absolute paths in every command (an agent shell's working directory resets between commands), and keep the
   fake home, recordings and captures in the scratch folder — a reboot wipes it, so record results elsewhere.

## The test workspace and profiles

- **A Slack workspace with two test accounts, A and B**, exists only for developing ttyak. Its name, the accounts and
  the profile names that hold their sessions are in `AGENTS.local.md`. B plays the other person: posts, reacts, rings,
  sends test video (`ttyak --profile <B> ctl post …`, `ctl huddle-media …`).
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
FAKE=<scratch>/home                                # a literal path in your scratch folder
mkdir -p "$FAKE"/{config,data,cache} && chmod 700 "$FAKE" "$FAKE"/*
mkdir -p "$FAKE/config/ttyak/<A>" && install -m 600 ~/.config/ttyak/<A>/session.json "$FAKE/config/ttyak/<A>/"
cat > "$FAKE/config/ttyak/settings.toml" <<'EOF'
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
HOME="$FAKE" XDG_CONFIG_HOME="$FAKE/config" XDG_DATA_HOME="$FAKE/data" XDG_CACHE_HOME="$FAKE/cache" \
  ./target/debug/ttyak --profile <A>
```

- Copy only the session files you need (0600); never print them — they hold tokens. Delete the fake home after.
- A profile with two workspaces = the `workspaces` lists of two `session.json` files combined (in Python, never
  printed) into a temporary profile with a name of its own.
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
- `tools/launch.sh` opens a *visible* `ttyak-test` window as profile `default` and opens the DM with B: use it only if
  `default` is your A profile and the maintainer is watching.
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

A bare system: run the binary in a podman Debian container with only `libasound2t64`, sessions mounted read-only,
the cache folder from the scratch folder.

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

- **Shared machines.** Another agent may be using a test machine. Before using it check whether it is running and
  whether someone is on it (its state, logged-in users, others' processes); never shut down, restart or revert it while
  someone else uses it; work only in your own folder (`ttyak-test` in the user's home); if it is busy, wait or ask.
- **Restore it as found**: if it was running, leave it running; if you started it, shut it down after. Delete copied
  sessions, binaries, recordings and caches there; check each path is gone (one path per check). Delete the local
  cross-build directory (`target/x86_64-pc-windows-gnu` grows past 10 GB).
- **Windows**: `ssh -T -o BatchMode=yes <host> "…" < /dev/null` (without closing stdin ssh hangs); the default shell
  is cmd — use `powershell -NoProfile -Command "& …"`, and run anything multi-step from a `.ps1` file copied over, not
  through nested quotes. An SSH session runs in a services session (no desktop, no notifications, no screen capture):
  to run something on the desktop, register a scheduled task with `-LogonType Interactive` for the logged-on user;
  inject keys and clicks and capture **the machine's own desktop** with PowerShell helpers (SendKeys, `mouse_event`,
  `keybd_event`; the screen capture of that desktop, never the maintainer's): [tools/vm/](../../tools/vm/README.md).
  Run Windows checks this way, without asking the maintainer to sit at the console. ttyak's folders there:
  `%APPDATA%\ttyak\config\<profile>`, `%LOCALAPPDATA%\ttyak\cache\…`. A TUI test: a kitty window running
  `ssh -t <host> "<folder>\ttyak.exe --profile <B>"`.
  **Never switch an app's notifications off through the registry**: Windows copies the setting into its notification
  database and never reads the registry back; undoing it needs the notification service stopped and the database
  edited.
- **macOS**: copy the source with `git archive HEAD | ssh <host> 'mkdir -p ~/ttyak-test && tar -x -C ~/ttyak-test'`;
  a non-interactive ssh shell lacks `/opt/homebrew/bin` (`export PATH=/opt/homebrew/bin:$PATH`; cmake comes from
  Homebrew). ttyak's folder there is `~/Library/Application Support/ttyak/<profile>` (a space: pipe files through
  `ssh … 'cat > …'`). ttyak's notifications appear as Terminal's (or the host terminal's): that app's notifications
  must be allowed in System Settings; screen sharing mutes banners (Notification Centre still has them); read macOS's
  view with `log show` for usernoted / NotificationCenter. Real speakers, microphone, camera and screen there: ask the
  maintainer first; their prompts and clicks need the maintainer at the Mac.

## After the check

- Cleanup: the fake home, temporary profiles and their cache folders, drafts the test wrote, files written into a
  real config (list the folder after: only what was there before), test windows, processes you started, recordings
  and captures, remote leftovers.
- Record what was checked LIVE, UNIT or NOT (in the task's ledger entry), with what was not tried.
