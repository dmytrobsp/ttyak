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
