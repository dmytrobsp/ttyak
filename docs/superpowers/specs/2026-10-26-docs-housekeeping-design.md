# Documentation housekeeping — design (2026-10-26)

## Intent

The user: "before we do any more work, i'd like to do some housekeeping on our documentation. add a CLAUDE.md (or
AGENTS.md if claude auto-loads it), split and clean up our PROGRESS.md (it's all over the place right now and still
carries things that were supposed to be tracked by github issues), move the docs artifacts under `docs/`." And: "i
would like to make the documentation detailed enough so another agent can pick up the project on a completely
different machine without any prior knowledge or memory of it."

Today the knowledge needed to work on ttyak is spread over: PROGRESS.md (910 lines, 742 KB — a status table with very
long rows and a stack of "NEXT UP" paragraphs from every era), the agent's private memory notes (≈30 files of rules
and lessons), the user's global CLAUDE.md, an after-compact hook, the plans' git-ignored ledgers, and the user's head.
A fresh agent on another machine, given only the clone, cannot work safely: it would not know never to use the
machine's default ttyak profile, how live checks are done, where the queue is, or how a release goes.

**Done means:** a fresh agent with only the clone and one local file (filled in from a template) can set up the
machine, understand where the project stands, and pick up the next issue following the project's workflow.

## Goals

1. **`AGENTS.md`** at the root (read by many coding agents) holds the agent instructions; **`CLAUDE.md`** is one line,
   `@AGENTS.md`, because Claude Code loads `CLAUDE.md` (and follows `@` imports), not `AGENTS.md`. AGENTS.md covers:
   - what ttyak is and where everything lives (a short map, pointing at the detailed docs);
   - build/test commands (incl. the Windows cross-build and the tool tests);
   - the hard rules (shell: bash not fish, `set -euo pipefail`, no recursive rm/chmod on variable paths; no `cargo
     fmt`; never build `target/release`; never write the maintainer's real ttyak folders; never use a ttyak profile
     other than the test profiles named in the local file; test windows never appear on the maintainer's screen;
     never capture mic/camera/screen; sound only when agreed; one test per Slack account at a time; save/restore the
     clipboard around anything that copies; bounded live waits; every clone and new repo commits with the repo's own
     identity, never a global one; public wording in commits, issues and docs);
   - the whole workflow (below, "Workflow"), compact, linking to the detailed pages;
   - "read `AGENTS.local.md` if present" (the local file, Goal 5).
2. **The workflow written down** (a compact version in AGENTS.md; the full text in `docs/agents/workflow.md` — planning,
   queue, commits/pushes, verification, docs, releasing — and `docs/agents/live-testing.md` — profiles, windows,
   clipboard, sound/mic/camera, test machines, waits; setup in `docs/agents/setup.md`): planning (brainstorm → design in
   `docs/superpowers/specs` → a builder's read → the maintainer's approval → plan → subagent-driven execution with a
   ledger, per-task review, final review, deferred items filed as issues; a handoff before big work); the queue
   (GitHub Issues, milestones, labels incl. `triage`/`needs-user`, `Fixes #N`, public wording; registry submissions in
   issue #40); commits and pushes (one step per commit, explicit paths, the co-author trailer, never chaining a
   scripted edit with a commit; routine pushes of master need no asking, tags/releases/force-push need the
   maintainer's word; docs-only pushes skip CI; watch CI after every push that runs it); verification (TDD with the
   RED shown; the whole `cargo test` before a Rust commit; LIVE/UNIT/NOT honesty; evidence before claims); live
   testing (as in Goal 1, plus "wait for ready" when the maintainer takes part, the test-machine rules); docs (the
   wiki is edited in `docs/wiki/` and published at each release; the README stays short; the site's approved wording
   and its privacy check); releasing (`docs/RELEASING.md`).
