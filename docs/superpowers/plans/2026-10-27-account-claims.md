# Account claims Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** A ledger of ttyak's Slack test accounts and a claim per account, so only one test uses an account at a
time, and a claimed account's session can be put into a fake home (later: an instance) without anyone copying session
files by hand.

**Architecture:** One Python tool, `tools/accounts/accounts.py`, run on the desktop only. Its data lives in a
git-ignored folder in the checkout (`.accounts/`): `ledger.json` (the accounts, their profiles, roles and rules),
`sessions/<account>.json` (the master copies, 0600), and `claims/<account>/` (claim, queue, log). State updates are
serialised with `fcntl.flock` on a per-account lock file (one host, so no mkdir mutex). The claim semantics and exit
codes match the box claim tool's, so agents use both the same way.

**Tech Stack:** Python 3.9+ stdlib (`fcntl`, `json`, `argparse`, `hashlib`, `unittest`); Linux and macOS (the desktop).

**Spec:** [docs/superpowers/specs/2026-10-27-test-machines-design.md](../specs/2026-10-27-test-machines-design.md)
— Part 4 (ttyak test accounts) and the account lines of Part 5.

## Global Constraints

- Shell rules (every command an agent writes): bash, never fish (`NAME=value`); `set -euo pipefail` on multi-command
  scripts; never a recursive `rm`/`chmod`/`chown`/`mv`/`cp` (or `find -delete`) on a variable path — literal path, or
  `${VAR:?}` after checking it. In Python, never `shutil.rmtree`; the tool deletes only single files it recorded.
- Python 3.9 floor, standard library only, `from __future__ import annotations` in every module. Tests: `unittest`,
  added to CI's `Tool tests` step and its copy in `docs/agents/setup.md` in the same commit.
- **Secrets**: a session file's contents are never printed, logged, put in an error message or written anywhere but
  the master copy and an installed copy. Folders 0700, session files 0600, checked on every read; the tool refuses a
  session file that is group- or world-readable. Tests use fake sessions only.
- **Never write the maintainer's real ttyak folders** (`~/.config/ttyak` etc.): the tool reads them only in `import`;
  `install` refuses a target inside them. The data folder is `$TTYAK_ACCOUNTS` if set (tests always set it), else
  `.accounts/` next to `tools/`.
- Public wording: the tool's code, tests and docs are public; account names, profiles, user ids, workspace names live
  only in the git-ignored ledger. Tests use stand-ins (`A`, `B`).
- Claim semantics as the box tool (by design, so agents use one habit): holder + purpose; FIFO queue; lease 30 min,
  ticket TTL 15 min, poll 5 s (`$TTYAK_ACCOUNTS_POLL` in tests); `--wait DUR|forever` (default forever, `0` = one try),
  `--report-every DUR`, `--ticket T` to resume, `--lease DUR` (> 0); ids/tickets `[A-Za-z0-9_-]{1,64}` with
  `re.fullmatch`; a stale claim is broken by the first waiter; `release --force` needs `--reason`; every grant,
  release, break, force and expiry logged in `claims/<account>/log.txt`. Exit codes: 0 done, 2 usage/ledger error,
  3 wait limit, 6 no live claim, 10 report interval (still queued), 130 Ctrl-C (ticket left). An unknown account → 2
  listing the known ones.
- The ttyak binary does not rewrite `session.json` during a normal run (it writes only on sign-in, logout or a
  sign-in-again at start — `crates/ttyak/src/auth.rs`), so copies are not written back: a changed copy is reported at
  release, never copied over the master.

## Review Focus

1. **A session leaking**: an exception message, a `status` line, a log line or a test failure printing a token —
   Task 3, `test_no_session_bytes_in_any_output` (a fake session with a marker string; every command's stdout,
   stderr and the log are searched for it).
2. **Two claims on one account**: two processes claiming at once — Task 1, `test_concurrent_claims_one_winner`
   (8 processes).
3. **An install into the maintainer's real folders** through a symlink or `..` — Task 3,
   `test_install_refuses_real_config_paths` (resolved paths compared).
4. **A dead agent's claim**: lease expiry and the head waiter breaking it — Task 1, `test_stale_claim_broken_by_head`.
5. **A copy left behind**: `release` removes the installed copies it recorded (single files), keeps everything else,
   and says what it removed — Task 3, `test_release_removes_recorded_copies_only`.

---

### Task 1: Ledger and claim rules

