from __future__ import annotations

import json
import os
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


if __name__ == "__main__":
    unittest.main()
