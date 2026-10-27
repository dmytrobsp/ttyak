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
- Task 1: dispatched (<tool> empty, no base commit; ttyak 51d0615).
- Task 1: ca055ff; review: spec ✅, not approved — 1 critical (a stale mutex that cannot be removed spins forever),
  4 important (two breakers can both hold the mutex; release does not check ownership; a Windows release can fail
  silently while a waiter reads `info.json`; corrupt state reads as absent, failing open).
- Ruling: the commit trailer names the model that wrote the commit (rules.md said one fixed name) — costs nothing.
- Task 1: minor (deferred): wait deadline on `time.time()` instead of monotonic; no fsync before `os.replace`; no
  test for "`at` fresh, folder mtime old"; `log.txt` gets CRLF on Windows.
- Task 1: fix round 1/5 (5 addressed, 0 open; 37944ba). Complete (ca055ff..37944ba, review clean). UNIT: 19 tests.
- Task 1: minor (deferred): a rare remaining race — a holder still inside the mutex 30 s after taking it (a
  suspended VM, a clock step) can lose it to a breaker; to be stated in a code comment and the README (Task 6).
- Task 1: minor (deferred): `_break_stale` returns True when the folder could not be removed, so the wait spins
  without sleeping until StoreBusy (10 s); a persistent PermissionError surfaces as PermissionError, not StoreBusy;
  a release error in `mutex()`'s `finally` can mask the body's result or error (carried into Task 2's brief); clearing
  an abandoned `mutex.break` is itself a small race.
