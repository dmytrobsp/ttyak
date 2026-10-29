# How work is done on ttyak

The full workflow behind [AGENTS.md](../../AGENTS.md): finding where things stand, the queue, planning, executing a
plan with subagents, making a change, verifying it, committing and pushing, docs, releasing. Live checks have their
own page: [live-testing.md](live-testing.md). A new machine is set up with [setup.md](setup.md).

The project uses the [superpowers](https://github.com/obra/superpowers) skills for Claude Code (brainstorming, writing
plans, subagent-driven development, test-driven development, verification before completion). An agent without them
follows the same steps by hand; this page says what they are.

## Where things stand

Read, in this order:

1. [docs/PROGRESS.md](../PROGRESS.md) — the latest release, what is in flight (milestone, active plan and its ledger),
   outside waits, the next topics. It is a short current-state page: it is rewritten when things change, never
   appended to.
2. The open issues of the current milestone: `gh issue list -R dmytrobsp/ttyak -m <milestone>`.
3. The newest ledger in [docs/superpowers/ledgers/](../superpowers/ledgers/README.md) that still has unfinished tasks,
   with its design and plan. Plans before 2026-10-27 kept their ledgers git-ignored: none is committed for them, and
   none in the folder means no plan is in flight.
4. `git log --oneline -20`, `git status`, and the latest runs: `gh run list -R dmytrobsp/ttyak -L 5`.

Never re-run a task that the ledger or `git log` shows done. History (every step since the start, with its
LIVE/UNIT/NOT record and commit) is in `docs/history/`.

## The queue: GitHub Issues

