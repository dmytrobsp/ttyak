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
- Task 6: box list and README by the implementer (1dadc72, not yet reviewed). Mac: a read-only deploy key, the ssh
  alias, the clone and `self` set up (2026-10-27).
- Refused action: the Windows VM's setup (registering its deploy key, the ssh alias, the clone) was refused by the
  permission check as weakening security. Nothing of it was applied except the VM's new key pair (created by the
  step before, not registered on GitHub). Waiting for the maintainer.
- Task 6 docs: review: not approved — 3 important (printed `<tool> …` lines are not runnable as printed; the
  shorthand does not work on Windows; README promises a stale marker the `status` table lacks) → fix round 1.
- Ruling: the README's wording minors (a number for `--report-every`, giving up with `--wait 0`, the clones-first
  rule, two exit-code rows) go into the same round — same file, only correcting what it says — costs nothing.
- Outside event: the reviewer ran `tb.py status` once (read-only `op status` over ssh to both boxes; nothing written).
- Task 6 docs: fix round 1/5 (3 addressed + wording minors; 12e06a2); new important: POSIX quoting of the printed
  lines breaks them on a Windows box → fix round 2 (one quoting helper: shlex on POSIX, list2cmdline on Windows).
- Windows deploy key registered by the maintainer (read-only), 2026-10-27.
- Task 6 docs: fix round 2/5 (1 addressed, 0 open; bd91628). Review clean (303963c..bd91628). UNIT: 66 tests.
- Task 6: minor (deferred): a `--purpose` with `%`, `&`, `|`, `^` and no space breaks a resume line pasted into
  cmd.exe on the Windows box (README sentence wanted); cosmetic import order in cli.py.
- Task 6: the Windows VM's alias, clone and `self` set up by the maintainer running the script (the stdin form of
  PowerShell ran nothing; a copied script with `-File` under the VM's RemoteSigned policy worked); the Mac's clone
  pulled to bd91628.
- Task 6: complete (303963c..bd91628). LIVE (2026-10-27, from the desktop and on the Mac): claim of the Windows VM;
  a second claimant with `--wait 20s` exits 3 naming the holder; a third with `--report-every 10s` exits 10 and its
  printed resume line, run as printed, is granted after the release; `run` under the claim runs on the VM (cmd.exe
  semantics, exit code 7 passed through); `run` with a wrong id exits 6 and runs nothing; push/pull round trip with
  non-ASCII content; a local claim on the Mac while a desktop client waits for it, hand-over on release (FIFO across
  hosts); withdraw → a new claim exits 4 → reinstate; the box logs record every step; the 66-test suite passes on
  the Mac's Python 3.9 under a claim via `run`; both boxes end free with empty queues. NOT: the suite on Windows (the
  tests use POSIX shell helpers); a stale-claim break live (unit-tested).
- Task 6: minor (deferred): a `renew_failed` log line names the current holder, not the caller that failed.
- Task 7: the maintainer's word (2026-10-27) for the other project; it had no git-ignored local file, and the
  maintainer chose an untracked `CLAUDE.local.md` there (kept out of git by `.git/info/exclude`; nothing tracked
  changed). `AGENTS.local.md` points at the claim tool and the box names. Public docs dispatched (ttyak base fed8b50).
- Task 7: e5318bb; review: not approved — 3 important (the Windows and macOS recipes still ssh around the tool; the
  10-minute live-wait cap vs claim waits is left unclear) + a line over 120 columns.
- Ruling: `run --tty` added to the claim tool (ssh `-t` with a command), because ttyak's Windows TUI recipe needs a
  terminal and `run` only did `-T` — a plan gap found by the docs review — costs one flag if wrong.
- `run --tty`: 82c3efd; review approved. UNIT: 67 tests, pristine. Minor (deferred): the ssh run line in
  transport.py is over 120 columns; with a pty, stderr merges into stdout (README note).
- Task 7: fix round 1/5 (3 addressed + wrapping; 16501c6). Complete (fed8b50..16501c6, review clean). LIVE: stdin
  passes through `run` (piped two lines to the Mac under a claim, counted there), as the macOS recipe relies on.
- Task 7: minor (deferred): the tool's README does not say `run` forwards stdin; lines over 120 columns that predate
  this plan in AGENTS.md and live-testing.md.
- Final review (<tool> 82c3efd, ttyak 554a26a): seams hold; 3 important — a claim on an unreachable box waited
  silently forever; `status --json` printed claim ids and tickets (anyone could act under another's claim); the lock
  is per OS user (README) — plus five small fix-now items (empty `--ticket`, `$` in the id check, ASCII requests,
  stderr on a non-JSON answer, the transport-error test). One fix wave dispatched. The other minors triaged into
  follow-up issues (in the private repository) or dropped.
- The local files now advise `--report-every 5m` and the resume line instead of `--wait 10m` (which leaves the queue).
- Final fix wave: 89ccb85 (<tool>); scoped re-review: all findings addressed. UNIT: 73 tests, pristine. LIVE: a
  claim on an unreachable box exits 5 at once with ssh's reason (default `--wait forever`); both boxes' clones at
  89ccb85, both boxes free.
- Deferred items filed in the private repository's issues (#1 store robustness, #2 diagnosing corrupt state,
  #3 bounded orphans and Ctrl-C, #4 lease/TTL validation, #5 test gaps, #6 cmd.exe metacharacters); the rest dropped
  by the final review's triage.
- Plan complete (2026-10-27): <tool> 14 commits from the empty tree to 89ccb85; ttyak docs fed8b50..16501c6.
  Next in the design's order: part 2 — ttyak's Slack test-account ledger and claims (its own plan).
