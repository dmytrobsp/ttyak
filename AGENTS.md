# ttyak — instructions for coding agents

ttyak is a Slack client for the terminal, written in Rust (ratatui + crossterm): messages, threads, huddles with audio
and video, notifications, on Linux, macOS and Windows. One maintainer owns the project and the GitHub repository
`dmytrobsp/ttyak`; agents do most of the building. This page is the front door: the rules that always apply and the
workflow in short. Every detail is one link away.

**Read `AGENTS.local.md` (repository root, git-ignored) if it exists.** It holds what is specific to this machine:
the test profiles, the test workspace, the test machines, the paths of private lists. It is made from
[docs/agents/local-template.md](docs/agents/local-template.md). Without it, read-only work and docs/code work go
ahead, but nothing live: no running ttyak against Slack, no test machines. Ask the maintainer for the local file.

## Where things are

| Path | What |
|---|---|
| `crates/slack-core` | sessions, Slack Web API, real-time events, message formatting |
| `crates/huddle` | huddles: Chime signaling, WebRTC audio/video, devices, camera, screen sharing |
| `crates/ttyak` | the terminal UI and the `ctl` command line (scripting and test helper) |
| `vendor/` | patched copies of crates (rtc-turn, crossterm) |
| [docs/PROGRESS.md](docs/PROGRESS.md) | where the project stands now: latest release, what is in flight, outside waits, next topics |
| `docs/history/` | the old status rows by era, with their LIVE/UNIT/NOT records and commit hashes |
| `docs/agents/` | for agents: [workflow.md](docs/agents/workflow.md), [live-testing.md](docs/agents/live-testing.md), [setup.md](docs/agents/setup.md) (a new machine), [local-template.md](docs/agents/local-template.md) |
| [docs/DESIGN.md](docs/DESIGN.md) | architecture and portability rules |
| `docs/protocol/` | how the undocumented Slack endpoints ttyak uses were worked out |
| `docs/wiki/` | the user manual, published to the GitHub wiki at each release |
| `docs/superpowers/` | `specs/` (designs), `plans/` (implementation plans), `ledgers/` (committed plan state) |
| [docs/RELEASING.md](docs/RELEASING.md) | the release checklist; accounts and secrets: `docs/distribution/accounts.md` |
| `tools/` | dev and test helpers ([tools/README.md](tools/README.md)), the site's tools, the wiki tools |
| `site/`, `packaging/`, `.github/workflows/` | the site at ttyak.sh, package recipes, CI / Release / Pages |
| [README.md](README.md), [CONTRIBUTING.md](CONTRIBUTING.md) | the short front door for users; building and testing for contributors |

The work queue is [GitHub Issues](https://github.com/dmytrobsp/ttyak/issues) (milestones, labels), not a file.

## Build and test

```sh
cargo build                                    # debug build: target/debug/ttyak
cargo test                                     # all crates; the whole run before every Rust commit
cargo build --target x86_64-pc-windows-gnu     # Windows must compile too (MinGW cross-build)
cargo test --no-run --target x86_64-pc-windows-gnu
sh tools/check-name.sh .                       # no old project names (lines marked old-name-ok excepted)
python3 tools/wiki/check_links.py              # wiki, README, site and agent-doc links
```

The tool tests (Python's unittest, node's test runner) are listed in [workflow.md](docs/agents/workflow.md#tests);
`tools/container-build.sh debian|ubuntu|fedora|arch` builds a clean checkout in a fresh container (podman).

## Hard rules

- **Shell.** Agent shells may be bash even where the maintainer's login shell is not: write bash (`NAME=value`,
  never fish's `set NAME value`); start every multi-command script with `set -euo pipefail`. Never run a recursive
  `rm`, `chmod`, `chown`, `mv`, `cp` (or `find -delete`) on a path built from a variable: write the literal path, or
  use `${VAR:?}` after checking it is set, exists and lies where you expect. (An empty variable once turned
  `chmod -R 700 $S/home` into `chmod -R 700 /home`.)
- **Never `cargo fmt`**: the code base is not rustfmt-clean; hand-format, match the surrounding code.
- **Never build into `target/release`**: the maintainer may be running ttyak from it. Optimised experiments go into a
  scratch `--target-dir`. `cargo build` before any live run (`cargo test` does not rebuild the binary).
- **Never write the maintainer's real ttyak folders** (`~/.config/ttyak`, `~/.local/share/ttyak`, `~/.cache/ttyak`
  and their macOS/Windows twins): live checks run under a fake home. Never kill a ttyak you did not start (no
  `pkill` by name, never `pkill -f`): kill by the PID you started.
- **Profiles.** Never use a ttyak profile you were not given. Every ttyak command in a test names one of the test
  profiles from `AGENTS.local.md` with `--profile`; a bare `ttyak` uses the machine's default profile, which can be
  the maintainer's real work account.
- **Nothing on the maintainer's screen.** Test windows open hidden (a special workspace); no screenshots of the real
  screen; no prompts (gpg, keyring, polkit, sudo) popping up on their desktop.
- **No capture**: never the real microphone, camera or screen. Sound only when agreed; by default a test's settings
  switch sounds off.
- **One test per Slack account at a time**, across all agents and machines.
- **Shared test boxes** (the Windows and macOS test machines) are used only under a claim, through the claim tool
  named in `AGENTS.local.md`: no claim, no use — the maintainer claims too.
- **Clipboard**: anything that copies saves the maintainer's clipboard first and restores it after.
- **Bounded live waits**: cap any live wait or observation (not a wait for a claim) at
  about 10 minutes; run only experiments whose answer changes what gets built.
- **Git identity**: every clone and every new repository (wiki, tap, bucket, scratch) commits with this repository's
  own identity, set repo-locally — never a machine's global one (it may be someone's work identity).