Everything to do is an [issue](https://github.com/dmytrobsp/ttyak/issues); nothing is queued in a file.

- **Milestones**: one per coming release (`0.13.0`, …) plus `later`. When a release milestone is open, take its
  issues. When none is open, do not pick from `later` on your own: propose a milestone to the maintainer (a few
  related issues, and why they go together); they decide. `needs-user` issues wait for the maintainer. A milestone is
  created when the maintainer agrees to it; it is closed when its issues are closed and its release is out (in the
  repository: `gh api -X PATCH 'repos/{owner}/{repo}/milestones/<number>' -f state=closed`).
- **Labels**: the kind (`bug`, `minor`, `feature`, `known-issue`); `area:*` (notifications, tui, linux, macos, windows,
  install, release, ci, huddles, docs); `size:s` / `size:m` / `size:l` (`l` needs a plan first); `process` (how the work
  gets done); `needs-user` (a decision, an account or a live check with the maintainer); `triage` (a user report not
  looked at yet — the issue forms set it; blank issues are off). `-label:triage` is the curated queue. Only
  collaborators can set labels and milestones.
- **Commits that fix an issue** say `Fixes #N` in the message.
- **New items** — a review finding outside the plan's scope or waiting for the maintainer's decision, a bug found on
  the way, a wish the maintainer adds mid-task — become issues with public wording (in-scope review findings are
  fixed, not filed: [Reviews](#executing-a-plan-with-subagents)). If the current step is still open, a small addition
  is folded into it instead. Filing issues publishes them: for a batch, show the maintainer the drafts first.
- **Security findings** are not issues: open a private security advisory (the Security tab → Advisories → New draft
  security advisory) and tell the maintainer.
- **Registry submissions** (winget pull requests, Chocolatey moderation) are tracked in issue #40, updated at each
  release. Older winget pull requests stay open (the maintainer's call); they are not closed as superseded.

## Planning

A focused fix (`size:s`, `size:m`) goes straight to a step. Anything bigger, or anything the maintainer wants to
shape, is planned first:

1. **Brainstorm** with the maintainer: one question at a time; for a small visual tweak propose one concrete option in
   prose rather than a multiple-choice list. Think through how the user's click or key leads to the action before
   ruling on selection or focus behaviour.
2. **Design** in `docs/superpowers/specs/YYYY-MM-DD-<topic>-design.md`: intent (the maintainer's words), goals,
   non-goals, anti-goals, constraints, verification, and what is decided, guessed and open.
3. **A builder's read**: a fresh agent that would build it reads the design and lists questions and gaps; the answers
   go into the design ("Answers for the builder").
4. **The maintainer approves** the design.
5. **Plan** in `docs/superpowers/plans/YYYY-MM-DD-<topic>.md`: Global Constraints (the rules every task carries,
   copied from AGENTS.md and live-testing.md as they apply), Review Focus, and tasks small enough for one commit each,
   with files, interfaces, the test to write first and the checks.
6. **Execute** it (next section).

**Naming** (a project, package or command name): vet every candidate with a full web search before showing it.
Registries and GitHub alone once missed chat apps of the same name; ordinary chat words are all taken, coined words
fare better.

**Handoff before big work.** At the start of a large new piece of work the maintainer may ask for a session handoff
first: write the design, update [docs/PROGRESS.md](../PROGRESS.md), commit and push, and give a self-contained
continuation prompt (where things are, how to resume, the hard rules, the test setup). Do not start implementing in
that session.

## Executing a plan with subagents

A controller agent runs the plan; it dispatches one implementer per task and reviewers after each.

- **Ledger.** The plan's decisions are committed in
  [`docs/superpowers/ledgers/<plan>.md`](../superpowers/ledgers/README.md): its head (plan, design, start commit, the
  task list) as soon as the plan starts, so a plan in flight always has one, then at every task boundary: each task's
  status and commit, what was checked LIVE/UNIT/NOT, the controller's rulings, findings dropped as nits (with why) and
  items filed as issues (out of scope or the maintainer's decision, with their numbers).
  Bulky scratch — the rules file, task briefs, review packages, reports — stays git-ignored in
  `.superpowers/sdd/<plan>/` (never `git add -f` it). After a crash or a lost session, the ledger and `git log` are how
  work resumes; an implementer's transcript survives and can be resumed.
- **Briefs.** Each implementer gets a self-contained brief: the plan's Global Constraints (the shell rules always),
  its task, the files to read, what it must not do. Write the brief to its file before dispatching, never in the same
  chained command.
- **Implementers** never push, tag, publish the wiki, file or close issues, post outside the test workspace, edit files
  outside the repository, or dispatch subagents. They never quote real emails, names or ids in anything they write.
  Each one's live checks use its own window class and kitty socket, never the controller's or one the maintainer is
  typing into. In a milestone two things differ: the implementer's own worktree counts as the repository (nothing
  outside it and the plan's scratch folder), and its live checks run in instances only
  ([live-testing.md](live-testing.md#live-checks-in-a-milestone)).
- **Reviews.** A review per task (the diff against the plan and the rules), fix rounds sent to the same implementer
  (resume it, it keeps its context), then a final whole-branch review. Reviewers find real things in most rounds; one
  implementer per batch of issues keeps that cheap.
- **Review findings are fixed, not filed.** Every real finding that is in scope is fixed before the plan ends — the
  minors too, in the task's fix round or one fix wave after the final review, each fix reviewed in turn. The
  maintainer (2026-10-27): "reviews have found issues that you've decided to just file away instead of fixing?" Only
  three kinds of finding leave the plan unfixed: true nits (cosmetic or negligible), dropped with a one-line reason in
  the ledger; work outside the plan's scope; and anything that needs the maintainer's decision. Only the last two
  become issues (public wording). The final report says plainly what was fixed, dropped and filed.
- **Machines.** Before a session where the maintainer works on a test machine, stop every subagent using it.
- **Cleanup.** Reviewers' scratch copies and target directories fill the disk quickly (tens of GB): remove them, by
  literal path, when the plan ends.

## Running a milestone

A milestone of small fixes and improvements runs faster with several implementers at once. One controller session
drives it, and the maintainer answers questions in one sitting at the start and looks again at the end. The design is
[2026-10-29-milestone-workflow-design.md](../superpowers/specs/2026-10-29-milestone-workflow-design.md). A lone fix,
or a plan outside a milestone, runs as in the sections above.

- **A milestone** is some known bugs and some improvements. **A unit** is one issue, or a small group of related
  issues (the controller groups them): its own branch, its own worktree with its own build folder, its own
  implementer and its own pull request into the milestone branch. A big new feature is a milestone of its own with
  nothing else in it.
- **The controller** is the only agent that talks to the maintainer, pushes, opens pull requests or files issues. It
  plans the units and their order, writes the briefs, dispatches implementers and reviewers, runs the merge gate, the
  final review, the live smoke check and the report. It keeps its own context lean: short reports, details in scratch
  files. Implementers and reviewers keep the limits of
  [Executing a plan with subagents](#executing-a-plan-with-subagents).

### The start sitting

1. Agree on the milestone's issues, as in [The queue](#the-queue-github-issues): proposed by the controller when none
   is open, decided by the maintainer.
2. The controller reads every issue, its history and the code it touches. It brings, in one message or one short
   back-and-forth, every question the milestone needs answered (behaviour, naming, `needs-user` items) and its
   batching: which units run at once because they touch separate code, which wait for another to merge first, and
   why. Questions asked later cost the maintainer's focus, so ask them all here.
3. A `size:l` issue in a mixed milestone is raised in the sitting: it moves to a milestone of its own, or it gets its
   design first. A mixed milestone holds `size:s` and `size:m` units.
4. The answers and the batching go into the milestone's plan (`docs/superpowers/plans/<date>-milestone-<name>.md`)
   and its ledger, committed on `master` before the first dispatch. Then the controller runs without the
   maintainer.

**A big-feature milestone** goes through [Planning](#planning) in full first (brainstorm, written design, a builder's
read, the maintainer's approval, a plan). Its plan's tasks are the units, run at once where their dependencies allow,
with the same pull requests, reviews, gate and report.

### Branches and worktrees

The git part is done by `tools/milestone/milestone.py` ([tools/README.md](../../tools/README.md)); exit 0 is done, 2
refused, 3 a merge conflict.

- **`python3 tools/milestone/milestone.py start NAME`** makes the milestone branch `milestone/NAME` from `master` and
  the gate worktree `<root>/NAME/_gate`. The controller pushes the branch.
- **`add NAME UNIT`** makes the unit branch `m/NAME/UNIT` from the milestone tip, in the worktree `<root>/NAME/UNIT`
  with its own `target/`. It refuses below the free-disk floor. `rm NAME UNIT` removes the worktree and its `target/`
  once the unit has merged; it refuses any uncommitted work and keeps the branch (`--delete-branch` deletes it only
  when it is merged into `milestone/NAME`, and deleting branches on GitHub is the maintainer's word). A unit name is
  used once per milestone. `end NAME` removes the gate when no unit is left; `ls [NAME]` shows the worktrees with
  their sizes and the free space.
- **The root** is `$TTYAK_WORKTREES` (an absolute path outside the repository) or, by default, the folder
  `<repository folder>-worktrees` next to the repository. It is not the scratch folder, which a reboot wipes.
- **The main checkout stays on `master`**, where the ledger is committed (docs-only commits, no CI). Merging into
  the milestone branch happens in the gate worktree only.
- **An urgent fix during a milestone** goes to `master` as usual; the controller merges `master` into the milestone
  branch at the next gate.

### How many at once

- **Three implementers at once** (`CAP`), and the controller does not dispatch while free space is below 40 GB
  (`FLOOR_GB`, the helper's default).
- **Measured on 2026-10-29** on the maintainer's desktop (32 threads, 60 GB RAM): `cargo test --no-run` from fresh
  worktrees took 37 s with 1 at once, 65 s with 3 and 78 s with 4; the lowest available RAM was 36.4, 29.6 and 25.3
  GB. Each worktree's `target/` is 4.6 GiB, 4.9 GiB after a full `cargo test`. Warm, the whole `cargo test` took 13 s
  with 3 at once. Three leaves RAM and disk to spare; a fourth buys little. Another machine measures again before it
  changes the numbers.
- **Disk**: a worktree goes as soon as its unit merges. The Windows cross-build (`target/x86_64-pc-windows-gnu`, over
  10 GB) is not made in every unit. CI's `windows-cross` job runs on every unit pull request (CI fires on a pull
  request with no branch filter), so the per-unit Windows compile happens there; the local cross-build and
  `cargo test --no-run` for Windows run only at the milestone's end.
- **Batching**: units in one wave touch separate files and modules as far as the controller can tell from reading the
  code; units that share files run one after the other; a unit that needs another one's change waits for its merge.
  A conflict at the gate means the batching missed something: it is noted in the ledger.

### A unit, from brief to merge

1. **Brief.** The controller writes it to the plan's scratch folder (`.superpowers/sdd/<plan>/`), never in the same
   chained command as the dispatch, with the Write tool (not a heredoc: [Permission
   refusals](#permission-refusals)). It holds the plan's Global Constraints (the shell rules always), the issue and the
   maintainer's answers for it, the files to read, the worktree, branch and build folder, the absolute path of the
   unit's evidence folder (the main checkout's `.superpowers/sdd/<plan>/<unit>/`), the live-check rules of
   [Live checks in a milestone](live-testing.md#live-checks-in-a-milestone), and what not to do. Then it dispatches the
   implementer in the background.
2. **Implementation.** TDD with the RED shown, commits on the unit branch, the whole `cargo test` in the worktree,
   live checks where the change needs them, and a short report with the paths of its evidence.
3. **Pull request.** The controller runs the privacy check on what will become public (the diff and the
   description), pushes the unit branch and opens a pull request into the milestone branch, in public wording. CI
   runs on it.
4. **Review.** A fresh reviewer (opus for security- or concurrency-heavy units) reads the diff against the plan and
   the rules and checks the live evidence. Findings go to the same implementer, and the same reviewer, resumed, does a
   scoped re-review after each fix round. Every real finding is fixed; nits are dropped with a reason in the ledger;
   out-of-scope items and the maintainer's decisions become issues. Review text stays in the scratch folder; the pull
   request gets a short public summary (rounds, the kind of things fixed). Security findings never go to a pull
   request: a private advisory.
5. **Merge gate**, one unit at a time, and no pushed branch is ever rewritten (no rebase, no force-push). The
   controller runs `milestone.py gate NAME UNIT`, which merges the unit branch into the milestone tip in the gate
   worktree (`--no-ff`); then it runs the whole `cargo test` on that merge result, and CI on the pull request must be
   green (GitHub tests the pull request merged into its base). If both pass, the controller pushes
   the milestone branch, a plain fast-forward push, and GitHub marks the pull request merged. A merge with conflicts
   (exit 3, the files listed, the merge aborted) goes back to its implementer, who merges the milestone tip into its
   own branch and resolves them; the author resolves its own conflicts. The controller pushes that, and the gate
   starts again. Afterwards the unit's worktree and build folder are removed with `rm`. Units still in flight take the
   new tip only at their own gate. A docs-only unit's pull request gets no CI run (`ci.yml` ignores docs): it passes
   the gate on the whole `cargo test` alone.
6. **The ledger** gets an entry at each step
   ([A milestone's ledger](../superpowers/ledgers/README.md#a-milestones-ledger)).

### Questions mid-milestone

Implementers never ask the maintainer: a question goes back to the controller in the implementer's report. The
controller answers it itself when it can (a ruling in the ledger, with what it costs if wrong) and asks the
maintainer only when the answer would change the implementation significantly. The other units keep running.

### Permission refusals

The maintainer runs sessions in auto mode, and a refusal must not hold a milestone up. So the work is in preventing
them, and the few that remain reach the maintainer at once.

- **Fewer refusals.** A project allowlist of the commands the workflow needs, written as narrowly as works; the
  maintainer approves it before it goes in. Briefs and recipes use plain, separate commands with literal paths (or
  `${VAR:?}`), never a scripted edit chained with a commit and never a computed command name. A call that runs
  the claim tool against a shared box, above all one carrying a recursive delete, holds only that one command, with
  the claim tool's path and the box claim id written out literally: the safety hook reads the quoted remote command
  too and refuses a recursive delete next to variables without `set -euo pipefail` (the recipe blocks keep their
  variables). A file whose text quotes a recursive delete or another guarded command (a report, a brief, a script) is
  written with the Write or Edit tool, never a Bash heredoc: the hook scans heredoc bodies. A dry run before the first
  milestone (one implementer, one throwaway unit, every recipe the workflow uses) finds the refusals that are left,
  and each is fixed in the allowlist or the recipes.
- **A refusal that still happens** stops only the step that hit it, as the hard rule says: never reworded, split or
  retried. The implementer reports it with the command exactly as sent and the refusal text verbatim; the controller
  checks the worktree for partial writes (`git status`), parks that unit and keeps the others running. After a
  refused cleanup on a shared box the implementer stops and keeps its box claim held, so its report gives its box and
  account claim ids; the controller does the whole cleanup under that still-live box claim, renewing it, by the
  recipe, then releases it. It also renews and then releases the implementer's account claims, once the session
  copies on the box are gone.
- **The controller tells the maintainer at once**: the exact command, why it was needed, what waits on it, and the
  allowlist line that would prevent it next time. Refusals that arrive together go in one message. When the
  maintainer has run or allowed it, the parked unit goes on.
- **Every refusal goes into the ledger.** The retro after a milestone turns recurring ones into allowlist or recipe
  changes.

#### Allowlist

The rules live in the git-ignored `.claude/settings.local.json` (they name the claim tool's path); the maintainer
approved them on 2026-10-29 and installs changes to that file themselves: an agent's write to its own settings is
refused as self-modification, so the controller writes the new file to its scratch folder and gives the maintainer the
copy command. In patterns (paths as each machine has them):

- **Allowed without a check**: `cargo build`, `cargo test`; the Python tool tests and the name, link and privacy
  checks; `tools/milestone/milestone.py` and `tools/accounts/accounts.py`; `git` status, diff, log, show, rev-parse,
  add, commit, merge (also with `-C` into a milestone worktree); `git push` of `master`, `m/*` and `milestone/*`;
  `gh pr` create, view, checks, list, diff, comment; `gh run` list, view, watch; `gh issue` view, list;
  `<tool>` status, claim, renew, release, instance, run, push, pull.
- **Always asked**, even in auto mode (each needs the maintainer's word anyway): any force-push (`--force`, `-f`,
  a `+` refspec), deleting by push (`--delete`, `:`), `git -C … push`, `git -C … branch -D`,
  `git -C … reset --hard`, `gh api` DELETE, `gh repo delete`, `gh release`, `git tag`.
- **Not listed**, so auto mode checks them as usual: `rm`, `git branch -D`, `gh pr merge`, `<tool> shell`.
- `<tool> run` lets any command reach a shared box without a check: the box claim and the recipes are the guard
  there ([live-testing.md](live-testing.md#live-checks-in-a-milestone)).

### The end of a milestone

1. All units merged, then a final whole-milestone review (the milestone branch against `master`) and one fix wave,
   its fixes going through the same review and gate. Then the checks: the whole `cargo test`, the Windows
   cross-build and `cargo test --no-run` for Windows, CI on the milestone pull request, and the controller's live
   smoke check of the milestone branch.
2. **The milestone pull request** (milestone branch into `master`) carries the report, made from
   [milestone-report.md](milestone-report.md). The privacy check runs on it before it is posted.
3. On the maintainer's word ("merge") the controller merges with a merge commit, not squashed, so `master` keeps each
   unit's commits and `Fixes #N` closes the issues. The report asks, in the same line, for leave to delete the
   milestone's branches on GitHub; deleting is the maintainer's word, never assumed. The controller removes the
   gate worktree (`end`) and any worktree left, and, under its own claim, the folders a dead agent left on a shared
   box. A release follows [RELEASING.md](../RELEASING.md), on the maintainer's word.

### Judging a trial milestone

- **Time** is the maintainer's impression, not a measurement; the ledger keeps each unit's dispatch, approval and
  merge times and the milestone's start and end as a rough record to talk over.
- **Quality**: the final review's real findings per unit and red runs on the milestone branch, against the final
  reviews in the committed ledgers since 2026-10-27, and what the maintainer finds after the merge.
- **Cost**: usage limits hit, if any, are noted, not a gate.
- **Adjustment**: a short retro in the ledger, and the changes to this page and to live-testing.md it calls for.

## Making a change

- **Small verified steps**: one focused change, verified, committed; then the next. Run autonomously through an agreed
  plan or queue; check in only at a genuine fork. Broad multi-feature steps get stopped and lose more.
- **"Pause at the nearest checkpoint"** means: finish the open step to a commit (tests green, ledger updated), then
  stop and hand off. Never leave a half-made change in the tree.
- **A refused action** (a permission prompt, a safety check stopping a step): stop, check `git status` for partial
  writes, explain what was refused and why it was needed, and give the maintainer the exact command to run or allow.
  Never reword, split or retry it to get around the refusal; the maintainer's go-ahead does not override a safety
  check either.
- **The shell's working directory resets** between commands in agent shells: use absolute paths in every command.
  The scratch folder (often under `/tmp`) is wiped by a reboot: keep nothing there that is needed later — what must
  survive goes into the ledger or a commit.
- **Test first**: write the failing test, run it and show the RED, then make it pass. Tool scripts (`tools/`) too.
- **The whole `cargo test`** before every Rust commit; never commit on red (a scripted chain checks
  `cargo test 2>&1 | grep -q FAILED && exit 1` first). Windows must compile:
  `cargo build --target x86_64-pc-windows-gnu` and `cargo test --no-run --target x86_64-pc-windows-gnu`.
- **No `cargo fmt`**, never `target/release` (AGENTS.md). Unit tests do not mutate global singletons (the theme
  palette, the clock zone, the translation table): they run in parallel; test pure helpers instead.
- **Real data before parsers**: probe an endpoint with `ttyak --profile <test> ctl api …` (in a fake home with a
  claimed account, [live-testing.md](live-testing.md#a-fake-home)) and capture real-time events with `ctl listen`
  before writing a parser; use the captured shapes (keys and types, not private values) in the unit tests. A made-up
  fixture for a reply nobody has seen hid a real bug. What is learned goes into `docs/protocol/`.
- **Validation of outside data** (a feed, a reply) is checked against the live data before pushing: a check that is too
  strict breaks every user at once.
- **Links meant for a phone** (a sign-in link, a link sent by email) are checked with a phone's user agent too, not
  only a desktop browser: slack.com's sign-in page sends phone browsers to the app download page. A throwaway headless
  Chrome with its profile in the scratch folder and `--user-agent=<a phone's>` shows where the link lands.
- **Product conventions** the maintainer set: every new behaviour (sounds, rings, reminders, …) can be changed in F6
  and switched off; durations are in seconds; user-visible words go through the translation table (`tr`); the
  in-app help (F6, `?`) matches the code.
- **Code lessons**: anything that "keeps X in view" runs when X changes, never on every draw (it fights the wheel);
  views (Threads, Activity, Huddles) follow the same selection and scrolling rules as the message panes; anything
  printed outside ratatui redraws on resize; tokens, cookies and join tokens are never logged.
- **Names**: old project names (slacktui, klack) only on lines marked `<!-- old-name-ok -->`; `tools/check-name.sh .`
  enforces it in CI.

## Tests

| What | Command |
|---|---|
| Rust, all crates | `cargo test` |
| Windows compiles | `cargo build --target x86_64-pc-windows-gnu`, `cargo test --no-run --target x86_64-pc-windows-gnu` |
| Tool tests (as CI runs them) | exactly the `Tool tests` step of `.github/workflows/ci.yml`, spelled out in [setup.md](setup.md#2-test-tools) |
| Packaging | `python3 -m unittest discover -s packaging -p 'test_*.py'` |
| Site | `python3 -m unittest discover -s tools/site -p 'test_*.py'` and `node --test "tools/site/*.test.mjs"` |
| Names, links | `sh tools/check-name.sh .`, `python3 tools/wiki/check_links.py` |
| Clean checkout per distro | `tools/container-build.sh debian\|ubuntu\|fedora\|arch` (podman) |
| Message-pane cost | `cargo test --release --target-dir <scratch> -p ttyak build_lines_cost -- --ignored --nocapture` |

A new tool test is added to CI's `Tool tests` step in the same commit (and to its copy in setup.md). Pages that stay
current (setup, PROGRESS) quote no test counts: they rot. A ledger or release record that quotes one takes it from one
crate's `test result` line (`cargo test -p ttyak 2>&1 | grep 'test result'`), not a workspace-wide grep.

## Verification and honesty

- Every step says what was checked how: **LIVE** (exercised in the real app against the test workspace, and checked
  on screen or by reading back as the other account), **UNIT** (`cargo test` only), **NOT** (written, not exercised).
  Write down what was not tried; never round it up.
- Evidence before claims: run the command, read the output, then say it passed. Quote real numbers (CI timings from
  the run, not from memory).
- README, wiki pages and release notes never state something untried as tried.

## Commits and pushes

- **One step, one commit** (a second for its docs is fine). `git add` and `git commit` with explicit paths; while
  someone else works in the tree, commit with a pathspec (`git commit -- <paths>`).
- **Never chain a scripted edit with `git commit`** in one command: a failed edit once still committed and pushed the
  old file. Edit, check, then commit.
- **The trailer**: `Co-Authored-By: Claude <model> <noreply@anthropic.com>`, naming the model that did the work.
- **Identity**: the repository commits with its own identity, set repo-locally (`git config user.name …`,
  `git config user.email …`; copy them from a maintainer commit: `git log -1 --format='%an <%ae>'`). Every fresh clone
  and new repository (wiki, tap, bucket, a scratch clone) gets them before its first commit. GitHub credentials are
  repo-local too: `git config credential.https://github.com.helper '!gh auth git-credential'` with `gh` logged in as
  the maintainer's account.
- **Pushes**: routine pushes of `master` go ahead without asking. Tags, releases, force-pushes, history rewrites,
  deleting things on GitHub and repository settings need the maintainer's word.
- **CI**: `.github/workflows/ci.yml` ignores `**.md`, `docs/**`, `site/**`, `tools/site/**` and the issue templates, so
  docs-only pushes run nothing — keep it so, and give any new workflow that runs on push the same `paths-ignore`
  (unless it reads docs). Any push touching a path outside that list runs CI — `crates/`, `tools/` (outside
  `tools/site/`), `packaging/`, `vendor/`, the workflows, but also `Cargo.toml`, `Cargo.lock`, `.gitignore`, the
  licences: **watch it after every such push, before the next step** — one waiter per run
  (`gh run watch <id> --exit-status`, with the run id checked non-empty and a time cap). A push touching `site/**`,
  `tools/site/**` or the paths in `pages.yml` deploys the site: watch the Pages run.
- **Timeouts**: every new or changed job, and every network step in it, gets a `timeout-minutes` (a hung apt step
  once sat for 80 minutes under the 6-hour default). `ci.yml`, `release.yml` and `install-check.yml` have them;
  `pages.yml` does not yet.
- **A history rewrite** (only with the maintainer's word): a backup branch and bundle first; rewrite only the range
  needed; remap commit hashes quoted in docs (pair old and new commits by position and subject) and check each is an
  ancestor of `master`; push with `--force-with-lease=master:<old sha>`; drop `.git/refs/original`.

## Docs

- **README.md** is a short front door; `tools/wiki/test_readme.py` pins its outline and the anchors shipped builds
  link to. It links only wiki pages that exist on the published wiki.
- **The wiki** (the user manual) is edited in `docs/wiki/` (`Page-Name.md`, `[text](Page-Name)` links), checked with
  `python3 tools/wiki/check_links.py`, and published at each release with `tools/wiki/publish.sh` (`--push` only with
  the maintainer's word). It describes the latest release; an edit in GitHub's wiki editor is lost at the next publish.
  A task that changes behaviour updates its wiki page in the same task.
- **The site** (`site/`, built by `tools/site/fill.py`) keeps the wording the maintainer approved. Before any site or
  public-page commit run `python3 tools/site/check_private.py <dir>` (exit 0 = clean; it never prints the strings).
  Screenshots: the workspace name blurred, made-up names and made-up conversations, built with `tools/site/shots.py`
  (never a capture of the real screen); the maintainer sees them before they are committed.
- **The privacy check on new public docs**: copy the new or moved files into a scratch folder and run
  `check_private.py` on it. It reads a private list (`~/.config/ttyak-site/forbidden.txt` by default; the local file
  names the private lists); where no list exists, skip it and say so.
- **[docs/PROGRESS.md](../PROGRESS.md)** stays short and current; history goes to [docs/history/](../history/README.md)
  and the ledgers. Old specs and plans keep their historical text.

## Releasing

[RELEASING.md](../RELEASING.md) is the checklist (milestone → bump → dry run → notes → tag → channel checks →
wiki → close out). Each tag only with the maintainer's word. From past release days:

- Only release blockers hold a tag; polish goes to the next milestone. Offer "blockers only, then tag" up front.
- One waiter per run, with a cap; notarization never blocks the release.
- Quote real timings from the runs.
- Update issue #40 (registries) at each release.

## Private specifics

What is specific to one machine or person lives in `AGENTS.local.md` (git-ignored), made from
[local-template.md](local-template.md): the test profiles and workspace notes, the test machines and how to reach them,
the paths of the private lists (the privacy check's forbidden strings, the made-up names for screenshots, the history
redaction map), anything else live work needs. Never copy its contents into a tracked file.
