What ttyak needs, and what makes it better. `ttyak ctl check` reports what your terminal and system offer (no Slack,
no network; run it in your terminal, not through a pipe) and prints one line of each kind of glyph for you to judge:
look for empty boxes or misaligned columns.

## The terminal

Any system:

- needs: UTF-8, and 80 × 24 cells. A thread opens beside the conversation when the part right of the sidebar is at
  least 100 columns wide (a terminal of 132 columns; 138 with the workspace rail, shown with two or more workspaces);
  narrower, it takes the conversation's place until you close it or go back to the messages or the message box
  (which closes it)
- works best with: mouse reporting; true colour for the built-in themes (otherwise choose the `terminal` theme in F6 →
  Appearance, which uses your terminal's own palette); focus reporting (the bell, the tab-title badge and read marks
  need it); OSC 8 links
- pictures: kitty, WezTerm and foot draw them (tried), and Windows Terminal answers as a sixel terminal (tried); so
  can any other sixel terminal, and iTerm2 and Ghostty speak protocols ttyak draws with (iTerm2's own, kitty's),
  though neither is tried yet. Other terminals get half blocks (more on [Images and video](Images-and-video))
- macOS Terminal.app: 24-bit colour came with macOS 26; on macOS 26.5 Terminal.app sets `COLORTERM` and `ctl check`
  says yes (seen on a Mac). Before macOS 26 use the `terminal` theme. It answers none of `ctl check`'s questions about
  mouse, focus, bracketed paste or OSC 8 ("not known"), and its OSC 8 sample is not a link.

## Fonts

The terminal draws with what your system has:

- an emoji font: Noto Color Emoji (Debian/Ubuntu `fonts-noto-color-emoji`, Fedora `google-noto-color-emoji-fonts`, Arch
  `noto-fonts-emoji`); macOS has Apple Color Emoji; Windows has Segoe UI Emoji
- a symbol font for `● ○ ◆ ★ ⏎ ↪ ⬆ ✕` and the box and block characters: DejaVu Sans (`fonts-dejavu-core`, Fedora
  `dejavu-sans-fonts`, Arch `ttf-dejavu`) or Noto Sans Symbols (`fonts-noto-core`, `google-noto-sans-symbols-fonts`,
  `noto-fonts`); macOS and Windows have them
- optional: CJK for Chinese, Japanese and Korean names and text (`fonts-noto-cjk`, Fedora `google-noto-sans-cjk-fonts`,
  Arch `noto-fonts-cjk`); macOS and Windows have them
- emoji drawn as boxes: install the font above, or F6 → Messages & media → "Show emoji as :names:" shows `:smile:`
  instead; rows that look misaligned mean the terminal measures an emoji's width differently from ttyak (the same
  setting is the way out)

## Linux

- needs: ALSA (`libasound.so.2`), glibc 2.34
- works best with: a notification server for the bubbles (swaync, mako, dunst, GNOME or KDE's own) — without one there
  are none, and ttyak's bell and title badge still work
  ([Notifications and attention](Notifications-and-attention)); `xdg-open` (xdg-utils) to open web and mail links; a
  Wayland compositor with the data-control protocol (or X11) for the clipboard, else ttyak copies through the terminal
  (OSC 52); a browser for the [browser sign-in](Signing-in#in-your-browser); `ffmpeg` / `ffprobe` for video, cameras
  and screen sharing, and GStreamer (with the PipeWire plugin) for screen sharing on Wayland
  ([Huddles](Huddles#video-cameras-and-screen-shares))

## macOS

- needs: macOS 11 or newer
- notifications: macOS shows them as the host terminal's (Terminal's in Terminal.app, tested on macOS 26), and only if
  that app's notifications are allowed in System Settings → Notifications; while the screen is shared or mirrored macOS
  keeps them in the Notification Center without a banner, unless "Allow notifications when mirroring or sharing the
  display" is on. A click on one brings the tab forward in Terminal.app (tried); in kitty ttyak uses kitty's own
  notifications instead (not tried on a Mac). More on [Notifications and attention](Notifications-and-attention#macos)
- optional: `ffmpeg` / `ffprobe` for video, cameras and screen sharing

## Windows

- needs: Windows 10 or 11, 64-bit
- Windows Terminal is recommended (it draws sixel pictures); the classic console (conhost) draws emoji in monochrome and
  no pictures, only half blocks (tried)
- `ctl check`: run it in the terminal on the desktop — over ssh (no desktop session) its Focus Assist and notification
  rows read wrongly. Windows Terminal 1.24 and conhost answer its questions about mouse, focus and bracketed paste
  (tried); OSC 8 links can't be asked about ("not known" everywhere) — Windows Terminal draws them. Windows Terminal
  has true colour but sets no `COLORTERM`: `ctl check` takes its `WT_SESSION` as the sign and says yes (not tried yet;
  not inside tmux or screen, which inherit it); conhost's true colour stays "not known"
- toasts appear under PowerShell's name; do-not-disturb (Focus Assist) is read through an undocumented state, so ttyak
  says when it can't tell; with Follow the system's do-not-disturb on, the toast still goes out under Windows'
  do-not-disturb, without its sound, for Windows to file in the notification center (tried under the automatic
  do-not-disturb for a full-screen app; with Do not disturb switched on by hand not tried yet). More on
  [Notifications and attention](Notifications-and-attention#windows)
- optional: `ffmpeg` / `ffprobe` for video, cameras and screen sharing

## If it doesn't work

- [Pictures show as half blocks](Troubleshooting#pictures-show-as-half-blocks)
- [Emoji or symbols show as boxes](Troubleshooting#emoji-or-symbols-show-as-boxes)
- [Colours look wrong](Troubleshooting#colours-look-wrong)
- [The thread doesn't open beside the conversation](Troubleshooting#the-thread-doesnt-open-beside-the-conversation)
- [Links don't open in the browser](Troubleshooting#links-dont-open-in-the-browser)
- [Copy or paste doesn't work](Troubleshooting#copy-or-paste-doesnt-work)
- [No desktop notifications](Troubleshooting#no-desktop-notifications)
- everything else: [Troubleshooting](Troubleshooting)
