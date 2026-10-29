# Ledger — plan docs/superpowers/plans/2026-10-29-milestone-workflow.md

Design: docs/superpowers/specs/2026-10-29-milestone-workflow-design.md (approved 2026-10-29). Start: dbd688a.
Tasks: 1 the milestone helper, 2 measure the machine (controller), 3 the workflow docs, 4 live checks by
implementers, 5 the allowlist (controller, with the maintainer), 6 the dry run (controller and one implementer),
7 final review, PROGRESS, close.

## Pre-flight scan

| Rows | Produces → consumes | Found |
|---|---|---|
| 1 → 2, 6 | `start`, `add`, `rm`, `end`, `ls`, `gate` → the measurement and the dry run | agree |
| 2 → 1 | the measured `FLOOR_GB` → the helper's default (one line in Task 1's file) | agree |
| 2 → 3 | `CAP`, `FLOOR_GB` with their numbers → the workflow section | agree |
| 3 ↔ 4 | each links the other's new anchor (`workflow.md#running-a-milestone`, `live-testing.md#live-checks-in-a-milestone`) | circular: the first commit's link check fails without the other |
| 3 → 5 | the milestone section → its "Allowlist" paragraph | agree, sequential |
| 1–5 → 6 | everything → the dry run | agree |
| 1 (self) | tests call `add` on any machine; CI runners may have less free disk than the default floor | a test without `--floor-gb` fails in CI |
| 1 (self) | `test_bad_names_refused` patches `subprocess.run` | the helper must run git through `subprocess.run` |

- Ruling: Tasks 3 and 4 go to one implementer, the links checked after both — they are the same kind of work and
  link each other; one commit per task stays — costs nothing if wrong.
- Ruling: Task 1's tests pass `--floor-gb 0` to every `add` except the floor test — CI runners' free disk is not this
  machine's — costs a red CI run if wrong.
- Ruling: Task 1's helper runs git through `subprocess.run` only, so the bad-name test can assert no git ran —
  costs a rewrite of one test if wrong.
- Ruling: every real review finding is fixed, minors too (workflow.md); only nits are dropped, with a reason here —
  the project's rule over the skill's deferral of minors.

## Events

- Plan started (2026-10-29, the maintainer: "yes, go ahead"); rules file and briefs in the git-ignored scratch
  folder.
- Task 1: dispatched (base 73e9eeb), implementer sonnet.
- Task 1: ec454fb (implementer: 14 helper tests, the Tool tests line 260 OK, pristine); review dispatched (opus).
- Task 1 review (opus): needs fixes — 3 important (`rm --delete-branch` checked "merged" against master, not the
  milestone branch; a relative worktree root landed inside the repository; `rm` deleted untracked, not-ignored files),
  5 minors (gate not checking its branch, `merge --abort` result unchecked, a check→remove window, names git refuses
  passing the name check, a silent typo in `rm`), test gaps. All sent to fix round 1.
- Ruling: `rm` refuses untracked files that are not ignored (plain `git status --porcelain`), against the plan's
  `--untracked-files=no` — the spec's anti-goal "lost state" binds, the plan's flag was the designer's slip — costs
  an extra refusal for a stray file if wrong.
