# ttyak 0.12.0 — attention (notifications, sounds, bell, badge, status), lighter CI, requirements

Brainstormed with the user on 2026-10-24/25. "The user:" marks their decisions; "Claude's call:" marks mine (open to
their veto). Everything else is left to the builder, as said per section.

## Why

The user runs ttyak all day: kitty on Hyprland, ttyak in its own window, on its own workspace or one it shares with
another messenger. They check it now and then, and notifications plus the sound help. Their real pain: **clicking a
notification doesn't bring them to it.** ttyak opens the conversation, out of sight, and they have to find the
workspace and focus the terminal by hand. Around that sit a few smaller wishes: see at a glance from the tab whether
anything is unread, show ttyak's state in a status bar, and have the terminal or window manager flag ttyak as urgent.

The user: notifications, sounds, the bell and the badge must be **one combined function that works in cooperation**,
not four features bolted on. The user: it must not be kitty- or Hyprland-specific: "function whenever it can
function", on every terminal, desktop and OS where it can work.

## Scope of the release (the user's list)

1. Tab title with a read/unread mark (no count).
2. `ttyak ctl status` (`--json`, `--follow`).
3. A bell when the terminal is not focused.
4. Notifications whose click lands on what they showed and brings the window forward.
5. One notification per conversation, replacing itself.
6. ~~Ctrl+↑/↓ scrolls by one line~~ — moved to the navigation rework (section 6).
7. Less load on GitHub Actions.
8. Requirements written down, and a self-check command.

Version: 0.12.0 (Claude's call). Tag only with the user's word.

## Goals, non-goals, anti-goals

Goals: the attention items work together from one decision; a click lands on exactly what the notification showed,
whatever the platform allows; CI does less useless work.

Non-goals (the user dropped or declined these): a tray icon; an unread **count** in the title; `--format waybar` or
any bar-specific output ("waybar is very specific and far from the only bar out there"); optimised builds in CI /
the tag reusing CI's binaries ("we build debug builds on regular pushes and release builds on releases"); Windows
code signing; GNOME Shell extensions.

Anti-goals:
- The user: turning ttyak into a GUI application (a macOS .app bundle, a Windows installer-registered app id) just to
  make clicks focus a window. Where that's the price, the click still opens the message inside ttyak and the window
  is not raised.
- The user: a click never leaves ttyak in INSERT mode.
- A chime without a bubble, or a bubble during the pause or schedule (the cooperation rule below).
- A bar or status output that shows message text or authors.

## 1–5: Attention — one decision, two outputs

### The decision (per incoming event)

Each incoming message or huddle invitation is classified once, by one function, from what ttyak already knows: the
conversation muted or not, the Slack notification level for it (everything / mentions & DMs / nothing), the user's
keywords, mentions (incl. what `mentions_me` counts today), DMs and group DMs, replies in the user's threads, the
pause and schedule (`dnd_active`), whether the user is **viewing** it (terminal focused AND that conversation or
thread on screen — today's rule, which matches Slack's: Slack holds a notification back only when its window is active
and you are in that conversation), and the system's own do-not-disturb where ttyak can read it (below).

The result is one of:
- **nothing**: the user's own messages, hidden or system ones, muted conversations.
- **unread**: it counts as unread but doesn't interrupt.
- **for you**: it interrupts — what today's `notify::notification` returns Some for (DMs, mentions, keywords, replies
  in your threads, group DMs, everything-level channels; huddle starts in DMs).

Today's `notify::notification` / `huddle_notification` rules stay the source of truth for "for you"; the builder
reshapes them into the one decision rather than changing who gets notified. (The builder: the exact type/shape.)

### Output 1 — state (the badge): always the truth

State is not an event: it follows what is unread right now, and it is **not** held back by the pause, the schedule,
the system's DND or focus — as Slack's own badge isn't.

