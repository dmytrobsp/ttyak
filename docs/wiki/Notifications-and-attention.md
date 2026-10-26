How ttyak gets your attention: a mark in the terminal's tab title, desktop notifications, the terminal's bell and a
sound. All of it is set in F6 → Notifications (the other F6 pages are on [Settings](Settings)). What your system needs
for it is on [Requirements](Requirements). Clicking a notification opens what it showed and brings ttyak's window
forward, by different means on each system (below).

## The tab title

The terminal's tab title carries a mark: `○ ttyak - <workspace>` with nothing unread, `●` with something unread (in any
workspace of the profile), `◆` with something for you (a direct message, a mention). A long workspace name is cut to
keep the title within 80 cells.

When ttyak ends, your terminal's own title comes back in terminals that keep a title stack (kitty, …); elsewhere the
title is cleared. F6 → Notifications → Mark in the tab title (`title_mark`) turns the mark off; switched off while
ttyak runs, the terminal's own title comes back at once the same way, and ttyak then never touches the title. In tmux
the mark reaches the terminal's title only with `set -g set-titles on` (a tmux option; not tried).

## Desktop notifications

What notifies you follows Slack's notification settings, kept on Slack (F6 → Notifications shows and changes them):

- direct messages and group DMs, mentions of you, your keywords (Keywords in F6: they count as mentions), and replies
  in threads you started or replied in
- in a channel whose level is "all new messages", every new message too (thread replies only in threads you are in;
  F6 → This conversation notifies about; the other channels follow Channels notify about (default))
