A link to a channel, a message, a thread reply, a person or a file in one of your profile's workspaces opens in ttyak,
not in the browser.

## How such a link looks

It is shown as `↪ <label>` in a colour of its own (`app_link` in the theme: see [Themes](Themes#your-own-theme)). The
label is the link's own text when its author gave it one, else where it goes: `#deploys`,
`#deploys › message from <author>` (once ttyak has looked the author up; `› message` until then), `› thread reply`,
`<name>'s profile`, `file <name>`. Where it goes into another workspace of the profile, the label starts with that
workspace's name (`<workspace> · #deploys`).

Hovering a link names where it goes in the status line ("↪ opens … in ttyak"); for any other link it shows the
address.

## Opening it

- A plain click opens it in ttyak. A link into another workspace of the profile switches to it first. A link to
  something ttyak cannot find says so in the status bar.
- A link to a file opens the file's viewer when a loaded message carries it and ttyak has one (pictures, videos,
  snippets: [Images and video](Images-and-video)); otherwise the status line says ttyak can't show that file here.
- `␣o` on the message lists what it holds: "In ttyak" first, then "In the browser", then "Files".
- Ctrl+Shift+click (kitty) still opens the link in the browser: ttyak marks links for the terminal (OSC 8), and
  terminals that know them open them on their own click.

Links to other places open in the browser as before, with the system's opener (`xdg-open`, `open`, Windows' URL
handler) or the program `TTYAK_OPEN_WITH` names; only web and mail links are opened.

## If it doesn't work

- [Links don't open in the browser](Troubleshooting#links-dont-open-in-the-browser)
- everything else: [Troubleshooting](Troubleshooting)
