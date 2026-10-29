# Multi-agent milestone workflow — design (2026-10-29)

Issue: [#29](https://github.com/dmytrobsp/ttyak/issues/29). Machine-specific facts (accounts, boxes, paths) are not
in this file: they live in `AGENTS.local.md` and the claim tool named there.

Approved by the maintainer on 2026-10-29. Plan:
[2026-10-29-milestone-workflow.md](../plans/2026-10-29-milestone-workflow.md).

## Intent

Today one controller runs a plan's tasks one after another ([workflow.md](../../agents/workflow.md)): a milestone of
a handful of small fixes takes a day or more of wall-clock time. The maintainer wants it faster: "mostly speed, yes. i
want multiple agents to work on things in parallel".

- **A milestone** is "some known bugs/issues, some improvements"; "if it's a big new feature, then i imagine it goes
  as a separate milestone with nothing else in it."
- **The maintainer during a milestone**: "mostly at keyboard, so mostly available to answer questions if any arise.
  but i'm likely focused on something else, like a work project." So questions are rare and worth the interruption.
- **Shape**: "one session that drives the milestone that's managing several other agents that actually implement the
  milestone."
- **Where they look**: "only at the end, when the milestone branch is ready. unless an agent has a specific question
  that would change the implementation in a significant way." Questions: "one sitting at the start".
- **Running the result themselves**: "mostly no, but sometimes yes".
- **Live checks**: the maintainer leans "towards implementers doing live checks"; on shared boxes "recipes only"
  ("let's try recipes only and then adjust if something throttles us").
- **Failure**: "if it would subjectively take more time to implement a milestone than the workflow we're using now
  ... a significant drop in quality would also be a failure".
- **Build it**: the full version (docs, small helpers, a measurement), then "a trial run with small issues".

**Done means:** a milestone is run by one controller session with several implementers working at once, each issue a
pull request into a milestone branch that passes its own review-fix loop and a merge gate; the maintainer answers
questions in one sitting at the start and judges the milestone once at the end, from one pull request into `master`
with a report; and after a trial milestone of 3–5 small issues the maintainer finds it faster than today at the
same quality.

## Goals

1. **Parallel units.** Each issue of a milestone (or a small group of related issues, the controller's grouping) is a
   unit: its own branch, its own git worktree with its own build folder, its own implementer, its own pull request
   into the milestone branch.
2. **One sitting at the start, one look at the end.** All questions the milestone needs from the maintainer are asked
   before any implementer starts; the maintainer then sees the milestone pull request and its report.
3. **The quality bar of today, per unit**: TDD with the RED shown, the whole `cargo test`, a review per unit with fix
   rounds by the same implementer, every real finding fixed (only nits dropped, with a reason), then a final
   whole-milestone review and one fix wave.
4. **A merge gate** that keeps the milestone branch green: units merge one at a time, each merged into the current
   milestone tip and tested there first ("not rocket science" rule, below).
5. **Implementers run their own live checks**, under claims, with evidence the reviewer checks.
6. **Measured capacity**: how many implementers this machine carries at once, measured before the trial and written
   down.
7. **Few permission refusals**: a project allowlist the maintainer approves and a dry run that finds refusals before
   the trial; any refusal that still happens reaches the maintainer at once.
8. **A trial milestone** of 3–5 small issues the maintainer picks, then an adjustment of the docs from what it
   showed.

## Non-goals

- Several independent sessions, or sessions on several machines, picking issues on their own (the maintainer chose
  one driving session; independent sessions need a pick-up protocol and each could interrupt the maintainer).
- Claude Code's agent teams (experimental, no worktrees, no resume of teammates after a lost session) and `/batch`
  (one large change split up, not a milestone of issues).
- GitHub's merge queue, branch protection, bots or new GitHub Actions: the controller is the queue.
- New features in the claim tool. Only ttyak's recipes change.
- Replacing the single-controller workflow: a lone fix or a plan outside a milestone still runs as today.

## Anti-goals (a failure even if it works)

- **Slower than today**, from the end of the start sitting to the milestone pull request being ready.
- **A drop in quality**: more found in the final review or by the maintainer after the merge than with today's
  workflow; a red milestone branch.
- **Questions mid-milestone** that could have been asked in the start sitting.
- **An implementer improvising on a shared box** (the Mac, the shared Windows VM): installing, changing settings,
  starting or stopping a VM, cleaning up what it did not make.
- **Anything private in public**: review text, test-account names, ids, host names or a security finding in a pull
  request, its description or comments. Pull requests are public.
- **Lost state**: a controller that loses its session and cannot resume from the ledger, `git log` and the open pull
  requests.
- **A full disk**: the disk is at 88 % (below); a worktree's build folder can pass 20 GB.

## Constraints

- **The hard rules** of [AGENTS.md](../../../AGENTS.md) and [live-testing.md](../../agents/live-testing.md) hold for
  every agent, and every brief carries them (the shell rules always). Implementers keep today's limits from
  [workflow.md](../../agents/workflow.md#executing-a-plan-with-subagents): they never push, tag, file or close issues,
  post outside the test workspace, edit files outside their worktree, or dispatch subagents.
- **Claude Code** (checked 2026-10-29 in its docs): background subagents run in parallel and can be resumed with their
  context (`SendMessage`). A subagent's own worktree (`isolation: worktree`) starts only from the default branch or
  the caller's current HEAD (`worktree.baseRef`: `fresh` or `head`), never from a named branch: the controller makes
  the worktrees itself with `git worktree add`.
- **This machine** (2026-10-29): 32 threads, 60 GB RAM, 210 GB free of 1.8 TB (88 % used); the main checkout's
  `target/` is 43 GB; a Windows cross-build directory passes 10 GB.
- **CI**: `ci.yml` runs on every `pull_request` (any base branch) that touches more than docs, and on pushes to
  `master` only; a newer push to the same ref cancels the older run. CI does not run the Windows tests (#61).
- **Slack accounts**: two test accounts (A, B), one test per account at a time, so at most two Slack live checks run
  at once, and one when a check needs both. The Mac and the shared Windows VM take one claim each; instances take
  RAM slots from the claim tool's pool.
- **GitHub**: `master` is not protected; merge commits, squash and rebase are allowed; branches are not deleted on
  merge. `Fixes #N` closes an issue only when the commit reaches `master`. Deleting anything on GitHub (branches
  included) needs the maintainer's word.

## The design

### Roles

- **The controller**: the one session. It runs the start sitting, plans the units and their order, writes briefs,
  dispatches implementers and reviewers, pushes unit branches and opens their pull requests, runs the merge gate,
  the final review and fix wave, the final live check and the report. It is the only agent that talks to the
  maintainer, pushes, or files issues. It keeps its own context lean: short reports, details in scratch files.
- **Implementers**: one per unit, in the background, each in its own worktree. They are resumed for fix rounds and
  for merges with conflicts.
- **Reviewers**: a fresh one per unit (opus for security- or concurrency-heavy code, as today), resumed for scoped
  re-reviews; a fresh one for the final whole-milestone review.

### The start of a milestone: one sitting

1. The maintainer and the controller agree on the milestone's issues (as today: proposed by the controller when no
   milestone is open, decided by the maintainer).
2. The controller reads every issue, its history and the code it touches, and brings, in one message or one short
   back-and-forth: every question the milestone needs from the maintainer (behaviour, naming, `needs-user` items),
   and its batching: which units run at once because they touch separate code, which wait for another to merge
   first, and why.
3. A `size:l` issue in a mixed milestone is raised in the sitting: it moves to a milestone of its own, or it gets its
   design first. A mixed milestone holds `size:s` and `size:m` units.
4. The answers and the batching go into the milestone's plan (`docs/superpowers/plans/<date>-milestone-<name>.md`) and
   its ledger, committed on `master` before the first dispatch. Then the controller runs without the maintainer.

**A big-feature milestone** goes through the full design process first (brainstorm, written design, a builder's read,
the maintainer's approval, a plan). Its plan's tasks are the units, run at once where the plan's dependencies allow,
with the same pull requests, reviews, gate and report.

### Branches and worktrees

- **The milestone branch** is made from `master` at the start and pushed.
- **A unit branch** is made from the milestone tip when the unit is dispatched, in its own worktree with its own
  Rust build folder. Worktrees live outside the scratch folder (it is wiped by a reboot) and are git-ignored.
- **The controller's main checkout stays on `master`**, where the ledger is committed (docs-only commits, no CI).
  Merging into the milestone branch happens in a worktree of its own.
- **An urgent fix during a milestone** goes to `master` as today; the controller merges `master` into the milestone
  branch at the next gate.
- **A helper** in `tools/` does the mechanical part: make the milestone branch; add a unit worktree with its build
  folder; remove one with its build folder, by literal path, after checking it is a worktree of this repository; list
  them with their disk use. It is public code (nothing private in it), tested, and added to CI's `Tool tests` step.

### How many at once

- Start with **3 implementers** at once; the number is set by a measurement before the trial: several `cargo test`
  runs from fresh and from warm worktrees at once — wall time, peak RAM, disk per worktree. The result and the cap go
  into workflow.md.
- **Disk**: a worktree and its build folder are removed as soon as their unit merges; the controller does not
  dispatch while free space is below a floor set by the measurement.
- **Batching**: units in the same wave touch separate files and modules as far as the controller can tell from
  reading the code; units that share files run one after the other; a unit that needs another one's change waits for
  that merge. Conflicts at the gate are a sign the batching missed something: noted in the ledger.

### A unit, from dispatch to merge

1. **Brief.** The controller writes the brief to the plan's scratch folder (`.superpowers/sdd/<plan>/`, as today):
   the Global Constraints, the issue and the maintainer's answers for it, the files to read, the worktree, branch and
   build folder, the live-check rules below, what not to do. Then it dispatches
   the implementer in the background.
2. **Implementation.** TDD with the RED shown; commits on the unit branch; the whole `cargo test` in the worktree;
   live checks where the change needs them; a short report with paths to its evidence.
3. **Pull request.** The controller runs the privacy check on what will become public (the diff and the description),
   pushes the unit branch and opens a pull request into the milestone branch, in public wording. CI runs on it.
4. **Review.** A reviewer reads the diff against the plan and the rules and checks the live evidence. Findings go to
   the same implementer; a scoped re-review follows each fix round. Every real finding is fixed; nits are dropped with
   a reason in the ledger; out-of-scope items and the maintainer's decisions become issues. Review text stays in the
   scratch folder; the pull request gets a short public summary (rounds, what kind of things were fixed). Security
   findings never go to the pull request: a private advisory, as today.
5. **Merge gate**, one unit at a time, without rewriting any pushed branch (force-pushes need the maintainer's
   word). In the milestone's own worktree the controller merges the unit branch into the current milestone tip
   (`--no-ff`) and runs the whole `cargo test` on that merge result; CI on the pull request is green (GitHub tests
   the pull request merged into its base). If both pass, the controller pushes the milestone branch, a plain
   fast-forward push, and GitHub marks the pull request merged. A merge with conflicts goes back to its implementer,
   who merges the milestone tip into its own branch and resolves them (the author resolves its own conflicts); the
   controller pushes that and the gate starts again. After the merge the unit's worktree and build folder are
   removed. Units still in flight take the new tip only at their own gate.
6. **The ledger** gets an entry at each step (below).

### Questions mid-milestone

Implementers never ask the maintainer: a question goes back to the controller in the implementer's report. The
controller answers it itself when it can (a ruling in the ledger with what it costs if wrong), and asks the maintainer
only when the answer would change the implementation significantly. Other units keep running meanwhile.

### Permission prompts and refusals

The maintainer runs sessions in auto mode. A refusal must not hold the milestone up: "i want the milestone to
actually be completed, so the refusals has to be addressed ASAP. i just want to minimize the amount of refusals
happening". So the work is in preventing them, and the few that remain reach the maintainer at once.

**Fewer refusals, up front:**

- **A project allowlist** of the commands the workflow needs (cargo in the worktrees, git in the worktrees and the
  milestone's worktree, the helper, the accounts tool, the claim tool's recipe commands, `gh pr` for the controller),
  written as narrowly as works. Changing settings is the maintainer's call: the plan proposes the list, the
  maintainer approves it before it goes in. It goes into the untracked `.claude/settings.local.json` (it names the
  private claim tool's paths), which the repository's `.gitignore` then lists too (today only this machine's global
  git ignore covers it); a template with placeholders may be tracked.
- **Command shapes that pass**: the briefs and recipes use plain, separate commands with literal paths (or
  `${VAR:?}`), never a scripted edit chained with a commit, never computed command names — the shapes the safety hook
  and the worktree checks accept.
- **A dry run before the trial**: one implementer, one throwaway unit, every recipe the milestone workflow uses (a
  worktree, a build, a pull request into a scratch milestone branch, an instance, an account claim, the Mac build),
  in auto mode. Every refusal it hits is fixed in the allowlist or the recipes, with the maintainer, before the trial
  starts. The scratch branch and pull request are removed afterwards, with the maintainer's word (deleting on GitHub).

**A refusal that still happens:**

- It stops only the step that hit it, as the hard rule says: never reworded or retried to get around it. The
  implementer reports it with the exact command; the controller checks the worktree for partial writes, parks that
  unit, and keeps the other units running.
- **The controller tells the maintainer at once**: the exact command, why it was needed, what is waiting on it, and
  the allowlist line that would prevent it next time. Refusals that arrive together go in one message. When the
  maintainer has run or allowed it, the parked unit goes on.
- Every refusal goes into the ledger; the retro after the trial turns recurring ones into allowlist or recipe
  changes.

### Live checks by implementers

Three places, three rules:

1. **Instances** (the Linux containers, the Hyprland VM, the Windows clones, the macOS VMs) are throwaway: live
   checks run there by default; the implementer makes and removes its own, by name.
2. **Shared boxes** (the Mac itself, the shared Windows VM) are someone else's machine, never throwaway. An
   implementer uses one only under a box claim and only through a recipe: the step-by-step command lists in
   live-testing.md (build ttyak on the Mac, clean up after). It works only in its own folder, removes exactly what it
   made, checks it is gone, and leaves the box as it found it. It never installs anything, changes a setting, starts,
   stops, restarts or reverts a VM. Anything a recipe does not cover goes back to the controller.
3. **The maintainer's desktop**: for implementers, nothing but instances — no test windows on the desktop's hidden
   workspace. `ttyak ctl` in a fake home with a claimed account, with no window, stays allowed (it shows nothing).
   What an instance cannot do and needs the hidden workspace ([Test windows](../../agents/live-testing.md#test-windows))
   is the controller's, as is anything with the maintainer present.

- **One folder per unit on the Mac.** The recipes use one fixed folder today; with several agents, one that dies
  before cleaning up leaves files the next one could reuse or remove. Each implementer works in a folder named after
  its unit (inside the recipes' folder), so the cleanup under a later box claim knows what to delete. Folders left by
  an agent that died are the controller's to remove, under its own claim, by the same recipe: at the end of the
  milestone it lists the recipes' folder on the Mac and removes only the folders of this milestone's units.
- **Claims do the queuing**: accounts, boxes and RAM slots, as today; waiting for a claim is not a live observation.
- **Evidence**: the implementer saves what shows the check (`ctl` output, the instance's own screenshot, log lines;
  never the maintainer's screen) in the unit's scratch folder; the reviewer checks it before anything counts as LIVE.
- **The controller keeps** checks with the maintainer and one live smoke check of the merged milestone branch.

### The end of a milestone

1. All units merged → a final whole-milestone review (the milestone branch against `master`) → one fix wave, its
   fixes going through the same review and gate → the whole `cargo test`, the Windows cross-build and
   `cargo test --no-run` for Windows, CI on the milestone pull request, the controller's live smoke check.
2. **The milestone pull request** (milestone branch → `master`) carries the report, from a template in `docs/agents/`:
   1. **Calls made for you**: every decision the controller or an implementer made that the maintainer might have
      made differently, one line each, with why.
   2. **Things to try yourself**: a short hands-on list for the changes the maintainer will notice.
   3. **Per issue**: what changed, the merged unit pull request, what was checked LIVE, UNIT or NOT, and what was not
      tried.
   4. **Reviews**: what was found and fixed, dropped as a nit (why), filed as an issue.
   5. **Health**: the tests, the Windows build, CI, the final live check.
   6. **Refused actions** during the milestone, if any (each was raised at once): what it was, how it was settled,
      the allowlist or recipe change it led to.
   7. **A build to try**: where a debug build of the milestone branch is (a scratch `--target-dir`, never
      `target/release`) and the one command to run it, for when the maintainer wants to.
   Public wording throughout; the privacy check runs on it before it is posted.
3. **The maintainer's word** ("merge") → the controller merges with a merge commit (not squashed), so `master` keeps
   each unit's commits and `Fixes #N` closes the issues. The report asks in the same line for leave to delete the
   milestone's branches on GitHub. A release then follows [RELEASING.md](../../RELEASING.md), on the maintainer's word.

### The ledger

One ledger per milestone, committed on `master`, in the shape of
[ledgers/README.md](../ledgers/README.md) plus: per unit, its branch, pull request, dispatch and merge times, review
rounds, gate results (merge conflicts, red runs); the batching and why; the maintainer's answers from the start
sitting. After a lost session, the ledger, `git log`, the open pull requests and the worktrees on disk are how the
controller resumes; implementers' transcripts survive and can be resumed.

### Judging the trial

- **Time** is the maintainer's impression: "this isnt something we can reliably measure. so yeah, its more
  impression based". The ledger keeps each unit's dispatch, approval and merge times and the milestone's start and end
  as a rough record to talk over, not as a measurement.
- **Quality**: the final review's real findings per unit and red runs on the milestone branch, against the final
  reviews in the committed ledgers since 2026-10-27; what the maintainer finds after the merge.
- **Cost**: usage limits hit, if any, noted, not a gate.
- **Adjustment**: a short retro in the ledger and the changes to workflow.md and live-testing.md it calls for.

### Docs that change

- [workflow.md](../../agents/workflow.md): a section on running a milestone with parallel agents; AGENTS.md's
  workflow summary points to it.
- [live-testing.md](../../agents/live-testing.md): the three places, one folder per claim in the Mac recipes, the
  evidence rule.
- [ledgers/README.md](../ledgers/README.md): the milestone ledger's extra lines.
- The report template in `docs/agents/`; `.gitignore` for the worktrees; [PROGRESS.md](../../PROGRESS.md) at the
  start and end.

## Expected effort

About 1½–2 days to build: the written design and its builder's read, the docs, the helper with its tests, the
measurement, the allowlist and the dry run, and reviews as usual. Mostly docs, so most pushes skip CI. The trial milestone costs what its issues cost
and ships real fixes. A lighter version (docs only, worktrees by hand, the measurement during the trial) was offered;
the maintainer chose the full one.

## Calls made by the designer (not the maintainer's)

- The controller pushes unit branches and opens the pull requests; implementers still never push — so everything that
  becomes public passes the controller's privacy check first.
- Review text stays in scratch; pull requests get a public summary only.
- The milestone's ledger lives on `master`, not on the milestone branch.
- An urgent fix goes to `master` and is merged into the milestone branch at the next gate.
- Mixed milestones hold `size:s` and `size:m` units; a `size:l` issue moves out or gets its design first.
- Start at 3 implementers until measured; worktrees removed at merge; a disk floor.
- The final merge into `master` is a merge commit, not a squash.
- The report always ends with a build to try.
- The gate merges instead of rebasing, so no pushed branch is ever rewritten (no force-pushes, no new rule).
- Implementers' live checks are instances only; the desktop's hidden test workspace stays the controller's.
- One folder per unit on the Mac; the controller removes a dead agent's folder at the milestone's end.
- The trial's quality is judged against the recent ledgers' final reviews (its time is the maintainer's impression).
- A refused action parks only its unit and reaches the maintainer at once; the allowlist and a dry run before the
  trial are there to keep refusals rare.

## Left to the builder

- The helper's language (Python, like the other tools), its interface and where worktrees live (git-ignored, outside
  the scratch folder).
- Branch names (for example `milestone/<name>` and `m/<name>/<issue>-<slug>`).
- Whether the Windows cross-build runs per unit or only at the gate and the end, within the disk budget; whether build
  folders share a compiler cache.
- The batching heuristics beyond the rule above; the report template's layout; how the controller keeps track of
  background agents.
- The measurement's exact runs and the disk floor.
