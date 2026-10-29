# Milestone Workflow Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** A milestone can be run by one controller with several implementers at once — each unit in its own
worktree, a pull request into a milestone branch, a review-fix loop, a merge gate — with the docs, a small helper, a
measured capacity, an approved allowlist and a dry run that leaves the trial milestone free of known refusals.

**Architecture:** One Python helper (`tools/milestone/milestone.py`, stdlib only) does the mechanical git work: the
milestone branch and its gate worktree, unit worktrees, their removal by checked path, a listing with disk use, and
the gate's merge. Everything else is written workflow: a milestone section in `workflow.md`, the three places and
per-unit folders in `live-testing.md`, the milestone ledger lines, a report template. The controller measures the
machine's capacity, proposes the allowlist (the maintainer approves it) and runs a dry run before the trial.

**Tech Stack:** Python 3.12 stdlib (`argparse`, `subprocess`, `shutil`, `pathlib`, `unittest`), git ≥ 2.43
worktrees, `gh`, Claude Code background subagents and `.claude/settings.local.json` permission rules.

**Spec:** [docs/superpowers/specs/2026-10-29-milestone-workflow-design.md](../specs/2026-10-29-milestone-workflow-design.md)
(approved 2026-10-29). Read it whole: the plan argues from it.

## Global Constraints

- **Shell**: bash, never fish (`NAME=value`); `set -euo pipefail` in every multi-command script; never a recursive
  `rm`, `chmod`, `chown`, `mv`, `cp` (or `find -delete`) on a variable path — literal, or `${VAR:?}` after checking it
  is set, exists and lies where expected. In Python, `shutil.rmtree` only on a path the code has just checked.
- **Never `cargo fmt`; never build into `target/release`**; `cargo build` before any live run.
- **Nothing private in tracked files**: never the claim tool's name, folders or variables (`<tool>` only), no
  account, workspace, host or person names; a case-insensitive `git grep` for the claim tool's name (in `AGENTS.local.md`) over `docs` and `tools` finds nothing; new or moved docs pass
  `python3 tools/site/check_private.py --allow-owner <the repository's owner> <folder of copies>`.
- **Implementers** never push, tag, file or close issues, edit settings files, post outside the test workspace, edit
  outside their task's files, or dispatch subagents. Only the controller pushes and talks to the maintainer.
- **Force-pushes, deleting anything on GitHub, settings changes**: only with the maintainer's word.
- **Tests first**: the failing test shown, then the code; the helper's tests run pristine
  (`python3 -W error -m unittest tools/milestone/test_milestone.py`); a new tool test goes into CI's `Tool tests` step
  and its copy in `docs/agents/setup.md` in the same commit.
- **Commits**: explicit paths; never a scripted edit chained with `git commit`; trailer
  `Co-Authored-By: Claude <model> <noreply@anthropic.com>`; `Fixes #29` only on the plan's last commit. Lines ≤ 120.
  Docs-only pushes skip CI; a push touching `tools/` or `.github/` runs CI — watch it (`gh run watch <id>
  --exit-status`, capped).
- **Docs style**: plain short sentences as in the existing agent docs; `python3 tools/wiki/check_links.py` and
  `sh tools/check-name.sh .` pass.
- **Test repositories** in the helper's tests live in `tempfile.TemporaryDirectory()`, get a repo-local identity
  (`git config user.name/user.email` in the temp repo), and never touch this repository's worktrees or branches.

## Review Focus

1. **`rm` aimed at the wrong place** — the main checkout, the gate worktree, a path outside the worktree root, a
   symlink into elsewhere, a worktree of another repository: refused with exit 2, nothing deleted (Task 1 tests).
2. **A full disk** — `add` below the free-space floor refuses before creating a branch or a folder (Task 1 tests).
3. **A gate merge with conflicts** — aborted, the gate worktree left clean on the milestone tip, exit 3 naming the
   conflicting files (Task 1 tests).
4. **Hostile names** — a NAME or UNIT with `/`, `..`, spaces, a leading `-` or upper case: refused before git runs
   (Task 1 tests).
5. **A worktree folder deleted by hand** — `ls` shows it as missing; `rm` prunes git's record without error
   (Task 1 tests).

---

### Task 1: The milestone helper

**Files:**
- Create: `tools/milestone/milestone.py`, `tools/milestone/__init__.py` (empty), `tools/milestone/test_milestone.py`
- Modify: `.github/workflows/ci.yml` (the `Tool tests` step: add `tools/milestone/test_milestone.py`),
  `docs/agents/setup.md` (its copy of that step), `tools/README.md` (one entry)