- nothing from a conversation whose level is "nothing", nor from a muted one
- someone starting a huddle in a DM or group DM with you (not in channels; the invitation also rings: see
  [Huddles](Huddles#invitations))

Nothing fires for a conversation (or its thread) you are looking at in a focused terminal. Your own messages never
notify.

A conversation shows one desktop notification at a time: a newer message there (a thread reply too) replaces it
instead of stacking. It is taken away once what it shows is read — in ttyak, in another client, or, for the other
workspaces of the profile, by their next 30-second check — where the notification server allows it (Linux desktops,
macOS and Windows). A thread reply's notification stays until that thread is read (reading the conversation does not
take it away).

Notifications from the other workspaces of the profile still arrive, titled with the workspace; clicking one switches
there ([Profiles and sessions](Profiles-and-sessions#several-workspaces-in-one-profile)). A new version of ttyak is told
by a notification too, once: see [Updates](Updates#the-check).

F6 → Notifications → Desktop notifications (`desktop`, on by default) turns them off; there is no sound then either,
but the bell (below) still rings.

## The bell

F6 → Notifications → Bell when not focused (`bell`, off by default) also rings the terminal's bell with a notification
while the terminal is not focused. Terminals turn it into an urgent mark (the window manager or bar highlights the
window), and tmux flags the window (neither checked here). Some desktops bring the window forward and switch to its
workspace on a bell instead — kitty's default `window_alert_on_bell` with Hyprland's `misc:focus_on_activate = 1` does
(tried) — and kitty may also beep (`enable_audio_bell`).

The bell never rings while notifications are paused, nor in a terminal that does not report focus. It does ring with
desktop notifications off.

## The sound

F6 → Notifications → Sound (`sound`): `off` (the default), `chime`, `ping`, `knock` (built in), or `system`, the
desktop's own notification sound. Notification sound volume (`sound_volume`, 0 to 200 %, 100 by default) sets how loud
ttyak plays it, and the huddle's own sounds too ([Huddles](Huddles#sounds-f6--huddles)).

The sound goes with the notification where the notification server can play it (it lists the `sound` capability). Then
the system's do-not-disturb silences both, and ttyak's volume and speaker setting do not apply. swaync's server does
not list it (seen here); by their documentation GNOME's does, mako and dunst do not. Where the server cannot, ttyak
plays the sound itself — except `system`, which then stays silent, since there is none to play. On macOS and Windows
only `system` goes with the notification; ttyak plays its own sounds there.

A file from before 0.12 with `[audio] sound_volume` keeps its value; saving writes it under `[notifications]`.

## Do-not-disturb

ttyak's own pause and schedule (F6 → Notifications → Pause notifications and Notification schedule, the same settings
as in Availability; kept on Slack) always hold notifications back: no notification, no sound, no bell.

With F6 → Notifications → Follow the system's do-not-disturb (`follow_system_dnd`, on by default), ttyak also asks the
system whether notifications are suppressed before an interruption, whoever plays the sound. These can be asked:

- swaync (its do-not-disturb; tried)
- mako (a `do-not-disturb` or `dnd` mode), dunst (paused), GNOME (banners off), KDE (notifications inhibited) — asked
  as their documentation says; not tried yet
- Windows: see [Windows](#windows) below

While the answer is yes, ttyak fires nothing: no notification, no sound, no bell, and a huddle invitation's ring keeps
quiet (asked when it starts ringing; it rings if no answer comes within 2 seconds). Tried with swaync: on, nothing
fired; off, the notification and its sound. On Windows the toast still goes out, without its sound or the bell (see
below).

An answer stands for the next ones for up to 2 seconds, so a burst of messages asks once — and a do-not-disturb
switched on or off just before may be missed for that long. With any other server, or when it does not answer within a
second, notifications go through. On macOS the system can't be asked (see [macOS](#macos)).

Switched off, only ttyak's own pause and schedule hold notifications back (tried with swaync's do-not-disturb on: ttyak
played its sound, swaync held the notification back).

## Settings in F6

F6 → Notifications is the one place for all of this, in this order:

- what notifies you (kept on Slack): This conversation notifies about, Channels notify about (default), Keywords, VIPs
  (the list; you add or remove one on their profile card)
- how you are told: Desktop notifications, Sound, Notification sound volume, Bell when not focused, Mark in the tab
  title
- quiet hours: Pause notifications, Notification schedule, Follow the system's do-not-disturb
- shown only, what this machine does: Notifications shown by (who shows them and who plays their sound), A click (what
  a click does and how the window is brought forward), System do-not-disturb (what the system says now, read in the
  background)
- **Send a test notification** (Enter): a note like one for you, through the same route, with the sound as set; 3
  seconds later the bell, if the bell is on and the terminal is then unfocused (switch away to see it). Clicking it
  opens nothing in ttyak but brings the window forward as usual, so the click path can be checked. With desktop
  notifications off only the bell is tried; held back by the system's do-not-disturb, the status line says so.

The settings in `settings.toml`, all under `[notifications]`:

| Key | F6 row | Default |
|---|---|---|
| `desktop` | Desktop notifications | `true` |
| `sound` | Sound | `"off"` |
| `sound_volume` | Notification sound volume | `100` |
| `bell` | Bell when not focused | `false` |
| `title_mark` | Mark in the tab title | `true` |
| `follow_system_dnd` | Follow the system's do-not-disturb | `true` |

`ttyak ctl check` shows the same about this machine: the notification service and what it offers, do not disturb, who
shows the notifications and plays their sound, what a click does, and how the window is brought forward.

## Notifications through the terminal

In a terminal that shows notifications itself, ttyak hands them to the terminal (its OSC 99 protocol) instead of the
system's notifier: kitty (tried with 0.48; the query needs 0.36 or newer, by kitty's documentation), and other
terminals that answer the query —
foot 1.18 or newer, Konsole 24.12 or newer, Contour, by the protocol; not tried. ttyak asks the terminal at start
whether it can (`p=?`, answered before ttyak's screen is up).

Clicking one makes the terminal focus its own window **and tab**, and tell ttyak, which opens what it showed — the
message, a reply in its open thread, the conversation of a huddle — in NORMAL (a draft being typed stays; a
notification from another workspace of the profile switches there first). One per conversation, replaced by a newer
one and taken away once read, as above.

The terminal passes the notification on to the desktop's notification server, so the system's do-not-disturb still
applies. The sound: `system` goes with the notification (where the server can play sounds); ttyak's own sounds (chime,
…) are played by ttyak, since the terminal can only ask for the desktop's sounds.

Inside tmux the system's notifier shows them (tmux would hand the terminal's answer about a click to whichever pane is
active), as in every other terminal. `TTYAK_NOTIFY_ROUTE=system` (for debugging) keeps to the system's notifier.

Whether a click can bring the window forward across workspaces is up to the desktop: the terminal asks for focus, and
the window manager may only mark it urgent. Tried: a click on kitty's popup from another workspace (Hyprland) brought
the window forward and opened the message in NORMAL. kitty raises its window for a click on its popup, but not for one
in swaync's panel: when the terminal takes no focus within 0.4 s of the click, ttyak brings the window forward itself
as below (tried).

## A click brings the window forward

A click on a notification the system's notifier showed (every terminal that does not show them itself, tmux,
`TTYAK_NOTIFY_ROUTE=system`; also the update notice) opens what it showed in ttyak — a message its conversation, a
thread reply the thread with the reply selected, a huddle's notification its conversation — for 30 minutes after it
was shown. That is on Linux desktops with an XDG notification server; on macOS and Windows see their sections below
(on Windows only the toast on screen takes a click, not one in the notification center). Then it brings ttyak's window
forward by the first of these that works here:

