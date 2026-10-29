# Setting up a new machine

What a fresh Linux machine needs before an agent can build, test and run live checks on ttyak: the build tools, the
test tools, GitHub, the test workspace and its two accounts, the local file, the private lists, and the test
machines. The rules for using all this are in [AGENTS.md](../../AGENTS.md), [workflow.md](workflow.md) and
[live-testing.md](live-testing.md); this page only sets things up.

Most steps need the maintainer: signing in to GitHub and to Slack, Slack accounts, the Windows and macOS machines.
An agent prepares the exact commands and asks; it never signs anything in or out on its own.

Each section ends with how it was checked. "Here" means the maintainer's Linux machine (an Ubuntu 24.04-based
desktop, x86_64), which was set up long before this page was written: the commands were run there on 2026-10-26, but
**no step of this page has been tried on a fresh machine**.

## 1. Build tools

- **Rust**: rustup's stable toolchain (edition 2024), from [rustup.rs](https://rustup.rs). No `rust-toolchain` file
  pins a version; CI uses the latest stable.
- **Packages**: a C/C++ compiler, `pkg-config`, `cmake` (the bundled Opus codec) and the ALSA headers. Per
  distribution they are on the wiki's [Building from source](../wiki/Building-from-source.md); on Debian or Ubuntu:
  `sudo apt install build-essential pkg-config cmake libasound2-dev git curl`.
- **Windows cross-build** (Windows must compile at every Rust commit):
  `rustup target add x86_64-pc-windows-gnu` and MinGW-w64 — `sudo apt install mingw-w64`, or only its x86_64 part as
  CI does: `gcc-mingw-w64-x86-64 g++-mingw-w64-x86-64`. `.cargo/config.toml` links the C runtime statically, so the
  `.exe` needs no MinGW DLLs.

```sh
rustup show active-toolchain           # stable-x86_64-unknown-linux-gnu (default)
rustup target list --installed         # must list x86_64-pc-windows-gnu
x86_64-w64-mingw32-gcc --version       # the cross compiler
cargo build                            # target/debug/ttyak; never build target/release (AGENTS.md)
cargo test                             # all crates
cargo build --target x86_64-pc-windows-gnu
cargo test --no-run --target x86_64-pc-windows-gnu
```

Disk: `target/` grows to tens of GB, the Windows cross-build directory alone past 10 GB.

*Checked here*: rustup 1.29.1, rustc 1.98.1 stable; targets include `x86_64-pc-windows-gnu`; MinGW gcc 13 (`mingw-w64`
and its x86_64 packages installed); `cargo build` and the Windows `cargo build` succeeded; `cargo test` passed (the
whole suite). The package list for a fresh Debian, Ubuntu, Fedora and Arch is checked by
`tools/container-build.sh` (below); a fresh install of rustup plus MinGW on a new desktop is NOT tried.

## 2. Test tools