**Interfaces:**
- Produces (used by Tasks 2, 3, 6 and every milestone): the command line
  - `milestone.py start NAME` — makes local branch `milestone/NAME` from `master` and the gate worktree
    `<root>/NAME/_gate` on it; prints the gate path. Refuses (exit 2) if the branch or folder exists.
  - `milestone.py add NAME UNIT [--floor-gb N]` — makes branch `m/NAME/UNIT` from `milestone/NAME` and worktree
    `<root>/NAME/UNIT`; prints its path. Refuses (exit 2) when free space on the root's filesystem is below the floor
    (default `FLOOR_GB`), before creating anything.
  - `milestone.py rm NAME UNIT [--delete-branch]` — removes worktree `<root>/NAME/UNIT` with its ignored build folder
    `target/`; `--delete-branch` then runs `git branch -d m/NAME/UNIT` (merged branches only).
  - `milestone.py end NAME` — removes the gate worktree `<root>/NAME/_gate` (the same checks as `rm`) when no unit
    worktree of NAME is left; exit 2 otherwise. Never deletes a branch.
  - `milestone.py ls [NAME]` — one line per worktree under the root: `NAME UNIT BRANCH SIZE_GB PATH` (`missing` for
    SIZE_GB when the folder is gone), then `free GB`.
  - `milestone.py gate NAME UNIT` — in the gate worktree: refuses if it is not clean; `git merge --no-ff --no-edit
    m/NAME/UNIT`; on conflict `git merge --abort`, prints the conflicting files, exit 3; on success prints the new
    HEAD. Runs no tests and never pushes.
  - Python: `root() -> Path` (`$TTYAK_WORKTREES` if set, else `<parent of the repository>/<repository folder
    name>-worktrees`), `check_name(s: str) -> str` (raises `Refused`), `FLOOR_GB: int` (40 until Task 2 sets it),
    `class Refused(Exception)`, `main(argv: list[str]) -> int`.
  - Exit codes: 0 done, 2 refused (bad name, exists, disk floor, not our worktree, dirty gate), 3 merge conflict.

Worktrees live outside the repository (a sibling folder), so no `.gitignore` change is needed and the repository's
own checks (`check-name.sh .`, link checks, indexers) never walk them. Each worktree's Rust build folder is its own
`target/`, cargo's default.

- [ ] **Step 1: Write the failing tests** in `tools/milestone/test_milestone.py`, each in a temp repository with a
  `master` commit and `TTYAK_WORKTREES` pointing into the temp folder:
  - `test_start_makes_branch_and_gate` — `start trial` → branch `milestone/trial` at `master`'s commit, gate folder
    exists and is on that branch; a second `start trial` → exit 2.
  - `test_add_makes_unit_from_milestone_tip` — after a commit on `milestone/trial`, `add trial fix-1` → branch
    `m/trial/fix-1` at the milestone tip; the printed path equals `<root>/trial/fix-1`.
  - `test_add_refuses_below_floor` — `--floor-gb` above the filesystem's free space → exit 2, no branch, no folder.
  - `test_bad_names_refused` — for `a/b`, `..`, `a b`, `-x`, `Fix`, `` (empty), a 42-character name: exit 2 and no
    git command run (patch `subprocess.run` and assert it was not called).
  - `test_rm_removes_worktree_and_target` — a unit with an ignored `target/debug/x` file → `rm` exits 0, the folder
    is gone, `git worktree list` no longer shows it; `--delete-branch` on a merged branch deletes it, on an unmerged
    one exits non-zero and keeps it.
  - `test_rm_refuses_foreign_paths` — `rm` refused (exit 2) and nothing deleted when `<root>/trial/fix-1` is: a plain
    folder (not a worktree), a symlink to another folder, a worktree of a second temp repository; `rm trial _gate`
    refused too.
  - `test_rm_prunes_missing_folder` — the folder removed by the test first → `rm` exits 0 and git's record is gone.
  - `test_ls_lists_sizes_and_missing` — two units, one folder removed → its line has `missing`; last line starts
    with `free `.
  - `test_gate_merges_no_ff` — a commit on the unit → `gate` exits 0, the milestone tip is a merge commit with two
    parents.
  - `test_gate_conflict_aborts` — the unit and the milestone change the same line → exit 3, stdout names the file,
    the gate worktree is clean and its HEAD is the old milestone tip.
  - `test_gate_refuses_dirty` — an untracked or changed file in the gate → exit 2, no merge.
  - `test_end_removes_gate_only_when_units_gone` — with a unit left → exit 2, gate kept; after `rm` of the unit →
    exit 0, gate folder gone, branch `milestone/trial` still there.