- inside tmux: ttyak's tmux window and pane are selected and the client last active on its session is switched to
  them (one `tmux` call); then the terminal around that client is brought forward as below, found from that client's
  environment — its desktop's variables (`DISPLAY`, `XAUTHORITY`, `HYPRLAND_INSTANCE_SIGNATURE`, `SWAYSOCK`, …) are
  the ones used, so a tmux server started in an earlier session does not matter
- kitty with remote control on (`allow_remote_control` and `listen_on`, so that ttyak sees `KITTY_LISTEN_ON`; ttyak
  checks with `kitty @ ls` that the window runs one of its parents): `kitty @ focus-window` picks ttyak's window and
  tab; WezTerm: `wezterm cli activate-pane`. Then the window manager as well, which switches to the workspace
- the window manager, for the terminal's window (of the nearest of ttyak's parent processes that has one), by its id:
  Hyprland (`hyprctl dispatch focuswindow address:…`), sway (`swaymsg [con_id=…] focus`), niri (`niri msg action
  focus-window --id …`), KDE Plasma 5 and 6 (a small KWin script loaded over D-Bus with `dbus-send`), X11 (EWMH
  `_NET_ACTIVE_WINDOW`, the window from `WINDOWID` or a parent's `_NET_WM_PID`). When that process has several windows
  (kitty with more than one OS window, footclient, WezTerm, Konsole), the one with ttyak's tab is picked by its title —
  kitty's remote control tells it (kitty retitles the window once it has shown ttyak's tab), otherwise the title ttyak
  set itself (Mark in the tab title on, and ttyak's tab the one shown); when the title can't tell them apart, the
  window manager is left out rather than raising the wrong window
- macOS and Windows: see their sections below

Tried: Hyprland with kitty, on a real desktop (also with `TTYAK_NOTIFY_ROUTE=system`: raised through `kitty @` and
`hyprctl`); sway (headless; also two footclient windows of one foot server, told apart by title); X11 (Xvfb with
openbox and an `.Xauthority` cookie); tmux inside sway, and inside X11 with a stale tmux environment — raised for real
in a container. Not tried: niri, WezTerm, KDE (by their documentation).

## The commands ttyak runs

Each command runs in the background without a shell and gets at most 2 seconds — on macOS an AppleScript gets a
minute, one at a time, since the first one waits for the answer to macOS's question (below). Every step is in the
debug log with how it ended. F6 → Notifications → A click, and `ttyak ctl check`, name the route found here ("via kitty
and Hyprland", or that the window can't be raised on this desktop).

## When the window can't come forward

- GNOME on Wayland: no program may raise another's window there; only a terminal that shows notifications itself
  brings its window forward
- the right tab in terminals without remote control (Ghostty, foot, Alacritty; kitty without `listen_on`, since ttyak
  never talks to kitty over the terminal it draws in): the window comes forward on the tab last used. Windows Terminal
  is the exception (its tab is picked by title, see [Windows](#windows))
- a detached tmux session
- ttyak over `ssh -X`: its parents are the remote machine's, so only `WINDOWID` would tell the window
- a window manager that refuses focus requests from other programs

## macOS

ttyak stays a plain program, not an app bundle, so it uses the Notification Center's older interface: its
notifications come from the terminal's app — Terminal.app, iTerm2, WezTerm, Ghostty, kitty, known by `TERM_PROGRAM`,
kitty by `KITTY_WINDOW_ID`, inside tmux by the app the tmux server was started from (`__CFBundleIdentifier`) — with
that app's icon. They show only where that app's notifications are allowed (System Settings → Notifications →
Terminal, …). While the screen is shared or mirrored macOS keeps them in the Notification Center without a banner
(seen), unless "Allow notifications when mirroring or sharing the display" is on.

macOS 26 refuses the older interface for apps that use the newer one themselves. Tried: Terminal.app's notifications
are shown; kitty's are refused, and so are Finder's, which ttyak sends as when the terminal's app isn't known. In kitty
ttyak uses kitty's own notifications instead ([through the terminal](#notifications-through-the-terminal); not tried
on a Mac). iTerm2, WezTerm and Ghostty are not tried.

A click — on the banner or in the Notification Center — brings the app forward (macOS does that), and ttyak, whose main
thread listens to the Notification Center (everything else runs beside it), opens what the notification showed in
NORMAL and selects its tab: Terminal.app's tab running ttyak (by its tty) and its window, iTerm2's session
(`ITERM_SESSION_ID`) in its tab and window, by AppleScript (`osascript`); after kitty's or WezTerm's remote control,
and for Ghostty, the terminal app is brought forward with its own front window (`open -b`). Tried in Terminal.app. Up
to 32 notifications wait for a click at once; the log says when one goes out without.

The AppleScript may make macOS ask once whether the terminal may control Terminal (or iTerm2) — System Settings →
Privacy & Security → Automation; ttyak waits a minute for the answer, and with "Don't Allow" the tab stays as it is.
(The run on record asked nothing; the question is not seen yet.)

A newer notification of a conversation takes the older one out of the Notification Center, and reading takes it out
(tried). The older interface knows a notification only by its title and text, so two that read the same both go.

Can't do on macOS:

- the Focus modes: no program can read them without extra rights, so Follow the system's do-not-disturb has nothing to
  ask. macOS itself holds the notification and the `system` sound back (not tried); ttyak's own sounds and a huddle
  invitation's ring still play
- inside tmux, raising the terminal around it (tmux's pane is selected; macOS brings the app forward on the click)

## Windows

ttyak stays a plain program too, with no app id of its own (that would take an installer and a Start menu shortcut),
so its toasts go out as Windows PowerShell's — they say "Windows PowerShell". F6 and `ttyak ctl check` say when Windows
has PowerShell's notifications turned off (they would vanish without a trace; tried), and the log says such a toast
was not shown. That setting is read in the background, never while a toast goes out: F6 reads "reading…" until the
first answer.

Each toast carries its conversation as its tag, in a group of its profile's own: two ttyaks on other profiles leave
each other's alone (tried: two profiles in two Windows Terminal tabs each get their own toast and notification-center
entry). A newer one replaces it in place (tried: one entry in the notification center; Windows shows the newer one as
a new popup), and reading takes it off the screen and out of the notification center (tried).

A click on the toast while it is on screen opens what it showed in ttyak, in NORMAL, and brings the console's window
forward: Windows Terminal's window (Windows Terminal 1.14 and newer own the hidden window of the console they give
ttyak; older ones are not supported, see below) or conhost's own window is restored when minimised and brought to the
foreground; after WezTerm's `activate-pane` too (WezTerm on Windows not tried). Windows lets a program take the
foreground only in some cases (the user's last input went to it, or the program in front allows it); otherwise the
window's taskbar button flashes instead. Tried at the desktop in Windows Terminal 1.24 and in conhost.

In Windows Terminal (`WT_SESSION`) ttyak's tab is then selected through UI Automation: the tab titled as ttyak titled
it (Mark in the tab title on; the read/unread mark may differ). Tried: ttyak's tab behind a cmd tab, Windows Terminal
unfocused, a click on the toast showed ttyak's tab with the message. The tab is left as it is, and the log says why,
when no tab or several carry that title:

- two ttyaks of one workspace (two profiles of one workspace in two tabs carry the same title; tried: the tab in front
  stays)
- ttyak in a split pane that isn't the focused one (Windows Terminal names the tab after its focused pane)
- a Windows Terminal profile with `tabTitle` or `suppressApplicationTitle`
- a tab renamed by hand

A click on the toast later, in the notification center, does nothing (tried: no click reaches ttyak and no Windows
PowerShell opens; it has no COM activator registered, which takes an installer). Only the `system` sound goes with the
toast (Windows' own, silenced with it); ttyak plays its own sounds itself.

Follow the system's do-not-disturb asks Windows two things:

- Focus Assist's profile — Windows 11's Do not disturb — read through an undocumented Windows state
  (`WNF_SHEL_QUIETHOURS_ACTIVE_PROFILE_CHANGED`). Priority only and alarms only count as on, since ttyak can't be on
  the priority list. If Windows changes that state, ttyak can't tell and plays its own sound
- the documented notification state (`SHQueryUserNotificationState`): presentation mode and full-screen Direct3D count
  as on; busy (a full-screen app, presentation settings) only while that app is not ttyak's own terminal; and quiet
  time — the first hour after the first sign-in on a new install or after an upgrade — too

While it is on, the toast still goes out, without ttyak's sound, Windows' sound or the bell (and the huddle ring keeps
quiet): Windows' do-not-disturb files it in the notification center, as it does other apps' toasts. In a busy state
with Windows' full-screen rule off Windows may still show a banner (not tried). F6's System do-not-disturb row reads
"Focus Assist: on (undocumented state)…", and the test notification says "sent without sound: Windows' do-not-disturb
is on".

Tried at the desktop: Do not disturb switched on reads as on (profile 1), and so does Windows 11's automatic one for a
full-screen app (profile 2, Windows Terminal full screen); under the automatic one the toast went out without sound,
no banner, filed in the notification center. The same with Do not disturb switched on by hand is not tried yet.

Not supported: Windows Terminal older than 1.14 (it doesn't own the hidden window of the console it gives ttyak, so
ttyak can't find its window; by the code, not tried). A full-screen ttyak there counts as busy, like any other
full-screen app, so its toasts go out without sound or the bell; and a click on a toast opens what it showed in ttyak
but neither brings the window forward nor selects ttyak's tab. Update Windows Terminal.

Can't do on Windows: picking Windows Terminal's tab when it doesn't carry ttyak's title alone (above); a click after
ttyak let go of the toast (ttyak ended, or 30 minutes passed).

## If it doesn't work

- [No desktop notifications](Troubleshooting#no-desktop-notifications)
- [No notification sound](Troubleshooting#no-notification-sound)
- [Notifications during do-not-disturb](Troubleshooting#notifications-during-do-not-disturb)
- [No bell, or no mark in the tab title](Troubleshooting#no-bell-or-no-mark-in-the-tab-title)
- [A click on a notification doesn't bring the window forward](Troubleshooting#a-click-on-a-notification-doesnt-bring-the-window-forward)
- everything else: [Troubleshooting](Troubleshooting)
