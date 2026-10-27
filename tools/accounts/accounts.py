"""ttyak's Slack test accounts: the ledger and one claim per account, so one test uses an account at a time.

The data folder (git-ignored, 0700) holds ledger.json, sessions/<NAME>.json (0600) and claims/<ACCOUNT>/. This part
is the library: it loads and checks the ledger (never reading a session file) and keeps the claim rules. Claim
semantics match the tool for test machines, so agents use one habit: a FIFO queue of tickets, a lease on the
grant, the first waiter breaks a stale claim.
"""
from __future__ import annotations

import fcntl
import json
import os
import re
import stat
import time
from datetime import datetime, timezone
from pathlib import Path
from typing import Optional

LEASE = 30 * 60  # seconds a claim lives without being renewed
TICKET_TTL = 15 * 60  # seconds a waiting ticket lives without being refreshed

NAME_RE = re.compile(r"[A-Za-z0-9_-]{1,64}")
ACCOUNT_KEYS = {"profile", "role", "rules", "session"}


class LedgerError(Exception):
    """A bad ledger or data folder. The message names the file and the problem, never file contents."""


def data_dir() -> Path:
    """$TTYAK_ACCOUNTS if set, else .accounts/ next to tools/."""
    env = os.environ.get("TTYAK_ACCOUNTS")
    if env:
        return Path(env)
    return Path(__file__).resolve().parents[2] / ".accounts"


def _mode(p: Path) -> int:
    return stat.S_IMODE(os.lstat(p).st_mode)


def _check_dir(p: Path) -> None:
    try:
        st = os.lstat(p)
    except OSError:
        raise LedgerError("%s: folder missing" % p)
    if not stat.S_ISDIR(st.st_mode):
        raise LedgerError("%s: not a folder" % p)
    if stat.S_IMODE(st.st_mode) != 0o700:
        raise LedgerError("%s: mode is %04o, must be 0700" % (p, stat.S_IMODE(st.st_mode)))


def load_ledger(d: Path) -> dict:
    """Load and check d/ledger.json and the permissions of the folders and session files it lists."""
    d = Path(d)
    _check_dir(d)
    _check_dir(d / "sessions")
    path = d / "ledger.json"
    try:
        with open(path, encoding="utf-8") as f:
            led = json.load(f)
    except OSError:
        raise LedgerError("%s: cannot read" % path)
    except ValueError:
        raise LedgerError("%s: not valid JSON" % path)
    if not isinstance(led, dict) or set(led) != {"accounts"}:
        raise LedgerError('%s: top level must be {"accounts": {...}} only' % path)
    accts = led["accounts"]
    if not isinstance(accts, dict):
        raise LedgerError("%s: accounts must be an object" % path)
    for name, a in accts.items():
        if not NAME_RE.fullmatch(name):
            raise LedgerError("%s: account name %r is not [A-Za-z0-9_-]{1,64}" % (path, name))
        if not isinstance(a, dict):
            raise LedgerError("%s: account %s must be an object" % (path, name))
        if set(a) != ACCOUNT_KEYS:
            raise LedgerError("%s: account %s: keys must be exactly %s (got %s)"
                              % (path, name, sorted(ACCOUNT_KEYS), sorted(a)))
        for k in ("profile", "role", "session"):
            if not isinstance(a[k], str) or not a[k]:
                raise LedgerError("%s: account %s: %s must be a non-empty string" % (path, name, k))
        if not isinstance(a["rules"], list) or not all(isinstance(r, str) for r in a["rules"]):
            raise LedgerError("%s: account %s: rules must be a list of strings" % (path, name))
        if a["session"] != "sessions/%s.json" % name:
            raise LedgerError("%s: account %s: session must be sessions/%s.json" % (path, name, name))
        sp = d / a["session"]
        try:
            st = os.lstat(sp)
        except OSError:
            raise LedgerError("%s: session file missing" % sp)
        if not stat.S_ISREG(st.st_mode):
            raise LedgerError("%s: not a regular file" % sp)
        if stat.S_IMODE(st.st_mode) != 0o600:
            raise LedgerError("%s: mode is %04o, must be 0600" % (sp, stat.S_IMODE(st.st_mode)))
    return led


