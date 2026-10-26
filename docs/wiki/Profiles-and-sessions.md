A profile is a set of sign-ins: one or more workspaces, and in each one or more accounts. Plain `ttyak` uses the
default profile (`default` until you choose another); `--profile NAME` on any command picks another. How to sign in
is on [Signing in](Signing-in).

## Where sessions are kept

Each profile's sessions are saved in `session.json` in the profile's folder:

| System | Folder |
|---|---|
| Linux | `~/.config/ttyak/<profile>/` (or under `$XDG_CONFIG_HOME`) |
| macOS | `~/Library/Application Support/ttyak/<profile>/` |
| Windows | `%APPDATA%\ttyak\config\<profile>\` |

The file holds each workspace's token and `d` cookie; only you can read it (from 0.12.2 on, on Windows too). With each
session it keeps where the session came from (which browser profile, the address an email code went to, or a cookie —
names only, nothing secret): `auth status` shows it, and signing in again starts there. One profile may hold two
accounts of the same workspace.

## Checking and forgetting sign-ins

- `ttyak auth status` — each workspace of the profile: its name and address, who you are there, where the session
  came from and whether it still works (asked of Slack, 10 s at most: `works`, `signed out` or `can't check: …`);
  ★ marks the one plain `ttyak` opens (the workspace signed in to first, while it is there; `ttyak --team NAME` opens
  another).
- `ttyak auth logout` — in a terminal lists the profile's sign-ins (Space marks several, Enter forgets the marked
  ones, Esc changes nothing); `--team NAME` (a team id or name) or `--all` for scripts. It forgets the sign-in on this
  machine only; Slack is not signed out (a session taken from a browser stays that browser's). The profile stays,
  empty if it was the last.

## When a session ends

Slack can end a session. At every start ttyak first checks each workspace of the profile with Slack (3 s at most; one
that can't be checked starts as usual). In a terminal it asks about each signed-out one:

- **sign in again**, starting from where the session came from: the code way with your email filled in, or the
  browser profile it came from (then the browser way with the workspace's own link); otherwise the guide
- **remove it from the profile**
- **launch anyway** without it, while another workspace can start; when none can, **quit**

Without a terminal it prints a warning for each signed-out workspace and starts the others (and fails when none can
start). A session that ends while ttyak runs is dim with ⊘ in the workspace rail and shown in the
[status line](The-screen#the-status-line): `Acme: signed out — ttyak auth login, then restart ttyak`.

## Several workspaces in one profile

Signing in to more workspaces with the same profile adds them to it. Inside the app, `Alt+1` … `Alt+9` switch between
them and `/workspace [name]` lists them or switches by name ([Keys](Keys#everywhere)). The
[rail](The-screen#several-workspaces-the-rail) on the left (shown with two or more workspaces, in a terminal at least
60 columns wide) shows unread messages in the others (checked every 30 s), and their
[desktop notifications](Notifications-and-attention#desktop-notifications) still arrive, titled with the workspace;
clicking one switches there. To tell workspaces apart, give each a theme of its own (F6 → Appearance → Theme for this
workspace: [Themes](Themes#choosing-one)).

## Several profiles

Use `--profile NAME` to keep several accounts apart. A name has letters, digits, `-` and `_`, and `.` or `@` inside
it (`john.doe`, `me@work`; never first or last, never two dots together). ttyak's own file names next to the profiles
(`settings.toml`, `keys.toml`, `themes`, `locales`, `default-profile`, …) and Windows device names (`con`, `nul`, …)
can't be profile names. Signing in under a new name makes that profile.

- `ttyak profile list` — every profile, ★ on the default, with its workspaces (local files only); a folder with
  sign-ins whose name can't be a profile's is named (`skipped NAME: why`).
- `ttyak profile default NAME` — plain `ttyak` (and `auth`, `ctl`) use that profile from now on (kept in
  `default-profile` in ttyak's config folder: `~/.config/ttyak/` on Linux); `--profile NAME` always wins. Only a
  profile that exists can be the default.
- `ttyak profile remove NAME` — lists and, after a y/N question (or `--yes`), deletes the profile's sign-ins folder
  (above) and its cache folder (log, caches: `~/.cache/ttyak/NAME/` on Linux, `~/Library/Caches/ttyak/NAME/` on
  macOS, `%LOCALAPPDATA%\ttyak\cache\NAME\` on Windows); drafts and the sidebar layout stay. Refused while a ttyak runs
  on that profile. When it was the default, `default` is the default again.

Profiles separate accounts only: the preferences (`settings.toml`, everything in F6), `keys.toml`, `themes/` and
`locales/` are shared by all profiles. Each profile has its own sessions and its own log and cache folder. Drafts and
the sidebar layout are kept per workspace and account, in ttyak's data folder (`~/.local/share/ttyak/` on Linux).

## Deleting everything

`ttyak purge` deletes everything ttyak wrote, as if it was never launched: its config folder (every profile's
sign-ins, `settings.toml`, `keys.toml`, `locales/`, `themes/`), its cache folder (logs, caches), its data folder
(drafts, sidebar layout; on Windows also `%APPDATA%\ttyak\data`) and ttyak's own `ttyak-cookies-*` / `ttyak-<pid>-*`
files in the temporary folder (on Linux and macOS only yours). It lists them first and asks y/N (or `--yes`); it is
refused while a ttyak runs. A folder that is a symbolic link loses only the link (what it points to stays, and ttyak
says so). Files you saved from messages stay; Slack is not signed out.

`ttyak uninstall` removes the program as well: see [Install](Install#uninstall).

## If it doesn't work

- [A workspace is signed out](Troubleshooting#a-workspace-is-signed-out)
- [Signing in: no browser found, or a browser can't be read](Troubleshooting#signing-in-no-browser-found-or-a-browser-cant-be-read)
- [keys.toml or settings.toml is ignored](Troubleshooting#keystoml-or-settingstoml-is-ignored)
- everything else: [Troubleshooting](Troubleshooting)
