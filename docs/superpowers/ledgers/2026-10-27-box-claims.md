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
- Task 2: dispatched (<tool> base 37944ba).
- Ruling: Task 2's open points settled in the dispatch (answer states incl. `no_claim` and `left`; `status` drops
  expired tickets; an idempotent re-claim renews) — costs a rename in Task 3 if wrong.
- Task 2: d4ada83; review: spec ✅, approved. Complete (37944ba..d4ada83, review clean). UNIT: 32 tests.
- Task 2: minor (deferred): equal `enqueued` ties order by ticket id; no tests for a resumed ticket after expiry, the
  TTL boundary, a stale break with an empty queue, `position` after an expiry.
- Task 2: note: a head waiter that stopped polling (exited with a report) holds up later arrivals on a free box until
  it resumes or its ticket expires (15 min) — the README (Task 6) tells agents to resume promptly or `leave`.
- Task 3: dispatched (<tool> base d4ada83).
- Ruling: `op` answers a store error (busy, corrupt) with `ok: false` and a `kind`, exit 0; a malformed request
  exits 2 — the brief only names the malformed case — costs an exit-code change in Task 4 if wrong.
- Task 3: 6a4f7f9; review: spec ✅, approved with 1 important (the `_Held` no-op mutex wrapper in cli.py, made to
  read the clock inside the mutex, is fragile and Task 4 would have to copy it) → fix round 1: claims functions take
  `now=None` = read the box clock inside the mutex.
- Task 3: minor (deferred): NaN/Infinity/non-positive lease or ttl accepted; a deeply nested JSON request gives a
  RecursionError traceback; no test for the busy answer; the 8-process test does not strictly start all before
  serving (cannot pass falsely, may miss a regression).
- Task 3: fix round 1/5 (1 addressed, 0 open; c884314). Complete (d4ada83..c884314, review clean). UNIT: 38 tests.
- Task 4: dispatched (<tool> base c884314).
- Ruling: Task 4's open points settled in the dispatch — a busy answer retries like unreachable; a corrupt state
  answer exits 2 (a human must look); ssh timeout ~30 s; `$<TOOL>_SSH` split with shlex; the fake ssh injects
  failures through a counter file — costs small changes in Task 5 if wrong.
- Task 4: f261d92 (implementer). Ruling: `release --force` on a box with no claim should exit 0 ("no claim"), not 6
  — nothing was refused — goes to the fix round with the review's findings. Ruling: LocalTransport raising
  Unreachable on a busy mutex is accepted (busy retries like unreachable, by the earlier ruling) — costs a clearer
  message if wrong.
- Task 4: review: spec ✅, not approved — 2 important (`--wait` restarted on every resume, so an agent following its
  resume lines never reaches the limit; `--ticket` unchecked into a box-side file name and the resume line) → fix
  round 1 with the `release --force` no-claim ruling.
- Task 4: minor (deferred): request sent with `ensure_ascii=False`; stderr dropped when the answer is not JSON (a
  wrong `remote` path waits forever as "unreachable"); corrupt state shown as unreachable, `status BOX` unreachable
  exits 0; a grant lost to a timed-out last poll leaves an orphan claim until its lease lapses; Ctrl-C while waiting
  leaves the ticket to its TTL; `--lease 0` accepted; a non-object `boxes.json` raises AttributeError; no test for
  busy retries or corrupt → 2; the transport-error test starts failing before the waiter queued.
- Task 4: fix round 1/5 (3 addressed, 0 open; 4d69af7). Complete (c884314..4d69af7, review clean). UNIT: 55 tests.
- Task 4: minor (deferred): `--ticket ""` silently makes a new ticket (an empty shell variable loses the agent its
  place); `_SAFE_ID` uses `$`, so a trailing newline passes (use `re.fullmatch`); a malformed id now exits 2, not 6
  (README); the remaining-wait test's 6 s bound may be tight on a slow machine.
- Task 5: dispatched (<tool> base 4d69af7).
- Ruling: Task 5 takes the claim id as a required positional; `$<TOOL>_CLAIM` is dropped (an agent passes the id it
  was printed) — costs one env lookup to add back if wanted.
- Task 5: a38723a; review: spec ✅ but the renewer is not joined; not approved — 2 important (a missing executable
  gives a raw traceback; local `run` sends one argument through the shell but execs several, unlike ssh) plus
  ResourceWarnings in the tests → fix round 1.
- Task 5: minor (deferred): Ctrl-C while running prints a traceback (return 130); a repeated renewal failure warns
  every lease/3; scp exit codes are passed through, not mapped to 5/6; the long-command test has ~1.3 s of slack.
- Task 5: fix round 1/5 (4 addressed, 0 open; 303963c). Complete (4d69af7..303963c, review clean). UNIT: 63 tests,
  pristine under `-W error`.
- Task 5: minor (deferred): the missing-ssh half of the 127 test checks only "no traceback" (and has a dead
  assignment).
- Task 6: steps 1 and 3 (box list, README) dispatched to an implementer (<tool> base 303963c); steps 2, 4, 5 (keys,
  clones, live check) by the controller.