def _view(c: Optional[dict], now: float) -> Optional[dict]:
    if c is None:
        return None
    v = dict(c)
    v["held_for"] = now - c["since"]
    v["since_renewal"] = now - c["renewed"]
    v["stale"] = now > c["renewed"] + c["lease"]
    return v


def _check_id(what: str, v: str) -> None:
    if not isinstance(v, str) or not NAME_RE.fullmatch(v):
        raise ValueError("%s must match [A-Za-z0-9_-]{1,64}" % what)


class Claims:
    """The claim of one account: state.json (claim + queue) and log.txt, under a flock on `lock`."""

    def __init__(self, d: Path, account: str):
        _check_id("account", account)
        self.dir = Path(d) / "claims" / account
        self.dir.mkdir(mode=0o700, parents=True, exist_ok=True)
        os.chmod(self.dir, 0o700)

    # -- storage -----------------------------------------------------------------------------------------------

    def _locked(self):
        return _Lock(self.dir / "lock")

    def _read(self) -> dict:
        try:
            with open(self.dir / "state.json", encoding="utf-8") as f:
                s = json.load(f)
        except FileNotFoundError:
            return {"claim": None, "queue": []}
        s.setdefault("claim", None)
        s.setdefault("queue", [])
        return s

    def _write(self, s: dict) -> None:
        tmp = self.dir / "state.json.tmp"
        fd = os.open(tmp, os.O_WRONLY | os.O_CREAT | os.O_TRUNC, 0o600)
        with os.fdopen(fd, "w", encoding="utf-8") as f:
            json.dump(s, f)
            f.flush()
            os.fsync(f.fileno())
        os.replace(tmp, self.dir / "state.json")

    def _log(self, now: float, event: str, **fields) -> None:
        t = datetime.fromtimestamp(now, timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ")
        line = " ".join([t, event] + ["%s=%s" % (k, json.dumps(v)) for k, v in fields.items()])
        fd = os.open(self.dir / "log.txt", os.O_WRONLY | os.O_CREAT | os.O_APPEND, 0o600)
        with os.fdopen(fd, "a", encoding="utf-8") as f:
            f.write(line + "\n")

    # -- rules -------------------------------------------------------------------------------------------------

    @staticmethod
    def _queue(s: dict) -> list:
        return sorted(s["queue"], key=lambda t: (t["enqueued"], t["ticket"]))

    def _expire(self, s: dict, now: float, ttl: float) -> None:
        keep = []
        for t in s["queue"]:
            if t["seen"] + ttl < now:
                self._log(now, "expire", ticket=t["ticket"], holder=t["holder"])
            else:
                keep.append(t)
        s["queue"] = keep

    def _answer(self, s: dict, now: float, state: str, ticket: Optional[str] = None) -> dict:
        q = self._queue(s)
        pos = None
        for i, t in enumerate(q):
            if t["ticket"] == ticket:
                pos = i + 1
        return {
            "state": state,
            "claim": _view(s["claim"], now),
            "position": pos,
            "queue": [{"ticket": t["ticket"], "holder": t["holder"], "purpose": t["purpose"],
                       "waited": now - t["enqueued"]} for t in q],
            "now": now,
        }

    # -- operations --------------------------------------------------------------------------------------------

    def claim(self, now: Optional[float], ticket: str, holder: str, purpose: str, lease: float,
              ttl: float = TICKET_TTL) -> dict:
        _check_id("ticket", ticket)
        with self._locked():
            now = time.time() if now is None else now
            s = self._read()
            self._expire(s, now, ttl)
            c = s["claim"]
            if c is not None and c["id"] == ticket:
                c["renewed"] = now
                self._log(now, "claim", id=ticket, holder=c["holder"], again=True)
                self._write(s)
                return self._answer(s, now, "granted")
            old = next((t for t in s["queue"] if t["ticket"] == ticket), None)
            s["queue"] = [t for t in s["queue"] if t["ticket"] != ticket]
            s["queue"].append({"ticket": ticket, "holder": holder, "purpose": purpose,
                               "enqueued": old["enqueued"] if old else now, "seen": now})
            head = self._queue(s)[0]["ticket"]
            stale = c is not None and now > c["renewed"] + c["lease"]
            if head == ticket and (c is None or stale):
                if stale:
                    self._log(now, "break", id=c["id"], holder=c["holder"], by=ticket, reason="lease lapsed")
                s["claim"] = {"id": ticket, "holder": holder, "purpose": purpose, "since": now,
                              "renewed": now, "lease": lease, "installs": []}
                s["queue"] = [t for t in s["queue"] if t["ticket"] != ticket]
                self._log(now, "claim", id=ticket, holder=holder, purpose=purpose)
                self._write(s)
                return self._answer(s, now, "granted")
            self._write(s)
            return self._answer(s, now, "waiting", ticket)

    def renew(self, now: Optional[float], id: str, lease: Optional[float] = None) -> dict:
        with self._locked():
            now = time.time() if now is None else now
            s = self._read()
            c = s["claim"]
            if c is None or c["id"] != id:
                self._log(now, "renew_failed", id=id, holder=c["holder"] if c else None)
                return self._answer(s, now, "not_holder")
            c["renewed"] = now
            if lease is not None:
                c["lease"] = lease
            self._write(s)
            return self._answer(s, now, "renewed")

    def release(self, now: Optional[float], id: str) -> dict:
        with self._locked():
            now = time.time() if now is None else now
            s = self._read()
            c = s["claim"]
            if c is None or c["id"] != id:
                return self._answer(s, now, "not_holder")
            s["claim"] = None
            self._write(s)
            self._log(now, "release", id=id, holder=c["holder"])
            return self._answer(s, now, "released")

    def force_release(self, now: Optional[float], by: str, reason: str) -> dict:
        with self._locked():
            now = time.time() if now is None else now
            s = self._read()
            c = s["claim"]
            if c is None:
                return self._answer(s, now, "no_claim")
            s["claim"] = None
            self._write(s)
            self._log(now, "force", id=c["id"], holder=c["holder"], by=by, reason=reason)
            return self._answer(s, now, "released")

    def leave(self, now: Optional[float], ticket: str, ttl: float = TICKET_TTL) -> dict:
        with self._locked():
            now = time.time() if now is None else now
            s = self._read()
            self._expire(s, now, ttl)
            s["queue"] = [t for t in s["queue"] if t["ticket"] != ticket]
            c = s["claim"]
            # an untouched grant whose answer never reached the client (a renewed claim is in use: keep it)
            if c is not None and c["id"] == ticket and c["renewed"] == c["since"]:
                s["claim"] = None
                self._log(now, "release", id=ticket, holder=c["holder"], by="leave")
            self._write(s)
            return self._answer(s, now, "left")

    def status(self, now: Optional[float], ttl: float = TICKET_TTL) -> dict:
        with self._locked():
            now = time.time() if now is None else now
            s = self._read()
            self._expire(s, now, ttl)
            self._write(s)
            return self._answer(s, now, "status")


class _Lock:
    """flock(LOCK_EX) on a file, held for one state update."""

    def __init__(self, path: Path):
        self.path = path

    def __enter__(self):
        self.fd = os.open(self.path, os.O_RDWR | os.O_CREAT, 0o600)
        fcntl.flock(self.fd, fcntl.LOCK_EX)
        return self

    def __exit__(self, *exc):
        os.close(self.fd)  # closing releases the flock
        return False
