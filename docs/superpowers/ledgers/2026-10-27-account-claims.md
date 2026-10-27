# Ledger — plan docs/superpowers/plans/2026-10-27-account-claims.md

Design: docs/superpowers/specs/2026-10-27-test-machines-design.md (Part 4, the account lines of Part 5).
Start: 8295fba. Tasks: 1 ledger and claim rules, 2 the claim commands, 3 sessions in and out, 4 ttyak adopts them
(docs, the local ledger, a live check), final review.

## Pre-flight scan

| Rows | Produces → consumes | Found |
|---|---|---|
| 1 ↔ 2 | `Claims` methods and answer shape → CLI | agree |
| 1 ↔ 3 | the claim's `installs` field → `install`/`release` | agree; Task 1 keeps `installs` in the claim record |
| 2 ↔ 3 | `release` (Task 2) gains cleanup (Task 3) | agree; Task 3 extends it |
| 1, 4 | CI `Tool tests` step and setup.md's copy | Task 1 adds the test file; Task 4 changes neither |
| 4 (self) | step 1 is docs, steps 2–3 change local data and run ttyak live | ruling 1 |
| 1–3 (self) | tests match the code they specify | agree |

- Ruling: Task 4 step 1 goes to an implementer; steps 2 and 3 (the local ledger, importing the sessions, the live
  check) are the controller's — they touch private data and run ttyak against Slack — costs time if wrong.
- The maintainer's decisions (2026-10-27): the plan approved; the ledger may be made from `AGENTS.local.md` and A's
  and B's sessions imported from the real config folder (read only there).

## Events

- Task 1: dispatched (base 8295fba).
