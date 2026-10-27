# Ledger — plan docs/superpowers/plans/2026-10-27-box-claims.md

Design: docs/superpowers/specs/2026-10-27-test-machines-design.md (Parts 1–2). Start: d062ce6 (ttyak). The code
lives in a new private repository, `<tool>` (its commits are named here by hash; ttyak commits say "ttyak").
Tasks: 1 repository and state store, 2 claim rules, 3 box-side `op`, 4 box list, transports and claim commands,
5 enforced box work, 6 the boxes join (live), 7 ttyak and the local files adopt the claims, final review.

## Pre-flight scan

| Rows | Produces → consumes | Found |
|---|---|---|
| 1 ↔ 2 | `Store` methods → claim rules | agree |
| 1 ↔ 3 | `tb.py` (Task 1) calls `<tool>.cli.main()`, which Task 3 creates | `tb.py` would be dead for two tasks — ruling 1 |
| 2 ↔ 3 | `claims.*(store, now, …)` → `op` dispatch | agree; `op` names match the function names |
| 2 ↔ 4 | `claims.*` → `LocalTransport` | agree |
| 3 ↔ 4 | `op` JSON over stdin → `SshTransport` | agree |
| 4 ↔ 5 | `Transport.run/push/pull`, `cli.py` | agree; Task 5 extends both |
| 4 ↔ 6 | test `boxes.json` via `$<TOOL>_BOXES` → real entries | agree |
| 1–5 | "commit, push" in every task | the project's implementers never push — ruling 2 |
| 6 | keys and clones on the boxes, deploy keys on GitHub, live use | outside a repository — ruling 3 |
| 7 | step 3 asks the maintainer | the controller asks — ruling 3 |
| 4 (self) | `--wait` has no "none" value but a test is named `test_wait_none_…` | ruling 4 |
| 1, 2, 3, 5 (self) | tests match the code they specify | agree |

- Ruling: `tb.py` is created in Task 3 with `cli.py`, not in Task 1 — avoids an entry point that fails for two tasks
  — costs nothing if wrong.
- Ruling: implementers commit only; the controller pushes `<tool>` after each task's review — the project's rule
  that implementers never push — costs nothing if wrong.
- Ruling: Task 6's box-side work (keys, clones, the live check) and Task 7 step 3 are done by the controller or a
  dedicated agent briefed for it, not by a code implementer — they change machines and GitHub settings outside the
  repository — costs time if wrong.
- Ruling: `--wait 0` (or `0s`) means one try; at the wait limit, if the last poll was unreachable the exit is 5,
  otherwise 3 — the plan names a "none" case without a value — costs a flag rename if wrong.

## Events

- The `<tool>` repository was created private on the maintainer's account (their word, 2026-10-27); the local
  repository uses the repository's own identity and the user-named https origin (the machine's global URL rewrite to
  ssh would use another key).
