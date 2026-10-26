# Screenshots

The pictures the README and the wiki show. Everything in them is made up: the people, the workspace and the
conversation.

| File | What it shows |
| --- | --- |
| `main.png` | The main view: the sidebar, a channel with a short conversation (a code block, reactions, a link to another channel's message) and a thread open beside it. |
| `huddle.png` | A huddle: the other person shares a screen (a synthetic test pattern), the two tiles with cameras off, the call bar. |
| `signin.png` | The sign-in guide, the code way: a QR code for the phone, the link, and the `___-___` prompt, for `demo@example.com`. |

## How they were made

- **Where.** In the test workspace, by its two test accounts, A and B. The conversation is the made-up one posted
  for the project site (`tools/site/CAPTURES.md`, `#prod-deploys`). The huddle was held in `#coffee`: B ran
  `ttyak ctl huddle-media coffee 150 --share-test` (a synthetic shared screen, no camera, no microphone); A joined
  muted with `␣hh` and opened the huddle view with `␣hv`, then left.
- **Capture.** ttyak (debug build) ran as A in a kitty window on a hidden workspace, 136 × 34 cells (wide enough
  for a thread beside the conversation), with a separate config folder holding only the test accounts' sign-ins,
  sounds and desktop notifications off, huddle volume 0, a microphone name that matches no device, and
  `TTYAK_AVATARS=0` (initials instead of profile pictures). Each screen was read as text with
  `kitty @ get-text --extent screen --ansi` — never a screenshot of the desktop.
- **Drawing.** `python3 tools/site/shots.py RAW_DIR docs/images --blur <workspace name> --fixes <file>` draws each
  screen on its cell grid (`tools/site/ansi2html.py`, as for the site), replaces every real name with a made-up one
  of the same width (a private list kept outside the repository), puts made-up initials into the huddle tiles,
  blurs the workspace name wherever it shows (it is made up as well, and blurred on top), draws the test pattern
  over the shared screen's cells, and renders the page with headless Chrome at 2× into a PNG. The sign-in guide is
  the site's own capture (`site/captures/signin.html`, made from ttyak's layout function with a made-up address).
- **Checks.** `python3 tools/site/check_private.py docs/images`; the drawn text was checked against the private list
  of real names, handles, emails, ids and the workspace's name (no match, and no 5-character piece of one other
  than generic words such as "channel" and "slack.com"); and every picture was looked at.
- **After each rebuild**, look at every picture again, and at a 2× crop of each blur (the workspace name must stay
  unreadable). `shots.py` names a capture it could not find ("skipped …") and then exits 1.

Font: [Space Mono](https://fonts.google.com/specimen/Space+Mono) (SIL Open Font License, `site/fonts/OFL-space-mono.txt`).
