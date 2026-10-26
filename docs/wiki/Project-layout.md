A short map of the [repository](https://github.com/dmytrobsp/ttyak), for finding your way around the code.

| Where | What |
|---|---|
| `crates/slack-core` | sessions and signing in, the Web API client, real-time events (websocket), profiles, Do Not Disturb, message formatting |
| `crates/huddle` | huddles: Chime signaling (protobuf), WebRTC audio and video, Opus, H.264, audio devices, camera and screen capture |
| `crates/ttyak` | the program: the terminal UI (`src/tui/`), the `ctl` commands, `auth`, `profile`, `purge`, `uninstall`, the update check and self-update, `ctl check`, the status file |
| `vendor/rtc-turn` | webrtc-rs' TURN client 0.21.1 with a fix for "438 Stale Nonce" answers, which dropped a huddle's audio about 10 minutes in (why, what changed, when to drop it: `vendor/rtc-turn/TTYAK-PATCH.md`) |
| `vendor/crossterm` | crossterm 0.29.0 with the mouse's side buttons (back, forward) and kitty's notification replies decoded (`vendor/crossterm/TTYAK-PATCH.md`) |
| `tools/` | development and test helpers (`tools/README.md`), the site's and the wiki's checks, the install scripts' tests |
| `packaging/` | the package recipes (Homebrew, Scoop, winget, Chocolatey, `.deb`/`.rpm`, Arch) and the site's `latest.json`, rendered from a release's `SHA256SUMS` |
| `site/` | the site, ttyak.sh, with the install scripts `install.sh` and `install.ps1` |
| `docs/protocol/` | how the undocumented Slack endpoints ttyak uses were worked out (sign-in, huddles, notifications, …) |
| `docs/wiki/` | the sources of this wiki |
| `docs/RELEASING.md`, `docs/distribution/` | how a release is made, and the accounts it needs |
| `.github/workflows/` | CI, the release, the site's deployment, the install check |

At the top: `DESIGN.md` (the architecture and portability rules), `PROGRESS.md` (the development log),
`CONTRIBUTING.md` (see [Contributing](Contributing)) and the licences: ttyak is MIT or Apache-2.0, at your option;
`vendor/rtc-turn` is webrtc-rs' code (MIT or Apache-2.0, its licence files are in that folder) and `vendor/crossterm`
crossterm's (MIT).