- **read**: nothing unread in any signed-in workspace (muted conversations don't count).
- **unread**: anything the sidebar shows as unread (bold or with a count), in any workspace; the Threads and Activity
  rows count when they show unread.
- **for you**: Claude's call (builder check): **Slack's own counts** decide it — an unread DM / group DM or a mention
  as Slack counts them (the numbers ttyak already reads: the sidebar's badges here, `others::badge` /
  `channel_badges` for the other workspaces), as Slack's dock icon shows a number for exactly these. Not ttyak's
  own event rule: that one decides interruptions only. So `◆` agrees with Slack's badge and other clients, and works
  for messages that came before this run.

The user (builder check): **no read marks while the terminal is unfocused.** Today `event_loop.rs` marks the
conversation on screen read on every pass, focused or not, so a DM arriving while the user is elsewhere is read at
once (on Slack too), the title shows `○`, and the withdraw-on-read rule would close its bubble. From now on the
conversation (and thread) on screen is marked read only while the terminal is focused; on focus gain it is marked read
as today. A terminal that never reports focus behaves as today (focus assumed). Slack's own client is believed to do
the same — the builder confirms it live.

Reading — here, or in another client (Slack's read events) — clears it. Claude's call (builder check): for the
workspaces not on screen, the state follows their existing 30-second `client.counts` poll (`others.rs`); no live read
tracking is added for them.

### Output 2 — interruption: all together or nothing

Only for **for you** events, and only when not held back by the pause/schedule (or the system's DND where known):
the bubble, its sound, and the bell (the bell only while the terminal is unfocused) fire **together**. Nothing chimes
without a bubble; nothing pops while paused. While the terminal is focused and the user is in another conversation, a
bubble still shows (Slack does the same); no bell.

Claude's call (builder check): the **gates** above (pause, schedule, system DND, viewing) hold back every output; the
user's **switches** pick which outputs exist: desktop notifications on/off (the bubble), the sound (belongs to the
bubble: no bubble, no sound — as today), the bell (its own switch: it still rings for a for-you event with desktop
notifications off, for a user who wants only the urgency hint). Sound `off` actively mutes the route (the freedesktop
`suppress-sound` hint, OSC 99 silent) — today `off` sends no hint and a server may play its own.

Special: the update notice keeps its own rules (silent, follows only the notifications on/off setting; Ruling X/Y of
the 0.11.0 plan).

### The sound goes with the notification

The user's earlier worry ("ttyak keeps chiming while I'm away") came from ttyak playing its own sound beside the
desktop notification, so the system's DND hid the popup but not the chime. Fix at the root:

- Where the route can carry a sound (a freedesktop server with the `sound` capability, kitty's OSC 99 `s=` key, macOS,
  Windows toasts), the notification plays it — the system's DND then silences both. The `system` sound value maps to
  the route's default sound; ttyak's own sounds (chime, ping, knock) are used where the route can play a named/file
  sound, else (the builder's call) the route's default.
- Where it can't (e.g. this machine: swaync 0.9.0 has no `sound` capability), ttyak's own player stays, and ttyak asks
  the system whether notifications are suppressed, where it can: swaync, mako, dunst, GNOME (`show-banners`), KDE
  (`Inhibited`), Windows (`SHQueryUserNotificationState`), macOS Focus if any unprivileged way exists. Unknown =
  not suppressed. (The builder: how to ask each; any that need a privileged/entitled API are skipped.)

### 1. Tab title

- The user: the mark is **always** in the title: `○` read, `●` unread, `◆` for you. (The user chose `◆` over `@`, `★`,
  `!`; `◉` was too close to `●`.)
- Claude's call: format `○ ttyak - <workspace>` — the mark first, because tab bars cut long titles from the right.
  `<workspace>` = the workspace on screen; the mark covers all workspaces.
- Set with OSC 2 (crossterm `SetTitle`); the terminal's previous title is saved and restored on quit (XTWINOPS 22/23
  push/pop where supported; the builder handles terminals that don't). The workspace name is sanitised (no control or
  bidi characters).
- Setting: on/off, default on, in `settings.toml` and F6 → Notifications ("Mark in the tab title", see Settings below).
- Inside tmux the title reaches the pane; the outer terminal only with tmux's `set-titles` — README says so.

### 2. `ttyak ctl status`

- Running ttyak writes a small status file whenever the state changes (atomically: write + rename), one per profile,
  readable only by the user (0600, in the session's runtime folder: `$XDG_RUNTIME_DIR` on Linux, an equivalent per-user
  folder elsewhere — the builder picks). Removed on a clean quit; a stale file (ttyak's pid gone) reads as not running.
- `ttyak ctl status` reads that file — never Slack — so a bar can poll it every second. Output:
  - plain: one readable line (e.g. `◆ rslack-testbed: 2 for you · paused until 14:00`);
  - `--json`: the full object;
  - `--follow`: a new line (plain or `--json`) on each change, for bars that read a stream.
  - With `--profile` as every `ctl` command. Not running → says so (`"running": false` in JSON), exit 0 (Claude's call:
    a bar shouldn't see an error just because ttyak is closed).
- Fields (Claude's draft; the builder may rename, keep it versioned): `version` (schema), `running`, `state`
  (`read`/`unread`/`for_you`), `mark` (`○●◆`), `for_you` (count, total), per workspace `{name, current, unread,
  for_you}`, `paused` (`until` or null; the schedule's quiet time counts), `huddle` (`{in, where}` or null), `update`
  (a waiting version or null), `updated_at`.
- The user: no bar-specific format. README shows one or two `jq` one-liners (Claude's call: docs only).
- Never in it: message text, authors.

### 3. Bell

- BEL when a **for you** interruption fires and the terminal is unfocused (crossterm focus events; ttyak already
  enables them and tracks `terminal_focused`). Terminals turn it into an urgency hint (the window manager / bar
  highlights the workspace), tmux flags the window.
- Claude's call: a terminal that never reported focus → no bell (better silent than ringing while you read).
- Setting: on/off, `settings.toml` + F6 → Notifications ("Bell when not focused"). Claude's call: default on.
- Some terminals also beep on BEL (kitty's `enable_audio_bell` is on by default). ttyak can't turn that off; the README
  names the setting.

### 4. Click lands on what was shown, and brings the window forward

- The user: a click lands on **exactly what the notification showed** — the message; a reply inside its open thread;
  the huddle's conversation for a huddle invite — and always in **NORMAL**, never INSERT (a draft being typed is kept).
- The user: a **full attempt** on Linux, macOS and Windows, proven on this machine, the Mac `macbook` and the VM
  `win11-dev`; but ttyak stays a plain binary (anti-goal above).
- Two jobs: (a) hearing about the click, (b) raising the window.

Routes, best first (from the research of 2026-10-24; the builder re-checks versions):

(a)+(b) together — the terminal's own notification protocol, **OSC 99** (kitty ≥ 0.36 for query/replace/close; foot ≥
1.18; Konsole ≥ 24.12; Contour): query support at start (`p=?`, fenced by DA1); send with the conversation as id,
`a=focus,report`; the terminal focuses its window and tab and reports the click back to ttyak (`OSC 99;i=…`), which
then opens the target. Under tmux only with `allow-passthrough` (wrapped), and a report may arrive in the active pane
only — the builder decides whether tmux uses this route or the next.

Otherwise (a) via the system notifier (today's notify-rust route; on macOS and Windows notify-rust's click callbacks,
which ttyak doesn't use yet), then (b) the best raise route found:
1. inside tmux: select ttyak's window and pane, switch the client — then raise the outer terminal by a route below;
2. the terminal's remote control: kitty (`KITTY_WINDOW_ID` + remote control on), WezTerm (`WEZTERM_PANE`);
3. the window manager: Hyprland (`hyprctl dispatch focuswindow pid:<terminal pid>`, found by walking up ttyak's parent
   processes), sway (`[pid=…] focus`), niri (window by pid → `focus-window --id`), KDE (KWin script over D-Bus),
   X11 (`_NET_ACTIVE_WINDOW`, window from `WINDOWID` or `_NET_WM_PID` of an ancestor);
4. macOS: iTerm2 / Terminal.app by AppleScript (selects the tab). Claude's call: accepted although macOS asks the user
   once "ttyak wants to control …" (a prompt, not packaging — the user didn't object); otherwise activate the host
   terminal app;
5. Windows: bring the console window forward (best effort; Windows Terminal's tab can't be picked from outside).

Terminals whose own notifications focus but don't report the click (Ghostty, iTerm2, Windows Terminal): Claude's call:
use the system notifier there, so the click always lands on the message; the window manager raises the window, the
tab may not be the right one. Landing on the right message beats the right tab.

Known can't-do (said in README): GNOME on Wayland unless the terminal speaks OSC 99 itself; the right tab in Ghostty,
foot, Alacritty, Windows Terminal; Windows Action Center clicks after the toast left the screen; macOS click reporting
if the unbundled notification API doesn't deliver it.

F6 shows what this machine got (Claude's call), e.g. "A click opens it here and brings the window forward via
Hyprland" / "… opens it here (the window can't be raised on this desktop)". Read-only, for knowing why it does or
doesn't work.

The builder: the route detection and fallbacks, how `notify.rs`'s `post` is split per route, the tmux decision, and
the order inside each OS.

### 5. One notification per conversation

- A newer for-you event in the same conversation (thread replies: Claude's call, the conversation — the builder may key
  threads apart if Slack's own client does) replaces the bubble instead of stacking: freedesktop `replaces_id`, OSC 99
  same `i=`; macOS/Windows where notify-rust allows, else a small patch / direct OS calls (Windows toast `Tag`/`Group`);
  where impossible without packaging (macOS's bundle-only API), it stacks as today.
- Claude's call: a bubble is **withdrawn once its conversation is read** anywhere (freedesktop `CloseNotification`,
  OSC 99 close), where the route allows.
- One click waiter per live bubble; a replaced bubble's waiter ends.

### Settings: F6 → Notifications becomes the one place (the user: "ok, let's try this")

Today the attention settings sit in three sections (Availability: pause + schedule; Notifications: desktop, sound,
levels, keywords, VIPs; Audio & video: the notification sound's volume). The Notifications section is reworked around
the one system (layout as shown to the user; the builder adapts the rows to F6's row/help machinery):

```
Notifications
  What notifies you   (kept on Slack, as today)
    This conversation · Channels by default · Keywords · VIPs
  How you're told
    Desktop notifications      on
    Sound                      chime      volume 80 %   (moved from Audio & video)
    Bell when not focused      on
    Mark in the tab title      on         ○ ● ◆
  Quiet
    Paused until / schedule    (shown here and in Availability; same setting, both ways in)
    Follow the system's do-not-disturb   on
  On this machine   (shown only)
    Notifications shown by     kitty           sound played by   ttyak
    A click                    opens it and brings the window forward via kitty
    System do-not-disturb      swaync: off
    Send a test notification   (Enter)
```

Claude's calls inside it:
- "Follow the system's do-not-disturb" is a switch (default on), for a system whose detection misfires.
- "Send a test notification" sends a real one through the route found, with its sound, and — after a few seconds, so
  the user can switch away — the bell if unfocused; for checking a setup (and the live tests).
- Pause and schedule stay in Availability (they are Slack availability) and also appear here.
- `settings.toml`: the new switches under `[notifications]`; the sound volume's key may move there too, with the old
  `[audio] sound_volume` still read (old files keep working; the builder decides whether to move the key at all).
- Every row gets its F6 help text (W1's per-item help), words through `tr`.

## 6. Ctrl+↑/↓ → moved into a rethink of scrolling and navigation

The builder check found macOS takes Ctrl+↑/↓ (Mission Control / App Exposé). The user: "let's instead re-think
scrolling and navigation in general. it was originally designed when the application behaved quite differently." —
The user then: "let's put the navigation re-work back in the queue and work on the rest instead." So **not in this
release**: Ctrl+↑/↓ and one-line scrolling go into the navigation rework (PROGRESS queue), with what is known so far:
one-line scrolling should exist, NORMAL only (the user), the selection follows the scroll (VY rule), macOS takes
Ctrl+↑/↓ by default (vim's Ctrl+E / Ctrl+Y were floated, not decided), and the keys were laid out in queue U, before
the main-pane views, the selection that follows scrolling and the history existed.

## 7. Less load on GitHub Actions (the user: "take all of them")

1. Release builds don't **save** a build cache (they may restore one). The repo's cache sits at 10.7 GB of 10 GB;
   ~2.6 GB of it is release-build caches GitHub drops after 7 days unused — releases come further apart.
2. CI's Windows job builds **debug**, not `--release` (still proves it compiles; the MinGW-DLL check keeps working on
   the debug exe or is moved to the release workflow — the builder's call).
3. A newer push to master **cancels** the older CI run still in progress (`concurrency`, `cancel-in-progress`).
4. CI skips pushes that change only `site/**` and `tools/site/**` (as it skips docs); Pages tests those.
5. Only master **saves** caches; pull requests restore only (`Swatinem/rust-cache` `save-if`, the apt cache likewise).
6. Drop the duplicate `cargo build` before `cargo test` in the Linux and macOS jobs.

Keep: debug builds + tests on every push, optimised builds only for releases, timeouts, docs `paths-ignore`.

## 8. Requirements and `ttyak ctl check`

README (and the site's install section, mirrored) gets a **Requirements** section, per system: **needs** vs **works
best with**:
- fonts: an emoji font (Noto Color Emoji / Apple Color Emoji / Segoe UI Emoji), a symbol font (DejaVu / Noto Sans
  Symbols) for `● ○ ◆ ★ ⏎ ↪ ⬆ ✕` and box/block characters, CJK fonts optional — with package names per OS; F6 →
  "Emoji as names" as the way out; misaligned rows = the terminal's emoji width;
- the terminal: UTF-8, mouse reporting, true colour for the built-in themes (`terminal` theme otherwise), focus
  reporting (bell/badge), OSC 8 links, pictures (as today), the minimum size;
- Windows: Windows Terminal recommended (conhost draws emoji as boxes, no pictures); macOS Terminal.app's colour
  support (the builder checks on the Mac);
- desktop helpers: link opener (`xdg-open`/`open`), notification service, clipboard on Wayland, a browser for the
  browser sign-in, ffmpeg/GStreamer (already listed).

`ttyak ctl check` (the user: yes): reports what the terminal answered (colours, pictures, focus reporting, OSC 99,
links where queryable), what the system has (link opener, notifier and its capabilities, click/raise route found,
ffmpeg, GStreamer), and prints one sample line per glyph group (symbols, arrows, boxes/blocks, emoji, CJK) for the
user to judge — a program can't tell which font the terminal draws with. Words through `tr`. No network, no Slack.

## Constraints (standing rules apply)

- Everything that waits is configurable in seconds (`settings.toml` + F6); new words through `tr`; match the
  surrounding code.
- Live tests: never post under the `test` profile; rslack-testbed A/B only. Never capture mic/camera/screen. No desktop
  prompts from tests (gpg/keyring/polkit). Live checks that **move focus or switch the user's workspace** need the
  user's word at the time, or run where they can't touch the user's screen (e.g. a headless/nested compositor). The
  macOS automation prompt needs the user at the Mac.
- Clipboard saved/restored around live checks; `kitty @ get-text` instead of screenshots; close test windows.
- Mac and VM: clean up, restore the VM's power state.

## Left to the builder

The decision type and where it lives; per-route code layout; route detection; the status file's path per OS and JSON
names; how `--follow` waits for changes; the settings' names and F6 places; the tmux route choice; system-DND queries
per server; how each OS's click callback and replace are wired; test strategy (unit for the decision and title/status
rendering, live per route on the machines above).