3. **PROGRESS becomes a short current-state page**, `docs/PROGRESS.md` (≈100 lines, plain short lines): the latest
   release, what is in flight (milestone, active plan and its ledger), outside waits (pointing at #40), the next topics
   (pointing at issues), and where history lives. It is rewritten, not appended to; the old root `PROGRESS.md` goes.
4. **History is archived, not lost**: the status-table rows (with their LIVE/UNIT/NOT records and commit hashes) move
   to `docs/history/`, one file per era (e.g. `0.1-0.9.md` slacktui/klack, `0.10-0.11.md` rename and distribution,
   `0.12.md` attention, README/wiki, 0.12.2 — the builder picks the cut), wrapped to readable line lengths, content
   unchanged apart from wrapping. The old NEXT UP paragraphs are not carried over once the audit (Goal 6) has taken
   what is still open out of them. The bug log goes to history too.
5. **The public/private split.** Everything general goes into the public docs; what is truly specific to one machine
   or person goes into **one git-ignored file, `AGENTS.local.md`**, made from a committed template
   (`docs/agents/local-template.md`): the test profile names, the test machines' host aliases and how to reach them,
   the paths of the site's private lists, and anything else the template asks for. Concretely:
   - "the default profile is the maintainer's work workspace" → the general rule "never use a profile you were not
     given; every ttyak command names a test profile from the local file";
   - the test workspace and its two accounts → "a Slack workspace with two test accounts, A and B" — the current one
     is on the **free** plan (note: it reports `team.plan = "std"`, so ttyak's — and Slack's client's — paid-plan check
     treats VIP features as available there; checks that depend on the plan say so); setting one up and signing it in
     on a new machine (the email-code sign-in) is on the setup page;
   - the Windows VM and the Mac → what a test machine needs and how it is driven (Goal 7), host details local;
   - the site's private lists (`forbidden.txt`, `rename.json`) → their format and how to make them, contents local;
   - not-public security findings → **GitHub private security advisories** (draft advisories in the repo's Security
     tab), documented in AGENTS.md; nothing such goes into a public file;
   - the global-git-identity trap → a public rule (Goal 1).
6. **The audit**: every item in the old PROGRESS "Next", "Known gaps" and "Bug log" sections, and in the agent's
   memory notes, is sorted into: already an issue (link it), still open (file an issue, public wording), done or
   obsolete (dropped, listed), or a rule/lesson (goes into AGENTS.md / `docs/agents/`). The sorted list is shown to the
   maintainer before anything is dropped.
7. **A setup page, `docs/agents/setup.md`**: everything a new machine needs — Rust toolchain and targets
   (`x86_64-pc-windows-gnu`), mingw, podman (and the images the tests use), `gh` auth, Python/node for the tool tests,
   the test workspace and test profiles (signing in A and B with the email code), filling in `AGENTS.local.md`, the
   site's private lists, the test machines: a Windows machine (SSH, how the existing tools drive it — interactive
   scheduled task, injected input, desktop captures) and a Mac (SSH, the GUI-session trick). The tools that drive the
   Windows VM currently live in another of the maintainer's repos; they are brought into this repo (`tools/vm/`) if
   that is straightforward, otherwise documented with what they do (the builder decides, says so).
8. **In-flight plan state is committed**: a plan's ledger decisions (task status, rulings, deferred items) live in
   `docs/superpowers/ledgers/<plan>.md`, committed at task boundaries; only bulky scratch (briefs, review packages,
   reports) stays git-ignored in `.superpowers/sdd/`. AGENTS.md says so (it extends the subagent-driven skill's
   convention for this project).
9. **Docs under `docs/`**: `DESIGN.md` → `docs/DESIGN.md`. `README.md`, `CONTRIBUTING.md`, `AGENTS.md`, `CLAUDE.md`
   and the licences stay at the root (GitHub and agents look for them there).
