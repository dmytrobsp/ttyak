ttyak draws pictures in the terminal — image thumbnails in messages, profile pictures, custom emoji, workspace logos —
opens a picture full-window in its viewer, and plays videos with sound.

## Terminals that draw pictures

Real pictures need a terminal with graphics: one that speaks kitty's protocol, sixel or iTerm2's. Other terminals get
half blocks: pictures drawn with block characters. Which terminals were tried, and how each one answers, is on
[Requirements](Requirements#the-terminal).

`ttyak ctl check` says what your terminal answered: its "pictures" row lists the ways it can draw, or "half blocks
only". Run it in the terminal itself, not through a pipe.

## What is drawn

- **Image thumbnails** in the messages (F6 → Messages & media → Show image thumbnails) — as half blocks where the
  terminal draws no pictures.
- **Profile pictures** next to names, only where the terminal draws pictures; elsewhere initials (F6 → Messages &
  media → Show profile pictures; Two-row profile pictures makes them bigger, beside the name and the first line).
- **Custom emoji** (your workspace's own picture emoji) as pictures in messages and reactions, where the terminal draws
  pictures; else they show as `:name:` (F6 → Messages & media → Custom emoji as pictures).
- **Workspace logos** on the rail, with two or more workspaces; else their initials
  ([The screen](The-screen#several-workspaces-the-rail)).
- **The profile card's picture** (`␣u`), big on top; their initials while it loads and where the terminal draws no
  pictures.

Each attached file has a row under its message with an icon: 🖼️ a picture, 🎞️ a video, 🎵 audio, 📎 anything else.

## How pictures are drawn

F6 → Messages & media → Draw pictures with (`[messages] graphics` in `settings.toml`): `auto` (the default: asked from
the terminal at start), `kitty`, `sixel`, `iterm2` or `halfblocks`. It offers only the ways the terminal said at start
it can draw — `auto` and `halfblocks` always — and its help marks the others "not available in this terminal". A
`settings.toml` naming one of those draws as `auto`, with a note in the status bar (the file is left as it is).

Two environment variables (all of them are on [Settings](Settings#environment-variables)):

- `TTYAK_GRAPHICS` (`kitty`, `sixel`, `iterm2` or `halfblocks`) forces the way it names, for when the terminal is not
  asked right (over ssh or tmux) — with a warning in the log when the terminal did not report it.
- `TTYAK_AVATARS=0` shows initials instead of profile pictures next to names (as on terminals without real
  graphics).

## Opening a picture or a video

`␣o` on a message opens what it holds: a link, a picture, a video, a snippet. With several, a list to pick from ("In
ttyak", "In the browser", "Files"). A picture opens in the viewer, with the message's other pictures a step away; a
video plays in the player. A link to a file in your Slack opens the same way ([Links into your
Slack](Links-into-your-Slack)).

`␣s` saves a message's files to disk ([Keys](Keys#on-a-message)). In the composer, Ctrl+V attaches a picture from the
clipboard, such as a screenshot ([Keys](Keys#typing-insert)).

## The viewer

The picture fills the window, in its large version, fitted to the window. Its title has the file's name and size,
and "2/3" when the message has several pictures.

| Key | |
|---|---|
| ← / → · `h` / `l` | the previous / next picture of the message (it wraps round) |
| Esc · `q` | close |

## Video

Videos play in the terminal, with sound. That needs `ffmpeg` and `ffprobe` on your PATH, on every system (optional:
only video, cameras and screen sharing need them; `ctl check` has a row for them). Without them the status line says
"cannot play video: …". Cameras and screen shares in a huddle are on
[Huddles](Huddles#video-cameras-and-screen-shares).

| Key | |
|---|---|
| Space · `p` | pause / play |
| ← / → · `h` / `l` | jump 10 seconds back / forward |
| Esc · `q` | stop |

## If it doesn't work

- [Pictures show as half blocks](Troubleshooting#pictures-show-as-half-blocks)
- [Emoji or symbols show as boxes](Troubleshooting#emoji-or-symbols-show-as-boxes)
- [Video, the camera or screen sharing doesn't work](Troubleshooting#video-the-camera-or-screen-sharing-doesnt-work)
- [Requirements](Requirements)
- everything else: [Troubleshooting](Troubleshooting)
