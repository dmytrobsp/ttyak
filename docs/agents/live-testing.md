# Live testing

How to run ttyak against Slack, on this machine and on test machines, without touching the maintainer's real accounts,
files, screen, clipboard, microphone or time. Read it before any live check. The rules in short are in
[AGENTS.md](../../AGENTS.md#hard-rules); the names, hosts and paths are in `AGENTS.local.md` — without that file, no
live work: ask the maintainer.

The helpers assume Linux with Hyprland, kitty and PipeWire, as on the maintainer's machine; `AGENTS.local.md` says what
this machine has; on any other desktop see [Other desktops](#other-desktops).

## Live checks in a milestone

In a milestone, several implementers work at once and run their own live checks
([workflow.md](workflow.md#running-a-milestone)). Everything on this page holds for them; three places have rules of
their own, and every milestone brief quotes this section.

- **Instances** (the Linux containers, the Hyprland VM, the Windows clones, the macOS VMs) are throwaway. Live checks
  run there by default. An implementer makes its own instance and removes it, by name.
- **Shared boxes** (the Mac itself, the shared Windows VM) are someone else's machine, never throwaway. An implementer
  uses one only under a box claim and only through a recipe. A recipe is a step-by-step command list on this page; on
  a shared box an implementer runs only those, and anything else goes back to the controller. It works only in its own
  folder, removes exactly what it made, checks that it is gone, and leaves the box as it found it. It never installs
  anything, changes a setting, starts, stops, restarts or reverts a VM.
- **The maintainer's desktop**: for implementers, instances only. No test window on the hidden workspace of
  [Test windows](#test-windows). `ttyak ctl` in a fake home with a claimed account and no window stays allowed: it
  shows nothing. What an instance cannot do and needs the hidden workspace is the controller's, as is anything with the
  maintainer present.

Also in a milestone:

- **One folder per unit on a shared box.** The recipes work in `~/ttyak-test/<unit>` on the Mac and
  `%USERPROFILE%\ttyak-test\<unit>` on the shared Windows VM, `<unit>` being the unit's name (outside a milestone, the
  agent's task name). An agent that dies before it cleans up then leaves a folder with a name, which no other agent
  reuses or removes. The folders of a dead agent are the controller's to remove, at the end of the milestone, under
  its own box claim and with the same cleanup recipe, only for this milestone's units: it lists the folder first and
  touches no other.
- **Claims do the queuing**: accounts, boxes and RAM slots, as everywhere on this page. Waiting for a claim or a slot
  is not a live observation.
- **Evidence.** The implementer saves what shows the check in the unit's scratch folder (the main checkout's
  `.superpowers/sdd/<plan>/<unit>/`, given in the brief as an absolute path), one file each: every claim and every
  release (accounts and boxes), the instance's `new` and `rm` output and `instance ls --here`, `ctl` output, the
  instance's own screenshot, log lines, the build's last lines with its exit status, each removal with its gone-check,
  and `git status` after any refusal. Never a capture of the maintainer's screen, and nothing private in it. Its
  report says where. The reviewer reads the evidence before anything counts as LIVE. Anything on this list
  without its file is reported NOT backed, and is never re-created after the fact.
- **Command shape.** A call that runs the claim tool against a shared box, above all one that carries a recursive
  delete, holds only that one command, with the claim tool's path and the box claim id written out literally, not as
  variables: the safety hook reads the quoted remote command too, and refuses a recursive delete next to variables
  without `set -euo pipefail`. The recipe blocks on this page keep their variables and their single-call background
  steps, and `$ACC_ID` in accounts commands is fine. A file whose text quotes a recursive delete or another guarded
  command (a report, a brief, a script) is written with the Write or Edit tool, never through a Bash heredoc or
  `python3 - <<EOF`: the hook scans heredoc bodies. The `settings.toml` heredoc of [A fake home](#a-fake-home) stays.
- **A refusal**: quote the refused command exactly as sent and the refusal text verbatim. After a refused cleanup on a
  shared box, the implementer stops and keeps its box claim held (it releases nothing). Its report gives its box and
  account claim ids. The controller does the whole cleanup under that still-live box claim, renewing it, by the
  recipe, then releases it.
- **Small notes.** `ctl status` says "ttyak is not running" unless a ttyak runs in that instance: start the TUI first
  for a meaningful status. `pgrep -af <path>` also matches the agent's own shell: find a process by the PID you
  started (a match by name could be the maintainer's own ttyak). `instance new` prints the matching `instance rm`
  command: use it.
- **The controller keeps** the checks with the maintainer and one live smoke check of the merged milestone branch.

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
   work on the box goes through the claim tool. A Linux check in an instance needs a slot, which `instance new`
   claims itself ([A Linux instance](#a-linux-instance)); keep that id too.
4. Check the account's state before blaming the code: a do-not-disturb schedule or snooze silences notifications
   (`ctl api dnd.info` on A's profile, with the claimed copy in the fake home of [A fake home](#a-fake-home), the four
   variables set on that command only; see [Do-not-disturb](#do-not-disturb-on-the-test-accounts)).
   For example, after `install`: `HOME="$FAKE" XDG_CONFIG_HOME="$FAKE/.config" XDG_DATA_HOME="$FAKE/.local/share"
   XDG_CACHE_HOME="$FAKE/.cache" "$REPO/target/debug/ttyak" --profile <A> ctl api dnd.info`.
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
  (`ttyak --profile <B> ctl post …`, `ctl huddle-media …`, run with B's claimed copy in a fake home, the same four
  variables on the command: [A fake home](#a-fake-home)).
- **Its plan**: the current one is on Slack's **free** plan, but Slack reports `team.plan = "std"` for it, so ttyak's
  paid-plan check — like Slack's own client — treats paid features (VIPs, for one) as available there. A check that
  depends on the plan says so in its record.
- **Every ttyak command names a test profile** with `--profile` and runs in a fake home with a claimed account
  ([A fake home](#a-fake-home)); no recipe uses a test profile in the real config folder (those copies are the
  maintainer's own). A bare `ttyak` (or a bare `ctl`) uses the machine's
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
`dnd.endSnooze`, restore a snooze with `dnd.setSnooze num_minutes=<to the old end>`. Every `ctl` call runs under the
claim, in the fake home, with the four variables of [A fake home](#a-fake-home) on the command.

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

## A Linux instance

The default place for a Linux live check: a throwaway desktop in a rootless podman container, with a headless sway
(or GNOME), a notification daemon and a null audio sink, none of which the maintainer sees or hears. A check that
needs Hyprland itself runs in a Hyprland VM instead ([below](#the-hyprland-vm)). It is made and driven with the claim
tool named in `AGENTS.local.md` (`<tool>` below; its README has every option). ttyak runs in it with the fake home of
[A fake home](#a-fake-home) as its home, so the maintainer's screen, sound and clipboard are never involved.

```sh
set -euo pipefail
REPO=<the clone>; FAKE=<scratch>/home              # as in "A fake home": folders made, settings.toml written
python3 "$REPO/tools/accounts/accounts.py" claim A --purpose "<what>"   # prints `claimed A id=ID lease=…`, more lines
ACC_ID=<the ID from its "claimed A id=ID" line>
python3 "$REPO/tools/accounts/accounts.py" install A "$ACC_ID" --home "$FAKE"
# the instance: the fake home is its home, the debug binary is mounted read-only
<tool> instance new --desktop sway --purpose "ttyak: <what>" --home "$FAKE" \
    --mount "$REPO/target/debug/ttyak:/opt/ttyak/ttyak"
# it prints `instance NAME id=ID slot=N`: ID is the slot id, not `$ACC_ID`
python3 "$REPO/tools/accounts/accounts.py" renew A "$ACC_ID"            # right before ttyak starts
<tool> instance exec NAME ID -- "setsid kitty --class ttyak-test timeout 25m /opt/ttyak/ttyak --profile <A> \
    >/dev/null 2>&1 < /dev/null &"
<tool> instance windows NAME ID               # the ttyak-test window, focused
<tool> instance shot NAME ID out.png          # what the window shows (look at it before relying on it)
<tool> instance notifications NAME ID         # the notifications it shows
<tool> instance record NAME ID 5 out.wav      # what the null sink heard (at most 300 s); silent unless the
                                              # fake home's settings turn sounds on and sound is agreed
<tool> instance exec NAME ID -- "cat ~/.cache/ttyak/<A>/ttyak.log"     # the log, before the instance goes
<tool> instance rm NAME ID
python3 "$REPO/tools/accounts/accounts.py" release A "$ACC_ID"          # after ttyak is gone
# then delete the fake home as in "A fake home"
```

Not tried yet (NOT): a Slack notification reaching the instance's `notifications`, a raise on a mention, a sound in
`record` — these commands are the tool's documented behaviour.

- **Home.** Inside the instance `HOME` is the mounted fake home (read-write), so ttyak finds
  `.config/ttyak/<profile>/session.json` there with no extra environment; do not set the four variables. The
  instance's `~/.cache` and `~/.local/state` live in RAM and vanish with it: read the log (above) before `rm`.
  The binary is mounted from the clone's `target/debug` (`cargo build` first) and never from a release folder.
- **Slot.** An instance needs a slot, granted by free RAM, and queues like a box claim (`--wait`, default forever;
  `--report-every 5m` and re-run its `resume:` line, or run `new` in the background). Waiting for a slot is not a live
  observation: the 10-minute cap does not apply. A slot lapses after 30 minutes without use, then the next `new` or
  `reap` removes the instance; `exec`, `shot` and the others renew it, but only the slot, not the account claim
  (30 minutes): ttyak therefore starts under `timeout 25m` after a fresh `renew`, so a dead agent's ttyak cannot
  outlive the claim; for a check longer than 25 minutes renew the account and start it again. Remove the instance
  when done (`rm`), then release the accounts and delete the fake home; `<tool> instance ls --here` (it asks no other
  box) must show none of yours.
- **GNOME.** `--desktop gnome` gives GNOME instead of sway; `windows`, `shot`, `notifications` and `record` work there
  too. Use it when a check depends on how GNOME shows or raises things.
- **Keys.** `instance exec --tty` runs a command with a terminal. Driving the window with `kitty @` through `exec`
  (start kitty with `-o allow_remote_control=yes --listen-on unix:@<socket>`) is the same as on the desktop below, but
  has not been tried in an instance yet (NOT).
- **What it cannot do.** A container cannot run Hyprland (use [the Hyprland VM](#the-hyprland-vm)). No instance has a
  real microphone, camera or screen, or the desktop's clipboard; in a VM only KVM and vsock reach the guest. On a VM
  `--mount` is read-only and takes folders only (see below). Checks that need the maintainer's own desktop use the
  hidden workspace of [Test windows](#test-windows).
- **Do-not-disturb** schedules of the test accounts still apply: check `dnd.info` first, as in
  [Before every live check](#before-every-live-check).

### The Hyprland VM

`--desktop hyprland` makes a small Arch Linux VM with Hyprland (2 GiB, 1280x800) in place of the container: use it for
what depends on Hyprland (how ttyak raises or focuses its window there). Claim the accounts, install A into the fake
home and renew as in [A Linux instance](#a-linux-instance); these lines differ:

```sh
REPO=<the clone>; FAKE=<scratch>/home              # as above; `new` needs a built base ([The bases](#the-bases))
<tool> instance new --desktop hyprland --purpose "ttyak: <what>" --home "$FAKE" --mount "$REPO/target/debug:/opt/ttyak"
<tool> instance exec NAME ID -- "setsid kitty --class ttyak-test timeout 25m /opt/ttyak/ttyak --profile <A> \
    >/dev/null 2>&1 < /dev/null &"
```

- **Ready** about 10 s after `new` (LIVE: 10.4 s). `--home` is the same as in the container: read-write as `HOME`,
  ttyak finds its session there, and `~/.cache` and `~/.local/state` are in RAM (read the log before `rm`).
- **`--mount` takes a folder, never a file.** The VM shares folders over 9p, so a file mount (as for sway,
  `target/debug/ttyak:/opt/ttyak/ttyak`) is refused (exit 2): mount the folder, `target/debug`, and run
  `/opt/ttyak/ttyak`.
- `exec`, `shot`, `windows`, `notifications` and `record` work as on sway. `push` and `pull` copy a file in or out
  (`<tool> instance push NAME ID SRC DST`), useful for the log.
- **A notification click** is `<tool> instance exec NAME ID -- makoctl invoke`. LIVE: ttyak ran in kitty and B's DM
  reached `notifications`; the click opened the message, but with Hyprland's default `misc:focus_on_activate = false`
  the window was not raised (ttyak issue #63). After `<tool> instance exec NAME ID -- hyprctl keyword
  misc:focus_on_activate true` (the maintainer's own setting) a click raised it. The base keeps Hyprland's default, so
  set it only for a check of the raise, and say so in the record.
- A VM holds its memory (about 2.5 GB with overhead), so it queues for a slot like any instance; `rm` it when done.

### The bases

A VM instance starts from a frozen base, built once on the machine and rebuilt on command, never by `new`:
`<tool> instance base build hyprland` (or `windows`, which downloads an 8 GB evaluation image the first time),
`<tool> instance base ls` (age, size, which is current), `<tool> instance base gc <desktop>` (removes builds no clone
uses). A build holds a slot and takes minutes; do not build while the maintainer is busy on the machine without
asking. Where the bases live is in `AGENTS.local.md` (field "Bases"). A Windows base is an evaluation image: `new`
warns 14 days before its end and refuses in its last day; the fix is a rebuild.

## Test windows

On Linux a live check runs in [an instance](#a-linux-instance) by default; the hidden workspace below is for what an
instance cannot do (the maintainer's own desktop). Test windows never appear on the maintainer's screen. Start
them on a hidden special workspace with a class and a kitty socket of your own:

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
- Another size: `kitty @ set-font-size 6` gives about 170×44 cells (`0` restores); the default is a "narrow" ~95
  columns.
- `TTYAK_FAKE_HOME="$FAKE" sh tools/launch.sh <A> [<B's name>]` opens a *visible* `ttyak-test` window on your A
  profile and opens the DM with B: use it only while the maintainer is watching. It refuses without a profile and
  without `TTYAK_FAKE_HOME`, a fake home that holds the profile's `session.json` (claim the account and `install` it
  first, [A fake home](#a-fake-home)); the window's ttyak runs with the four variables pointing into it.
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
  `resume:` line, so your shell is not blocked. Release when done. Work only in your own folder (`ttyak-test/<unit>`
  in the user's home, `<unit>` as in [Live checks in a milestone](#live-checks-in-a-milestone)). The tool never
  starts or stops a VM: a stopped VM is started first (from the desktop), then claimed.
  Before claiming, still check whether the box is running and whether someone is on it (its state, logged-in users,
  others' processes), and never shut down, restart or revert it while someone else uses it.
- **Sessions for a box** come from `accounts.py install` into a local fake home under your account claim, then
  `push` or `run` them over; never from `~/.config/ttyak`. The claim does not track copies pushed to a box: delete
  them there (and check) before you release the account.
- **Restore it as found**: leave the box as you found it unless the goal was to change it (if it was running, leave
  it running; if you started it, shut it down after). Delete copied sessions, binaries, recordings and caches there;
  check each path is gone (one path per check). Delete the local cross-build directory
  (`target/x86_64-pc-windows-gnu` grows past 10 GB).
- **Windows, in a clone by default.** A Windows live check runs in a throwaway clone made by the claim tool
  ([Windows in a clone](#windows-in-a-clone)), not on the shared Windows VM. The shared VM is claimed only for what
  needs that VM itself (the maintainer's work machine, its real setup); `AGENTS.local.md` names it. The rest of this
  bullet is that shared VM.
- **Windows, the shared VM**: copy a file with `<tool> push BOX ID …`, and run a command with this line (the
  arguments are joined and run by the box's shell, cmd.exe):
  `<tool> run BOX ID -- "<command>"`
  For PowerShell:
  `<tool> run BOX ID -- "powershell -NoProfile -File <script>"`
  Run anything multi-step from a `.ps1` file pushed over, not through nested quotes. Everything you push goes into
  `C:\Users\<user>\ttyak-test\<unit>` (`<user>` from `AGENTS.local.md`; `<unit>` is never empty). When done, remove
  that one folder with this line, the whole path written out with the unit's name in it, never built from a variable:
  `<tool> run BOX ID -- "rmdir /s /q C:\Users\<user>\ttyak-test\<unit>"`
  Then check it:
  `<tool> run BOX ID -- "if exist C:\Users\<user>\ttyak-test\<unit> echo left"`
  The `tools/vm` scripts are copied into `<unit>\vm` there, and `cleanup.ps1` is always called with `-Dir` set to that
  literal unit folder ([tools/vm/](../../tools/vm/README.md)). A `run` executes in a services session (no desktop, no
  notifications, no screen capture): to run something on the desktop, register a scheduled task with `-LogonType
  Interactive` for the logged-on user; inject keys and clicks and capture **the machine's own desktop** with
  PowerShell helpers (SendKeys, `mouse_event`, `keybd_event`; the screen capture of that desktop, never the
  maintainer's): [tools/vm/](../../tools/vm/README.md). Run Windows checks this way, without asking the maintainer to
  sit at the console. ttyak's folders there: `%APPDATA%\ttyak\config\<profile>`, `%LOCALAPPDATA%\ttyak\cache\…`. A TUI
  test, in a kitty window: `<tool> run --tty BOX ID -- "<folder>\ttyak.exe --profile <B>"` (the flag goes before the
  box name; the session there is the claimed copy `install` put on the box, not a profile in the user's real folder).
  **Never switch an app's notifications off through the registry**: Windows copies the setting into its notification
  database and never reads the registry back; undoing it needs the notification service stopped and the database
  edited.
- **macOS, in a VM by default.** A macOS live check runs in a throwaway macOS VM on the Mac, made by the claim tool
  ([macOS in a VM](#macos-in-a-vm)), not on the Mac itself. The Mac is claimed (a box claim) only to build ttyak there
  and for what needs the real machine: the maintainer's own setup, real speakers, microphone, camera or screen (ask
  the maintainer first), and their prompts and clicks. The rest of this bullet is the Mac itself.
- **macOS, the Mac itself**: copy the source with
  `git archive HEAD | <tool> run BOX ID -- "mkdir -p ~/ttyak-test/<unit>/src && tar -x -C ~/ttyak-test/<unit>/src"`
  (`run` passes stdin through; `<unit>` as in [Live checks in a milestone](#live-checks-in-a-milestone)); a
  non-interactive shell there lacks `/opt/homebrew/bin` (`export PATH=/opt/homebrew/bin:$PATH`; cmake
  comes from Homebrew). ttyak's folder there is `~/Library/Application Support/ttyak/<profile>` (a space: pipe files
  in through `<tool> run BOX ID -- "cat > …"`, and bring files back with `<tool> pull`). ttyak's notifications appear
  as Terminal's (or the host terminal's): that app's notifications must be allowed in System Settings; screen sharing
  mutes banners (Notification Centre still has them); read macOS's view with `log show` for usernoted /
  NotificationCenter. Real speakers, microphone, camera and screen there: ask the maintainer first; their prompts and
  clicks need the maintainer at the Mac.

### Windows in a clone

`<tool> instance new --desktop windows` makes a Windows 11 evaluation VM (4 GiB, 1280x800) on a frozen base, ready
about 15 s after `new` (LIVE: 14.6 s). Defender and Windows Update are off in it. Do-not-disturb is switched off in
the base, and the clone's ready file (its path is in the claim tool's README) records the mode as `notifications=`:
a banner check reads it rather than assumes. A clone is charged 6.5 GB of the pool (its RAM plus its 2 GB of clone
files), so `new` may wait longer for a slot than for a Linux instance. `instance rm` throws it away;
`<tool> instance ls --here` (it asks no other box) must show none of yours at the end. Windows' default foreground
lock is kept, so a new window may not take the focus: `windows` says which window has it.

```sh
set -euo pipefail
REPO=<the clone>; FAKE=<scratch>/home              # as in "A fake home"; A claimed and installed there
ACC_ID=<the ID from its "claimed A id=ID" line>
EXE="$REPO/target/x86_64-pc-windows-gnu/debug/ttyak.exe"        # cargo build --target x86_64-pc-windows-gnu first
<tool> instance new --desktop windows --purpose "ttyak: <what>"   # prints `instance NAME id=ID slot=N`
CFG='C:\Users\tbuser\AppData\Roaming\ttyak\config'
<tool> instance exec NAME ID -- "mkdir $CFG\<A>"
<tool> instance push NAME ID "$EXE" 'C:\Users\tbuser\ttyak.exe'         # about 615 MiB, a few seconds, no extra DLLs
<tool> instance push NAME ID "$FAKE/.config/ttyak/<A>/session.json" "$CFG\<A>\session.json"
<tool> instance push NAME ID "$FAKE/.config/ttyak/settings.toml" "$CFG\settings.toml"   # sounds off
python3 "$REPO/tools/accounts/accounts.py" renew A "$ACC_ID"                              # right before ttyak starts
<tool> instance exec --desktop --timeout 25m NAME ID -- start '""' 'C:\Users\tbuser\ttyak.exe' --profile <A>
<tool> instance windows NAME ID               # the ttyak window, and which has the focus
<tool> instance shot NAME ID out.png          # what the screen shows (look at it before relying on it)
<tool> instance notifications NAME ID         # ttyak's toasts (the toast history of its app id)
<tool> instance pull NAME ID "C:\Users\tbuser\AppData\Local\ttyak\cache\<A>\ttyak.log" ./ttyak.log
<tool> instance rm NAME ID
python3 "$REPO/tools/accounts/accounts.py" release A "$ACC_ID"          # after ttyak is gone
```

- **Files go in with `push`.** `--home`, `--mount` and `record` are refused (exit 2). The session reaches the clone as
  a file: ttyak's folders on Windows are the system's (`%APPDATA%\ttyak\config\<profile>`, as in
  [Test machines](#test-machines)), so there is no fake home to point it at: `push` the account's `session.json`
  (and `settings.toml`) there. The clone is throwaway, so this is not the maintainer's folder.
- **`exec` and `exec --desktop`.** `exec` runs in session 0: no desktop, so files and `ttyak ctl` only. `exec
  --desktop` runs in the logged-on user's session, where windows and toasts are; its `--timeout` (default 25m) ends the
  command and replaces `timeout 25m`, so a dead agent's ttyak cannot outlive the account claim: renew the account
  right before it starts, as for Linux. A program that must stay is started with `start '""' PROGRAM`. Both pass
  one command line to cmd.exe: quote for it; run anything multi-step from a `.ps1` file pushed over.
- **`notifications`** lists the toast history of ttyak's app id, which is Windows PowerShell's (ttyak's toasts use it
  as a fallback), and that is the default; `--app APPID` reads another. A banner check first reads `notifications=` in
  the clone's ready file.
- **Space.** A clone's files are capped at 2 GB; deleting a file gives its space back. The debug `ttyak.exe` is about
  615 MiB: push it once.
- **Log and cleanup.** Pull the log before `rm`; the clone and its files vanish with it, so no sessions are left on
  it. Release the accounts after ttyak is gone, then delete the fake home and the local cross-build directory
  (`target/x86_64-pc-windows-gnu` grows past 10 GB).
- **LIVE** (2026-10-28): the debug exe pushed in 4.2 s; `session.json` and `settings.toml` in `%APPDATA%\ttyak\config`
  were read by ttyak; `exec --desktop --timeout 25m … start '""' …` started it; with Notepad focused, B's @mention was
  listed by `notifications` (PowerShell's app id) and showed as a banner in `shot`; `pull` fetched the log from
  `%LOCALAPPDATA%\ttyak\cache\<profile>\ttyak.log`. No environment variable moves ttyak's folders on Windows (the
  `directories` crate asks the system for them), hence the pushes. NOT tried: sound, and a click on a Windows toast.

### macOS in a VM

`<tool> instance new --desktop macos` makes a macOS VM on the Mac, from the desktop (the claim tool relays it there),
ready in under a minute. ttyak is built on the Mac, under a box claim as in [Test machines](#test-machines), and the
build is mounted into the VM; the session is pushed in. The VM is the fake home. It needs 6 GB of the Mac's pool beside
the maintainer's 6 GB reserve, and macOS VMs run one at a time, so `new` may queue like any instance (a wait for a slot,
like a wait for a claim, is not a live observation: the 10-minute cap does not apply). A start is refused unless the
Mac's firewall is on and Tart is blocked; the claim tool's README has the setup (the maintainer does it once), and it is
refused on battery.

```sh
set -euo pipefail
REPO=<the clone>; FAKE=<scratch>/home              # as in "A fake home"; A claimed and installed there
ACC_ID=<the ID from its "claimed A id=ID" line>
# 1. build ttyak on the Mac under a box claim (BOX and ID: [Test machines](#test-machines)); keep only the binary
#    <unit> is your unit's name (outside a milestone, your task name): your folder is ~/ttyak-test/<unit>
git archive HEAD | <tool> run BOX ID -- "mkdir -p ~/ttyak-test/<unit>/src && tar -x -C ~/ttyak-test/<unit>/src"
<tool> run BOX ID -- "export PATH=/opt/homebrew/bin:\$HOME/.cargo/bin:\$PATH; cd ~/ttyak-test/<unit>/src \
    && cargo build && mkdir -p ~/ttyak-test/<unit>/build && cp target/debug/ttyak ~/ttyak-test/<unit>/build/"
<tool> release BOX ID                              # the VM needs no box claim; the Mac is free for others again
# 2. the VM: the build folder is snapshotted read-only; SRC is the Mac's absolute path (its user is in AGENTS.local.md)
<tool> instance new --desktop macos --purpose "ttyak: <what>" --mount <the Mac's home>/ttyak-test/<unit>/build:ttyak
# it prints `instance NAME id=ID slot=N`: ID is the slot id, not `$ACC_ID`; the binary is at
# '/Volumes/My Shared Files/ttyak/ttyak' in the guest
SUPPORT='Library/Application Support/ttyak'        # ttyak's macOS folder, from the guest user's home
<tool> instance exec NAME ID -- "mkdir -p '$SUPPORT/<A>'"
<tool> instance push NAME ID "$FAKE/.config/ttyak/<A>/session.json" "$SUPPORT/<A>/session.json"
<tool> instance push NAME ID "$FAKE/.config/ttyak/settings.toml" "$SUPPORT/settings.toml"   # sounds off
python3 "$REPO/tools/accounts/accounts.py" renew A "$ACC_ID"                                # right before ttyak starts
# ttyak in Terminal, in the logged-on session; it blocks until ttyak ends: run it in the background, keep the PID
<tool> instance exec --desktop --timeout 25m NAME ID -- "'/Volumes/My Shared Files/ttyak/ttyak' --profile <A>" \
    >out.txt 2>&1 &
TTYAK_PID=$!
<tool> instance windows NAME ID               # the apps and the front one, no window titles
<tool> instance shot NAME ID out.png          # what the screen shows (look at it before relying on it)
<tool> instance notifications NAME ID         # ttyak's notifications: delivered or presented, app Terminal
# SIGTERM (the default of kill) closes the relay connection; the Mac then interrupts the command and ends the run.
# Not SIGINT: a `&` job ignores it
kill "$TTYAK_PID" 2>/dev/null || true
wait "$TTYAK_PID" 2>/dev/null || true
<tool> instance pull NAME ID 'Library/Caches/ttyak/<A>/ttyak.log' ./ttyak.log
<tool> instance rm NAME ID
python3 "$REPO/tools/accounts/accounts.py" release A "$ACC_ID"          # after ttyak is gone
# then delete the fake home as in "A fake home"; the build folder on the Mac, under a new box claim (see below)
```

- **Files go in with `push`; the build with `--mount`.** `--mount SRC:NAME` takes a read-only APFS snapshot of the
  folder SRC (a Mac path, a folder, never a file) when `new` runs, so later builds stay out; the guest sees it at
  `/Volumes/My Shared Files/NAME`. `--home` and `record` are refused (exit 2): there is no fake home to point ttyak at,
  the VM is it. The session goes to ttyak's macOS folder, `~/Library/Application Support/ttyak/<profile>/`
  (`settings.toml` one level up); the log is `~/Library/Caches/ttyak/<profile>/ttyak.log` (LIVE, 2026-10-29). Measured
  on 2026-10-28 between the desktop and the Mac: `push` of 100 MiB about 24 MB/s (about 4 s), `pull` about 38 MB/s.
- **`exec` and `exec --desktop`.** `exec` runs over ssh in the guest user's home: files, `ttyak ctl`, `sw_vers`. `exec
  --desktop` runs the command in Terminal, in the logged-on session, and streams its output back; its `--timeout`
  (default 25m) ends the command and replaces `timeout 25m`, so a dead agent's ttyak cannot outlive the account claim:
  renew the account right before it starts. The command line goes to `sh`: quote the path with spaces as above. There
  is no `python3` in the guest (calling it opens the developer-tools dialog): use macOS's own programs. ttyak's stdin
  is Terminal's terminal and nothing types into it, so a TUI check reads the screen with `shot`.
  If the run does not end after the `kill`, the watchdog stops it a minute after `--timeout`, and `rm` removes the VM
  anyway.
- **Notifications.** ttyak's notifications appear as Terminal's (`com.apple.Terminal`). `notifications` lists each as
  `delivered`, or `presented` once a banner showed; macOS 26 logs no title or body, so a check proves that a
  notification arrived and showed, not its text (read the text from `shot` while the banner is up). Terminal in front
  suppresses banners: the tool starts Terminal in the background, and `windows` says which app is in front. `--app
  BUNDLEID` reads another app's rows.
- **`windows` and `shot`.** `windows` lists the apps with a Dock icon and the front one, with no window titles (macOS
  gives none without the Accessibility permission). `shot` is a VNC screenshot taken on the Mac, local only.
  **Never point a VNC viewer at a VM**: Virtualization's VNC server ends the whole VM when a client lacks the
  DesktopSize extension.
- **Software Update and Rosetta.** Software Update is off in the VM (Apple's update servers are blocked), so an update
  prompt cannot appear and a Rosetta install would fail: build for the Mac's own architecture (Apple Silicon) only.
- **Cleanup.** Pull the log before `rm`; the VM and the pushed session vanish with it. Release the accounts after
  ttyak is gone, then delete the fake home. The build folder and source copy of your unit sit in
  `~/ttyak-test/<unit>` on the Mac: delete them under a new box claim, with the command run on the Mac through the
  box's `run`, the two paths written literally
  (`<tool> run BOX ID -- "rm -rf ~/ttyak-test/<unit>/src ~/ttyak-test/<unit>/build"`), then the empty unit folder
  (`<tool> run BOX ID -- "rmdir ~/ttyak-test/<unit>"`, which refuses a folder that is not empty), check each of the
  three is gone (one path per check), and release. `<tool> instance ls` must show none of yours. Other folders in
  `~/ttyak-test` are not yours: leave them, whatever they hold.
- **LIVE** (2026-10-29): ttyak built on the Mac in 64 s under a box claim. The VM was ready 53 s after `new`, from the
  desktop through the relay. ttyak, run with `exec --desktop --timeout 25m` in the background, signed in as A, with
  Finder in front. B's DM showed in `notifications` as `com.apple.Terminal` `presented`, and `shot` showed the banner
  with the sender and the text. The log was at `~/Library/Caches/ttyak/<profile>/ttyak.log`. `kill` and `wait` of the
  backgrounded run ended it, and no ttyak was left in the guest. A was in its quiet hours, so the check ended A's
  do-not-disturb and put it back ([Do-not-disturb](#do-not-disturb-on-the-test-accounts)). NOT tried: sound (no audio
  device), a click on a notification, ttyak raising its own window.

## After the check

- Cleanup: the fake home, temporary profiles and their cache folders, drafts the test wrote, files written into a
  real config (list the folder after: only what was there before), test windows, processes you started, recordings
  and captures, remote leftovers.
- Record what was checked LIVE, UNIT or NOT (in the task's ledger entry), with what was not tried.
