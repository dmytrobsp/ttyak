# ttyak — where things stand

The current state, in short. This page is rewritten when things change, never appended to; keep it near 100 lines.
What happened before is in [history/](history/README.md), the plans' ledgers and `git log`.

Last rewritten: 2026-10-29.

## Latest release

- **0.12.2**, released 2026-10-26: [release page](https://github.com/dmytrobsp/ttyak/releases/tag/v0.12.2).
- What it brought: private files owner-only on Windows; the Windows uninstall counts only regular files as old
  copies; showing a Windows notification no longer waits for reading the toast setting, and quitting never hangs (an
  update being installed finishes first); late notification bubbles tracked, huddle bubbles gone when the huddle
  ends; VIPs get through a pause when Slack's VIP setting says so (with an F6 row); the in-app help matches the code;
  `tools/wiki/publish.sh` hardened.
- Before it: 0.12.1 (2026-10-26, fixes from the maintainer's live checks of 0.12.0) and 0.12.0 (2026-10-25,
  attention: notifications, the bell, the tab title, clicks; `ctl status` and `ctl check`).
- The wiki was published at the v0.12.2 tag; the README is the short front door, the
  [wiki](https://github.com/dmytrobsp/ttyak/wiki) is the manual.
- The whole `cargo test` passed at 0.12.2: in CI on Linux and macOS, and on a Windows test machine (CI does not run
  the Windows tests: #61). How to run it: [AGENTS.md](../AGENTS.md).
- How a release is made: [RELEASING.md](RELEASING.md).

## In flight

- **Done: test machines and shared resources** ([#30](https://github.com/dmytrobsp/ttyak/issues/30), 2026-10-27 –
  2026-10-29): design [2026-10-27-test-machines-design.md](superpowers/specs/2026-10-27-test-machines-design.md),
  five plans and their ledgers in [superpowers/plans/](superpowers/plans/) and
  [superpowers/ledgers/](superpowers/ledgers/README.md) (box claims, account claims, Linux instances, VM clones,
  macOS VM). Every shared test box is used under a claim, the Slack test accounts are claimed and installed into fake
  homes, and live checks run in throwaway instances by default: sway and GNOME containers, a Hyprland VM, Windows 11
  evaluation clones and a macOS VM on the Mac — all through the claim tool named in `AGENTS.local.md`
  ([docs/agents/live-testing.md](agents/live-testing.md)). ttyak ran in each and a Slack message reached it, live.
  Found on the way: [#63](https://github.com/dmytrobsp/ttyak/issues/63) (Hyprland raise). The history of `master`
  was rewritten on 2026-10-28 to keep the claim tool unnamed
  ([history-rewrite-2026-10-28.md](superpowers/ledgers/history-rewrite-2026-10-28.md)).
- **In flight: the multi-agent milestone workflow** ([#29](https://github.com/dmytrobsp/ttyak/issues/29), started
  2026-10-29): design
  [2026-10-29-milestone-workflow-design.md](superpowers/specs/2026-10-29-milestone-workflow-design.md) (approved),
  plan [2026-10-29-milestone-workflow.md](superpowers/plans/2026-10-29-milestone-workflow.md), ledger
  [2026-10-29-milestone-workflow.md](superpowers/ledgers/2026-10-29-milestone-workflow.md). One controller runs a
  milestone with several implementers at once, each issue a pull request into a milestone branch; then a trial
  milestone of 3–5 small issues the maintainer picks.
- No release milestone is open. Last done: the documentation housekeeping (2026-10-26 – 2026-10-27).

## Outside waits

- **Package registries** (winget, Chocolatey): submissions waiting for review — tracked in
  [#40](https://github.com/dmytrobsp/ttyak/issues/40), updated at each release.
- **The AUR**: publishing waits for its sign-up to reopen — [#4](https://github.com/dmytrobsp/ttyak/issues/4).
- **The Apple Developer ID certificate** expires 2027-11-22: renew it before then —
  [#62](https://github.com/dmytrobsp/ttyak/issues/62).

## Next topics

**How the next work is chosen:** with no milestone open, an agent does not pick from `later` on its own. It proposes
a milestone to the maintainer (a few related issues, and why they go together), and the maintainer decides.
`needs-user` issues wait for the maintainer.

The maintainer's next topics, all about how the work gets done (`process`, `needs-user`):

- [#29](https://github.com/dmytrobsp/ttyak/issues/29) — a multi-agent milestone workflow: agents pull issues, pull
  requests into a milestone branch, review-fix loops.
- [#39](https://github.com/dmytrobsp/ttyak/issues/39) — maintenance: watch dependencies and Slack for changes,
  re-test when they change.

The rest of the queue: the [`later` milestone](https://github.com/dmytrobsp/ttyak/milestones), curated with
`-label:triage`.

- Recent filings (2026-10-27, from the housekeeping audit): [#43](https://github.com/dmytrobsp/ttyak/issues/43) — what
  ttyak doesn't do yet (the wiki's [Not supported yet](wiki/Not-supported-yet.md) page), and #44–#62 — the minors,
  known issues and checks that were never filed.
- Known issues: #1 (Windows: keys typed right after start are lost), #55, #58, #59.
- Bigger features waiting for the maintainer: #2 (navigation rework), #3 (signed Linux builds), #5 (Windows signing).

List them: `gh issue list -R dmytrobsp/ttyak -m later -S '-label:triage'`.

## Where things live

| What | Where |
|---|---|
| Agent instructions, hard rules | [AGENTS.md](../AGENTS.md) (start here); `AGENTS.local.md` for this machine's specifics |
| The workflow | [agents/workflow.md](agents/workflow.md): queue, planning, executing, commits, docs, releasing |
| Live checks | [agents/live-testing.md](agents/live-testing.md) — read before any live check |
| A new machine | [agents/setup.md](agents/setup.md), [agents/local-template.md](agents/local-template.md) |
| The queue | [GitHub Issues](https://github.com/dmytrobsp/ttyak/issues); registries in #40 |
| Designs, plans | [superpowers/specs/](superpowers/specs/), [superpowers/plans/](superpowers/plans/) |
| Plan ledgers | [superpowers/ledgers/](superpowers/ledgers/README.md) (committed); scratch in `.superpowers/sdd/` (ignored) |
| History | [history/](history/README.md): every step up to 0.12.2 with its LIVE/UNIT/NOT record and commit; the bug log |
| Architecture | [DESIGN.md](DESIGN.md); Slack's endpoints in [protocol/](protocol/) |
| Releasing | [RELEASING.md](RELEASING.md) |
| The user manual | the [wiki](https://github.com/dmytrobsp/ttyak/wiki), edited in [wiki/](wiki/Home.md), published at each release |
| The site | `site/` (ttyak.sh), built by `tools/site/fill.py` |

## Keeping this page

- Rewrite it when a release ships, a plan starts or ends, an outside wait changes or the next topics change.
- Short plain lines; links to issues, plans and ledgers instead of detail. What is done goes to the ledger and
  `git log`, not here.
- Nothing private: no workspace, account, machine or person names (`AGENTS.local.md` holds those).