- [ ] **Step 2: Run them and see them fail**

  Run: `python3 -W error -m unittest tools/milestone/test_milestone.py`
  Expected: errors, `ModuleNotFoundError` / no `milestone` module.

- [ ] **Step 3: Implement `tools/milestone/milestone.py`**

  Names: `^[a-z0-9][a-z0-9.-]{0,40}$` and not containing `..`; `_gate` is reserved and accepted only internally.
  The repository is found with `git rev-parse --show-toplevel` from the script's own folder (`--git-common-dir` to
  get the main repository when run from a worktree). `rm` checks, in order: the expected path is not a symlink and
  resolves under `root()`; it appears in `git worktree list --porcelain` of this repository; then `git worktree remove
  --force <path>` (the force only covers ignored/untracked build output; refuse first if `git -C <path> status
  --porcelain --untracked-files=no` shows changes to tracked files), then `shutil.rmtree` of the same checked path if
  the ignored `target/` survived. A missing folder → `git worktree prune`. Free space: `shutil.disk_usage(root())`.
  Sizes: walk the folder without following symlinks. A docstring at the top in the style of
  `tools/accounts/accounts.py` says what it does and what it never does (push, delete remote branches, run tests).

- [ ] **Step 4: Run them and see them pass**

  Run: `python3 -W error -m unittest tools/milestone/test_milestone.py`
  Expected: `OK`, no warnings.

- [ ] **Step 5: Wire it in** — add `tools/milestone/test_milestone.py` to CI's `Tool tests` line and to its copy in
  `docs/agents/setup.md`; one entry in `tools/README.md` (what each subcommand does, the root, the exit codes). Run
  the whole `Tool tests` line as CI does; `sh tools/check-name.sh .`; `python3 tools/wiki/check_links.py`.

- [ ] **Step 6: Commit**

  ```bash
  git add tools/milestone/__init__.py tools/milestone/milestone.py tools/milestone/test_milestone.py \
      .github/workflows/ci.yml docs/agents/setup.md tools/README.md
  git commit -m "tools: milestone helper — milestone branch, unit worktrees, checked removal, gate merge (#29)"
  ```

  The controller pushes and watches CI.

### Task 2: Measure the machine (controller)

No code. The controller runs it itself (it needs the real machine, nothing else may build meanwhile).

**Interfaces:**
- Consumes: Task 1's `start`, `add`, `ls`, `rm`.
- Produces: `CAP` (implementers at once) and `FLOOR_GB` (the disk floor), with the numbers behind them, for Task 3
  and the helper's default.

- [ ] **Step 1:** `milestone.py start measure`, then `add measure m1` … `add measure m4` from the current `master`.
- [ ] **Step 2: Cold builds.** For N = 1, 3, 4: `cargo test --no-run` in N fresh worktrees at once (each under
  `/usr/bin/time -v`, a background `free -m` sampler every 5 s into the scratch folder). Record per N: wall time of
  the slowest, peak used RAM, the size of each `target/`. Remove the build folders between rounds with `rm` and a new
  `add` (no hand `rm -rf`).
- [ ] **Step 3: Warm builds.** In 3 built worktrees, touch one file in `crates/ttyak/src/` and run the whole
  `cargo test` in all 3 at once. Record wall time and peak RAM.
- [ ] **Step 4: Decide.** `CAP` = the largest N whose slowest cold build is within twice the single build's time and
  whose peak RAM leaves at least 16 GB available (room for two instances). `FLOOR_GB` = twice one worktree's
  `target/` after `cargo test` plus 20 GB, rounded up to 10. Write both, with the numbers, into the ledger.
- [ ] **Step 5:** `rm measure m1` … `rm measure m4`, `end measure`; then the never-pushed local branches by literal
  name: `git branch -D m/measure/m1 m/measure/m2 m/measure/m3 m/measure/m4 milestone/measure`. `milestone.py ls`
  shows nothing; free space is back to its start value within a few GB.
- [ ] **Step 6:** Set `FLOOR_GB` in `milestone.py` to the measured value (a one-line change with its test still
  green), commit `tools/milestone/milestone.py` with the numbers in the message, push, watch CI.

