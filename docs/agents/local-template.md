# AGENTS.local.md — template

Copy the block below to `AGENTS.local.md` at the repository root, `chmod 600 AGENTS.local.md`, and fill it in. That
file is git-ignored: it holds what is specific to one machine and one maintainer, and it is never committed, quoted in
a tracked file, or pasted into an issue. Agents read it after [AGENTS.md](../../AGENTS.md); without it they do no
live work.

Each section says what it is for, then gives example values (made up). Replace them; delete a section's lines that
don't apply and say so ("no Mac on this machine"). How to set things up is in [setup.md](setup.md).

```markdown
# AGENTS.local.md — <machine name>, filled in <date>

## Test profiles

What it is for: the only ttyak profiles an agent may use. Every ttyak command names one with `--profile`.

- Account A: profile `test-a` — the account that runs the TUI in most checks.
- Account B: profile `test-b` — plays the other person (`ctl post`, rings, test video).
- The machine's default profile: `work` — NOT a test profile. (Say whose it is and what may be done with it:
  "never use", or "read-only, unfocused, never post".)
- Other profiles on this machine that must never be touched: `personal`.
- The maintainer runs their own ttyak from: `~/src/ttyak/target/release/ttyak` (never kill or rebuild it).

## Test workspace

What it is for: knowing which workspace and which accounts the tests act on, and what state they are in.

- Workspace: `example-testbed` (team id `T0000000000`), on Slack's free plan (reports `team.plan = "std"`).
- A: display name `tester-a`, user id `U0000000001`. B: `tester-b`, `U0000000002`.
- Standing state that affects tests: B has a do-not-disturb snooze every night 23:00–07:00; restore it after any DND
  change.
- Channels the tests use: `#tests`, `#huddles`; the DM between A and B.
- Permissions the maintainer gave: "DND may be changed on A and B and restored"; "posts need not be deleted".

## Test machines

What it is for: reaching the Windows and macOS machines, and the rules for sharing them.

- Windows: VM `win-test` (libvirt: `virsh -c qemu:///system domstate win-test`), SSH alias `wintest`, user
  `tester`, key auth. Auto-logon: yes (console session 1). Shared with: another project's agent — check first.
  Work folder: `%USERPROFILE%\ttyak-test`. No sound card, no camera.
- The claim tool for the test machines: its clone at `<path to the claim tool's clone>`, its README there has the
  commands; the box names it uses: `<windows box name>`, `<mac box name>`.
- The tools that drive its desktop (interactive scheduled task, injected keys/clicks, desktop captures):
  `tools/vm/` in this repository, copied to `%USERPROFILE%\ttyak-test\vm` there.
- Mac: SSH alias `mactest` (`mactest.local`, user `tester`), Apple Silicon, macOS 26, Rust via rustup, Homebrew.
  Real speakers, microphone, camera: ask first.

## Private lists

What it is for: the files behind the privacy check and the public screenshots; their contents never leave them.

- Forbidden strings for `tools/site/check_private.py`: `~/.config/ttyak-site/forbidden.txt` (one string per line,
  `#` comments) — the default path the tool reads; `TTYAK_SITE_FORBIDDEN` points elsewhere.
- Made-up names for screenshots: `~/.config/ttyak-site/rename.json` (`{"real": "made-up", …}`) and
  `~/.config/ttyak-site/fixes.json` (single cells to overwrite: `{"huddle": {"row,col": "letter"}}`).
- The history redaction map: `~/.config/ttyak-site/redact.json` (for `tools/docs/split_history.py --redact`).

## Anything else live work needs

What it is for: facts about this machine that change how an agent works here.

- Desktop: Hyprland, kitty, PipeWire; login shell fish (agent shells run bash).
- Audio devices: a headset without a microphone (the "mic" source is silent); no camera.
- Git: the global identity is someone's work identity — set the repository's own identity on every clone; `gh` has
  two accounts, the active one must be `<maintainer handle>`.
- Signing keys and certificates: in `~/.config/ttyak-signing/` (never read by agents); renewal dates.
- Packages installed for this project, to remove when it ends: listed in `~/notes/installed-packages.md`.
- Hooks or scripts on this machine that agents meet (an after-compact recipe, a shell guard).
```
