# Documentation housekeeping — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or
> superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Documentation complete enough that a fresh agent on another machine, with only the clone and one local file,
can set up, understand where ttyak stands, and pick up the next issue the project's way.

**Architecture:** Root keeps README, CONTRIBUTING, AGENTS.md, a one-line CLAUDE.md and the licences. Everything else
lives under `docs/`: `PROGRESS.md` (short, current state), `history/` (the old status rows by era), `agents/`
(workflow, live testing, setup, the local-file template), `DESIGN.md`, plus what is there already. One git-ignored
`AGENTS.local.md` per machine holds what is specific to it.

**Tech Stack:** Markdown; a small Python 3 script for the history split and its check; bash.

**Spec:** `docs/superpowers/specs/2026-10-26-docs-housekeeping-design.md` — read it first (Goals 1–11, Answers for the
builder, Calls).

## Global Constraints

- The Bash tool runs bash, not fish: `NAME=value`; `set -euo pipefail` first; never a recursive chmod/chown/rm/mv/cp or
  `find -delete` on a path built from a variable (literal path or `${VAR:?}` + a check). Put these rules in every
  subagent prompt.
- **Nothing private in a public file**: no work-workspace name, no test-account names/emails/Slack ids, no host
  names/addresses, no maintainer work identity, no private-list contents. Run `python3 tools/site/check_private.py
  <folder copy>` on new/moved public docs (the list exists on this machine). Private specifics go to `AGENTS.local.md`
  (git-ignored) only.
- Small steps, one commit each; `git mv` for moves (history follows); explicit paths; never chain a scripted edit with
  `git commit`; trailer `Co-Authored-By: Claude <model> <noreply@anthropic.com>`. Docs-only pushes skip CI; a change to
  `tools/` runs CI — watch it.
- Implementers never push, file/close issues, or edit files outside the repo; the controller does, and shows the user
  first where the spec says so (the audit list, the after-compact recipe).
- No code changes beyond `tools/` scripts. Old specs/plans keep their historical text.
- Readable lines: new docs wrap at ~120 columns; AGENTS.md stays a page or two, details one link away.

## Review Focus

1. **A private detail leaking into a public doc** while mining memory notes — the privacy check on every new public
   doc, plus a reviewer read (Tasks 3, 4, 6).
2. **History altered while being moved** — Task 2's check: whitespace-normalised archive text equals the old rows.
3. **An open item silently dropped** — Task 5's audit list has every item of the old sections and memory notes in one
   of four bins; nothing is dropped before the user has seen it.
4. **A broken link after the moves** — `python3 tools/wiki/check_links.py` and a grep for the old paths (Task 6).
5. **A fresh agent still can't find something** — Task 8's cold read.

---

### Task 1: `DESIGN.md` → `docs/DESIGN.md`

**Files:** `git mv DESIGN.md docs/DESIGN.md`; update links in `README.md`, `CONTRIBUTING.md`, `docs/wiki/Project-layout.md`,
`docs/RELEASING.md` if any, `tools/check-name.sh` if it names it (+ `tools/test_check_name.py`).

- [ ] `grep -rn 'DESIGN.md' --exclude-dir=target --exclude-dir=vendor --exclude-dir=.git . | grep -v docs/superpowers`
  → every hit updated. `python3 tools/wiki/check_links.py` → 0; `tools/check-name.sh .` → 0.
- [ ] Commit `docs: DESIGN.md moves under docs/`.

### Task 2: History archive `docs/history/`

**Files:** Create `tools/docs/split_history.py` (+ `tools/docs/test_split_history.py`), `docs/history/README.md` (what
is here, how to read LIVE/UNIT/NOT), `docs/history/<era>.md` files, `docs/history/bug-log.md`. PROGRESS.md is NOT
changed in this task.

**Interfaces:** `split_history.py PROGRESS.md docs/history/` writes one file per era (cut chosen by the builder, e.g.
`0.1-0.9.md` slacktui/klack up to v0.9.0, `0.10-0.11.md` rename and distribution, `0.12.md` 0.12.0–0.12.2 incl. the
README/wiki split) from the `## Status` table rows — each row becomes a `### <id> — <title>` section with its text
wrapped at ~120 columns and its status/commit cells kept — and `bug-log.md` from `## Bug log`; `--check` compares the
whitespace-normalised text of the archive with the old rows and exits non-zero on any difference.

- [ ] Failing test first (a small fixture PROGRESS with 3 rows across 2 eras and a bug log → expected files; `--check`
  catches one changed word). RED, then implement, GREEN.
- [ ] Run on the real PROGRESS.md; `--check` → 0; spot-read three rows (oldest, a middle one, PT11).
- [ ] Commit `docs: history archive by era (tools/docs/split_history.py)`; the tool test goes into `ci.yml`'s unittest
  line (CI runs).

### Task 3: `AGENTS.md`, `CLAUDE.md`, `docs/agents/workflow.md`, `docs/agents/live-testing.md`, the local file

