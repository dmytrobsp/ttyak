# Dev tools

Helpers used while developing and testing; none of them is needed to build or run ttyak. The
live-test helpers assume Linux with Hyprland, kitty and PipeWire.

- `launch.sh PROFILE [DM_NAME]` — start the debug build on PROFILE, a test profile from `AGENTS.local.md` (it refuses
  to start without one), in the `ttyak-test` kitty window (abstract socket `unix:@ttyak-test`), and with DM_NAME open
  that DM. The window is **visible**: only while the maintainer watches (the hidden recipe is in
  `docs/agents/live-testing.md`).
  Drive it with `kitty @ --to unix:@ttyak-test send-key …/send-text …`; quit with `send-key ctrl+q`.
- `streamrec.py OUT_DIR TOTAL_S [PER_STREAM_S]` — record every new ttyak playback stream (never a source) with
  pw-record while it runs; `tonecheck.py HZ[,HZ…] FILE.wav…` — peak level per frequency (to tell sounds apart).
- `shot.sh OUT_DIR NAME` — screenshot that window (then Read the PNG).
- `jsgrep.py PATTERN CONTEXT MAX FILE…` — regex matches with context in minified JS (ugrep chokes on wide contexts).
- `apiargs.py METHOD DIR…` — find a Slack Web API method's webpack module and print its call sites/args.
- `wiki/check_links.py [--wiki DIR] [--readme FILE] [EXTRA…]` — check the wiki pages', README's and site's links (page names, `#anchors`) and the relative file links of other Markdown files (by default the agent docs, `SECURITY.md`, `docs/*.md`, `docs/distribution/`, `docs/protocol/`, the tools' READMEs); `wiki/publish.sh [--ref REF] [--push]` — check, then mirror `docs/wiki/` at REF (default: the latest `v*` tag) into the GitHub wiki repo; nothing is pushed without `--push`.
- `accounts/accounts.py` — the ledger of the Slack test accounts (git-ignored `.accounts/`) and their claims: `list`,
  `status`, `claim ACCOUNT --purpose P`, `renew`, `release ACCOUNT ID`, `import`, and `install ACCOUNT ID --home FAKE`
  (puts the account's session into a fake home under your live claim; `release` removes it; desktop only, and
  `install` writes the Linux layout `.config/ttyak/<profile>`). Claim an account before
  any live check: `docs/agents/live-testing.md`.
- `vm/` — PowerShell helpers that drive the Windows test machine's desktop from ssh (a program in the desktop
  session, keys and clicks, captures): see `vm/README.md`.

Web client bundles: `ttyak ctl get https://app.slack.com/client/<TEAM> client.html`, then chunk names/hashes from the
`__cf` map in it → `https://a.slack-edge.com/bv1-13/gantry-v2-async-<name>.<hash>.min.js`; entry: `client-boot.<hash>.min.js`
(see the end of client.html). The CDN serves gzip: decompress before searching. Details: `../docs/protocol/huddles.md`.

- `cdp.mjs PORT cookie|goto|eval|click|shot ARG` — drive a test Chrome over the DevTools protocol (node 22 or newer, no
  packages). Real Slack web client as B with a fake camera/mic (never real devices):
  `flatpak run com.google.Chrome --user-data-dir=$HOME/.var/app/com.google.Chrome/ttyak-fakecam
  --remote-debugging-port=9333 --use-fake-device-for-media-stream --use-fake-ui-for-media-stream --no-first-run about:blank`,
  then `cookie "$FAKE/.config/ttyak/<B>/session.json"` (sets `d`, never prints it; claim B and `accounts.py install` it
  into a fake home first, release after), `goto https://app.slack.com/client/<TEAM>/<CHANNEL>`,
  `click '[data-qa=huddle_channel_header_button]'`, in the preview window `click '[data-qa=huddle_camera_huddle_toolbar]'` and
  `click '[data-qa=huddle_invite_window_with_preview_content_join_button]'`; leave with
  `click '[data-qa=huddle_toolbar__leave_button]'`. Close Chrome (CDP `Browser.close`) and delete the profile folder afterwards.
  NEVER click the screen-share button there: the fake UI flag auto-picks the real screen.