10. **The after-compact recipe points at sources and holds no state** (agreed with the user; it stays local, see
    Non-goals). Today it hard-codes a release state ("0.12.0 is released, don't re-run its tasks") and goes stale. The
    rewritten recipe names no release or task; it points at and snapshots:
    - the rules and the workflow, by name: AGENTS.md (CLAUDE.md is reloaded anyway), `docs/agents/workflow.md`,
      `docs/agents/live-testing.md` (before any live check), `docs/RELEASING.md` (before anything release-related), and
      the memory index; plus the few rules that bite hardest after a compaction, in one line each (never a profile
      other than the local file's test profiles; tags, releases, force-pushes and outward posts only with the
      maintainer's word; never re-run finished tasks — check the ledger and `git log` first; LIVE/UNIT/NOT honesty);
    - where we are: `docs/PROGRESS.md`, the newest committed plan ledger in `docs/superpowers/ledgers/` that has
      unfinished tasks (with its design and plan), and the open issues of the current milestone;
    - a live snapshot it computes: the last commits, uncommitted files, commits not pushed, the latest CI and Release
      runs, open test windows (`hyprctl clients`, the test class), ttyak processes started by tests (`pgrep -x`), and
      whether a test VM is running;
    - what to do: report where things stand and continue only what the conversation summary shows was in progress.
    The edit is outside the repo: shown to the user before it is written.
11. **References follow the moves**: `tools/check-name.sh` (its allow-list names PROGRESS.md), the wiki's Project-layout
    page, CONTRIBUTING, RELEASING (its "PROGRESS.md" steps), README if it links them. Old specs/plans keep their
    historical text.

## Non-goals

- The after-compact recipe is not moved into the repo or generalized for other machines (the user: "let's leave it out
  for now … we'll generalize that later"). It stays at `~/.claude/hooks/after-compact.d/klack.sh`.
- No code changes; no wiki content changes beyond links; no rewriting of old specs/plans.
- The memory system is not removed: it keeps who the user is and their preferences, plus pointers into the repo docs.

## Anti-goals

- A private detail in a public file (the work workspace's name, test-account names/emails/ids, host names, the
  maintainer's work identity, private-list contents).
- Losing the honest history (LIVE/UNIT/NOT records, hashes).
- An open item silently dropped in the audit.
- Broken links after the moves; a PROGRESS page that grows back into a log.
- Docs so long a fresh agent doesn't read them: AGENTS.md stays a page or two, details one link away.

## Constraints

- Mostly docs-only commits (CI skips `**.md` and `docs/**`); `tools/check-name.sh` changes run CI. Small steps, one
  commit each; public wording; `python3 tools/site/check_private.py` on new public docs where useful (a folder copy).
- The user's memory notes and the hook are outside the repo: changes there are local and shown to the user.

## Verification

A **cold read**: a fresh subagent told to use only the repo (not memory; no prior context) and the filled-in local
file answers a checklist — how to set up, build, test; which profile it may use; where the queue is and what to pick
next; how a change goes from issue to commit to push; how a live check is done safely; how a release goes; what to
do with a security finding — and points out anything it couldn't find. Gaps found are fixed before the work is done.

## Answers for the builder

- **Memory notes** are readable by whoever builds this on the maintainer's machine:
  `~/.claude/projects/-home-poniraq-projects-klack/memory/` (MEMORY.md is the index) and the user's global
  `~/.claude/CLAUDE.md`. Mine them for rules and lessons; the hard-rules list in Goal 1 is a summary, not complete.
  Copy nothing private into public files.
- **No `AGENTS.local.md`** on a machine (my call): read-only work and docs/code work proceed; anything live (running
  ttyak against Slack, test machines) stops and asks the maintainer for the local file.
- **The privacy check** (`tools/site/check_private.py <dir>`, driven by the maintainer's private `forbidden.txt`) is
  mandatory for new or moved public docs where that list exists (it does on this machine), run on a folder copy;
  elsewhere it is skipped with a note (my call).
- **History wrapping** (my call): automated, then checked — the archive's text with whitespace normalised must equal
  the old rows' text. Rows may become one section per row (heading = the row's id and title) instead of a table;
  hashes and LIVE/UNIT/NOT words unchanged. The bug log stays one file.
- **The audit** (my call): the sorted list is produced first and shown to the maintainer in one round; the parts that
  don't depend on it (moves, AGENTS.md, setup, template) go ahead meanwhile; issues are filed and items dropped only
  after the maintainer has seen the list.
- **The VM tools** are at `~/projects/fastview-rea/tools/vm` on this machine. Bring them into `tools/vm/` when they hold
  no secrets and their host specifics can move to `AGENTS.local.md`; otherwise document them (my call, the builder
  checks).
- **The cold read** (my call): a fresh subagent in a clean clone in the scratchpad, with a copy of this machine's
  `AGENTS.local.md`, told not to use memory or prior context; it answers from the docs only (no live commands) and
  lists what it couldn't find.
- **Existing ledgers** under `.superpowers/sdd/` are not migrated or touched; the committed-ledger convention applies
  from the next plan.

## Calls I made (for the user to check)

`AGENTS.md` + one-line `CLAUDE.md`; `AGENTS.local.md` as the single git-ignored local file with a committed template;
`docs/PROGRESS.md` short and rewritten; history in `docs/history/` by era; ledgers committed under
`docs/superpowers/ledgers/`; security findings as GitHub private advisories; VM tools brought into `tools/vm/` if
straightforward; the after-compact recipe rewritten to point at sources with a computed snapshot and no hard-coded
state (shown to the user before it is written); the free test workspace documented with its `team.plan` quirk.