- **Public wording** in commits, issues, docs, release notes: no test-account names, emails or Slack ids, no
  workspace names, no host names or addresses, no one's work identity, no one's real or legal name (the maintainer
  appears only as their GitHub handle), nothing from the private lists.
- **A refused action** (a permission prompt, a safety check): stop, check `git status` for partial writes, explain,
  and give the maintainer the exact command. Never reword or retry it to get around the refusal.
- **Security findings** never go into a public file, issue or commit message: open a **GitHub private security
  advisory** (the repository's Security tab → Advisories → New draft security advisory) and tell the maintainer.
  Commits that fix one say "hardening", without describing an exploit.
  Outsiders report through [SECURITY.md](SECURITY.md) (GitHub's private vulnerability reporting).
- **The maintainer's word** is needed for: tags and releases, force-pushes and history rewrites, deleting anything
  on GitHub, repository settings, publishing the wiki (`--push`), posts outside the test workspace, anything that
  costs money or speaks for them. Routine pushes of `master` need no asking.

## The workflow in short

Full text: [docs/agents/workflow.md](docs/agents/workflow.md).

1. **Where we are**: [docs/PROGRESS.md](docs/PROGRESS.md), the open issues of the current milestone, the newest ledger
   in [docs/superpowers/ledgers/](docs/superpowers/ledgers/README.md) with unfinished tasks, `git log`. Never re-run a
   task the ledger or the log shows done.
2. **Pick work** from the open milestone. With none open, don't pick from `later` on your own: propose a milestone
   (a few related issues, and why) and the maintainer decides. `needs-user` issues wait for the maintainer; `triage`
   issues are user reports nobody has looked at.
3. **Plan** anything bigger than a focused fix: brainstorm with the maintainer → design in `docs/superpowers/specs/`
   → a builder's read → the maintainer's approval → a plan in `docs/superpowers/plans/` → subagent-driven execution
   with a committed ledger, a review per task and a final review; review findings in scope are fixed, not filed (only
   out-of-scope items and the maintainer's decisions become issues). Before big work the maintainer may ask for a
   handoff to a fresh session.
4. **Build in small verified steps**, one commit each: TDD with the failing test shown first, the whole `cargo test`
   before a Rust commit, never commit on red, Windows compiles.
5. **Verify honestly**: say what was checked LIVE (in the real app against Slack), UNIT (tests only) or NOT; evidence
   before claims. Live checks follow [docs/agents/live-testing.md](docs/agents/live-testing.md).
6. **Commit and push**: explicit paths, never a scripted edit chained with `git commit`, the co-author trailer,
   `Fixes #N`. Docs-only pushes skip CI; after any push that runs CI, watch it.
7. **Docs**: the wiki in `docs/wiki/` (published at each release), the README short,
   [docs/PROGRESS.md](docs/PROGRESS.md) rewritten (never appended to), the site checked for private strings.
8. **Release** with [docs/RELEASING.md](docs/RELEASING.md), only on the maintainer's word.

When something only the maintainer can decide comes up (an account, a live check with them, a public post, a
trade-off), stop and ask; label such issues `needs-user`.