- Dropped (nits): `ls` prints the resolved path; hard links counted twice in sizes (overcounts, the safe side); the
  `free N` line has no unit (the brief's format).
- Task 1: fix round 1/5 (9 addressed, 0 open; 23 helper tests; commits ec454fb..5ef9503). Re-review: 2 new minors
  (a merge failing without a conflict reported as "needs a hand"; stale docstring/README wording) and a small window
  between the ancestor check and the branch delete; a nit (an unused test folder). All sent to fix round 2.
- Task 1: fix round 2/5 (implementer: 4 fixed, 25 helper tests, the Tool tests line 271 OK; commits 5ef9503..1c161e9);
  scoped re-review dispatched.
- Task 1: re-review of round 2: 4 addressed (the hook case checked in scratch); new minor: a README line of 149
  characters; a minor: `update-ref -d` skips `branch -D`'s checked-out-elsewhere refusal and config cleanup. Both
  sent to fix round 3.
- Task 1: fix round 3/5 (2 addressed, 0 open; commits 1c161e9..e13ad2f). Re-review: approved.
- Dropped (nits): a comment two lines above the check it describes; a code span split across a README line break
  (renders the same). Out of scope, negligible: a branch being rebased in another worktree shows as detached, so the
  checked-out refusal misses it — the commit is checked to be in the milestone branch, nothing is lost.
- Task 1: complete (commits 73e9eeb..e13ad2f, review clean). UNIT: 26 helper tests, the Tool tests line 272 OK
  (pristine, rerun by the controller). NOT: run against this repository (Task 2 is the first real use).
- Task 2 (controller, 2026-10-29, this desktop: 32 threads, 60 GB RAM; nothing else building; worktrees from
  e36c6ba): `cargo test --no-run` from fresh worktrees — 1 at once 37 s, 3 at once 65 s (slowest), 4 at once 78 s;
  each `target/` 4.6 GiB (4.9 GiB after a full `cargo test`); peak used RAM 25.5 / 32.3 / 36.7 GB, lowest available
  36.4 / 29.6 / 25.3 GB. Warm (one file touched), the whole `cargo test` in 3 at once: 13 s, all green.
  CAP = 3 (4 at once is 2.1× the single build; 3 is 1.8×). FLOOR_GB = 40 (twice 5.3 GB plus 20, rounded up to 10;
  the helper counts decimal GB) — the helper's default already, no code change. All measurement worktrees and
  branches removed; free space back to its start.
- Ruling: the plan's Step 2 "`rm` and a new `add`" re-adds a unit whose branch `rm` keeps — the helper rightly
  refuses; new unit names were used. Task 3's docs say a unit name is used once per milestone — costs one line if
  wrong.
- Task 2: complete. LIVE (on this machine): the helper's `start`, `add`, `ls`, `rm`, `end` on this repository.
- Tasks 3–4: dispatched together (base 96dbe2b), implementer sonnet. Commits 4ca3c9a (Task 4) and 9a41990 (Task 3);
  checks on the final tree green (links, names, privacy, line length). Review dispatched (opus).
- Ruling: 4ca3c9a alone has one dangling link (the two new sections link each other); both commits go out in one
  push — costs a broken link in one intermediate commit if wrong.
- Task 5: the allowlist proposal (local, git-ignored) sent to the maintainer.
- CI 40886822489 (e36c6ba, the helper and its CI line): linux, macos, windows-cross green.
- Tasks 3–4 review (opus): needs fixes — 3 important (the shared Windows VM cleanup a recursive delete on a
  `%USERPROFILE%` path called literal; the Windows tool recipes and two setup lines still on the shared folder; the
  milestone section importing "never outside the repository" and desktop test windows, which contradict worktrees
  outside the repository and instances only), 4 minors (the Windows cross-build placed at the gate, which never runs
  it; no rule for a docs-only unit's missing CI; the evidence folder unclear; the reviewer rules missing), 1 nit. All
  sent to fix round 1.
- Ruling: tools/vm/README.md and two setup.md lines join Tasks 3–4 (docs only; cleanup.ps1 unchanged, every call
  passes `-Dir` with the unit folder) — costs a script default that still names the shared folder if wrong.
- Ruling: the design's two "rebase" words (Roles, the ledger section) become the merge wording the gate uses — the
  designer's slip, the approved gate is a merge — costs nothing if wrong.
- Tasks 3–4: fix round 1/5 (8 addressed, 0 open; commit f943589; the design's three "rebase" words now "merge").
  Re-review: a new minor (two commands split across source lines in the shared Windows VM bullet) and a nit (step 3
  in AGENTS.md: reflow, "per unit"); both sent to fix round 2.
- Tasks 3–4: fix round 2/5 (2 addressed, 0 open; commit 51151a2). Re-review: approved.
- Dropped (nit): `-LogonType Interactive` split inside a code span across a prose line (a flag in prose, not a
  command to copy).
- Task 4: complete (commits 96dbe2b..4ca3c9a, with fixes in f943589, 51151a2; review clean).
- Task 3: complete (commits 4ca3c9a..9a41990, with fixes in f943589, 51151a2; review clean). NOT: exercised — the
  dry run (Task 6) is the first use. Checks rerun by the controller: links, names, privacy on copies, no tool name.
- Task 5: the maintainer read the list, asked that force-pushes stay possible ("`push --force` can be useful when
  rewriting git history") and then: "move them all to ask" — every refusal rule became an `ask` rule. The controller
  closed a hole on the way (`git -C <worktree> *` would have allowed any git command, a force-push included): six
  specific `-C` rules and three `-C` ask rules instead.
- Task 5: refused — the controller's write to `.claude/settings.local.json` was refused by auto mode as
  self-modification. Nothing written (checked); the merged file was put in the scratch folder and the maintainer was
  given the copy command. workflow.md says so: settings changes are installed by the maintainer.
- Task 5: the maintainer installed the file (`cp` from the scratch folder); 53 allow, 13 ask rules in place.
  Task 5: complete (d413c03).
- Task 6: `start dryrun`, `milestone/dryrun` pushed, `add dryrun probe`; the probe implementer dispatched (sonnet) with
  a real brief: a comment, the whole `cargo test`, a Linux instance with A and `ctl status`, a Mac build and cleanup.
- CI 40907292284 (d413c03, the .gitignore line): green.
- Task 6, probe: 1ec2790 (a comment; the whole `cargo test` green in the worktree). LIVE: a Linux instance with A
  installed, `ctl status` run in it ("ttyak is not running": no TUI was started), removed, A released, fake home
  deleted; the Mac build (40 s) in `~/ttyak-test/probe/` under a box claim.
- Task 6, refusal 1: the bash-guard hook refused the probe's Mac cleanup — one call held `rm -rf` of the two literal
  paths and `$T`/`$ID` variables without `set -euo pipefail`. Not retried. The controller removed the folder under its
  own claim by the recipe (separate literal commands, each checked; gone), as the design gives a dead or stopped
  agent's folder to the controller. Fix: the recipes say one command per call with the literal tool path and claim id.
- Task 6, observation: `git -C <worktree> push -u origin m/dryrun/probe` ran without a prompt although
  `Bash(git -C * push*)` is an ask rule — the settings may not reload in a running session, or the pattern does not
  match as expected. To check with the maintainer.
- Task 6: m/dryrun/probe pushed; pull request #64 into milestone/dryrun; review dispatched (sonnet).
- Task 6, probe review (sonnet): the diff and the PR text fine; needs fixes in the report — the refused command not
  quoted exactly, five claims without an evidence file, `ctl status` overstated, the build's exit status, the
  separate release after a refused call. Sent to the probe implementer (report only; nothing re-created after the
  fact). For the docs: the evidence rule names what to save (claims, releases, removals, `git status`); a refusal is
  quoted verbatim; after a refused cleanup the implementer stops and leaves the cleanup, release included, to the
  controller.
- Task 6, refusal 2: the bash-guard hook refused the probe implementer's python heredoc that rewrote its report — it
  scans heredoc bodies, and the report quotes the refused `rm` command. Not retried in Bash; the file was written with
  the Write tool.
- Ruling: writing a file with the Write tool after a hook refused a heredoc that wrote the same file is not a way
  around the refusal — the hook guards recursive deletes, and no delete was ever intended; file writes belong in the
  Write/Edit tools — costs a missed hook-bypass pattern if wrong. For the docs: agents write files with the Write and
  Edit tools, never through a heredoc in Bash.
- Task 6, probe: report fix round 1/5 (5 addressed, 0 open); re-review approved. Dropped (nit): the report's line on
  the controller's Mac cleanup has no evidence file — the controller's cleanup and its checks are in this ledger.
- Task 6, gate: `gate dryrun probe` → 20dacbb (a `--no-ff` merge); the whole `cargo test` on it green (44 + 89 + 1024
  passed, 0 failed); CI on #64 (run 40908766465) green on linux, macos, windows-cross; a plain push of
  milestone/dryrun (d413c03..20dacbb); GitHub marked #64 merged. Then `rm dryrun probe --delete-branch`,
  `end dryrun`, `git branch -D milestone/dryrun`: no worktree, branch, instance or claim left.
- Task 6, observation 2: the gate's `git -C <gate> push origin milestone/dryrun` also ran without the ask prompt.
- Task 6: the maintainer: "yes" to deleting the dry run's remote branches; milestone/dryrun and m/dryrun/probe deleted
  on GitHub (none left; #64 stays as the record). On the ask rules not firing: "ok" — they are expected to apply from
  the next session.
- Task 6, docs: the dry run's lessons (command shape, files through Write/Edit, the evidence list, refusals quoted
  verbatim and a refused cleanup left to the controller, small notes) in 39eaaf1; review dispatched.
- Task 6, docs review: the lessons are in, but "one command per call" and the heredoc ban contradicted the recipes
  (shared variables, single-call background steps, the fake home's settings heredoc); 3 minors (NOT backed for every
  evidence item; the controller's cleanup blocked by the implementer's held claim; `pgrep -x` finds the maintainer's
  ttyak). Sent to a fix round.
- Ruling: the two rules are scoped to what the dry run showed (claim-tool calls against a shared box hold one literal
  command; files quoting guarded commands go through Write/Edit), the recipes stay — costs a refusal the narrower rule
  misses if wrong.
- Ruling: after a refused cleanup the controller cleans up under the implementer's still-live box claim and releases
  it — costs a claim held past its owner's report if wrong.
- Task 6, docs: fix rounds 06ada50, f7d8896 (two long lines), 0bae4ba (the box lines of a block run alone; the
  controller also renews and releases the implementer's account claims); re-reviews approved.
- Dropped (nit): a short line left by a reflow in live-testing.md (cosmetic).
- Task 6: complete (the dry run d413c03..20dacbb on the deleted branches, #64 merged into milestone/dryrun; the docs
  39eaaf1..0bae4ba, review clean). LIVE: the whole unit path — helper, worktree, the whole `cargo test`, a Linux
  instance with A, a Mac build, a pull request, a review with a fix round, the gate, a plain push, #64 marked merged,
  cleanup. Refusals found: 2, both settled (the docs changed). NOT: two units at once, a gate conflict, a macOS VM or
  Windows clone in a milestone, the ask rules (they apply from the next session).
- Task 7: final review dispatched (opus), dbd688a..730563b.
- Task 7, final review (opus): needs fixes — 1 critical (in a milestone the live-check recipes build, mount and
  archive the main checkout, not the unit's worktree; the dry run's brief had overridden them by hand), 6 important
  (the push commands not spelled out and the dry run's `git -C … push` shapes matching an ask rule; the gate's order
  and red path undefined; fix-round commits never pushed before the gate; the privacy check missing commit messages
  and the review summary; parallel units sharing a fake home and relative output paths; no local file in a worktree),
  9 minors, 3 nits; plus one item outside the plan for the maintainer (kept out of this ledger's wording). All in
  scope sent to one fix wave (opus).
- Ruling: a helper command `ungate NAME` resets the gate to the remote-tracking milestone tip after a red gate —
  the undo would otherwise be an always-ask `reset --hard` — costs one more helper command if wrong.
- Ruling: cleanup.ps1's `-Dir` becomes mandatory, revisiting the earlier ruling that kept its default — a forgotten
  `-Dir` would stop another unit's ttyak — costs one script change if wrong.
- Ruling: the final merge into master is `git merge --no-ff milestone/NAME` on master from the main checkout, then the
  whole `cargo test` and `git push origin master`, on the maintainer's "merge" — costs nothing if wrong.
- Task 7: fix wave 411e80f (helper: ungate, gate prints old and new tip, the whole help; 33 tests), 6694881
  (cleanup.ps1 `-Dir` mandatory), 1d24d90 (docs: all 19 items); the controller reran the helper tests and the Tool
  tests line (OK). Re-review (opus): all 19 addressed, `ungate`, a conflict and the final merge exercised in a temp
  repository with a bare origin; "Ready for the trial".
- Parked, with rulings: the docs-only gate checks run from the gate worktree have no allow rule (auto mode checks
  them; the trial's retro adds narrow rules with the maintainer if one prompts); a red `cargo test` after the final
  merge on master leaves a local merge commit (near impossible — master differs only by ledger commits and fixes
  already gated; the maintainer is told); the tools/vm README block for the shared Windows VM not yet run on the box
  (clones are the default; NOT tried until a unit needs it); `ungate` refusing a gate with files a red test left
  (no crate writes such files; unlikely).
- Dropped (nits): `-NonInteractive` on the two PowerShell lines of the tools/vm README (`-Dir` is always passed).
- Carried to the trial: the next milestone's rules file adds "never run `milestone.py`, never `git push`"; check at
  the start of the trial's session that the ask rules fire.
- Plan complete (2026-10-29). Final review: ready for the trial after one fix wave. Fixed in reviews: every real
  finding, minors included; dropped: the nits listed above; filed: none. Next: the trial milestone of 3–5 small
  issues the maintainer picks, with its own plan and ledger and a retro at its end.
