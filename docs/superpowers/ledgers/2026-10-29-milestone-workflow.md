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
