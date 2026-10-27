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
- Task 1: b6965ae; review: spec ✅, not approved — 2 important (folders created with the umask, not 0700; symlinks
  followed into other folders), 5 minor and 3 nits → fix round 1 with all of them (findings are fixed, not filed).
- Task 1: fix round 1/5 (10 addressed; 5ef7ae7); new: the `state.json` read followed a symlink (a session's bytes
  could be copied into the state) + 2 nits → fix round 2.
- Task 1: fix round 2/5 (3 addressed; 6f753d4). Complete (aaace61..6f753d4, review clean). UNIT: 22 tests in the
  file. Three remaining nits (an inaccurate "not a regular file" for any open error; the claim record's keys
  unchecked; the folder-fsync test detects folders through /proc only) go into Task 2's brief, not dropped.
- Task 2: dispatched (base 6f753d4).
- Task 2: 508cbb8; review: spec ✅, not approved — a FIFO at a state, temp or log file blocks the open forever inside
  the lock; no real-SIGINT test; 4 nits (a "-x" purpose in the resume line, sub-second lease shown as 0s, a key
  whitelist for status output, control characters on display) → fix round 1 with all of them.
- Task 2: fix round 1/5 (6 addressed; a727ab9). Complete (7c60978..a727ab9, review clean). UNIT: 43 tests in the
  file. Ruling: the `resume:` line stays raw (shlex-quoted) — it echoes only the caller's own text and must run as
  printed — costs nothing.
- Task 3: dispatched (base a727ab9).
- CI on 7c60978 (Task 1): success on Linux, macOS and the Windows cross-build.
- Task 3: ea811fb (implementer; 54 tests in the file, a mutation pass). Ruling: `install --home FAKE` writes
  FAKE/.config/ttyak/<profile>, which matches ttyak on Linux run with HOME=FAKE or XDG_CONFIG_HOME=FAKE/.config (the
  `directories` crate); the macOS layout waits for the macOS instance plan — costs a `--config-dir` flag later.
- Task 3: review: spec ✅, not approved — the real-folder guard compared path strings only (case-insensitive disks,
  bind mounts); untested guards (`leave` keeping a grant with installs; `remove_copy`'s checks); a misleading refusal
  under an exported HOME; 3 nits → fix round 1 with all of them. Kept, by the review's judgement: the head waiter's
  break removes the dead claim's copies; `install` refuses a lapsed claim (exit 6); `leave` keeps a grant with
  installs; $HOME/XDG count as real folders. Dropped (with a code comment): a SIGKILL mid-import leaves a 0600 temp
  file in the 0700 sessions folder.
- Task 3: fix round 1/5 (all addressed; 3a881f4). Complete (a727ab9..3a881f4, review clean). UNIT: 62 tests in the
  file; mutation pass 21/23 killed (the two survivors are redundant behind the no-symlink walk).
- Task 3: dropped (nits): "uninstall" log lines now precede the release line of the same event — that is the true
  order (copies go first, then the claim); a refused install leaves empty 0700 `.config/ttyak/<profile>` folders in
  the fake home — no secret, the fake home is removed by its owner.
- Task 4: step 1 (docs) dispatched (base 3a881f4).
