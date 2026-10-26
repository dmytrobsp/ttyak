When something does not work, start with `ttyak ctl check`, run in the terminal where ttyak runs (not through a pipe;
on Windows at the desktop, not over ssh). It says what your terminal and system offer and what is missing, without
Slack and without the network. Each entry below says what you see, why, and what to do, with a link to the page that
has the detail. If yours is not here, see [Reporting a bug](#reporting-a-bug).

## winget finds nothing

**What you see:** `winget install dmytrobsp.ttyak` finds no package.

**Why:** winget and Chocolatey review each new version before they list it, a few days after the release. winget has
not listed ttyak yet: its first version is waiting for review.

**What to do:** use the install script or Scoop. More: [Install](Install#package-managers).

## Windows: SmartScreen warns about ttyak.exe

**What you see:** SmartScreen warns the first time you run `ttyak.exe`.

**Why:** `ttyak.exe` is not code-signed yet (the macOS builds are signed and notarized).

**What to do:** **More info → Run anyway**. To check the download first, see [Verifying downloads](Verifying-downloads).
More: [Install](Install#the-install-script).

## Windows: the first keys after start are lost

**What you see:** keys typed in the first moment (about 0.4 s) after ttyak starts, in Windows Terminal and in conhost,
never arrive; keys typed before ttyak started are lost too.

**Why:** a known bug, not fixed yet: [#1](https://github.com/dmytrobsp/ttyak/issues/1). After that moment input is
fine.

**What to do:** until it is fixed, wait a moment after the start before you type.

## Signing in: no browser found, or a browser can't be read

**What you see:** in the guide the row "in your browser" is dim with `no browser found here — use the code`, or a
browser profile is dim with the reason (`can't read Chrome's key: …`). `--from-browser` stops with `no Slack session
in Firefox — sign in to Slack there first`, or `… has Slack cookies, but none signs in to …`; on Windows
`--from-browser chrome` says `Chrome's cookies cannot be read on this system (app-bound encryption)`.

**Why:** ttyak takes the session from the browser's own files on the machine it runs on. It reads Firefox everywhere,
and Chrome, Chromium, Edge and Brave on Linux and macOS. Chrome's family encrypts its cookies: on Linux and macOS ttyak
needs the browser's key from the system keyring or the Keychain; on Windows only Chrome itself can read them. Over SSH
the browser you use is on another machine.

**What to do:** sign in to Slack in that browser first; on Windows use Firefox. Or sign in with a code from your email
(it works over SSH and with no browser) or paste the `d` cookie. More: [In your browser](Signing-in#in-your-browser),
[From the browser you already use](Signing-in#from-the-browser-you-already-use).

## Signing in: the email code doesn't work

**What you see:** ttyak asks for the code again, or says `Slack says too many tries — wait a while, then ask for a new
code` and leaves the code way.

**Why:** the code was wrong or has expired, or Slack has seen too many tries. Asking for the code needs Slack's
captcha, so it has to happen in a real browser (your phone or any browser), not in ttyak.

**What to do:** ask for a new code on the phone (or after a while, when Slack said too many tries) and type it into
ttyak's `Code: ___-___` prompt; don't enter it on the phone. More:
[With a code from your email](Signing-in#with-a-code-from-your-email).

## A workspace needs SSO or two-factor

**What you see:** in the code way a workspace is dim: `needs your browser (SSO)` or `needs your browser (two-factor)`.
When every workspace of the address is like that, ttyak says so for each and shows no list.

**Why:** Slack's list marks these workspaces, and the email code can't sign in to them.

**What to do:** sign in to them in your browser: the browser way, `--from-browser`, or a pasted cookie (not tried yet
with such a workspace). More: [Read this first](Signing-in#read-this-first),
[In your browser](Signing-in#in-your-browser).

## The Keychain or the keyring asks

**What you see:** on macOS the Keychain asks whether ttyak may use a browser's key, once per browser of Chrome's
family, one after another. On Linux a locked keyring asks to be unlocked.

**Why:** Chrome, Chromium, Edge and Brave encrypt their cookies with a key kept there; ttyak needs it to read the
cookies (it only reads them).

**What to do:** on macOS answer “Allow”, not “Always Allow”: that would let any program read the key through the
`security` tool without asking. Without the key (refused, or no answer within a minute) that browser can't be read:
use Firefox, the code or a pasted cookie. (The Keychain question has not been tried yet with a real Chrome key, nor a
locked keyring.) More: [From the browser you already use](Signing-in#from-the-browser-you-already-use).

## A workspace is signed out

**What you see:** at start ttyak asks what to do about a signed-out workspace (sign in again, remove it, launch
anyway). While ttyak runs the status line says `Acme: signed out — ttyak auth login, then restart ttyak` and the rail
shows the workspace dim with `⊘`. Without a terminal ttyak prints a warning, or stops with `every workspace of this
profile is signed out — ttyak auth login`.

**Why:** Slack ended the session.

**What to do:** **sign in again** at start, or run `ttyak auth login` and restart ttyak. `ttyak auth status` says which
sessions still work. More: [When a session ends](Profiles-and-sessions#when-a-session-ends).

## Pictures show as half blocks

**What you see:** image thumbnails drawn with block characters, initials instead of profile pictures, custom emoji as
`:name:`. `ttyak ctl check`'s pictures row says "half blocks only".

**Why:** the terminal did not say at start that it draws with kitty's protocol, sixel or iTerm2's. It may not have
been asked right (over ssh or in tmux), or it draws no pictures at all (the Windows console, conhost, draws only half
blocks).

**What to do:** use a terminal that draws pictures: kitty, WezTerm, foot and Windows Terminal are tried. F6 → Messages
& media → Draw pictures with shows what the terminal offers; `TTYAK_GRAPHICS` forces a way when the terminal is not
asked right. More: [Images and video](Images-and-video#terminals-that-draw-pictures),
[How pictures are drawn](Images-and-video#how-pictures-are-drawn), [Requirements](Requirements#the-terminal).

## Emoji or symbols show as boxes

**What you see:** empty boxes instead of emoji, or instead of ttyak's symbols (`● ○ ◆ ★ ⏎ ↪ ⬆ ✕`, box lines); or rows
that look misaligned. `ttyak ctl check` prints a line of each kind of glyph to judge by.

**Why:** the terminal draws with the fonts your system has, and one is missing. Misaligned rows mean the terminal
measures an emoji's width differently from ttyak.

**What to do:** install an emoji font and a symbol font ([Requirements](Requirements#fonts) names the packages), or
turn on F6 → Messages & media → "Show emoji as :names:" (it also cures misaligned rows). The Windows console (conhost)
draws emoji in monochrome; Windows Terminal is recommended.

## Colours look wrong

**What you see:** the built-in themes' colours look off; `ttyak ctl check`'s true colour row says no or "not known".

**Why:** the terminal has no 24-bit colour, or does not say so (macOS Terminal.app before macOS 26, for one).

**What to do:** choose the `terminal` theme in F6 → Appearance → Colour theme: it uses your terminal's own palette.
More: [Themes](Themes#choosing-one), [Requirements](Requirements#the-terminal).

## The thread doesn't open beside the conversation

**What you see:** a thread takes the conversation's place instead of opening beside it.

**Why:** the part right of the sidebar is narrower than 100 columns (a terminal under 132 columns; 138 with the
workspace rail).

**What to do:** make the terminal wider. Narrower, close the thread or go back to the messages or the message box
(which closes it). More: [Requirements](Requirements#the-terminal), [The screen](The-screen#the-layout).

## Links don't open in the browser

**What you see:** opening a link (a click, or `␣o`) does nothing.

**Why:** ttyak hands web and mail links to the system's opener (`xdg-open` from xdg-utils on Linux, `open` on macOS,
Windows' URL handler) and opens no other kind. `ttyak ctl check`'s link opener row says whether one was found. Links
into your own Slack open in ttyak, not in the browser.

**What to do:** on Linux install xdg-utils, or set `TTYAK_OPEN_WITH` to the program to open links with. In kitty
Ctrl+Shift+click opens a link in the browser. More: [Links into your Slack](Links-into-your-Slack#opening-it),
[Settings](Settings#environment-variables).

## Copy or paste doesn't work

**What you see:** copying (`␣y`) does not reach other programs, or Ctrl+V attaches no picture.

**Why:** on Linux under Wayland the clipboard needs a compositor with the data-control protocol; without it ttyak
copies through the terminal (OSC 52). The terminal's own paste (Ctrl+Shift+V) carries only text, and over SSH there is
no clipboard to read. `ttyak ctl check`'s clipboard row says which clipboard ttyak uses.

**What to do:** paste pictures and files with Ctrl+V in the message box, on the machine ttyak runs on. More:
[Requirements](Requirements#linux), [Keys](Keys#typing-insert).

## Ctrl+Space does nothing

**What you see:** while you type or in a dialog, Ctrl+Space (the leader there) never opens the which-key panel.

**Why:** macOS uses Ctrl+Space to switch input sources, and Windows input methods may take it: the key never reaches
ttyak.

**What to do:** rebind `insert_leader` in F6 → Keys. More: [Keys](Keys#modes-and-the-leader),
[Rebinding keys](Rebinding-keys#when-a-binding-does-not-work).

## keys.toml or settings.toml is ignored

**What you see:** a key you bound does nothing, or a setting you wrote by hand has no effect; the status line names a
problem at start.

**Why:** a `keys.toml` entry with a problem keeps its default (the status line shows the first problem, "(and N more
key problems, see the log)"); plain characters can't be bound where you type. A `settings.toml` that does not parse is
not used at all: ttyak runs with the defaults and says why. Both files are read at start and at each switch of
workspace.

**What to do:** fix the entry the status line or the log names; or change it in F6, which writes the file for you (F6
writes the whole `settings.toml` again: comments in it are not kept). More:
[Rebinding keys](Rebinding-keys#when-a-binding-does-not-work), [Settings](Settings#settingstoml).

## No desktop notifications

**What you see:** messages arrive, but no desktop notification.

**Why, and what to do:**

- F6 → Notifications → Desktop notifications is off, or notifications are paused or outside your notification
  schedule (F6 → Availability; `/dnd off` resumes).
- What notifies follows Slack's settings: DMs and group DMs, mentions, keywords and replies in your threads; every
  message only in a channel set to "all new messages" (and replies only in threads you are in); nothing from a
  conversation set to "nothing" or muted. Nothing fires for the conversation you are looking at in a focused terminal,
  nor for your own messages.
- The system's do-not-disturb is on, and ttyak follows it (F6 → Notifications → System do-not-disturb says what the
  system says now).
- Linux: no notification server runs (`ttyak ctl check`'s notification service row says "none answered"). Start one:
  swaync, mako, dunst, or GNOME's or KDE's own.
- macOS: notifications come as the terminal app's and show only where that app's notifications are allowed (System
  Settings → Notifications). macOS 26 refuses them for kitty (in kitty ttyak uses kitty's own; not tried on a Mac) and
  for Finder, used when the terminal's app isn't known. While the screen is shared they go to the Notification Center
  without a banner.
- Windows: toasts come as Windows PowerShell's; when its notifications are turned off, F6 and `ttyak ctl check` say
  `Windows PowerShell's notifications are turned off (Settings → System → Notifications)`.

F6 → Notifications → **Send a test notification** sends one through the same route. More:
[Desktop notifications](Notifications-and-attention#desktop-notifications),
[Settings in F6](Notifications-and-attention#settings-in-f6), [macOS](Notifications-and-attention#macos),
[Windows](Notifications-and-attention#windows), [Requirements](Requirements#linux).

## No notification sound

**Why, and what to do:**

- F6 → Notifications → Sound is `off` by default: choose `chime`, `ping`, `knock` or `system`.
- With desktop notifications off there is no sound either.
- `system` (the desktop's own sound) stays silent where the notification server cannot play sounds: swaync's cannot
  (seen), and by their documentation neither can mako's or dunst's. Choose a built-in sound, which ttyak plays itself.
- Where the server plays the sound, the system's do-not-disturb silences it too, and ttyak's volume and speaker
  setting do not apply.
- Notification sound volume (0 to 200 %) sets how loud ttyak plays its own sounds.

More: [The sound](Notifications-and-attention#the-sound).

## Notifications during do-not-disturb

**What you see:** notifications or their sound come although the system's do-not-disturb is on.

**Why:**

- F6 → Notifications → Follow the system's do-not-disturb is off.
- ttyak can ask only some notification servers: swaync (tried), mako, dunst, GNOME and KDE (by their documentation;
  not tried yet). With any other server, or when it does not answer within a second, notifications go through. A
  do-not-disturb switched on just before may be missed for up to 2 seconds.
- macOS: no program can read the Focus modes. macOS itself holds the notification and the `system` sound back (not
  tried); ttyak's own sounds and a huddle invitation's ring still play.
- Windows: the toast still goes out, without its sound or the bell, for Windows to file in the notification center,
  as it does other apps' toasts.

**What to do:** ttyak's own pause and schedule always hold everything back: `␣an` pauses for 30 minutes, `/dnd 2h`
for a time, F6 → Availability has the schedule. On macOS choose the `system` sound or none. More:
[Do-not-disturb](Notifications-and-attention#do-not-disturb), [macOS](Notifications-and-attention#macos),
[Windows](Notifications-and-attention#windows).

## No bell, or no mark in the tab title

**Why, and what to do:**

- The bell is off by default: F6 → Notifications → Bell when not focused. It rings only with a notification while
  the terminal is not focused, never while notifications are paused, and not in a terminal that does not report focus.
- The mark in the tab title is on by default (F6 → Notifications → Mark in the tab title). In tmux it reaches the
  terminal's title only with `set -g set-titles on` (not tried).

More: [The bell](Notifications-and-attention#the-bell), [The tab title](Notifications-and-attention#the-tab-title).

## A click on a notification doesn't bring the window forward

**What you see:** a click opens the message in ttyak, but its window stays behind, or comes forward on another tab, or
its taskbar button flashes.

**Why:** bringing a window forward is up to the desktop. It can't be done on GNOME under Wayland, in a detached tmux
session, over `ssh -X`, or with a window manager that refuses focus requests; terminals without remote control (foot,
Ghostty, Alacritty, kitty without `listen_on`) come forward on the tab last used. On Windows a click in the
notification center does nothing (only the toast on screen works), Windows may only flash the taskbar button, and
Windows Terminal's tab is found by its title. On macOS a "Don't Allow" to the automation question leaves the tab as it
is. A click after 30 minutes, or after ttyak ended, opens nothing.

**What to do:** F6 → Notifications → A click, and `ttyak ctl check`, name the route found here, or say "the window
can't be raised on this desktop". In kitty turn on remote control (`allow_remote_control` and `listen_on`). Keep Mark
in the tab title on: several windows of one terminal, and Windows Terminal's tabs, are told apart by it. Send a test
notification from F6 → Notifications to try the click; every step is in the debug log. More:
[A click brings the window forward](Notifications-and-attention#a-click-brings-the-window-forward),
[When the window can't come forward](Notifications-and-attention#when-the-window-cant-come-forward),
[Windows](Notifications-and-attention#windows), [macOS](Notifications-and-attention#macos).

## A huddle invitation doesn't ring

**Why:** only a huddle started in a DM or group DM with you rings, and only while you are not in a huddle yourself.
While you are in Slack's Do Not Disturb (snoozed or in your schedule) Slack sends no invitation at all. The system's
do-not-disturb keeps the ring quiet when ttyak follows it (not on macOS). The ring sound can be turned off in F6 →
Huddles.

More: [Invitations](Huddles#invitations), [Sounds](Huddles#sounds-f6--huddles).

## Huddle audio: no sound, or the wrong microphone or speaker

**What you see:** others don't hear you, you hear nothing, or the call bar says `no sound: …`.

**Why, and what to do:**

- You join muted: `␣hm` unmutes. The microphone is opened only while you are unmuted.
- Microphone and Speaker are the system's default until you choose one in F6 → Audio & video. `ttyak ctl
  audio-devices` lists the devices. `TTYAK_AUDIO_INPUT` / `TTYAK_AUDIO_OUTPUT` win over F6; a device they name that
  is missing is an error (`no input device matching …`), with no silent fallback.
- `no sound: …` in the call bar: the speaker could not be opened (why follows). Without a sound card a huddle still
  works, silently.
- With a Speaking threshold set, quieter microphone input is sent as silence; Microphone gain raises the level.
- On macOS and Windows the speaker and the microphone have not been tried yet.

More: [Audio](Huddles#audio-f6--audio--video), [The call bar](Huddles#the-call-bar),
[Where it has been tried](Huddles#where-it-has-been-tried).

## Video, the camera or screen sharing doesn't work

**Why, and what to do:**

- Video playback says `cannot play video: …`: it needs `ffmpeg` and `ffprobe` on your PATH (`ttyak ctl check` has a
  row for them).
- `␣hc` says `no camera chosen (F6 → Audio & video → Camera)`: choose the camera there first (`ttyak ctl cameras` lists
  them). The camera is read through `ffmpeg`.
- Sharing the screen under Wayland needs GStreamer's `gst-launch-1.0` and its PipeWire plugin; the desktop then asks
  what to share. Elsewhere it goes through `ffmpeg` and shares the whole screen; on macOS the terminal needs the Screen
  Recording permission.
- On macOS and Windows a real camera and sharing the screen have not been tried yet.

More: [Video: cameras and screen shares](Huddles#video-cameras-and-screen-shares), [Video](Images-and-video#video),
[Requirements](Requirements#linux).

## No update notice

**Why:** ttyak checks once a day (F6 → Updates → Check at most every), and tells a new version once, ever: a desktop
notification (when they are on) and `⬆ <version>` in the status line. With F6 → Updates → Check for updates off it
does not look. Pre-releases are never offered.

**What to do:** F6 → Updates → **Check now** looks at once. More: [Updates](Updates#the-check).

## An update says it cannot write the folder

**What you see:** the update screen says `ttyak cannot write <folder>, so it cannot update itself` and shows a command
instead of **Update now**.

**Why:** ttyak updates itself only where the install script put it, in a folder it can write. A copy from a package
manager or a package shows its own command or the release page instead.

**What to do:** run the command shown (`y` copies it) as someone who can write there, or, when it is your folder,
once you can write there. More: [The update screen](Updates#the-update-screen), [Install](Install#update).

## Reporting a bug

Open an issue with the bug form: <https://github.com/dmytrobsp/ttyak/issues/new/choose> (there is a form for ideas
too). It asks for:

- what happened, and the steps to get there
- `ttyak --version`, your system and your terminal
- the output of `ttyak ctl check`, run in the terminal where the problem happens
- optionally, log lines: start ttyak with `TTYAK_LOG=ttyak=debug`, do it again, and copy the lines around it from
  `ttyak.log` in the profile's cache folder (`~/.cache/ttyak/<profile>/` on Linux; the others on
  [Settings](Settings#environment-variables))

Leave out anything private — tokens, cookies, message text, names of people or workspaces — and look log lines over
before you paste them. Known problems carry the
[known-issue](https://github.com/dmytrobsp/ttyak/issues?q=label%3Aknown-issue) label.