**Files:** Create `AGENTS.md`, `CLAUDE.md` (exactly `@AGENTS.md` plus a one-line comment if needed),
`docs/agents/workflow.md`, `docs/agents/live-testing.md`, `docs/agents/local-template.md`; add `AGENTS.local.md` to
`.gitignore`; create this machine's `AGENTS.local.md` (git-ignored, never committed — show its path, not its content,
in the report).

- [ ] Gather: the user's global `~/.claude/CLAUDE.md`, every memory note under
  `~/.claude/projects/-home-poniraq-projects-klack/memory/`, CONTRIBUTING, RELEASING, the last plans' rules files
  (`docs/superpowers/plans/2026-10-26-0.12.2.md` Global Constraints), PROGRESS's NEXT paragraphs for workflow facts.
  Generalise per spec Goal 5; record in the report which memory note fed which section.
- [ ] Write AGENTS.md per spec Goal 1 (incl. GitHub private security advisories for security findings; "read
  AGENTS.local.md if present; without it, no live work — ask the maintainer"); workflow.md and live-testing.md per Goal
  2; the template per Goal 5 (sections: test profiles, test workspace notes, test machines, private lists, anything
  else live work needs — each with what it is for and an example value).
- [ ] Privacy check on a folder copy of the new public files → 0; `check_links.py` → 0.
- [ ] Commit `docs: AGENTS.md and the agent workflow`.

### Task 4: `docs/agents/setup.md` and the VM tools

**Files:** Create `docs/agents/setup.md`; maybe `tools/vm/` (copied from `~/projects/fastview-rea/tools/vm` when it holds
no secrets and its host specifics can move to `AGENTS.local.md`; otherwise setup.md documents what the tools do).

- [ ] setup.md per spec Goal 7, every command tried on this machine where it is read-only/harmless (`rustup target
  list --installed`, `gh auth status`, `podman images`, …) — LIVE/UNIT/NOT honesty about what a new machine was not
  tried on.
- [ ] Privacy check; `check_links.py`; if `tools/vm/` is added, its scripts pass `bash -n` and hold no host names.
- [ ] Commit `docs: setup for a new machine` (+ `tools: the test-VM driver, brought in` if added).

### Task 5: The audit list (STOP point: the user sees it)

**Files:** Create a git-ignored `.superpowers/sdd/2026-10-26-docs-housekeeping/audit.md` (it may quote private memory
lines; it is not committed).

- [ ] Every item in PROGRESS's `## Next` (all NEXT UP paragraphs), `## Known gaps / not verified`, `## Bug log`, and in
  each memory note, sorted into: **issue exists** (link), **file an issue** (proposed public title + one line),
  **done/obsolete** (why), **rule/lesson** (where it went in Task 3, or where it should go). A count per bin; no item
  unbinned.
- [ ] The controller shows the list to the user; Tasks 6–7 wait for their word on it.

### Task 6: Apply the audit; `docs/PROGRESS.md`; references

**Files:** Create `docs/PROGRESS.md` (≈100 lines, spec Goal 3); `git rm PROGRESS.md`; create
`docs/superpowers/ledgers/README.md` (the committed-ledger convention, spec Goal 8); update `tools/check-name.sh` allow-list
(PROGRESS.md → `docs/PROGRESS.md`, add `docs/history/`) + its test; update `docs/RELEASING.md` (its PROGRESS steps),
`CONTRIBUTING.md`, `docs/wiki/Project-layout.md`, AGENTS.md/workflow.md if the audit added rules.

- [ ] The controller files the user-approved new issues (public wording) before this task; their numbers go into
  docs/PROGRESS.md where relevant.
- [ ] `grep -rn 'PROGRESS.md' . --exclude-dir={target,vendor,.git,docs/superpowers,docs/history}` → only the new path;
  `check_links.py` → 0; `tools/check-name.sh .` → 0; privacy check on docs/PROGRESS.md.
- [ ] Commit(s) `docs: PROGRESS is a short current-state page under docs/` (and the check-name change separately, CI runs).

### Task 7: Memory notes trimmed; the after-compact recipe (outside the repo; the user sees both first)

- [ ] Memory: each note whose content now lives in the repo becomes a one-line pointer or is removed from the index;
  user/preferences notes stay. Show the user the new MEMORY.md index before writing.
- [ ] The recipe `~/.claude/hooks/after-compact.d/klack.sh` rewritten per spec Goal 10 (no hard-coded state; named
  workflow docs; the four one-line rules; the computed snapshot). Show the user the new recipe; write it with their
  word; test it by piping a fake hook input (`{"cwd":"/home/poniraq/projects/klack"}`) through
  `~/.claude/hooks/after-compact.sh` and reading the output.

### Task 8: The cold read

- [ ] A fresh subagent, in a clean clone in the scratchpad with a copy of `AGENTS.local.md`, told to use only the repo
  (no memory, no prior context), answers the spec's checklist from the docs and lists what it couldn't find.
- [ ] Every gap fixed (docs commits); the cold read's report kept in the ledger.
- [ ] Final: PROGRESS (docs/PROGRESS.md) says the housekeeping is done; push.