| Tool | What for | Check |
|---|---|---|
| Python 3 (3.12 here; standard library only) | the tool, packaging and site tests ([Tests](workflow.md#tests)) | `python3 --version` |
| node 22 or newer (no packages) | the site's tests (`node --test "tools/site/*.test.mjs"`; Pages uses 22), `tools/cdp.mjs` (it needs node's built-in `fetch` and `WebSocket`, both there from 22) | `node --version` |
| podman | `tools/container-build.sh`, and container steps inside some tests (below) | `podman --version`, `podman images` |
| dash, bash; fish, wget, shasum | `tools/install/test_install_sh.py` runs `site/install.sh` under dash and bash (each one present); fish, wget and shasum each enable a particular test (the fish PATH line, the wget download, the shasum check) | `command -v dash bash fish wget shasum` |
| `ssh` to a Windows machine | `tools/install/test_install_ps1.py` (not in CI; skipped when the machine can't be reached) | see [A Windows machine](#a-windows-machine) |

**The tool tests** are exactly the `Tool tests` step of `.github/workflows/ci.yml`; at the time of writing:

```sh
python3 -m unittest tools/test_check_name.py tools/test_launch.py tools/test_move_to_ttyak.py \
  tools/install/test_install_sh.py tools/signing/test_apple_csr.py tools/publish/test_aur_key.py \
  tools/wiki/test_check_links.py tools/wiki/test_publish.py tools/wiki/test_readme.py \
  tools/docs/test_split_history.py tools/vm/test_vm_scripts.py tools/accounts/test_accounts.py \
  tools/milestone/test_milestone.py
```

When the two differ, `ci.yml` is right. The packaging tests run apart:
`python3 -m unittest discover -s packaging -p 'test_*.py'`; the site's as in [Tests](workflow.md#tests).

The container images the tests use (pulled on first use; `podman pull <image>` ahead of time saves waiting):

| Image | Used by |
|---|---|
| `docker.io/library/debian:trixie`, `docker.io/library/ubuntu:24.04`, `registry.fedoraproject.org/fedora:latest`, `docker.io/library/archlinux:latest` | `tools/container-build.sh debian\|ubuntu\|fedora\|arch` (it also makes the volume `ttyak-cargo-registry`, a crate cache) |
| `docker.io/library/archlinux:latest` | `packaging/test_render.py` (the AUR `.SRCINFO` against `makepkg`) |
| `docker.io/koalaman/shellcheck:stable` | `tools/install/test_install_sh.py` (shellcheck on `site/install.sh`) |
| `mcr.microsoft.com/powershell:7.5-ubuntu-24.04` | `tools/install/test_install_ps1.py` (PSScriptAnalyzer), `tools/vm/test_vm_scripts.py` (when there is no `pwsh`) |

In CI (the variable `CI` set) the podman steps of `packaging/` and `tools/vm/` run only with `TTYAK_PODMAN_TESTS=1`.

For live checks on this machine the helpers in [live-testing.md](live-testing.md) assume Hyprland (`hyprctl`), kitty
with remote control, PipeWire (`pw-record`, `pactl`), `wl-clipboard`, D-Bus tools (`dbus-monitor`, `gdbus`) and
ffmpeg. Another desktop: [Other desktops](live-testing.md#other-desktops) (a headless compositor or Xvfb; not
tried end to end).

*Checked here*: Python 3.12.3, node v24.22.0, podman 4.9.3. CI's `Tool tests` step, the packaging tests (the
`makepkg` one in podman) and the site's tests (Python and node) passed; the PowerShell image was pulled
and ran the `tools/vm` parse check. `test_install_ps1.py` was not run for this page (it uses the Windows machine).
`container-build.sh` was not re-run for this page.

## 3. GitHub

The repository is `dmytrobsp/ttyak`. The maintainer signs `gh` in (an agent never does):

```sh
gh auth login                      # the maintainer, in their browser; scopes repo and workflow at least
gh auth status                     # the active account must be the maintainer's: the owner of dmytrobsp/ttyak
```

`workflow` is needed to push changes under `.github/workflows/`. A machine may have several `gh` accounts (one of
them someone's work account): the active one must be the maintainer's.

Clone, then give the clone the repository's own identity and credentials — never the machine's global ones:

```sh
git clone https://<owner>@github.com/<owner>/ttyak.git          # <owner>: as in dmytrobsp/ttyak; picks gh's account
cd ttyak
git log -1 --format='%an <%ae>'                                  # a commit of the maintainer's: copy name and email
git config user.name '<name from that commit>'
git config user.email '<email from that commit>'
git config credential.https://github.com.helper ''               # drop helpers inherited from the global config
git config --add credential.https://github.com.helper '!gh auth git-credential'
git ls-remote origin HEAD                                        # works without a prompt
```

Every other clone or new repository (the wiki, the tap, the bucket, a scratch clone) gets the same before its first
commit ([workflow.md](workflow.md#commits-and-pushes)).

*Checked here*: `gh auth status` lists the maintainer's account as active (scopes include `repo` and `workflow`); the
clone has the repo-local identity, the empty-then-`gh` credential helper and the user name in the origin URL;
`git ls-remote origin HEAD` answered. A fresh `gh auth login` and clone are NOT tried.

## 4. The local file

`AGENTS.local.md` at the repository root holds what is specific to this machine and maintainer. It is git-ignored;
without it an agent does no live work.

```sh
cp docs/agents/local-template.md AGENTS.local.md   # then keep only the fenced block, and fill it in
chmod 600 AGENTS.local.md
git check-ignore -v AGENTS.local.md                # .gitignore:…:/AGENTS.local.md
```

The template ([local-template.md](local-template.md)) says what each section is for. Where the values come from:

- **Test profiles**: the profile names chosen when signing A and B in ([section 5](#5-the-test-workspace)); the
  machine's default profile and any other profiles: `ttyak profile list` (local files only, nothing is sent). Ask the
  maintainer which profiles are theirs and what may be done with them; if they run their own ttyak, from where.
- **Test workspace**: its name and id, A's and B's user ids (`ctl api auth.test` on A's profile prints `team_id`
  and `user_id`), standing state such as a do-not-disturb schedule (`ctl api dnd.info`), both run in a fake home
  after `claim` and `install`, as in the check at the end of section 5), and the permissions the
  maintainer gave.
- **Test machines**: the ssh aliases (`ssh -G <alias>` shows host and user), the libvirt domain of a VM
  (`virsh -c qemu:///system list --all`), the work folder, what each machine has (sound, camera, ffmpeg) and who else
  uses it.
- **Private lists**: their paths ([section 6](#6-the-private-lists)).
- **Anything else**: the desktop, the audio devices, the global git identity (whose it is), signing material (where,
  never its contents), packages installed for the project, hooks agents will meet.

Never copy its values into a tracked file, an issue, a commit message or a report.

*Checked here*: the file exists, is 0600 and is ignored (`.gitignore`). Filling it in on another machine is NOT tried.

## 5. The test workspace

Live checks act on **a Slack workspace with two test accounts, A and B**, made only for developing ttyak, never one
where real people work. The current one is on Slack's **free** plan; Slack reports `team.plan = "std"` for it, so
ttyak's paid-plan check — like Slack's own client — treats paid features (VIPs, for one) as available there.

**If there is none**, the maintainer makes one (an agent can't: it needs email addresses and a captcha): a free
workspace at slack.com with account A, then B invited by email (two addresses the maintainer reads), a couple of
channels for tests, and A and B in a DM. Workspaces that require SSO or two-factor sign-in can't use the email code.

**Signing A and B in on the new machine.** Sessions are per machine and per profile; the profile names go into the
local file. Two ways:

1. **The email code** ([Signing in](../wiki/Signing-in.md#with-a-code-from-your-email)), one account at a time. The
   maintainer runs it in a terminal of their own, so the code reaches only them (if an agent opens the window for
   them, it reads that screen only masked: [live-testing.md](live-testing.md#checks-with-the-maintainer)):

   ```sh
   REPO=<the clone>      # its absolute path
   FAKE=<scratch>/signin-home    # a fake home: this sign-in must not write the maintainer's real folders
   mkdir -p "$FAKE"/.config "$FAKE"/.local/share "$FAKE"/.cache && chmod 700 "$FAKE" "$FAKE"/.config
   cargo build --manifest-path "$REPO/Cargo.toml"
   HOME="$FAKE" XDG_CONFIG_HOME="$FAKE/.config" XDG_DATA_HOME="$FAKE/.local/share" XDG_CACHE_HOME="$FAKE/.cache" \
     "$REPO/target/debug/ttyak" --profile <A> auth login --email <A's address>
   ```
   ttyak shows a QR code and a link; on the phone (or any browser) Slack asks for a captcha and emails a code; the
   maintainer types it at ttyak's `Code: ___-___` prompt — never on the phone. ttyak lists the workspaces of that
   address: mark **only the test workspace**. Then the same for B with `--profile <B>`, in the same fake home.
   Every sign-in adds one more session on Slack's side; that is harmless, and the test accounts are never signed out.

   The session is made in this fake home, then imported into the ledger (below, after the ledger is made as in way 2)
   with `import A --from "$FAKE/.config/ttyak/<profile>/session.json"`; delete the fake home after. This is the one
   place a session is created; nothing writes a test profile in the real config folder.
2. **Copying the sessions** from a machine where A and B are signed in: each profile's `session.json` (Linux:
   `~/.config/ttyak/<profile>/session.json`; other systems: [Profiles and sessions](../wiki/Profiles-and-sessions.md))
   into the same place here, mode 0600, piped through ssh and never printed — it holds the tokens. Then
   put them into the ledger here; live checks take sessions from the ledger with `install`. The ledger is made once,
   by hand: create `.accounts/` and `.accounts/sessions/` (both mode 0700) and write `.accounts/ledger.json` as
   `{"accounts": {"A": {"profile": "<profile>", "role": "<role>", "rules": ["<rule>", …],
   "session": "sessions/A.json"}, "B": {…}}}` (exactly those four keys per account; the profile, role and rules come
   from the template's account section). Then, for A and B:

   ```sh
   python3 "$REPO/tools/accounts/accounts.py" import A --from <config folder>/ttyak/<profile>/session.json
   ```

   (`$REPO` is as above; `<config folder>` is where the sessions were copied to, as above; `--replace` renews an
   existing master.) The tool writes the master as `.accounts/sessions/A.json` (0600) and never prints it.

Then check, without posting anything: claim A and B and `install` them into a fake home
([A fake home](live-testing.md#a-fake-home)), and run ttyak with the four variables set on its command only:

```sh
ttyak_fake() { HOME="$FAKE" XDG_CONFIG_HOME="$FAKE/.config" XDG_DATA_HOME="$FAKE/.local/share" \
  XDG_CACHE_HOME="$FAKE/.cache" "$REPO/target/debug/ttyak" "$@"; }
ttyak_fake --profile <A> auth status      # the test workspace, "works"
ttyak_fake --profile <B> auth status
ttyak_fake --profile <A> ctl api dnd.info # A's do-not-disturb state, for the local file
```

Release the claims after.

*Checked*: the email-code sign-in was verified LIVE on account B (2026-10-20) and is the wiki's documented way; the
`team.plan = "std"` reading is from the live workspace. For this page nothing was signed in, out or checked against
Slack: both ways on a new machine are NOT tried here. Copying sessions to the test machines is how their checks ran
(LIVE); they now go from the ledger with `install`.

## 6. The private lists

Files outside the repository (in `~/.config/ttyak-site/`, mode 0600, never printed, never committed) that let the
tools keep private strings out of public files. Their paths go into the local file.

| File | Format | Used by |
|---|---|---|
| `forbidden.txt` | one string per line; blank lines and `#` comments ignored | `tools/site/check_private.py <dir>` (the privacy check before any public page or doc; `TTYAK_SITE_FORBIDDEN` points elsewhere) |
| `rename.json` | `{"real text": "made-up text", …}`, applied cell by cell to terminal captures | `tools/site/captures.py`, `tools/site/shots.py` (the site's screenshots) |
| `fixes.json` | `{"<capture>": {"row,col": "letter"}}`: single cells overwritten after renaming | the same, for captures that need it (`huddle`) |
| `redact.json` | `{"private string": "neutral words", …, "__keep__": ["regex", …]}` | `tools/docs/split_history.py --redact` (the history archive) |

**What goes into `forbidden.txt`**: everything that must never appear publicly — the names and domains of the
maintainer's own workspaces and of the test workspace, the test accounts' names, emails and Slack ids, the
maintainer's real name and work identity, host names and addresses of the test machines, other private profile names.
The check matches case-insensitively and also with spaces, dots, underscores and hyphens removed; the repository's own
public addresses (`github.com/<owner>/ttyak`, `<owner>.github.io`, `ttyak.sh`, the winget id) are allowed. It prints
the file, the line and the string's number in the list, never the string itself.

```sh
mkdir -p ~/.config/ttyak-site && chmod 700 ~/.config/ttyak-site
install -m 600 /dev/null ~/.config/ttyak-site/forbidden.txt     # then add the strings with an editor
python3 tools/site/check_private.py site                        # exit 0: the site is clean
```

`rename.json` and `fixes.json` are needed only to rebuild the site's captures, `redact.json` only to re-check the
history archive. Without `forbidden.txt` the privacy check can't run: say so in the record and ask the maintainer.

*Checked here*: the formats are read from the tools' code; the lists exist here, and `check_private.py site` and the
check on this page exited 0. Making them from scratch is NOT tried.

## 7. The test machines

What they are for and how they are used safely (shared machines, restoring them, ttyak's folders there) is in
[live-testing.md](live-testing.md#test-machines). Their aliases, users and folders go into the local file.
A new machine also gets the claim tool's clone and a read-only deploy key (details in the tool's README, found
through `AGENTS.local.md`).

### A Windows machine

Windows 11 — here a libvirt VM on the Linux machine (`virsh -c qemu:///system domstate|start|shutdown <domain>`) —
with:

- **OpenSSH Server** on, reached with a key, no password (`ssh -T -o BatchMode=yes <host> "ver" < /dev/null`). For an
  account in the Administrators group the key goes into `C:\ProgramData\ssh\administrators_authorized_keys`. The ssh
  session is elevated and runs cmd by default.
- **A test user who signs in automatically** (`tools/vm/autologon.ps1`, run once at the machine), so a desktop session
  exists after every start: notifications, windows and captures happen only there.
- **Windows PowerShell 5.1** (built in); PowerShell 7 (`pwsh`) as well for `test_install_ps1.py`'s second half.
  Optional: ffmpeg (through scoop) for camera and video checks. No sound card or camera is needed.
- A work folder, `%USERPROFILE%\ttyak-test`; each agent works in a subfolder of its own below it (`<unit>`, the unit's
  or task's name: [live-testing.md](live-testing.md#live-checks-in-a-milestone)).

`tools/install/test_install_ps1.py` reaches the machine as the ssh alias `windows` and serves its fixtures from
`192.168.122.1`, libvirt's default host address: name the alias so, or that test skips.

**How it is driven**: [tools/vm/](../../tools/vm/README.md) — a program started on the machine's desktop through a
one-off scheduled task (`-LogonType Interactive`), keys and clicks injected there (SendKeys, `mouse_event`,
`keybd_event`), captures of that desktop. Builds get there by cross-compiling here and copying the `.exe`:

```sh
cargo build --target x86_64-pc-windows-gnu
scp -q -o BatchMode=yes target/x86_64-pc-windows-gnu/debug/ttyak.exe '<host>:ttyak-test/<unit>/'
```

*Checked here*: `virsh` 10.0.0 is installed; the VM and the tools are described from the local notes and the tools'
code; nothing was run on the VM for this page (NOT). Setting up OpenSSH, the key and automatic sign-in on a new
Windows machine is NOT tried here (the current VM was set up before this page).

### A Mac

An Apple Silicon Mac on the local network with:

- **Remote Login** on (System Settings → General → Sharing), reached with a key
  (`ssh -o BatchMode=yes <host> true`).
- Xcode's command-line tools, Homebrew, `brew install cmake`, rustup's stable toolchain. A non-interactive ssh shell
  lacks `/opt/homebrew/bin`: `export PATH=/opt/homebrew/bin:$PATH` in every command.
- A work folder, `~/ttyak-test`, with a subfolder per unit; the source goes into `~/ttyak-test/<unit>/src` with
  `git archive HEAD | ssh <host> 'mkdir -p ~/ttyak-test/<unit>/src &&
  tar -x -C ~/ttyak-test/<unit>/src'`, then `cargo build` there.

**The GUI session.** An ssh session on macOS is not the logged-in user's GUI session: notifications, the Keychain and
`osascript` to Terminal behave differently there. To run something inside the GUI session, load a one-off launchd job
into the user's `gui/<uid>` domain and remove it after:

```sh
ssh <host> 'cat > /tmp/ttyak-probe.plist' <<'EOF'
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0"><dict>
  <key>Label</key><string>ttyak.probe</string>
  <key>ProgramArguments</key><array>
    <string>/bin/sh</string><string>-c</string><string>…the command…; echo "probe exit $?"</string>
  </array>
  <key>RunAtLoad</key><true/>
  <key>StandardOutPath</key><string>/tmp/ttyak-probe.out</string>
  <key>StandardErrorPath</key><string>/tmp/ttyak-probe.out</string>
</dict></plist>
EOF
ssh <host> 'launchctl bootstrap gui/$(id -u) /tmp/ttyak-probe.plist'
# wait for the job's last line, at most 30 s, then read its output
ssh <host> 'for i in $(seq 30); do grep -q "^probe exit" /tmp/ttyak-probe.out 2>/dev/null && break; sleep 1; done
  cat /tmp/ttyak-probe.out; launchctl bootout gui/$(id -u)/ttyak.probe'
ssh <host> 'rm -f /tmp/ttyak-probe.plist /tmp/ttyak-probe.out'
```

Prompts (the Keychain, notification permissions) and clicks there need the maintainer at the Mac; so do its real
speakers, microphone, camera and screen.

*Checked*: a `launchctl bootstrap gui/<uid>` job ran the probes of ttyak's macOS notification work LIVE (0.12.0,
[history](../history/0.12.md)); the plist above is the standard form, written for this page, NOT run. Setting up a new
Mac is NOT tried here.

## Done when

- `cargo test` and both Windows commands pass; the tool tests pass as in [Tests](workflow.md#tests).
- `gh auth status` shows the maintainer's account active; the clone has the repo-local identity and credential helper.
- `AGENTS.local.md` is filled in, 0600 and ignored; A and B answer `auth status` with "works" on their test profiles.
- `forbidden.txt` exists and `python3 tools/site/check_private.py site` exits 0.
- Each test machine answers a `BatchMode` ssh, and its row in the local file says how to reach it and who else uses it.
