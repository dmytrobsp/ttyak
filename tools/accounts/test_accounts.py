from __future__ import annotations

import contextlib
import io
import json
import os
import re
import shlex
import stat
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

REPO = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(REPO))

from tools.accounts import accounts  # noqa: E402

MARKER = "FAKE-SESSION-MARKER"


def make_data(root: Path, names=("A", "B")) -> Path:
    """A fake data folder with correct modes; session files hold a marker only."""
    d = root / "data"
    d.mkdir(mode=0o700)
    os.chmod(d, 0o700)
    (d / "sessions").mkdir(mode=0o700)
    os.chmod(d / "sessions", 0o700)
    ledger = {"accounts": {}}
    for n in names:
        s = d / "sessions" / (n + ".json")
        s.write_text(MARKER)
        os.chmod(s, 0o600)
        ledger["accounts"][n] = {"profile": "p-" + n, "role": "test", "rules": ["no posts"],
                                 "session": "sessions/%s.json" % n}
    (d / "ledger.json").write_text(json.dumps(ledger))
    return d


class LedgerTests(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.addCleanup(self.tmp.cleanup)
        self.d = make_data(Path(self.tmp.name))

    def write(self, ledger):
        (self.d / "ledger.json").write_text(json.dumps(ledger))

    def test_ledger_loads_and_rejects_bad_shapes(self):
        led = accounts.load_ledger(self.d)
        self.assertEqual(sorted(led["accounts"]), ["A", "B"])
        self.assertEqual(led["accounts"]["A"]["session"], "sessions/A.json")
        good = json.loads((self.d / "ledger.json").read_text())

        def bad(mutate):
            led = json.loads(json.dumps(good))
            mutate(led)
            self.write(led)
            with self.assertRaises(accounts.LedgerError) as cm:
                accounts.load_ledger(self.d)
            self.assertNotIn(MARKER, str(cm.exception))

        bad(lambda l: l["accounts"]["A"].update(extra="x"))
        bad(lambda l: l["accounts"]["A"].pop("role"))
        bad(lambda l: l["accounts"]["A"].update(rules="no"))
        bad(lambda l: l["accounts"]["A"].update(rules=[1]))
        bad(lambda l: l["accounts"]["A"].update(profile=3))
        bad(lambda l: l["accounts"]["A"].update(session="sessions/B.json"))
        bad(lambda l: l["accounts"]["A"].update(session="../x.json"))
        bad(lambda l: l["accounts"].update({"bad name": l["accounts"]["A"]}))
        bad(lambda l: l.update(more=1))
        bad(lambda l: l.update(accounts=[]))
        (self.d / "ledger.json").write_text("{not json")
        with self.assertRaises(accounts.LedgerError):
            accounts.load_ledger(self.d)
        (self.d / "ledger.json").unlink()
        with self.assertRaises(accounts.LedgerError):
            accounts.load_ledger(self.d)

    def test_missing_session_file_rejected(self):
        (self.d / "sessions" / "A.json").unlink()
        with self.assertRaises(accounts.LedgerError) as cm:
            accounts.load_ledger(self.d)
        self.assertIn("A.json", str(cm.exception))

    def test_ledger_rejects_loose_permissions(self):
        os.chmod(self.d / "sessions" / "A.json", 0o640)
        with self.assertRaises(accounts.LedgerError) as cm:
            accounts.load_ledger(self.d)
        self.assertNotIn(MARKER, str(cm.exception))
        os.chmod(self.d / "sessions" / "A.json", 0o600)
        accounts.load_ledger(self.d)
        real = self.d / "sessions" / "real.bin"
        real.write_text(MARKER)
        os.chmod(real, 0o600)
        (self.d / "sessions" / "A.json").unlink()
        os.symlink(real, self.d / "sessions" / "A.json")
        with self.assertRaises(accounts.LedgerError):
            accounts.load_ledger(self.d)
        (self.d / "sessions" / "A.json").unlink()
        (self.d / "sessions" / "A.json").write_text(MARKER)
        os.chmod(self.d / "sessions" / "A.json", 0o600)
        os.chmod(self.d / "sessions", 0o755)
        with self.assertRaises(accounts.LedgerError):
            accounts.load_ledger(self.d)
        os.chmod(self.d / "sessions", 0o700)
        os.chmod(self.d, 0o750)
        with self.assertRaises(accounts.LedgerError):
            accounts.load_ledger(self.d)
        os.chmod(self.d, 0o700)

    def test_data_dir_env_and_default(self):
        old = os.environ.get("TTYAK_ACCOUNTS")
        self.addCleanup(lambda: os.environ.__setitem__("TTYAK_ACCOUNTS", old) if old is not None
                        else os.environ.pop("TTYAK_ACCOUNTS", None))
        os.environ["TTYAK_ACCOUNTS"] = "/some/where"
        self.assertEqual(accounts.data_dir(), Path("/some/where"))
        del os.environ["TTYAK_ACCOUNTS"]
        self.assertEqual(accounts.data_dir(), REPO / ".accounts")


class ClaimTests(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.addCleanup(self.tmp.cleanup)
        self.d = make_data(Path(self.tmp.name))
        self.c = accounts.Claims(self.d, "A")

    def claim(self, now, ticket, holder="h", purpose="p", lease=accounts.LEASE, ttl=accounts.TICKET_TTL):
        return self.c.claim(now, ticket, holder, purpose, lease, ttl)

    def test_first_claim_granted_second_waits_fifo(self):
        a = self.claim(1000, "t1", "one")
        self.assertEqual(a["state"], "granted")
        self.assertEqual(a["claim"]["id"], "t1")
        self.assertEqual(a["claim"]["installs"], [])
        self.assertEqual(sorted(a), ["claim", "now", "position", "queue", "state"])
        self.assertEqual(a["now"], 1000)
        b = self.claim(1001, "t2", "two")
        c = self.claim(1002, "t3", "three")
        self.assertEqual((b["state"], b["position"]), ("waiting", 1))
        self.assertEqual((c["state"], c["position"]), ("waiting", 2))
        self.assertEqual([q["ticket"] for q in c["queue"]], ["t2", "t3"])
        self.assertEqual(c["claim"]["held_for"], 2)
        self.assertFalse(c["claim"]["stale"])
        # re-polling keeps the place (enqueued is kept)
        b2 = self.claim(1010, "t2", "two")
        self.assertEqual(b2["position"], 1)
        self.assertEqual(b2["queue"][0]["waited"], 9)
        self.assertTrue((self.d / "claims" / "A" / "log.txt").exists())
        self.assertEqual(os.stat(self.d / "claims" / "A").st_mode & 0o777, 0o700)

    def test_release_hands_over_to_head(self):
        self.claim(1000, "t1")
        self.claim(1001, "t2")
        self.claim(1002, "t3")
        self.assertEqual(self.c.release(1003, "t2")["state"], "not_holder")
        self.assertEqual(self.c.release(1004, "t1")["state"], "released")
        self.assertIsNone(self.c.status(1005)["claim"])
        self.assertEqual(self.claim(1006, "t3")["state"], "waiting")  # not the head
        g = self.claim(1007, "t2")
        self.assertEqual(g["state"], "granted")
        self.assertEqual(g["claim"]["id"], "t2")
        self.assertEqual([q["ticket"] for q in g["queue"]], ["t3"])

    def test_reclaim_by_holder_ticket_is_idempotent(self):
        g = self.claim(1000, "t1", "one", "first")
        r = self.claim(1100, "t1", "other", "other")
        self.assertEqual(r["state"], "granted")
        self.assertEqual(r["claim"]["since"], g["claim"]["since"])
        self.assertEqual(r["claim"]["renewed"], 1100)
        self.assertEqual(r["claim"]["holder"], "one")
        self.assertEqual(r["queue"], [])
        n = self.c.renew(1200, "t1", lease=60)
        self.assertEqual(n["state"], "renewed")
        self.assertEqual((n["claim"]["lease"], n["claim"]["renewed"]), (60, 1200))
        self.assertEqual(self.c.renew(1201, "zz")["state"], "not_holder")

    def test_stale_claim_broken_by_head(self):
        self.claim(1000, "t1", "dead", lease=100)
        w = self.claim(1050, "t2", "w2")
        self.assertEqual(w["state"], "waiting")
        self.assertFalse(w["claim"]["stale"])
        self.claim(1101, "t3", "w3")
        s = self.c.status(1102, 100)
        self.assertTrue(s["claim"]["stale"])
        self.assertEqual(s["claim"]["id"], "t1")  # status does not break it
        # a non-head waiter does not break it
        self.assertEqual(self.claim(1103, "t3", "w3")["state"], "waiting")
        self.assertEqual(self.c.status(1104)["claim"]["id"], "t1")
        g = self.claim(1105, "t2", "w2")
        self.assertEqual(g["state"], "granted")
        self.assertEqual(g["claim"]["id"], "t2")
        self.assertEqual(g["claim"]["since"], 1105)
        log = (self.d / "claims" / "A" / "log.txt").read_text()
        self.assertIn("break", log)
        self.assertIn('reason="lease lapsed"', log)

    def test_expired_ticket_dropped(self):
        self.claim(1000, "t1")
        self.claim(1001, "t2", ttl=100)
        s = self.c.status(1050, 100)
        self.assertEqual(len(s["queue"]), 1)
        s = self.c.status(1102, 100)
        self.assertEqual(s["queue"], [])
        self.assertIn("expire", (self.d / "claims" / "A" / "log.txt").read_text())

    def test_leave_releases_only_untouched_grant(self):
        self.claim(1000, "t1")
        self.assertEqual(self.c.leave(1001, "t1")["state"], "left")
        self.assertIsNone(self.c.status(1002)["claim"])
        self.claim(1010, "t2")
        self.claim(1020, "t2")  # re-claim renews: in use
        self.c.leave(1021, "t2")
        self.assertEqual(self.c.status(1022)["claim"]["id"], "t2")
        self.claim(1030, "t3")  # waiting
        a = self.c.leave(1031, "t3")
        self.assertEqual(a["queue"], [])
        self.assertEqual(a["claim"]["id"], "t2")

    def test_leave_keeps_untouched_grant_with_installs(self):
        self.claim(1000, "t1")
        home = Path(self.tmp.name) / "home"
        home.mkdir(mode=0o700)
        self.assertEqual(self.c.install(1000, "t1", os.path.realpath(home), "p", b"x")["state"], "installed")
        self.assertEqual(self.c.status(1000)["claim"]["renewed"], 1000)  # untouched: renewed == since
        self.c.leave(1001, "t1")
        c = self.c.status(1002)["claim"]
        self.assertEqual(c["id"], "t1")
        self.assertEqual(len(c["installs"]), 1)

    def test_force_release_logged(self):
        self.assertEqual(self.c.force_release(1000, "me", "why")["state"], "no_claim")
        self.claim(1001, "t1", "victim")
        a = self.c.force_release(1002, "me", "stuck")
        self.assertEqual(a["state"], "released")
        self.assertIsNone(a["claim"])
        line = [l for l in (self.d / "claims" / "A" / "log.txt").read_text().splitlines() if " force " in l][0]
        self.assertIn('by="me"', line)
        self.assertIn('reason="stuck"', line)
        self.assertIn('holder="victim"', line)

    def test_folders_created_0700_and_missing_data_folder_refused(self):
        old = os.umask(0o002)
        self.addCleanup(os.umask, old)
        accounts.Claims(self.d, "A")
        self.assertEqual(os.stat(self.d / "claims").st_mode & 0o777, 0o700)
        self.assertEqual(os.stat(self.d / "claims" / "A").st_mode & 0o777, 0o700)
        missing = Path(self.tmp.name) / "nope"
        with self.assertRaises(accounts.LedgerError):
            accounts.Claims(missing, "A")
        self.assertFalse(missing.exists())
        os.chmod(self.d, 0o755)
        with self.assertRaises(accounts.LedgerError):
            accounts.Claims(self.d, "B")
        os.chmod(self.d, 0o700)

    def test_symlinks_refused(self):
        target = Path(self.tmp.name) / "elsewhere"
        target.mkdir(mode=0o755)
        os.chmod(target, 0o755)
        os.symlink(target, self.d / "claims" / "B")
        with self.assertRaises(accounts.LedgerError):
            accounts.Claims(self.d, "B")
        self.assertEqual(os.stat(target).st_mode & 0o777, 0o755)
        self.assertEqual(os.listdir(target), [])
        for name in ("lock", "log.txt", "state.json.tmp"):
            d2 = Path(self.tmp.name) / ("d2" + name.replace(".", ""))
            d2.mkdir(mode=0o700)
            data = make_data(d2)
            c = accounts.Claims(data, "A")
            victim = d2 / "victim"
            victim.write_text("keep")
            os.symlink(victim, c.dir / name)
            with self.assertRaises(OSError):
                c.claim(1000, "t1", "h", "p", 60, 60)
            self.assertEqual(victim.read_text(), "keep")

    def test_state_json_symlink_refused_and_not_read(self):
        fake = Path(self.tmp.name) / "fake-session"
        body = json.dumps({"claim": None, "queue": [], "m": MARKER})  # parses as a state, so only O_NOFOLLOW saves it
        fake.write_text(body)
        os.chmod(fake, 0o600)
        os.symlink(fake, self.c.dir / "state.json")
        with self.assertRaises(accounts.LedgerError) as cm:
            self.c.status(1000)
        self.assertNotIn(MARKER, str(cm.exception))
        self.assertEqual(fake.read_text(), body)
        self.assertNotIn(MARKER, (self.c.dir / "log.txt").read_text() if (self.c.dir / "log.txt").exists() else "")

    def test_bad_queue_entries_are_ledger_error(self):
        good = {"ticket": "t", "holder": "h", "purpose": "p", "enqueued": 1, "seen": 1}
        for q in (["x"], [{"ticket": "t"}], [dict(good, seen="no")], [dict(good, ticket="bad id")]):
            (self.c.dir / "state.json").write_text(json.dumps({"claim": None, "queue": q}))
            with self.assertRaises(accounts.LedgerError):
                self.c.status(1000)

    def test_bad_claim_record_is_ledger_error(self):
        good = {"id": "t", "holder": "h", "purpose": "p", "since": 1, "renewed": 1, "lease": 60, "installs": []}
        (self.c.dir / "state.json").write_text(json.dumps({"claim": good, "queue": []}))
        self.assertEqual(self.c.status(1000)["claim"]["id"], "t")
        for bad in ({}, {"id": "t"}, dict(good, id="bad id"), dict(good, lease="x"), dict(good, since=True),
                    dict(good, installs="no"), {k: v for k, v in good.items() if k != "renewed"}):
            (self.c.dir / "state.json").write_text(json.dumps({"claim": bad, "queue": []}))
            with self.assertRaises(accounts.LedgerError):
                self.c.status(1000)

    def test_state_open_error_message_names_errno(self):
        os.symlink("/nonexistent-target", self.c.dir / "state.json")
        with self.assertRaises(accounts.LedgerError) as cm:
            self.c.status(1000)
        self.assertIn("cannot open (not a regular file?)", str(cm.exception))
        self.assertRegex(str(cm.exception), r"[A-Z]{4,}")

    def test_folder_fsync_failure_still_logs(self):
        from unittest import mock
        real = os.fsync
        calls = []

        def flaky(fd):
            calls.append(fd)
            if stat.S_ISDIR(os.fstat(fd).st_mode):
                raise OSError("no fsync on dirs")
            return real(fd)
        with mock.patch.object(accounts.os, "fsync", flaky):
            self.assertEqual(self.claim(1000, "t1")["state"], "granted")
        self.assertIn("claim", (self.c.dir / "log.txt").read_text())

    def test_failed_write_leaves_no_tmp(self):
        from unittest import mock
        with mock.patch.object(accounts.json, "dump", side_effect=RuntimeError("boom")):
            with self.assertRaises(RuntimeError):
                self.claim(1000, "t1")
        self.assertFalse((self.c.dir / "state.json.tmp").exists())

    def test_corrupt_state_is_ledger_error(self):
        for bad in ("{oops", "[]"):
            (self.c.dir / "state.json").write_text(bad)
            with self.assertRaises(accounts.LedgerError) as cm:
                self.c.status(1000)
            self.assertIn("state.json", str(cm.exception))
        (self.c.dir / "state.json").write_text('{"claim": 3, "queue": []}')
        with self.assertRaises(accounts.LedgerError):
            self.c.status(1000)

    def test_ids_checked_everywhere(self):
        for call in (lambda: self.c.renew(1, "bad id"), lambda: self.c.release(1, "../x"),
                     lambda: self.c.leave(1, "")):
            with self.assertRaises(ValueError):
                call()

    def test_clock_read_inside_lock_when_now_none(self):
        a = self.c.claim(None, "t1", "h", "p", 60, 60)
        self.assertEqual(a["state"], "granted")
        self.assertIsInstance(a["now"], float)

    def test_accounts_are_independent(self):
        self.claim(1000, "t1")
        other = accounts.Claims(self.d, "B")
        self.assertEqual(other.claim(1000, "t1", "h", "p", 60, 60)["state"], "granted")

    def test_concurrent_claims_one_winner(self):
        code = ("import sys, json; sys.path.insert(0, sys.argv[1]);"
                "from pathlib import Path; from tools.accounts import accounts;"
                "c = accounts.Claims(Path(sys.argv[2]), 'A');"
                "r = c.claim(None, sys.argv[3], 'h', 'p', 1800, 900);"
                "print(json.dumps([r['state'], r['position']]))")
        procs = [subprocess.Popen([sys.executable, "-c", code, str(REPO), str(self.d), "t%d" % i],
                                  stdout=subprocess.PIPE, text=True) for i in range(8)]
        out = []
        for p in procs:
            o, _ = p.communicate(timeout=60)
            self.assertEqual(p.returncode, 0)
            out.append(json.loads(o))
        self.assertEqual(sum(1 for s, _ in out if s == "granted"), 1)
        waiting = sorted(pos for s, pos in out if s == "waiting")
        self.assertEqual(waiting, list(range(1, 8)))
        # the positions at answer time are distinct, and the final queue holds 7
        self.assertEqual(len(self.c.status(None)["queue"]), 7)


TOOL = Path(accounts.__file__).resolve()


class CliBase(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.addCleanup(self.tmp.cleanup)
        self.d = make_data(Path(self.tmp.name))
        self.env = dict(os.environ, TTYAK_ACCOUNTS=str(self.d), TTYAK_ACCOUNTS_POLL="0.05",
                        TTYAK_ACCOUNTS_HOLDER="tester")

    def run_cli(self, *args, timeout=60):
        p = subprocess.run([sys.executable, str(TOOL)] + list(args), env=self.env, capture_output=True,
                           text=True, timeout=timeout)
        return p.returncode, p.stdout, p.stderr

    def inproc(self, *args):
        out, err = io.StringIO(), io.StringIO()
        from unittest import mock
        with mock.patch.dict(os.environ, self.env), contextlib.redirect_stdout(out), \
                contextlib.redirect_stderr(err):
            rc = accounts.main(list(args))
        return rc, out.getvalue(), err.getvalue()

    def grant(self, account="A", *extra):
        rc, out, err = self.run_cli("claim", account, "--purpose", "holding", *extra)
        self.assertEqual(rc, 0, err)
        line = [l for l in out.splitlines() if l.startswith("release: ")][0]
        return re.search(r"id=(\S+)", out).group(1), line[len("release: "):]

    def queue_len(self, account="A"):
        rc, out, _ = self.run_cli("status", account, "--json")
        self.assertEqual(rc, 0)
        return len(json.loads(out)[account]["queue"])


class CliTests(CliBase):
    def test_list_shows_rules_not_sessions(self):
        rc, out, err = self.run_cli("list")
        self.assertEqual(rc, 0, err)
        for want in ("A", "B", "p-A", "test", "no posts", "free"):
            self.assertIn(want, out)
        self.assertNotIn(MARKER, out + err)
        self.grant("A")
        self.assertIn("held by tester for holding", self.run_cli("list")[1])

    def test_claim_prints_runnable_release_line(self):
        cid, line = self.grant("A")
        argv = shlex.split(line)
        self.assertEqual(argv[0], sys.executable)
        self.assertEqual(argv[1], str(TOOL))
        self.assertEqual(argv[2:], ["release", "A", cid])
        self.assertIn("held by tester", self.run_cli("list")[1])
        p = subprocess.run(argv, env=self.env, capture_output=True, text=True)
        self.assertEqual(p.returncode, 0, p.stderr)
        self.assertIn("released A", p.stdout)
        self.assertIn("free", self.run_cli("list")[1])
        self.assertEqual(self.run_cli("release", "A", cid)[0], 6)

    def test_wait_limit_exit_3_and_leaves_queue(self):
        self.grant("A")
        rc, out, _ = self.run_cli("claim", "A", "--purpose", "x", "--wait", "0")
        self.assertEqual(rc, 3)
        self.assertIn("holder: tester", out)
        self.assertEqual(self.queue_len(), 0)
        self.assertEqual(self.run_cli("claim", "A", "--purpose", "x", "--wait", "0.3s")[0], 3)
        self.assertEqual(self.queue_len(), 0)

    def test_report_every_exit_10_resume_keeps_place(self):
        cid, release = self.grant("A")
        rc, out, _ = self.run_cli("claim", "A", "--purpose", "later one", "--report-every", "0.2")
        self.assertEqual(rc, 10)
        self.assertIn("your position: 1 of 1", out)
        resume = [l for l in out.splitlines() if l.startswith("resume: ")][0][len("resume: "):]
        argv = shlex.split(resume)
        self.assertEqual((argv[0], argv[1]), (sys.executable, str(TOOL)))
        self.assertIn("--purpose=later one", argv)
        self.assertEqual(self.queue_len(), 1)
        self.assertEqual(subprocess.run(shlex.split(release), env=self.env, capture_output=True).returncode, 0)
        p = subprocess.run(argv, env=self.env, capture_output=True, text=True, timeout=60)
        self.assertEqual(p.returncode, 0, p.stderr)
        self.assertIn("claimed A", p.stdout)
        self.assertEqual(self.queue_len(), 0)

    def test_resume_line_carries_remaining_wait(self):
        self.grant("A")
        rc, out, _ = self.run_cli("claim", "A", "--purpose", "x", "--wait", "60", "--report-every", "0.2",
                                  "--lease", "5m", "--holder", "me")
        self.assertEqual(rc, 10)
        argv = shlex.split([l for l in out.splitlines() if l.startswith("resume: ")][0][len("resume: "):])
        left = float(argv[argv.index("--wait") + 1].rstrip("s"))
        self.assertTrue(0 < left < 60, left)
        self.assertEqual(argv[argv.index("--lease") + 1], "5m")
        self.assertIn("--holder=me", argv)
        self.assertEqual(argv[argv.index("--report-every") + 1], "0.2")
        self.run_cli("claim", "A", "--purpose", "x", "--wait", "0", "--ticket", argv[argv.index("--ticket") + 1])

    def test_bad_ticket_exit_2(self):
        for t in ("", "bad id", "../x", "x" * 65, "a\n"):
            rc, _, err = self.run_cli("claim", "A", "--purpose", "x", "--ticket", t)
            self.assertEqual(rc, 2, t)
            self.assertIn("--ticket", err)
        self.assertEqual(self.queue_len(), 0)
        self.assertEqual(self.run_cli("status", "A")[1].count("free"), 1)
        for bad in (["--lease", "0"], ["--lease", "0s"], ["--lease", "-5"], ["--wait", "soon"],
                    ["--report-every", "x"]):
            self.assertEqual(self.run_cli("claim", "A", "--purpose", "x", *bad)[0], 2, bad)
        self.assertEqual(self.run_cli("renew", "A", "bad id")[0], 2)
        self.assertEqual(self.run_cli("release", "A", "../x")[0], 2)

    def test_unknown_account_exit_2_lists_known(self):
        for args in (("claim", "Z", "--purpose", "x"), ("status", "Z"), ("release", "Z", "id1"),
                     ("renew", "Z", "id1"), ("release", "Z", "--force", "--reason", "r")):
            rc, out, err = self.run_cli(*args)
            self.assertEqual(rc, 2, args)
            self.assertIn("A, B", err)
        self.assertEqual(self.run_cli("claim", "bad name", "--purpose", "x")[0], 2)

    def test_ledger_errors_exit_2_without_contents(self):
        os.chmod(self.d / "sessions" / "A.json", 0o644)
        for args in (("list",), ("status",), ("claim", "A", "--purpose", "x")):
            rc, out, err = self.run_cli(*args)
            self.assertEqual(rc, 2, args)
            self.assertIn("A.json", err)
            self.assertNotIn(MARKER, out + err)
            self.assertNotIn("Traceback", err)

    def test_status_hides_ids(self):
        cid, _ = self.grant("A")
        rc, out, err = self.run_cli("claim", "A", "--purpose", "w", "--wait", "0", "--ticket", "waiter-ticket-1")
        self.assertEqual(rc, 3)
        self.run_cli("claim", "A", "--purpose", "w2", "--report-every", "0.1", "--ticket", "waiter-ticket-2")
        rc, text, _ = self.run_cli("status")
        self.assertEqual(rc, 0)
        self.assertIn("holding", text)
        rc, js, _ = self.run_cli("status", "--json")
        data = json.loads(js)
        self.assertEqual(sorted(data), ["A", "B"])
        self.assertEqual(data["A"]["claim"]["holder"], "tester")
        self.assertEqual(len(data["A"]["queue"]), 1)
        for blob in (text, js, self.run_cli("status", "A")[1], self.run_cli("list")[1]):
            self.assertNotIn(cid, blob)
            self.assertNotIn("waiter-ticket-2", blob)
            self.assertNotIn("installs", blob)
        self.assertNotIn("id", data["A"]["claim"])
        self.assertNotIn("ticket", data["A"]["queue"][0])

    def test_status_marks_stale(self):
        self.grant("A", "--lease", "0.2s")
        import time
        time.sleep(0.4)
        self.assertIn("(stale)", self.run_cli("status")[1])
        self.assertIn("stale", self.run_cli("list")[1])
        self.assertTrue(json.loads(self.run_cli("status", "--json")[1])["A"]["claim"]["stale"])

    def test_ctrl_c_while_waiting_leaves_and_exits_130(self):
        self.grant("A")
        from unittest import mock
        with mock.patch.object(accounts.time, "sleep", side_effect=KeyboardInterrupt):
            rc, out, err = self.inproc("claim", "A", "--purpose", "x", "--ticket", "w1")
        self.assertEqual(rc, 130)
        self.assertNotIn("Traceback", err)
        self.assertEqual(self.queue_len(), 0)

    def test_renew_release_force(self):
        cid, _ = self.grant("A")
        rc, out, _ = self.run_cli("renew", "A", cid, "--lease", "10m")
        self.assertEqual(rc, 0)
        self.assertIn("renewed A", out)
        self.assertEqual(self.run_cli("renew", "A", "nope")[0], 6)
        self.assertEqual(self.run_cli("release", "A", "nope")[0], 6)
        self.assertEqual(self.run_cli("release", "A", "--force")[0], 2)
        self.assertEqual(self.run_cli("release", "A")[0], 2)
        self.assertEqual(self.run_cli("renew", "A", cid, "--lease", "0")[0], 2)
        rc, out, _ = self.run_cli("release", "A", "--force", "--reason", "stuck", "--by", "boss")
        self.assertEqual((rc, out.strip()), (0, "released A (forced)"))
        rc, out, _ = self.run_cli("release", "A", "--force", "--reason", "again")
        self.assertEqual((rc, out.strip()), (0, "no claim on A"))
        log = (self.d / "claims" / "A" / "log.txt").read_text()
        self.assertIn('by="boss"', log)
        self.assertIn('reason="stuck"', log)

    def cli_dir(self):
        self.run_cli("status", "A")  # creates claims/A
        return self.d / "claims" / "A"

    def test_fifo_or_folder_at_state_files_exit_2_without_hanging(self):
        d = self.cli_dir()
        (d / "state.json").unlink()
        os.mkfifo(d / "state.json")
        for args in (("status", "A"), ("claim", "A", "--purpose", "x", "--wait", "0")):
            self.assertEqual(self.run_cli(*args, timeout=20)[0], 2, args)
        os.unlink(d / "state.json")
        os.mkdir(d / "state.json")
        rc, _, err = self.run_cli("status", "A", timeout=20)
        self.assertEqual(rc, 2)
        self.assertIn("state.json", err)
        os.rmdir(d / "state.json")
        for name in ("state.json.tmp", "log.txt"):
            os.mkfifo(d / name)
            rc, _, err = self.run_cli("claim", "A", "--purpose", "x", "--wait", "0", timeout=20)
            self.assertEqual(rc, 2, name)
            self.assertNotIn("Traceback", err)
            if (d / name).exists():
                os.unlink(d / name)
            self.run_cli("release", "A", "--force", "--reason", "cleanup")

    @unittest.skipIf(os.name == "nt", "POSIX signals")
    def test_real_sigint_while_waiting(self):
        import signal
        import time
        self.grant("A")
        p = subprocess.Popen([sys.executable, str(TOOL), "claim", "A", "--purpose", "x", "--ticket", "w1"],
                             env=self.env, stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True)
        self.addCleanup(p.wait)
        self.addCleanup(p.kill)
        deadline = time.monotonic() + 30
        while self.queue_len() != 1:
            self.assertLess(time.monotonic(), deadline)
            time.sleep(0.05)
        p.send_signal(signal.SIGINT)
        _, err = p.communicate(timeout=30)
        self.assertEqual(p.returncode, 130)
        self.assertIn("interrupted", err)
        self.assertNotIn("Traceback", err)
        self.assertEqual(self.queue_len(), 0)

    def test_resume_line_survives_dash_values(self):
        self.grant("A")
        rc, out, _ = self.run_cli("claim", "A", "--purpose=-x y", "--holder=-me", "--report-every", "0.1")
        self.assertEqual(rc, 10)
        argv = shlex.split([l for l in out.splitlines() if l.startswith("resume: ")][0][len("resume: "):])
        self.assertIn("--purpose=-x y", argv)
        self.assertIn("--holder=-me", argv)
        p = subprocess.run(argv[:argv.index("--report-every")] + ["--wait", "0"], env=self.env,
                           capture_output=True, text=True)
        self.assertEqual(p.returncode, 3, p.stderr)

    def test_subsecond_lease_shown(self):
        rc, out, _ = self.run_cli("claim", "A", "--purpose", "x", "--lease", "0.2s")
        self.assertEqual(rc, 0)
        self.assertIn("lease=0.2s", out)
        self.assertEqual(accounts.fmt(0.5), "0.5s")
        self.assertEqual(accounts.fmt(0), "0s")
        self.assertEqual(accounts.fmt(1800), "30m")

    def test_hide_keys_whitelists(self):
        ans = {"claim": {"id": "i", "holder": "h", "purpose": "p", "since": 1, "renewed": 1, "lease": 5,
                         "installs": [1], "future": "x", "held_for": 1, "since_renewal": 1, "stale": False},
               "queue": [{"ticket": "t", "holder": "h", "purpose": "p", "waited": 1, "other": 2}],
               "position": None, "state": "status", "now": 3}
        out = accounts._hide_keys(ans)
        self.assertEqual(sorted(out["claim"]), ["held_for", "holder", "lease", "purpose", "renewed", "since",
                                                 "since_renewal", "stale"])
        self.assertEqual(sorted(out["queue"][0]), ["holder", "purpose", "waited"])

    def test_control_characters_replaced_on_display(self):
        esc = "\x1b[31mred\x1b[0m\nline"
        self.run_cli("claim", "A", "--purpose", esc, "--holder", esc)
        for args in (("list",), ("status",), ("status", "A")):
            out = self.run_cli(*args)[1]
            self.assertNotIn("\x1b", out, args)
            self.assertIn("?[31mred", out)
        rc, out, _ = self.run_cli("claim", "B", "--purpose", esc)
        self.assertNotIn("\x1b", out)
        self.run_cli("claim", "A", "--purpose", esc, "--wait", "0")
        self.assertNotIn("\x1b", self.run_cli("claim", "A", "--purpose", esc, "--wait", "0")[1])
        js = json.loads(self.run_cli("status", "--json")[1])
        self.assertIn("\x1b", js["A"]["claim"]["purpose"])  # raw data stays raw in JSON

    def test_default_holder_without_env(self):
        del self.env["TTYAK_ACCOUNTS_HOLDER"]
        self.run_cli("claim", "A", "--purpose", "x")
        holder = json.loads(self.run_cli("status", "--json")[1])["A"]["claim"]["holder"]
        self.assertRegex(holder, r".+@.+")


def files_under(root: Path) -> list:
    """Every non-folder entry under root, symlinks not followed."""
    out = []
    for dirpath, dirnames, filenames in os.walk(root):
        for n in filenames + [d for d in dirnames if os.path.islink(os.path.join(dirpath, d))]:
            out.append(os.path.join(dirpath, n))
    return sorted(out)


class SessionTests(CliBase):
    """import, install and the release cleanup. Fake sessions only; the "real" ttyak folders live under a fake
    real home ($TTYAK_ACCOUNTS_REAL_HOME), so nothing here comes near the maintainer's own folders."""

    def setUp(self):
        super().setUp()
        self.root = Path(self.tmp.name)
        self.real = self.root / "realhome"
        (self.real / ".config" / "ttyak" / "p-A").mkdir(parents=True)
        (self.real / ".config" / "ttyak" / "p-A" / "keep.txt").write_text("keep")
        self.env["TTYAK_ACCOUNTS_REAL_HOME"] = str(self.real)
        for v in ("XDG_CONFIG_HOME", "XDG_DATA_HOME", "XDG_CACHE_HOME"):
            self.env.pop(v, None)
        self.master = self.d / "sessions" / "A.json"

    def home(self, name="home") -> Path:
        h = self.root / name
        h.mkdir(mode=0o700)
        return h

    def target(self, home: Path, profile="p-A") -> Path:
        return Path(os.path.realpath(home)) / ".config" / "ttyak" / profile / "session.json"

    def state(self, account="A") -> dict:
        return json.loads((self.d / "claims" / account / "state.json").read_text())

    def installs(self, account="A") -> list:
        c = self.state(account)["claim"]
        return c["installs"] if c else []

    def log(self, account="A") -> str:
        p = self.d / "claims" / account / "log.txt"
        return p.read_text() if p.exists() else ""

    def umask0(self):
        old = os.umask(0)
        self.addCleanup(os.umask, old)

    def source(self, body=MARKER + "-import", mode=0o600) -> Path:
        src = self.root / "src"
        src.mkdir(exist_ok=True)
        p = src / "session.json"
        p.write_text(body)
        os.chmod(p, mode)
        return p

    # -- import --------------------------------------------------------------------------------------------------

    def test_import_copies_with_private_modes(self):
        self.umask0()
        self.master.unlink()  # a first import: the ledger names the account, its master is not there yet
        src = self.source()
        before = os.stat(src)
        rc, out, err = self.run_cli("import", "A", "--from", str(src))
        self.assertEqual(rc, 0, err)
        self.assertIn("imported A", out)
        self.assertEqual(self.master.read_text(), MARKER + "-import")
        self.assertEqual(stat.S_IMODE(os.lstat(self.master).st_mode), 0o600)
        self.assertEqual(stat.S_IMODE(os.lstat(self.d / "sessions").st_mode), 0o700)
        self.assertEqual(sorted(os.listdir(self.d / "sessions")), ["A.json", "B.json"])  # no temp file left
        after = os.stat(src)
        self.assertEqual((before.st_mtime_ns, before.st_mode), (after.st_mtime_ns, after.st_mode))
        self.assertEqual(src.read_text(), MARKER + "-import")
        led = accounts.load_ledger(self.d)
        self.assertEqual(led["accounts"]["A"]["session"], "sessions/A.json")
        # the source is read like a master: private, regular, no symlink, no FIFO
        os.chmod(src, 0o644)
        rc, _, err = self.run_cli("import", "B", "--from", str(src), "--replace")
        self.assertEqual(rc, 2)
        self.assertIn("group or others", err)
        os.chmod(src, 0o600)
        link = self.root / "src" / "link.json"
        os.symlink(src, link)
        fifo = self.root / "src" / "fifo.json"
        os.mkfifo(fifo)
        for bad in (link, fifo, self.root / "src", self.root / "src" / "missing.json"):
            rc, _, err = self.run_cli("import", "B", "--from", str(bad), "--replace", timeout=20)
            self.assertEqual(rc, 2, bad)
            self.assertNotIn("Traceback", err)
        self.assertEqual((self.d / "sessions" / "B.json").read_text(), MARKER)
        rc, _, err = self.run_cli("import", "Z", "--from", str(src))
        self.assertEqual(rc, 2)
        self.assertIn("A, B", err)
        self.assertFalse((self.d / "sessions" / "Z.json").exists())

    def test_import_refuses_overwrite(self):
        self.umask0()
        src = self.source()
        rc, _, err = self.run_cli("import", "A", "--from", str(src))
        self.assertEqual(rc, 2)
        self.assertIn("--replace", err)
        self.assertEqual(self.master.read_text(), MARKER)
        rc, out, err = self.run_cli("import", "A", "--from", str(src), "--replace")
        self.assertEqual(rc, 0, err)
        self.assertEqual(self.master.read_text(), MARKER + "-import")
        self.assertEqual(stat.S_IMODE(os.lstat(self.master).st_mode), 0o600)
        self.assertEqual(sorted(os.listdir(self.d / "sessions")), ["A.json", "B.json"])

    # -- install -------------------------------------------------------------------------------------------------

    def test_install_requires_claim(self):
        h = self.home()
        rc, _, err = self.run_cli("install", "A", "noclaim", "--home", str(h))
        self.assertEqual(rc, 6, err)
        cid, _ = self.grant("A")
        self.assertEqual(self.run_cli("install", "A", "wrong-id", "--home", str(h))[0], 6)
        self.assertEqual(self.run_cli("install", "B", cid, "--home", str(h))[0], 6)  # the id of another account
        self.assertEqual(self.run_cli("install", "A", "bad id", "--home", str(h))[0], 2)
        self.assertEqual(self.run_cli("release", "A", cid)[0], 0)
        self.assertEqual(self.run_cli("install", "A", cid, "--home", str(h))[0], 6)
        cid, _ = self.grant("A", "--lease", "0.2s")
        import time
        time.sleep(0.4)
        rc, _, err = self.run_cli("install", "A", cid, "--home", str(h))
        self.assertEqual(rc, 6)  # a lapsed claim is not a live one
        self.assertEqual(os.listdir(h), [])  # not even a folder made
        self.assertEqual(self.installs(), [])

    def test_install_writes_private_copy_and_records_it(self):
        self.umask0()
        h = self.home()
        cid, _ = self.grant("A")
        rc, out, err = self.run_cli("install", "A", cid, "--home", str(h))
        self.assertEqual(rc, 0, err)
        t = self.target(h)
        self.assertIn(str(t), out)
        self.assertEqual(t.read_text(), MARKER)
        self.assertEqual(stat.S_IMODE(os.lstat(t).st_mode), 0o600)
        for p in (h / ".config", h / ".config" / "ttyak", h / ".config" / "ttyak" / "p-A"):
            self.assertEqual(stat.S_IMODE(os.lstat(p).st_mode), 0o700, p)
        import hashlib
        sha = hashlib.sha256(MARKER.encode()).hexdigest()
        self.assertEqual(self.installs(), [{"path": str(t), "sha256": sha}])
        self.assertIn("install", self.log())
        # again into the same place: the recorded copy is replaced, still one record
        rc, _, err = self.run_cli("install", "A", cid, "--home", str(h))
        self.assertEqual(rc, 0, err)
        self.assertEqual(len(self.installs()), 1)
        # another profile name: a second record
        rc, _, err = self.run_cli("install", "A", cid, "--home", str(h), "--profile", "other")
        self.assertEqual(rc, 0, err)
        self.assertEqual(self.target(h, "other").read_text(), MARKER)
        self.assertEqual(len(self.installs()), 2)
        for bad in ("../x", "a/b", "", ".."):
            self.assertEqual(self.run_cli("install", "A", cid, "--home", str(h), "--profile", bad)[0], 2, bad)
        # a session.json this claim did not write is never overwritten
        h2 = self.home("home2")
        (h2 / ".config" / "ttyak" / "p-A").mkdir(parents=True)
        (h2 / ".config" / "ttyak" / "p-A" / "session.json").write_text("someone else's")
        rc, _, err = self.run_cli("install", "A", cid, "--home", str(h2))
        self.assertEqual(rc, 2)
        self.assertIn("exists", err)
        self.assertEqual((h2 / ".config" / "ttyak" / "p-A" / "session.json").read_text(), "someone else's")
        # a fake home reached through a symlink is fine; the record holds the resolved path
        link = self.root / "homelink"
        h3 = self.home("home3")
        os.symlink(h3, link)
        rc, _, err = self.run_cli("install", "A", cid, "--home", str(link))
        self.assertEqual(rc, 0, err)
        self.assertIn(str(self.target(h3)), [i["path"] for i in self.installs()])
        # a master that went loose is refused
        os.chmod(self.master, 0o644)
        self.assertEqual(self.run_cli("install", "A", cid, "--home", str(self.home("home4")))[0], 2)
        self.assertEqual(os.listdir(self.root / "home4"), [])

    def test_install_refuses_real_config_paths(self):
        cid, _ = self.grant("A")
        r = self.real
        other = self.home("other")
        link = self.root / "reallink"
        os.symlink(r, link)
        h1 = self.home("h1")
        os.symlink(r / ".config", h1 / ".config")
        h2 = self.home("h2")
        (h2 / ".config").mkdir()
        os.symlink(r / ".config" / "ttyak", h2 / ".config" / "ttyak")
        h3 = self.home("h3")
        (h3 / ".config" / "ttyak").mkdir(parents=True)
        os.symlink(r / ".config" / "ttyak" / "p-A", h3 / ".config" / "ttyak" / "p-A")
        (r / ".local" / "share" / "ttyak").mkdir(parents=True)
        (r / "Library" / "Application Support" / "ttyak").mkdir(parents=True)
        cases = [str(r), str(other) + "/../realhome", str(link), str(h1), str(h2), str(h3),
                 str(r / ".config" / "ttyak"), str(r / ".config" / "ttyak") + "/../ttyak/p-A",
                 str(r / ".local" / "share" / "ttyak"), str(r / "Library" / "Application Support" / "ttyak"),
                 str(self.root / "missing"), str(self.master)]
        for home in cases:
            rc, out, err = self.run_cli("install", "A", cid, "--home", home)
            self.assertEqual(rc, 2, (home, out, err))
            self.assertNotIn("Traceback", err)
        # a symlink on the way out of HOME to a harmless place is refused too: the target must lie inside HOME
        outside = self.home("outside")
        for i, hop in enumerate(((".config",), (".config", "ttyak"), (".config", "ttyak", "p-A"))):
            h5 = self.home("h5-%d" % i)
            if len(hop) > 1:
                (h5.joinpath(*hop[:-1])).mkdir(parents=True)
            os.symlink(outside, h5.joinpath(*hop))
            rc, _, err = self.run_cli("install", "A", cid, "--home", str(h5))
            self.assertEqual(rc, 2, hop)
            self.assertIn("not a plain folder", err)
            self.assertEqual(os.listdir(outside), [])
        # $XDG_CONFIG_HOME/ttyak is a real folder too
        h4 = self.home("h4")
        rc, _, err = self.run_cli("install", "A", cid, "--home", str(h4))
        self.assertEqual(rc, 0, err)  # allowed without it...
        self.env["XDG_CONFIG_HOME"] = str(h4 / ".config")
        rc, _, err = self.run_cli("install", "A", cid, "--home", str(h4), "--profile", "other")
        self.assertEqual(rc, 2)  # ...refused with it
        self.assertFalse((h4 / ".config" / "ttyak" / "other").exists())
        del self.env["XDG_CONFIG_HOME"]
        # nothing written into the "real" folders, nothing recorded there
        self.assertEqual(files_under(r), [str(r / ".config" / "ttyak" / "p-A" / "keep.txt")])
        self.assertEqual([i["path"] for i in self.installs()], [str(self.target(h4))])
        self.assertEqual(os.listdir(other), [])

    def test_install_refuses_real_folder_behind_a_symlinked_config(self):
        # dotfiles: the real home's .config is a symlink to a folder that a --home can name directly
        dots = self.home("dotfiles")
        (dots / ".config").mkdir()  # no ttyak folder yet: only the resolved path names it, no inode to compare
        real2 = self.home("realhome2")
        os.symlink(dots / ".config", real2 / ".config")
        self.env["TTYAK_ACCOUNTS_REAL_HOME"] = str(real2)
        cid, _ = self.grant("A")
        rc, _, err = self.run_cli("install", "A", cid, "--home", str(dots))
        self.assertEqual(rc, 2, err)
        self.assertIn("real folder", err)
        self.assertEqual(files_under(dots), [])
        self.assertFalse((dots / ".config" / "ttyak").exists())

    def test_install_refuses_real_folders_not_made_yet(self):
        fresh = self.home("fresh")  # a new machine: the real home has no .config yet
        self.env["TTYAK_ACCOUNTS_REAL_HOME"] = str(fresh)
        cid, _ = self.grant("A")
        rc, _, err = self.run_cli("install", "A", cid, "--home", str(fresh))
        self.assertEqual(rc, 2, err)
        self.assertEqual(os.listdir(fresh), [])

    def test_odd_umask_still_private_modes(self):
        cid, _ = self.grant("A")  # before the umask: the claim folders are Task 1's (they refuse a 0500 folder)
        src = self.source()
        old = os.umask(0o277)  # takes the owner's write bit: only an explicit chmod gets 0700 / 0600
        try:
            h = self.root / "home"
            os.mkdir(h, 0o700)
            os.chmod(h, 0o700)
            rc, _, err = self.run_cli("install", "A", cid, "--home", str(h))
            self.assertEqual(rc, 0, err)
            rc, _, err = self.run_cli("import", "A", "--from", str(src), "--replace")
            self.assertEqual(rc, 0, err)
        finally:
            os.umask(old)
        for p in (h / ".config", h / ".config" / "ttyak", h / ".config" / "ttyak" / "p-A"):
            self.assertEqual(stat.S_IMODE(os.lstat(p).st_mode), 0o700, p)
        self.assertEqual(stat.S_IMODE(os.lstat(self.target(h)).st_mode), 0o600)
        self.assertEqual(stat.S_IMODE(os.lstat(self.master).st_mode), 0o600)

    def test_real_folder_by_identity_refused(self):
        # a case-insensitive disk or a bind mount: a path no string compare matches is still the real folder
        from unittest import mock
        cid, _ = self.grant("A")
        h = self.home()
        (h / ".config" / "ttyak").mkdir(parents=True)
        st = os.stat(h / ".config" / "ttyak")
        same = {(st.st_dev, st.st_ino)}
        with mock.patch.object(accounts, "real_dir_ids", return_value=same):
            rc, _, err = self.inproc("install", "A", cid, "--home", str(h))
        self.assertEqual(rc, 2, err)
        self.assertIn("real folder", err)
        self.assertEqual(files_under(h), [])
        self.assertEqual(self.installs(), [])
        # a recorded copy that turns out to be in a real folder is left alone at release
        self.assertEqual(self.run_cli("install", "A", cid, "--home", str(h))[0], 0)
        with mock.patch.object(accounts, "real_dir_ids", return_value=same):
            rc, out, err = self.inproc("release", "A", cid)
        self.assertEqual(rc, 0, err)
        self.assertTrue(any(l.startswith("not removed %s:" % self.target(h)) for l in out.splitlines()), out)
        self.assertEqual(self.target(h).read_text(), MARKER)

    def test_real_folder_refusal_explains_the_environment(self):
        cid, _ = self.grant("A")
        hint = "set HOME/XDG only on the ttyak command"
        for home in (self.real, self.real / ".config" / "ttyak"):  # the target check; the --home check
            rc, _, err = self.run_cli("install", "A", cid, "--home", str(home))
            self.assertEqual(rc, 2)
            self.assertIn(hint, err)
        from unittest import mock
        with mock.patch.object(accounts, "resolve_home", side_effect=lambda x: os.path.realpath(x)):
            rc, _, err = self.inproc("install", "A", cid, "--home", str(self.real))  # the target check in install
        self.assertEqual(rc, 2)
        self.assertIn(hint, err)

    def test_install_refuses_path_recorded_by_another_account(self):
        h = self.home()
        cb, _ = self.grant("B")
        self.assertEqual(self.run_cli("install", "B", cb, "--home", str(h), "--profile", "x")[0], 0)
        self.target(h, "x").unlink()  # removed by hand: B's record stays
        ca, _ = self.grant("A")
        rc, _, err = self.run_cli("install", "A", ca, "--home", str(h), "--profile", "x")
        self.assertEqual(rc, 2, err)
        self.assertIn("account B", err)
        self.assertFalse(self.target(h, "x").exists())
        self.assertEqual(self.installs("A"), [])

    # -- release -------------------------------------------------------------------------------------------------

    def test_release_leaves_records_outside_its_rules_alone(self):
        import hashlib
        cid, _ = self.grant("A")
        real_session = self.real / ".config" / "ttyak" / "p-A" / "session.json"
        real_session.write_text("real")
        keep = self.real / ".config" / "ttyak" / "p-A" / "keep.txt"
        other = self.home() / "notes.txt"  # not in a real folder: only the name check stops it
        other.write_text("notes")
        st = self.state()
        st["claim"]["installs"] = [{"path": str(p), "sha256": hashlib.sha256(p.read_bytes()).hexdigest()}
                                   for p in (real_session, keep, other)]
        (self.d / "claims" / "A" / "state.json").write_text(json.dumps(st))
        rc, out, err = self.run_cli("release", "A", cid)
        self.assertEqual(rc, 0, err)
        lines = out.splitlines()
        for p in (real_session, keep, other):
            self.assertTrue(any(l.startswith("not removed %s:" % p) for l in lines), (p, out))
        self.assertEqual((real_session.read_text(), keep.read_text(), other.read_text()), ("real", "keep", "notes"))

    def test_release_removes_copies_before_saving_the_release(self):
        # a kill between the two must not leave copies nobody has a record of
        from unittest import mock
        cid, _ = self.grant("A")
        h = self.home()
        self.assertEqual(self.run_cli("install", "A", cid, "--home", str(h))[0], 0)
        c = accounts.Claims(self.d, "A")
        with mock.patch.dict(os.environ, self.env), \
                mock.patch.object(accounts.Claims, "_write", side_effect=OSError("killed")):
            with self.assertRaises(OSError):
                c.release(None, cid)
        self.assertFalse(self.target(h).exists())
        self.assertEqual(self.state()["claim"]["id"], cid)  # the record stays: the next release says "already gone"
        rc, out, _ = self.run_cli("release", "A", cid)
        self.assertIn("already gone %s" % self.target(h), out.splitlines())

    def test_remove_copy_refuses_a_file_swapped_while_hashed(self):
        from unittest import mock
        cid, _ = self.grant("A")
        h = self.home()
        self.assertEqual(self.run_cli("install", "A", cid, "--home", str(h))[0], 0)
        t = self.target(h)
        real_read = os.read
        swapped = []

        def read(fd, n):
            if not swapped:
                swapped.append(1)
                t.unlink()
                t.write_text("someone else's")
            return real_read(fd, n)
        with mock.patch.dict(os.environ, self.env), mock.patch.object(accounts.os, "read", read):
            self.assertEqual(accounts.remove_copy(str(t), "0" * 64), "refused")
        self.assertEqual(t.read_text(), "someone else's")


    def test_release_removes_recorded_copies_only(self):
        cid, _ = self.grant("A")
        h1, h2, h3 = self.home("h1"), self.home("h2"), self.home("h3")
        for h in (h1, h2, h3):
            self.assertEqual(self.run_cli("install", "A", cid, "--home", str(h))[0], 0)
        t1, t2, t3 = self.target(h1), self.target(h2), self.target(h3)
        (t1.parent / "settings.toml").write_text("keep")
        (h1 / "other.json").write_text("keep")
        t2.unlink()  # gone before the release
        # h3's profile folder swapped for a symlink to a victim folder holding a session.json: never followed
        victim = self.root / "victim"
        victim.mkdir()
        (victim / "session.json").write_text("victim")
        t3.unlink()
        os.rmdir(t3.parent)
        os.symlink(victim, t3.parent)
        rc, out, err = self.run_cli("release", "A", cid)
        self.assertEqual(rc, 0, err)
        self.assertIn("released A", out)
        lines = out.splitlines()
        self.assertIn("removed %s" % t1, lines)
        self.assertIn("already gone %s" % t2, lines)
        self.assertTrue(any(l.startswith("not removed %s:" % t3) for l in lines), out)
        self.assertFalse(t1.exists())
        self.assertEqual((victim / "session.json").read_text(), "victim")
        self.assertEqual((t1.parent / "settings.toml").read_text(), "keep")
        self.assertEqual((h1 / "other.json").read_text(), "keep")
        for p in (t1.parent, t2.parent, h1 / ".config" / "ttyak"):
            self.assertTrue(p.is_dir(), p)  # folders stay
        self.assertEqual(self.master.read_text(), MARKER)
        self.assertIn("uninstall", self.log())
        # --force cleans up too; --keep leaves the copies
        cid, _ = self.grant("A")
        self.assertEqual(self.run_cli("install", "A", cid, "--home", str(h1))[0], 0)
        rc, out, _ = self.run_cli("release", "A", "--force", "--reason", "stuck")
        self.assertEqual(rc, 0)
        self.assertIn("removed %s" % t1, out.splitlines())
        self.assertFalse(t1.exists())
        cid, _ = self.grant("A")
        self.assertEqual(self.run_cli("install", "A", cid, "--home", str(h1))[0], 0)
        rc, out, _ = self.run_cli("release", "A", cid, "--keep")
        self.assertEqual(rc, 0)
        self.assertIn("kept %s" % t1, out)
        self.assertEqual(t1.read_text(), MARKER)
        # a folder where the copy was is never removed
        t1.unlink()
        cid, _ = self.grant("A")
        self.assertEqual(self.run_cli("install", "A", cid, "--home", str(h1))[0], 0)
        t1.unlink()
        t1.mkdir()
        rc, out, _ = self.run_cli("release", "A", cid)
        self.assertEqual(rc, 0)
        self.assertIn("not removed %s" % t1, out)
        self.assertTrue(t1.is_dir())

    def test_stale_claim_broken_removes_its_copies(self):
        cid, _ = self.grant("A", "--lease", "0.3s")
        h = self.home()
        self.assertEqual(self.run_cli("install", "A", cid, "--home", str(h))[0], 0)
        import time
        time.sleep(0.5)
        rc, out, err = self.run_cli("claim", "A", "--purpose", "next")
        self.assertEqual(rc, 0, err)
        self.assertIn("removed %s" % self.target(h), out.splitlines())
        self.assertFalse(self.target(h).exists())

    def test_release_reports_changed_copy(self):
        cid, _ = self.grant("A")
        h = self.home()
        self.assertEqual(self.run_cli("install", "A", cid, "--home", str(h))[0], 0)
        t = self.target(h)
        t.write_text(MARKER + "-signed-in-again")
        rc, out, err = self.run_cli("release", "A", cid)
        self.assertEqual(rc, 0, err)
        self.assertIn("rewritten by ttyak", out)
        self.assertIn("the master is unchanged", out)
        self.assertIn(str(t), out)
        self.assertFalse(t.exists())
        self.assertEqual(self.master.read_text(), MARKER)

    def test_no_session_bytes_in_any_output(self):
        src = self.source()
        h, bad = self.home(), self.home("bad")
        os.symlink(self.real / ".config", bad / ".config")
        loose = self.source(body=MARKER + "-loose", mode=0o644)
        runs = []

        def run(*args, **kw):
            r = self.run_cli(*args, **kw)
            runs.append((args, r))
            return r
        run("list")
        run("status")
        run("status", "--json")
        run("status", "A")
        cid = re.search(r"id=(\S+)", run("claim", "A", "--purpose", "p")[1]).group(1)
        run("claim", "A", "--purpose", "w", "--wait", "0")
        run("install", "A", "wrong-id", "--home", str(h))
        run("install", "A", cid, "--home", str(bad))
        run("install", "A", cid, "--home", str(self.real))
        self.assertEqual(run("install", "A", cid, "--home", str(h))[0], 0)
        h2 = self.home("h2")
        (h2 / ".config" / "ttyak" / "p-A").mkdir(parents=True)
        (h2 / ".config" / "ttyak" / "p-A" / "session.json").write_text(MARKER + "-foreign")
        run("install", "A", cid, "--home", str(h2))
        run("status", "--json")
        run("list")
        run("import", "A", "--from", str(src))
        run("import", "A", "--from", str(loose), "--replace")
        run("import", "Z", "--from", str(src))
        run("import", "B", "--from", str(src), "--replace")
        run("release", "A", "wrong-id")
        self.target(h).write_text(MARKER + "-rewritten")
        run("release", "A", cid)
        cid = re.search(r"id=(\S+)", run("claim", "A", "--purpose", "p")[1]).group(1)
        run("install", "A", cid, "--home", str(h))
        run("release", "A", "--force", "--reason", "r")
        os.chmod(self.master, 0o640)
        run("list")
        run("install", "A", cid, "--home", str(h))
        self.assertGreater(len(runs), 20)
        for args, (rc, out, err) in runs:
            self.assertNotIn(MARKER, out + err, args)
            self.assertNotIn("Traceback", err, args)
        for name in ("A", "B"):
            for f in ("log.txt", "state.json"):
                p = self.d / "claims" / name / f
                if p.exists():
                    self.assertNotIn(MARKER, p.read_text(), (name, f))


if __name__ == "__main__":
    unittest.main()
