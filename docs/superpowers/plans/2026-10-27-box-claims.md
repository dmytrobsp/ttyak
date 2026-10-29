# Box claims Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** A private `<tool>` repository with a claim tool that gives every shared test box a mutex with a queue,
used by agents and the maintainer from the desktop over ssh or locally on the box, enforced by the tool.

**Architecture:** The claim state lives on each box, in a state folder guarded by an atomic-mkdir mutex; a box-side
`op` command applies one JSON request to it and answers in JSON. The client (`<tool> claim|run|…`) reaches a box
over ssh (or locally when it runs on that box), polls `op claim` while waiting, and refuses box work without a live
claim. Python 3.9+ standard library only, so the same files run on Linux, macOS (Apple's Python 3.9) and Windows.

**Tech Stack:** Python 3.9+ stdlib (`json`, `argparse`, `subprocess`, `threading`, `uuid`, `unittest`), OpenSSH, git.

**Spec:** [docs/superpowers/specs/2026-10-27-test-machines-design.md](../specs/2026-10-27-test-machines-design.md)
— Part 1 (the repository) and Part 2 (box claims). Parts 3–5 get their own plans after this one.

## Global Constraints

- Shell rules (every command an agent writes): bash, never fish (`NAME=value`); `set -euo pipefail` on every
  multi-command script; never a recursive `rm`/`chmod`/`chown`/`mv`/`cp` (or `find -delete`) on a variable path —
  literal path, or `${VAR:?}` after checking it. In Python, never `shutil.rmtree` on a computed path: the mutex is
  removed file by file, then `os.rmdir`.
- Python 3.9 floor: no `match`, no `tomllib`, `from __future__ import annotations` in every module, no runtime
  `X | Y` types. Standard library only. Configuration is JSON.
- The `<tool>` repository is private (on the maintainer's GitHub account) and commits with the maintainer's GitHub
  identity set repo-locally (`git log -1 --format='%an <%ae>'` of a maintainer commit in ttyak) — never the
  machine's global identity. Trailer: `Co-Authored-By: Claude <model> <noreply@anthropic.com>`.
- ttyak is public: nothing from the box list (host aliases, VM names, user names, paths) goes into ttyak's tracked
  files, this plan included; they live in the private repository and in `AGENTS.local.md`.
- All times are the box's clock (`time.time()` in the box-side `op`); the client never compares its own clock with
  the box's. Box-side tests inject time through the `now` argument or `<TOOL>_NOW`.
- Box-side I/O: read the request as UTF-8 bytes from `sys.stdin.buffer`, write the answer with
  `json.dumps(..., ensure_ascii=True)` plus `\n` — Windows consoles are not UTF-8.
- Defaults (names fixed here, values may be tuned in review): lease 30 min, ticket TTL 15 min, poll 5 s, mutex
  stale after 30 s, mutex wait up to 10 s.
- Client exit codes: 0 done, 2 usage/config error, 3 wait limit reached, 4 box withdrawn, 5 box unreachable,
  6 no live claim (not the holder, or lapsed), 10 report interval reached (still queued; resume with `--ticket`).
- Live use of the boxes follows ttyak's `docs/agents/live-testing.md` and `AGENTS.local.md` (check the box is free
  before the first claim exists; never shut down, restart or revert the shared Windows VM).

## Review Focus

1. **A waiting client loses the box for a while** (VM rebooting, ssh hiccup): it keeps waiting, says "unreachable" in
   its reports, and keeps its place if back within the ticket TTL — Task 4, `test_wait_survives_transport_errors`.
2. **A command longer than the lease** under `run`: the claim stays live for the whole command — Task 5,
   `test_run_renews_during_long_command`.
3. **A waiter killed mid-wait** (Ctrl-C, a dead agent): its ticket must not block the queue forever — Task 2,
   `test_expired_head_ticket_is_skipped`.
4. **Holder and purpose text with spaces, quotes, `& | ^ %` and non-ASCII** (Ukrainian) crosses ssh to a Windows
   `cmd.exe` intact — Task 3, `test_op_roundtrip_hostile_text`.
5. **The same agent claiming a box it already holds**: it must not queue behind itself — Task 2,
   `test_claim_by_holder_ticket_is_idempotent`; the client prints the held claim id with every grant so it can be
   reused.

---

### Task 1: The repository and the state store

**Files:**
- Create (in a new clone `~/projects/<tool>`): `README.md` (a stub: what it is; Task 6 fills it), `.gitignore`
  (`__pycache__/`), `<tool>.py` (entry: puts the repo folder on `sys.path`, calls `<tool>.cli.main()`),
  `<tool>/__init__.py`, `<tool>/store.py`, `tests/test_store.py`

**Interfaces:**
- Produces: `class Store(path: str)` with `mutex()` (context manager), `read_claim() -> dict | None`,
  `write_claim(d: dict | None)`, `read_queue() -> list[dict]` (sorted by `enqueued`, then `ticket`),
  `put_ticket(t: dict)`, `drop_ticket(ticket: str)`, `read_withdrawn() -> dict | None`, `write_withdrawn(d | None)`,
  `log(event: str, now: float, **fields)`; `class StoreBusy(Exception)`; `default_state_path() -> str`
  (`$<TOOL>_STATE`, else `~/.<tool>/state`).
- Layout of the state folder: `mutex/` (+ `mutex/info.json` {pid, host, at}), `claim.json`, `queue/<ticket>.json`,
  `withdrawn.json`, `log.txt` (one line per event: ISO time, event, `key=value` fields, values JSON-quoted).

- [ ] **Step 1:** Create the private repository (the maintainer's word, 2026-10-27): check that `gh`'s active
  account is the maintainer's, `gh repo create <tool> --private`, clone to `~/projects/<tool>`, and set
  `user.name`/`user.email` repo-locally before the first commit.
- [ ] **Step 2: Failing tests** in `tests/test_store.py`: `test_mutex_excludes` (two threads; a shared counter
  incremented read-sleep-write inside `mutex()` ends exact), `test_stale_mutex_is_broken` (a `mutex/` with
  `info.json` and mtime 31 s old is taken over; one 5 s old raises `StoreBusy` after the wait limit, patched to
  0.2 s), `test_writes_are_atomic` (a write goes through a temp file + `os.replace`; a leftover `*.tmp` is ignored by
  reads), `test_queue_order` (tickets read back in `enqueued` order), `test_log_line` (one line, fields quoted).
- [ ] **Step 3:** Run `python3 -m unittest discover -s tests -v` — FAIL (module missing).
- [ ] **Step 4:** Implement `<tool>/store.py` (mutex = `os.mkdir`; release = unlink `info.json`, `os.rmdir`).
- [ ] **Step 5:** Tests PASS; commit and push (`git add` explicit paths).

### Task 2: Claim rules

**Files:** Create `<tool>/claims.py`, `tests/test_claims.py`

**Interfaces:**
- Consumes: `Store` (Task 1).
- Produces: functions taking `(store: Store, now: float, ...)` and returning a response `dict`:
  `claim(store, now, ticket, holder, purpose, lease, ticket_ttl)`, `renew(store, now, id, lease=None)`,
  `release(store, now, id)`, `force_release(store, now, by, reason)`, `leave(store, now, ticket)`,
  `status(store, now)`, `withdraw(store, now, id, by, reason)`, `reinstate(store, now, by)`.
- Response shape: `{"state": "granted"|"waiting"|"withdrawn"|"released"|"renewed"|"not_holder"|"status"|…,
  "claim": {id, holder, purpose, since, renewed, lease, held_for, since_renewal, stale} | None,
  "position": int | None, "queue": [{ticket, holder, purpose, waited}], "withdrawn": {reason, by, since} | None,
  "now": float}`. `held_for`, `since_renewal`, `waited` are seconds computed here (box clock); `stale` is
  `now > renewed + lease`.

Rules (all under `store.mutex()`): drop tickets whose `seen + ticket_ttl < now` (log `expire`); a withdrawn box
answers `withdrawn` and drops the caller's ticket; a claim whose `id == ticket` answers `granted` (idempotent, renews
it); if the claim is stale and the caller is first in the queue (or the queue is empty), the claim is broken (log
`break` with the old holder); with no claim and the caller first, grant (`id = ticket`, ticket removed, log `claim`);
otherwise put/refresh the ticket (`seen = now`) and answer `waiting` with the 1-based position. `withdraw` requires
holding the claim, sets `withdrawn` and releases. Every grant, release, renew-failure, break, force, withdraw and
reinstate is logged.

- [ ] **Step 1: Failing tests**: `test_first_claim_granted`, `test_second_waits_with_position_and_holder`,
  `test_fifo_order_three_waiters`, `test_release_then_head_gets_it_others_still_wait`,
  `test_claim_by_holder_ticket_is_idempotent`, `test_renew_only_by_holder`,
  `test_stale_claim_broken_by_head_only` (the second waiter cannot break it; the head can; `log.txt` has `break`),
  `test_expired_head_ticket_is_skipped` (head not seen for 15 min + 1 s → the next waiter is granted),
  `test_force_release_logged_with_by_and_reason`, `test_withdraw_requires_holder_refuses_claims_until_reinstate`,
  `test_status_ages_from_box_clock` (`held_for == now - since`).
- [ ] **Step 2:** Run — FAIL.
- [ ] **Step 3:** Implement `<tool>/claims.py`.
- [ ] **Step 4:** Tests PASS; commit, push.

### Task 3: The box-side `op` command

**Files:** Create `<tool>/cli.py` (with `main(argv=None) -> int` and the `op` subcommand only), `tests/test_op.py`

**Interfaces:**
- Consumes: `claims.*`, `Store`, `default_state_path()`.
- Produces: `python3 <tool>.py op` — reads one JSON request `{"op": "claim"|"renew"|"release"|"force_release"|"leave"|
  "status"|"withdraw"|"reinstate", ...args}` from stdin, writes one JSON line `{"ok": true, ...response}` or
  `{"ok": false, "error": str}`; exit 0 unless the request is malformed (2). `now` = `$<TOOL>_NOW` or `time.time()`.

- [ ] **Step 1: Failing tests** (run `<tool>.py op` as a subprocess with `<TOOL>_STATE` in a temp folder):
  `test_op_claim_and_status`, `test_op_bad_request_exit_2`, `test_op_roundtrip_hostile_text` (holder
  `"агент \"q\" & | ^ %PATH%"` comes back identical in `status`; the raw output is pure ASCII),
  `test_concurrent_processes_one_winner` (8 subprocesses claim at once with distinct tickets → exactly one
  `granted`, 7 `waiting` with positions 1..7 in some order, no duplicates).
- [ ] **Step 2:** Run — FAIL. **Step 3:** Implement. **Step 4:** PASS; commit, push.

### Task 4: Box list, transports and the claim commands

**Files:** Create `<tool>/config.py`, `<tool>/transport.py`, `boxes.json`, `tests/fake_ssh.py`,
`tests/test_client.py`; modify `<tool>/cli.py`

**Interfaces:**
- `boxes.json` (in the repo, private): `{"boxes": {"<name>": {"ssh": "<alias>", "os": "linux"|"macos"|"windows",
  "remote": "<command that runs <tool>.py on the box>", "about": "<one line>", "libvirt": "<domain>" (optional)}}}`.
  Task 1–5 tests use their own `boxes.json` via `$<TOOL>_BOXES`; the real entries are written in Task 6.
- `config.load() -> dict`, `config.self_name() -> str | None` (`$<TOOL>_SELF`, else
  `~/.config/<tool>/self`, one line).
- `transport.for_box(name) -> Transport`; `Transport.op(req: dict) -> dict` (raises `Unreachable`);
  `Transport.run(argv: list[str], tty: bool) -> int`; `Transport.push(src, dst)`, `Transport.pull(src, dst)`.
  `LocalTransport` (box == self) calls `claims` directly; `SshTransport` runs
  `[ssh, "-T", "-o", "BatchMode=yes", "-o", "ConnectTimeout=10", alias, remote + " op"]` with the request on stdin;
  `ssh`/`scp` binaries are `$<TOOL>_SSH`/`$<TOOL>_SCP` or `ssh`/`scp` (tests point them at `tests/fake_ssh.py`,
  which runs the given remote command locally).
- CLI: `<tool> boxes`; `<tool> status [BOX] [--json]`; `<tool> claim BOX --purpose P [--holder H]
  [--wait DUR|forever] [--report-every DUR] [--lease DUR] [--ticket T]`; `<tool> renew BOX ID [--lease DUR]`;
  `<tool> release BOX ID`; `<tool> release BOX --force --reason R [--by H]`; `<tool> withdraw BOX ID --reason R`;
  `<tool> reinstate BOX`. DUR = `30s|10m|2h`. Holder default: `$<TOOL>_HOLDER`, else `user@host`. Wait default
  `forever`; no report interval by default.
- `claim` output on grant (stdout, one block): `claimed BOX id=ID lease=30m` and the release command; on exit 10:
  holder, purpose, held for, last renewal, position, waited, and `resume: <tool> claim BOX --ticket T …`; on 3: the
  same status, after sending `leave`. While unreachable it retries every poll and reports "unreachable since …".

- [ ] **Step 1: Failing tests**: `test_claim_grant_prints_id`, `test_wait_limit_exit_3_and_leaves_queue`,
  `test_report_every_exit_10_keeps_place_and_resume_gets_it`, `test_withdrawn_exit_4`,
  `test_wait_none_unreachable_exit_5`, `test_wait_survives_transport_errors` (fake ssh fails 3 polls, then the
  holder releases; the waiter is granted), `test_local_transport_when_self`, `test_status_all_boxes_table`,
  `test_force_release_needs_reason`. Poll interval patched small in tests (`$<TOOL>_POLL`).
- [ ] **Step 2:** Run — FAIL. **Step 3:** Implement. **Step 4:** PASS; commit, push.

### Task 5: Enforced box work: `run`, `push`, `pull`, `shell`

**Files:** Modify `<tool>/cli.py`, `<tool>/transport.py`; create `tests/test_enforced.py`

**Interfaces:**
- CLI: `<tool> run BOX ID [--] CMD...`, `<tool> push BOX ID SRC DST`, `<tool> pull BOX ID SRC DST`,
  `<tool> shell BOX ID`. `ID` may come from `$<TOOL>_CLAIM`. Each first renews; a `not_holder` answer exits 6
  with the current holder named, and nothing runs. While the command runs a thread renews every `lease / 3`; a
  renewal that fails prints a warning to stderr (the command is not killed). Exit code = the command's.
- `run` on ssh: `ssh -T alias CMD` (CMD joined for the box's shell as given; the doc says quoting is the caller's);
  `shell`: `ssh -t alias`; `push`/`pull`: `scp -r`. Local: `subprocess.run(CMD)`, `shutil.copytree`/`copy2`.

- [ ] **Step 1: Failing tests**: `test_run_without_claim_exit_6_nothing_runs` (a marker file is not created),
  `test_run_with_claim_returns_command_exit_code`, `test_run_renews_during_long_command` (lease 2 s, command sleeps
  5 s, `status` during and after shows not stale; a second claimant with `--wait 1s` gets exit 3),
  `test_lapsed_claim_refused` (claim, time passes beyond lease, someone else claims → `run` exits 6),
  `test_push_pull_local`.
- [ ] **Step 2:** Run — FAIL. **Step 3:** Implement. **Step 4:** PASS; commit, push.

### Task 6: The boxes join, and the instructions

**Files:** Modify `boxes.json`, `README.md` (<tool>); on the boxes: a clone of `<tool>` and a read-only deploy key

- [ ] **Step 1:** `boxes.json` gets the shared Windows VM and the Mac, with the aliases, users and paths from
  `AGENTS.local.md` (`remote`: Apple's `/usr/bin/python3` on the Mac; the scoop `python` on Windows, path through
  `%USERPROFILE%`).
- [ ] **Step 2:** Check each box is free (`AGENTS.local.md`'s checks). On each: a new key pair for this repository
  only, added with `gh repo deploy-key add … --title <box>` (read-only, the default), an ssh `Host` alias for it,
  the clone under the user's `projects` folder, `self` written in `~/.config/<tool>/self`. Nothing else on the box
  changes.
- [ ] **Step 3:** `README.md` for agents and the maintainer: what a claim is, the rules from the spec (everyone
  claims; holder and purpose; wait with a limit or forever, `--report-every` to be woken up, resume with
  `--ticket`; renew or use `run`; leave the box as found unless the goal was to change it; stale claims; force
  release only by the holder or the maintainer, with a reason; withdraw/reinstate; a stopped VM is started first,
  the tool never starts or stops one), the exit codes, adding a box, updating the clones (`git pull` on each box).
  Also: how an agent waits without blocking its shell (run `claim` in the background, or `--report-every` with the
  wait inside the shell's time limit).
- [ ] **Step 4: Live check** (from the desktop and on the Mac): claim the Windows VM from the desktop; a second
  claim with `--wait 20s` exits 3 naming the holder; a third with `--report-every 10s` exits 10 and resumes into the
  queue; `run` a harmless command (`hostname`) under the claim; release → the resumed waiter is granted; on the Mac,
  a local claim of the Mac while a desktop client waits for it (FIFO across hosts); `withdraw`/`reinstate` the Mac;
  `status` shows every step; both boxes end with no claim and empty queues. Record what was LIVE in the ledger.
- [ ] **Step 5:** Run the test suite on the Mac too (Python 3.9) under a claim, via `<tool> run`. Commit, push.

### Task 7: ttyak and the local files adopt the claims

**Files:** Modify `docs/agents/live-testing.md`, `docs/agents/local-template.md`, `AGENTS.md` (one hard-rule line),
`docs/PROGRESS.md`; outside git: `AGENTS.local.md`

- [ ] **Step 1:** `AGENTS.md` hard rules: "**Shared test boxes** are used only under a claim, through the claim tool
  named in `AGENTS.local.md`; no claim, no use." `live-testing.md`: the test-machine section says the same, links
  nothing private, and keeps the existing box-courtesy rules. `local-template.md`: fields for the claim tool's clone
  and the box names.
- [ ] **Step 2:** `AGENTS.local.md`: the test-machine section points at the `<tool>` clone and its README, and
  lists the box names; the old "check `query user`/`tasklist` before using it" stays as the check before claiming.
- [ ] **Step 3:** Ask the maintainer before editing the other project's git-ignored local file (spec Part 5); with
  their word, add the same pointer there.
- [ ] **Step 4:** `python3 tools/wiki/check_links.py`, `sh tools/check-name.sh .`, `tools/site/check_private.py` on a
  copy of the changed public docs. Commit (docs only), push.
- [ ] **Step 5:** Final whole-plan review (both repositories); deferred items filed as ttyak issues (public wording);
  `docs/PROGRESS.md` rewritten; the ledger's last entry.
