# How work is done on ttyak

The full workflow behind [AGENTS.md](../../AGENTS.md): finding where things stand, the queue, planning, executing a
plan with subagents, making a change, verifying it, committing and pushing, docs, releasing. Live checks have their
own page: [live-testing.md](live-testing.md). A new machine is set up with [setup.md](setup.md).

The project uses the [superpowers](https://github.com/obra/superpowers) skills for Claude Code (brainstorming, writing
plans, subagent-driven development, test-driven development, verification before completion). An agent without them
follows the same steps by hand; this page says what they are.

## Where things stand

Read, in this order:

1. `docs/PROGRESS.md` — the latest release, what is in flight (milestone, active plan and its ledger), outside waits,
   the next topics. It is a short current-state page: it is rewritten when things change, never appended to.
2. The open issues of the current milestone: `gh issue list -R dmytrobsp/ttyak -m <milestone>`.
3. The newest ledger in `docs/superpowers/ledgers/` that still has unfinished tasks, with its design and plan.
4. `git log --oneline -20`, `git status`, and the latest runs: `gh run list -R dmytrobsp/ttyak -L 5`.

Never re-run a task that the ledger or `git log` shows done. History (every step since the start, with its
LIVE/UNIT/NOT record and commit) is in `docs/history/`.

## The queue: GitHub Issues

Everything to do is an [issue](https://github.com/dmytrobsp/ttyak/issues); nothing is queued in a file.

- **Milestones**: one per coming release (`0.13.0`, …) plus `later`. Pick from the current milestone first, then
  `later`. A milestone is created when its first issue is picked; it is closed when its issues are closed and its
  release is out (in the repository: `gh api -X PATCH 'repos/{owner}/{repo}/milestones/<number>' -f state=closed`).
- **Labels**: the kind (`bug`, `minor`, `feature`, `known-issue`); `area:*` (notifications, tui, linux, macos, windows,
  install, release, ci, huddles, docs); `size:s` / `size:m` / `size:l` (`l` needs a plan first); `process` (how the work
  gets done); `needs-user` (a decision, an account or a live check with the maintainer); `triage` (a user report not
  looked at yet — the issue forms set it; blank issues are off). `-label:triage` is the curated queue. Only
  collaborators can set labels and milestones.
- **Commits that fix an issue** say `Fixes #N` in the message.
- **New items** — a review's deferred minors, a bug found on the way, a wish the maintainer adds mid-task — become
  issues with public wording. If the current step is still open, a small addition is folded into it instead. Filing
  issues publishes them: for a batch, show the maintainer the drafts first.
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
first: write the design, update `docs/PROGRESS.md`, commit and push, and give a self-contained continuation prompt
(where things are, how to resume, the hard rules, the test setup). Do not start implementing in that session.

## Executing a plan with subagents

A controller agent runs the plan; it dispatches one implementer per task and reviewers after each.

- **Ledger.** The plan's decisions are committed in `docs/superpowers/ledgers/<plan>.md` at task boundaries: each
  task's status and commit, what was checked LIVE/UNIT/NOT, the controller's rulings, deferred items (with the issues
  filed for them). Bulky scratch — the rules file, task briefs, review packages, reports — stays git-ignored in
  `.superpowers/sdd/<plan>/` (never `git add -f` it). After a crash or a lost session, the ledger and `git log` are
  how work resumes; an implementer's transcript survives and can be resumed.
- **Briefs.** Each implementer gets a self-contained brief: the plan's Global Constraints (the shell rules always),
  its task, the files to read, what it must not do. Write the brief to its file before dispatching, never in the same
  chained command.
- **Implementers** never push, tag, publish the wiki, file or close issues, post outside the test workspace, edit files
  outside the repository, or dispatch subagents. They never quote real emails, names or ids in anything they write.
  Each one's live checks use its own window class and kitty socket, never the controller's or one the maintainer is
  typing into.
- **Reviews.** A review per task (the diff against the plan and the rules), fix rounds sent to the same implementer
  (resume it, it keeps its context), then a final whole-branch review. Reviewers find real things in most rounds; one
  implementer per batch of issues keeps that cheap.
- **Deferred items** from reviews become issues (public wording) when the plan ends.
- **Machines.** Before a session where the maintainer works on a test machine, stop every subagent using it.
- **Cleanup.** Reviewers' scratch copies and target directories fill the disk quickly (tens of GB): remove them, by
  literal path, when the plan ends.

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
- **Real data before parsers**: probe an endpoint with `ttyak --profile <test> ctl api …` and capture real-time
  events with `ctl listen` before writing a parser; use the captured shapes (keys and types, not private values) in
  the unit tests. A made-up fixture for a reply nobody has seen hid a real bug. What is learned goes into
  `docs/protocol/`.
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
| Tool tests (as CI runs them) | the `Tool tests` line of `.github/workflows/ci.yml`: `python3 -m unittest tools/test_check_name.py …` |
| Packaging | `python3 -m unittest discover -s packaging -p 'test_*.py'` |
| Site | `python3 -m unittest discover -s tools/site -p 'test_*.py'` and `node --test "tools/site/*.test.mjs"` |
| Names, links | `sh tools/check-name.sh .`, `python3 tools/wiki/check_links.py` |
| Clean checkout per distro | `tools/container-build.sh debian\|ubuntu\|fedora\|arch` (podman) |
| Message-pane cost | `cargo test --release --target-dir <scratch> -p ttyak build_lines_cost -- --ignored --nocapture` |

A new tool test is added to CI's `Tool tests` line in the same commit. Test counts quoted in docs come from one crate's
`test result` line (`cargo test -p ttyak 2>&1 | grep 'test result'`), not a workspace-wide grep.

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
- **`docs/PROGRESS.md`** stays short and current; history goes to `docs/history/` and the ledgers. Old specs and plans
  keep their historical text.

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
