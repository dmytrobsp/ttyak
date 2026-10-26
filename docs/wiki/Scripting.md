`ttyak ctl …` runs one thing against Slack (or against this machine) and exits: for automation, tests and status
bars. Like every command it takes `--profile NAME` (and `--team NAME` when the profile has several workspaces);
`ttyak ctl <command> --help` shows each command's own options.

## The commands

A `<channel>` is a channel's id, its name (`general` or `#general`), or `@someone` for your DM with them. Messages are
named by their `ts`.

| Command | |
|---|---|
| `whoami` | who this profile is signed in as |
| `channels` | the conversations (`-` marks public channels you are not in) |
| `join <channel>`, `leave <channel>`, `create <name>` | join or leave a channel; create one (lowercase, spaces become hyphens) |
| `history <channel> [-n N]` | recent messages (10 by default) |
| `replies <channel> <ts>` | a thread, parent first |
| `search <query>` | matching messages, as raw JSON fields |
| `post <channel> <text> [--thread TS]` | post a message (mrkdwn) |
| `edit <channel> <ts> <text>`, `delete <channel> <ts>` | edit or delete a message |
| `react <channel> <ts> <emoji>`, `pin <channel> <ts> [--remove]` | add a reaction; pin or unpin |
| `upload <channel> <path>` | upload a file (`--comment`, `--thread TS`, `--snippet LANG` for a text snippet) |
| `typing <channel> [seconds]` | the typing indicator (in a thread with `--thread`) |
| `listen [seconds]` | live events as they come (`--types`: only each event's type) |
| `status` | what the running ttyak says, for bars (below) |
| `check` | what this terminal and system offer, and what is missing (see [Requirements](Requirements)) |
| `huddles` | the huddles happening now, `--recent` the latest ones you were in or missed, with ids shown as `C1`, `U1`, … |
| `huddle-media <channel> [seconds]` | join a huddle without the TUI, with a test tone, a test camera or screen, a real camera, and recordings (see [Huddles](Huddles#tests)) |
| `audio-devices`, `cameras` | the microphones and speakers, the cameras (names for `TTYAK_AUDIO_INPUT` / `TTYAK_AUDIO_OUTPUT` and F6) |
| `keys-template` | a complete `keys.toml` with every default (see [Rebinding keys](Rebinding-keys)) |
| `theme-template [base]`, `theme-import <path>` | a complete theme file; check and install one (see [Themes](Themes)) |
| `locale-template` | a locale file with every translatable string (see [Translations](Translations)) |

And a few research probes that redact secrets: `api <method> [key=value…]` (any Web API method), `get <url> <out>`
(a slack.com address downloaded with the session), `huddle-join`, `huddle-signal`, `huddle-leave` (a huddle's steps
one by one) and `huddles --shape` (only the shape of Slack's answers).

`status`, `check`, `theme-import` and the templates need no Slack session; the others use the profile's.

## Status for bars and scripts

`ttyak --profile NAME ctl status` prints what the running ttyak of that profile says, from a small file it keeps up to
date (`$XDG_RUNTIME_DIR/ttyak/NAME.json`; without `XDG_RUNTIME_DIR`, as on macOS, and always on Windows,
`status.json` in the profile's cache folder). Only you can read it (from 0.12.2 on, on Windows too). It needs no Slack
session and is cheap to poll every second.

The line looks like `◆ acme: 2 for you · paused until 14:00`; the mark is `○` all read, `●` unread, `◆` for you, the
same as the tab title. A huddle you are in and an update waiting are added (`· in a huddle (#channel)`,
`· update 0.13.0`). When no ttyak runs the profile it says `ttyak is not running` and exits 0.

- `--json` prints the whole object: `version` (of this format, 1), `running`, `pid`, `profile`, `state` (`read`,
  `unread` or `for_you`), `mark`, `for_you` (the count, in every workspace of the profile), `workspaces` (each with
  `name`, `current`, `unread`, `for_you`), `paused_until`, `huddle` (`{"where": …}`), `update` (the version waiting),
  `updated_at`; times in RFC 3339. Not running: `"running": false`, with only `version` and `profile`
  filled in.
- `--follow [--every SECS]` prints a new line only when it changes, for bars that read a stream (it looks every second
  by default, and stops when the pipe closes). It keeps running when ttyak isn't: it prints the `not running` line on
  the change and waits for the next.

`huddle.where` names no person: `a direct message`, `a group DM` or `#channel`. The file holds no message text and no
authors, and ttyak removes it when it quits.

```sh
ttyak ctl status --json | jq -r '"\(.mark) \(.for_you)"'   # a plain bar line: ◆ 2
ttyak ctl status --json | jq -r .mark                      # the mark only
```

(With no ttyak running the mark is empty, so the first prints ` 0`.)

## If it doesn't work

- [A workspace is signed out](Troubleshooting#a-workspace-is-signed-out)
- [Reporting a bug](Troubleshooting#reporting-a-bug)
- everything else: [Troubleshooting](Troubleshooting)