### Task 3: The workflow docs

**Files:**
- Modify: `docs/agents/workflow.md` (a new section "Running a milestone" after "Executing a plan with subagents"),
  `AGENTS.md` (the workflow summary: one line pointing to it), `docs/superpowers/ledgers/README.md` (the milestone
  ledger's extra lines)
- Create: `docs/agents/milestone-report.md` (the report template)

**Interfaces:**
- Consumes: Task 1's command line; Task 2's `CAP` and `FLOOR_GB`.
- Produces: the section anchor `workflow.md#running-a-milestone`, linked from AGENTS.md and from Task 4.

- [ ] **Step 1: The section in `workflow.md`**, in the spec's order and words, short: what a milestone and a unit
  are; the start sitting (questions all at once, the batching and why, `size:l` out or designed first, plan and
  ledger committed on `master` before the first dispatch); a big-feature milestone; branches and worktrees (the
  helper's commands, the gate worktree, `master` for the ledger, urgent fixes); how many at once (`CAP`, `FLOOR_GB`,
  with the measured numbers and date); a unit from brief to merge (the six steps of the spec, the gate exactly as the
  spec's step 5); questions mid-milestone; permission refusals (the spec's section); the end (final review, one fix
  wave, checks, the report, "merge" and leave to delete branches); judging the trial. Each brief carries the plan's
  Global Constraints and the live-check rules of Task 4.
- [ ] **Step 2: AGENTS.md** — in "The workflow in short", one line under step 3 or 6: a milestone runs with
  parallel implementers as in [workflow.md](docs/agents/workflow.md#running-a-milestone).
- [ ] **Step 3: The ledger README** — a subsection "A milestone's ledger": per unit its branch, pull request,
  dispatch, approval and merge times, review rounds, gate results; the batching and why; the start sitting's answers;
  every refusal and how it was settled.
- [ ] **Step 4: `docs/agents/milestone-report.md`** — the seven parts of the spec's report in order, each with one
  line of guidance and a placeholder, public wording, and the closing line asking for "merge" and leave to delete the
  milestone's branches on GitHub.
- [ ] **Step 5: Check** — `python3 tools/wiki/check_links.py`; `sh tools/check-name.sh .`; the privacy check on a
  folder of copies of the four files; the `git grep` for the claim tool's name finds nothing.
- [ ] **Step 6: Commit**

  ```bash
  git add docs/agents/workflow.md AGENTS.md docs/superpowers/ledgers/README.md docs/agents/milestone-report.md
  git commit -m "docs: running a milestone with parallel implementers (#29)"
  ```

### Task 4: Live checks by implementers

**Files:**
- Modify: `docs/agents/live-testing.md` (a short section "Live checks in a milestone" before "Before every live
  check"; the Mac and shared-Windows-VM recipes), `docs/agents/local-template.md` (the work-folder lines)

**Interfaces:**
- Consumes: Task 3's anchor.
- Produces: the anchor `live-testing.md#live-checks-in-a-milestone`, quoted in every milestone brief.

- [ ] **Step 1: The section** — the spec's three places and their rules, word for word in substance: instances
  (throwaway, made and removed by name); shared boxes (someone else's machine, under a claim, recipes only: "a recipe
  is a step-by-step command list on this page; on a shared box an implementer runs only those; anything else goes back
  to the controller"; never install, change settings, start, stop, restart or revert); the desktop (instances only
  for implementers, `ctl` in a fake home without a window allowed, the hidden test workspace is the controller's).
  Claims queue; the evidence rule (what to save, where — the unit's scratch folder — and that the reviewer checks it
  before LIVE).
- [ ] **Step 2: Per-unit folders** — every recipe that uses `~/ttyak-test` on the Mac (build, `--mount`, cleanup)
  and `%USERPROFILE%\ttyak-test` on the shared Windows VM takes a `<unit>` subfolder (`~/ttyak-test/<unit>/src`,
  `~/ttyak-test/<unit>/build`); outside a milestone `<unit>` is the agent's task name. The cleanup recipe deletes the
  two literal paths of its unit and checks each. A short paragraph: folders of a dead agent are removed by the
  controller at the end of the milestone, under its own claim, by the same recipe, only for this milestone's units.
  Same change to the matching lines of `local-template.md`.
- [ ] **Step 3: Check** — as Task 3 Step 5, for these two files.
- [ ] **Step 4: Commit**

  ```bash
  git add docs/agents/live-testing.md docs/agents/local-template.md
  git commit -m "docs: live checks in a milestone — three places, recipes only, a folder per unit (#29)"
  ```

### Task 5: The allowlist (controller, with the maintainer)

**Files:**
- Modify: `.gitignore` (add `/.claude/settings.local.json`), `docs/agents/workflow.md` (a short "Allowlist"
  paragraph in the milestone section: what is allowed, as patterns with `<tool>` in place of the private path)
- Local only (never committed): `.claude/settings.local.json`

- [ ] **Step 1: Propose.** The controller lists the commands the workflow runs (from Tasks 1–4 and the existing
  recipes): `cargo build`/`test` in worktrees, `git` subcommands used by the helper and the gate, the helper itself,
  `tools/accounts/accounts.py` subcommands, the claim tool's recipe subcommands, `gh pr create|view|checks|merge`,
  `gh run watch|view`. Each as the narrowest Claude Code permission rule
  that matches (e.g. `Bash(cargo test:*)`), none that allows `rm -rf`, `git push --force`, `gh api -X DELETE` or
  settings edits. The proposal goes to the maintainer as one message.
- [ ] **Step 2: The maintainer approves** (or edits) the list. Their words go into the ledger.
- [ ] **Step 3:** The controller merges the approved rules into `.claude/settings.local.json` (keeping its two
  existing rules), adds the `.gitignore` line, writes the public paragraph with `<tool>`. `git check-ignore -v
  .claude/settings.local.json` names the repository's `.gitignore`.
- [ ] **Step 4: Commit** `.gitignore docs/agents/workflow.md` ("docs: the milestone allowlist, approved by the
  maintainer (#29)"); push; `.gitignore` is outside `paths-ignore`, so watch CI.

### Task 6: The dry run (controller and one implementer)

**Interfaces:**
- Consumes: everything above.
- Produces: a ledger entry listing every step exercised and every refusal hit, each fixed or settled with the
  maintainer.

- [ ] **Step 1:** `milestone.py start dryrun`; push `milestone/dryrun`. `add dryrun probe`.
- [ ] **Step 2:** Dispatch one background implementer with a real brief (Task 3's format) for a throwaway change in
  the `probe` worktree: a one-line comment in a test file, the whole `cargo test`, a commit; then a live check by the
  recipes: a Linux instance with account A claimed and installed, `ttyak ctl status` in it, removed; the Mac build
  recipe in `~/ttyak-test/probe/` under a box claim and its cleanup; evidence saved as Task 4 says. In auto mode,
  as a milestone would run.
- [ ] **Step 3:** The controller: privacy check, push `m/dryrun/probe`, `gh pr create --base milestone/dryrun`
  (title "dry run of the milestone workflow — not for merge"); a reviewer subagent reviews it; `milestone.py gate
  dryrun probe`; the whole `cargo test` in the gate worktree; a plain push of `milestone/dryrun`; GitHub shows the
  pull request merged.
- [ ] **Step 4:** Every refusal met in Steps 1–3: stop that step, check for partial writes, take it to the maintainer
  at once with the exact command and a proposed allowlist or recipe change; apply what they approve (Task 5's file,
  Task 4's recipes) and repeat the step until it passes. Record each in the ledger.
- [ ] **Step 5: Clean up.** `milestone.py rm dryrun probe --delete-branch`; `milestone.py end dryrun`;
  `git branch -D milestone/dryrun` (literal); `milestone.py ls` empty; the Mac folder gone (checked); instance and claims
  released. Ask the maintainer for leave to delete `milestone/dryrun` and `m/dryrun/probe` on GitHub; delete them
  only after their word.

### Task 7: Final review, PROGRESS, close

- [ ] **Step 1:** A fresh final reviewer reads the whole plan's diff (`<start>..HEAD`) against the spec and the
  Global Constraints; one fix wave for every real finding, each fix reviewed; nits dropped with a reason in the
  ledger.
- [ ] **Step 2:** Rewrite `docs/PROGRESS.md` ("In flight": #29 built, next the trial milestone of 3–5 small issues
  the maintainer picks; the workflow's section linked). Close the ledger ("Plan complete").
- [ ] **Step 3:** Commit `docs/PROGRESS.md` and the ledger with `Fixes #29`; push. Remove reviewers' scratch copies
  and target directories by literal path.
- [ ] **Step 4:** Ask the maintainer to pick the trial milestone's 3–5 issues; that milestone is run by the new
  workflow, with its own plan and ledger, and ends with the retro the spec asks for.
