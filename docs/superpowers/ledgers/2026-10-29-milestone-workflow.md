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
