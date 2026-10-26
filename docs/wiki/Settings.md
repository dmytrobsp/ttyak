ttyak's preferences are in F6 (or `/prefs`). Most are saved in `settings.toml`; your availability and what notifies
you are kept on Slack, as in Slack's own app. A few environment variables change things F6 does not.

## F6: the preferences

| Page | What it has | More on |
|---|---|---|
| Availability | your status, Appear away, Notifications (on, or paused until when: Enter pauses for 30 minutes or resumes, + / - change a pause's length) and the Notification schedule (off, every day, weekdays or custom, with the hours you receive notifications in; for custom, per day: hours, receive all day or paused all day) — kept on Slack | [Notifications and attention](Notifications-and-attention#do-not-disturb) |
| Appearance | Colour theme, Theme for this workspace, Compact, Hide conversations quiet for | [Themes](Themes), [The screen](The-screen#the-sidebar) |
| Messages & media | Show image thumbnails, Show profile pictures, Two-row profile pictures, Refresh previewed channels, Refresh people's statuses, Mark feed threads read, Draw pictures with, Show emoji as :names:, Custom emoji as pictures, Save files to | [Images and video](Images-and-video), [The screen](The-screen#statuses) |
| Language & region | 24-hour clock, Date format, Time zone, Language | [Translations](Translations) |
| Notifications | what notifies you (kept on Slack), how you are told (desktop notifications, sound and its volume, the bell, the mark in the tab title), quiet hours, what this machine does, a test notification | [Notifications and attention](Notifications-and-attention#settings-in-f6) |
| Audio & video | Microphone, Speaker, Huddle volume, Microphone gain, Speaking threshold, Noise suppression, Echo cancellation, Camera | [Huddles](Huddles#audio-f6--audio--video) |
| Huddles | the huddle sounds: joining, someone joining or leaving, the ring and how long it rings, declining when it rings out, the alone reminder and when it starts | [Huddles](Huddles#sounds-f6--huddles) |
| Updates | the version and how ttyak was installed, the check on or off, how often, its timeout, the download timeout, Check now, the command to update, Update | [Updates](Updates) |
| Keys | which-key, the leaders, every key binding (saved in `keys.toml`) | [Rebinding keys](Rebinding-keys) |

Each row has its own help beside it in F6.

## settings.toml

F6 saves what you change at once, in `settings.toml` next to `keys.toml`:

| System | File |
|---|---|
| Linux | `~/.config/ttyak/settings.toml` (or under `$XDG_CONFIG_HOME`) |
| macOS | `~/Library/Application Support/ttyak/settings.toml` |
| Windows | `%APPDATA%\ttyak\config\settings.toml` |

One file serves every profile. ttyak reads it at start and at each switch of workspace. You can edit it by hand: an
entry left out keeps its default. A file that does not parse is not used — ttyak runs with the defaults and says why
in the status line at start — and F6 writes the whole file again when you change something (comments are not kept).

## Environment variables

| Variable | |
|---|---|
| `TTYAK_LOG` | log filter, e.g. `ttyak=debug` (default: `ttyak=info,slack_core=info,huddle=info`). The log is `ttyak.log` in the profile's cache folder: `~/.cache/ttyak/<profile>/` on Linux, `~/Library/Caches/ttyak/<profile>/` on macOS, `%LOCALAPPDATA%\ttyak\cache\<profile>\` on Windows. The websocket library's handshake and frames (`tungstenite::handshake::client`, `tungstenite::client`, `tungstenite::protocol`), which would print the token, cookie and messages, stay off unless named |
| `TTYAK_AUDIO_INPUT`, `TTYAK_AUDIO_OUTPUT` | the microphone and speaker (part of a device id or name from `ttyak ctl audio-devices`), over F6 → Audio & video; a device named but missing is an error. See [Huddles](Huddles#audio-f6--audio--video) |
| `TTYAK_DOWNLOAD_DIR` | where `␣s` saves attachments first, over F6 → Messages & media → Save files to (default: your downloads folder) |
| `TTYAK_OPEN_WITH` | the program `␣o` opens links with, given the address (default: the system's opener — `xdg-open`, `open`, Windows' URL handler); only web and mail links are opened. See [Links into your Slack](Links-into-your-Slack#opening-it) |
| `TTYAK_GRAPHICS` | how pictures are drawn: `kitty`, `sixel`, `iterm2` or `halfblocks`, over F6 → Messages & media → Draw pictures with, for when the terminal is not asked right (over ssh or tmux). See [Images and video](Images-and-video#how-pictures-are-drawn) |
| `TTYAK_AVATARS=0` | initials instead of profile pictures next to names. See [Images and video](Images-and-video) |
| `TTYAK_NOTIFY_ROUTE=system` | for debugging: notifications through the system's notifier even in a terminal that shows them itself. See [Notifications through the terminal](Notifications-and-attention#notifications-through-the-terminal) |
| `TTYAK_COOKIE_D` | the `d` cookie for `ttyak auth login`, as `--cookie VALUE` gives it (but kept out of your shell history). See [Signing in](Signing-in#for-scripts) |

The install scripts read their own (`TTYAK_VERSION`, `TTYAK_DIR`, `TTYAK_NO_MODIFY_PATH`, `TTYAK_YES`,
`TTYAK_UNINSTALL`, `TTYAK_KEEP_DATA`, the same as their flags): see
[Install](Install#the-install-script). Debug builds read a few more for tests (`TTYAK_FAIL`, `TTYAK_DAYS_LATER`,
`TTYAK_HUDDLE_DROP`), described in
[CONTRIBUTING.md](https://github.com/dmytrobsp/ttyak/blob/master/CONTRIBUTING.md).

## If it doesn't work

- [Troubleshooting](Troubleshooting)