**Files:** Create `tools/accounts/__init__.py` (empty), `tools/accounts/accounts.py` (library part),
`tools/accounts/test_accounts.py`; modify `.gitignore` (`/.accounts/`), `.github/workflows/ci.yml` (`Tool tests`),
`docs/agents/setup.md` (its copy of that step).

**Interfaces (produced):**
- `data_dir() -> Path`; `load_ledger(d: Path) -> dict` — `{"accounts": {NAME: {"profile": str, "role": str,
  "rules": [str], "session": "sessions/NAME.json"}}}`; raises `LedgerError` on a bad shape, an unknown key, a missing
  session file or loose permissions (folder not 0700, session not 0600).
- `Claims(d: Path, account: str)` with methods taking `now: Optional[float]` (None = read the clock inside the lock):
  `claim(now, ticket, holder, purpose, lease, ttl)`, `renew(now, id, lease=None)`, `release(now, id)`,
  `force_release(now, by, reason)`, `leave(now, ticket)`, `status(now)`, each returning
  `{"state", "claim", "position", "queue", "now"}` (claim: id, holder, purpose, since, renewed, lease, held_for,
  since_renewal, stale, installs) — the same rules as the box tool (expire tickets, FIFO, idempotent re-claim by the
  holder's ticket, stale break by the head only, `leave` releases only an untouched grant).
- The lock: `fcntl.flock(LOCK_EX)` on `claims/<account>/lock`, held only for one state update; writes atomic (temp file,
  fsync, `os.replace`).

- [ ] **Step 1: Failing tests**: `test_ledger_loads_and_rejects_bad_shapes`, `test_ledger_rejects_loose_permissions`,
  `test_first_claim_granted_second_waits_fifo`, `test_release_hands_over_to_head`,
  `test_reclaim_by_holder_ticket_is_idempotent`, `test_stale_claim_broken_by_head`,
  `test_expired_ticket_dropped`, `test_leave_releases_only_untouched_grant`, `test_force_release_logged`,
  `test_concurrent_claims_one_winner` (8 subprocesses, distinct tickets: exactly one granted, positions 1..7).
- [ ] **Step 2:** Run — FAIL. **Step 3:** Implement. **Step 4:** PASS (the whole `Tool tests` command); commit.

### Task 2: The claim commands

**Files:** Modify `tools/accounts/accounts.py` (CLI: `main(argv=None) -> int`), `tools/accounts/test_accounts.py`.

**Interfaces:** `accounts.py list` (name, profile, role, rules, claim state — never session data); `status
[ACCOUNT] [--json]` (no claim ids or tickets printed); `claim ACCOUNT --purpose P [--holder H] [--wait DUR|forever]
[--report-every DUR] [--lease DUR] [--ticket T]`; `renew ACCOUNT ID [--lease DUR]`; `release ACCOUNT ID`; `release
ACCOUNT --force --reason R [--by H]`. Printed command lines (`release:` on a grant, `resume:` on exit 10) run as
printed: `shlex.quote`d, starting with `sys.executable` and the tool's absolute path. Holder default
`$TTYAK_ACCOUNTS_HOLDER`, else `user@host`.

- [ ] **Step 1: Failing tests**: `test_list_shows_rules_not_sessions`, `test_claim_prints_runnable_release_line`,
  `test_wait_limit_exit_3_and_leaves_queue`, `test_report_every_exit_10_resume_keeps_place` (the resume line, run as
  printed, is granted after the release), `test_resume_line_carries_remaining_wait`, `test_bad_ticket_exit_2`,
  `test_unknown_account_exit_2_lists_known`, `test_status_hides_ids`, `test_ctrl_c_while_waiting_leaves_and_exits_130`.
- [ ] **Step 2:** Run — FAIL. **Step 3:** Implement. **Step 4:** PASS; commit.

### Task 3: Sessions in and out: `import`, `install`, release cleanup

**Files:** Modify `tools/accounts/accounts.py`, `tools/accounts/test_accounts.py`.

**Interfaces:**
- `import ACCOUNT --from PATH` — copies a `session.json` into `sessions/ACCOUNT.json` (0600; folder 0700) and records
  it in the ledger; refuses to overwrite an existing master without `--replace`; reads `PATH`, never writes it.
- `install ACCOUNT ID --home HOME [--profile NAME]` — requires the caller's live claim (exit 6 otherwise); writes the
  master to `HOME/.config/ttyak/<profile>/session.json` (default profile: the ledger's), creating folders 0700, file
  0600; refuses (exit 2) a target whose resolved path is inside the maintainer's real ttyak folders
  (`~/.config/ttyak`, `~/.local/share/ttyak`, `~/.cache/ttyak` and the macOS twins) or outside `HOME`; records
  `{path, sha256}` in the claim's `installs`.
- `release` (and `release --force`) removes each recorded installed copy whose file still exists (one `os.unlink`
  each — never a folder), reports a copy whose hash changed ("rewritten by ttyak — a sign-in? the master is
  unchanged"), and prints what it removed; `--keep` leaves the copies.

- [ ] **Step 1: Failing tests**: `test_import_copies_with_private_modes`, `test_import_refuses_overwrite`,
  `test_install_requires_claim`, `test_install_writes_private_copy_and_records_it`,
  `test_install_refuses_real_config_paths` (a symlink and a `..` path into a fake "real" folder via patched home),
  `test_release_removes_recorded_copies_only`, `test_release_reports_changed_copy`,
  `test_no_session_bytes_in_any_output`.
- [ ] **Step 2:** Run — FAIL. **Step 3:** Implement. **Step 4:** PASS; commit.

### Task 4: ttyak adopts the account claims (docs, the local ledger, a live check)

**Files:** Modify `AGENTS.md` (the "One test per Slack account" hard rule), `docs/agents/live-testing.md` ("Before
every live check", "A fake home"), `docs/agents/local-template.md` (the test-profile section points at the ledger),
`tools/README.md` (one entry), `docs/PROGRESS.md`; outside git: `.accounts/` and `AGENTS.local.md`.

- [ ] **Step 1 (docs, implementer):** the hard rule becomes "**One test per Slack account at a time**: claim the
  account with `tools/accounts/accounts.py` before using it, across all agents and machines". live-testing.md: a
  live check claims the accounts it uses and puts their sessions into the fake home with `install` (instead of
  copying session files by hand); `release` removes the copies; the account rules (DND restore, never sign out) are
  in the ledger and `list` shows them. Public wording only. Checks: links, names, the privacy check on a copy.
- [ ] **Step 2 (controller, with the maintainer's word):** create `.accounts/` and the ledger from `AGENTS.local.md`'s
  test-profile section (accounts A and B: profiles, roles, rules); `import` their sessions from the real config
  folder (read-only there); `AGENTS.local.md` points at the ledger and keeps only what is not in it.
- [ ] **Step 3 (controller, live):** `cargo build`; claim A; `install` into a fake home in the scratch folder; run a
  read-only ttyak command with that fake home and `--profile` (per live-testing.md) and see it signed in; a second
  claim of A from another holder waits; `release` removes the copy and hands A over; release everything. Ledger the
  LIVE/UNIT/NOT record.
- [ ] **Step 4:** Final whole-plan review; every real in-scope finding fixed (workflow.md); the ledger's last entry;
  `docs/PROGRESS.md` rewritten.

### Task 5: The remaining recipes use claimed accounts in fake homes (added 2026-10-27)

The maintainer's decision after the final review: no agent recipe uses the test profiles in the real config folder;
those copies stay there for the maintainer's own use.

**Files:** Modify `tools/launch.sh`, `tools/test_launch.py`, `docs/agents/live-testing.md`, `docs/agents/setup.md`,
and any other tracked doc or tool that runs ttyak with a test profile against the real config folder (search for
`--profile` and `ctl` in `docs/agents/`, `tools/`, `AGENTS.md`).

- `tools/launch.sh PROFILE [DM_NAME]` requires `TTYAK_FAKE_HOME` (an existing directory holding
  `.config/ttyak/PROFILE/session.json`, as `accounts.py install` makes it) and refuses to start without it (exit 2, a
  message saying to claim the account and `install` it first); the ttyak inside the kitty window runs with
  `HOME`, `XDG_CONFIG_HOME`, `XDG_DATA_HOME`, `XDG_CACHE_HOME` pointing into the fake home, set on that command only
  and passed through the environment as the profile already is (never interpolated into the quoted command).
- Every recipe that runs `ttyak --profile <A|B> …` (the DND check before a live check, `ctl post` as B, setup.md's
  `auth status` checks, others found by the search) shows the claim, the `install` and the fake-home environment on
  the ttyak command, or points at the "A fake home" recipe; none uses the real folder.
- Tests (`tools/test_launch.py`): refused without `TTYAK_FAKE_HOME`; refused when the fake home lacks the profile's
  session; the started command gets the fake-home environment (checked through the existing test's fake kitty).
