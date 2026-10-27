"""ttyak's Slack test accounts: the ledger and one claim per account, so one test uses an account at a time.

The data folder (git-ignored, 0700) holds ledger.json, sessions/<NAME>.json (0600) and claims/<ACCOUNT>/. This part
is the library: it loads and checks the ledger (never reading a session file) and keeps the claim rules. Claim
semantics match the tool for test machines, so agents use one habit: a FIFO queue of tickets, a lease on the
grant, the first waiter breaks a stale claim.
"""
from __future__ import annotations

import argparse
import errno
import fcntl
import getpass
import json
import os
import re
import shlex
import socket
import stat
import sys
import time
import uuid
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
        raise LedgerError("%s: folder missing" % p) from None
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
        raise LedgerError("%s: cannot read" % path) from None
    except ValueError:
        raise LedgerError("%s: not valid JSON" % path) from None
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
            raise LedgerError("%s: session file missing" % sp) from None
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


def _good_ticket(t: object) -> bool:
    return (isinstance(t, dict) and isinstance(t.get("ticket"), str) and NAME_RE.fullmatch(t["ticket"]) is not None
            and isinstance(t.get("holder"), str) and isinstance(t.get("purpose"), str)
            and all(isinstance(t.get(k), (int, float)) and not isinstance(t.get(k), bool)
                    for k in ("enqueued", "seen")))


def _num(v: object) -> bool:
    return isinstance(v, (int, float)) and not isinstance(v, bool)


def _good_claim(c: object) -> bool:
    return (isinstance(c, dict) and isinstance(c.get("id"), str) and NAME_RE.fullmatch(c["id"]) is not None
            and isinstance(c.get("holder"), str) and isinstance(c.get("purpose"), str)
            and all(_num(c.get(k)) for k in ("since", "renewed", "lease"))
            and isinstance(c.get("installs"), list))


class Claims:
    """The claim of one account: state.json (claim + queue) and log.txt, under a flock on `lock`."""

    def __init__(self, d: Path, account: str):
        _check_id("account", account)
        d = Path(d)
        _check_dir(d)
        for p in (d / "claims", d / "claims" / account):
            p.mkdir(mode=0o700, exist_ok=True)
            _check_dir(p)  # a symlink or a loose folder is refused before anything is written into it
        self.dir = d / "claims" / account

    # -- storage -----------------------------------------------------------------------------------------------

    def _locked(self):
        return _Lock(self.dir / "lock")

    def _read(self) -> dict:
        path = self.dir / "state.json"
        try:
            fd = os.open(path, os.O_RDONLY | os.O_NOFOLLOW | os.O_NONBLOCK)  # never block on a FIFO
        except FileNotFoundError:
            return {"claim": None, "queue": []}
        except OSError as e:
            raise LedgerError("%s: cannot open (not a regular file?): %s"
                              % (path, errno.errorcode.get(e.errno, "error"))) from None
        try:
            with os.fdopen(fd, encoding="utf-8") as f:
                s = json.load(f)
        except ValueError:
            s = None
        except OSError as e:
            raise LedgerError("%s: cannot read: %s" % (path, errno.errorcode.get(e.errno, "error"))) from None
        if (not isinstance(s, dict) or not (s.get("claim") is None or _good_claim(s["claim"]))
                or not isinstance(s.get("queue", []), list)
                or not all(_good_ticket(t) for t in s.get("queue", []))):
            raise LedgerError("%s: not a valid claim state" % path)
        s.setdefault("claim", None)
        s.setdefault("queue", [])
        return s

    def _write(self, s: dict) -> None:
        tmp = self.dir / "state.json.tmp"
        fd = os.open(tmp, os.O_WRONLY | os.O_CREAT | os.O_TRUNC | os.O_NOFOLLOW | os.O_NONBLOCK, 0o600)
        try:
            with os.fdopen(fd, "w", encoding="utf-8") as f:
                json.dump(s, f)
                f.flush()
                os.fsync(f.fileno())
            os.replace(tmp, self.dir / "state.json")
        except BaseException:
            try:
                os.unlink(tmp)
            except OSError:
                pass
            raise
        try:  # best effort: the state is saved, so the log lines must still follow
            dfd = os.open(self.dir, os.O_RDONLY | os.O_DIRECTORY)
            try:
                os.fsync(dfd)
            finally:
                os.close(dfd)
        except OSError:
            pass

    def _log(self, now: float, event: str, **fields) -> None:
        t = datetime.fromtimestamp(now, timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ")
        line = " ".join([t, event] + ["%s=%s" % (k, json.dumps(v)) for k, v in fields.items()])
        fd = os.open(self.dir / "log.txt", os.O_WRONLY | os.O_CREAT | os.O_APPEND | os.O_NOFOLLOW | os.O_NONBLOCK, 0o600)
        with os.fdopen(fd, "a", encoding="utf-8") as f:
            f.write(line + "\n")

    # -- rules -------------------------------------------------------------------------------------------------

    @staticmethod
    def _queue(s: dict) -> list:
        return sorted(s["queue"], key=lambda t: (t["enqueued"], t["ticket"]))

    def _expire(self, s: dict, now: float, ttl: float) -> list:
        gone = [t for t in s["queue"] if t["seen"] + ttl < now]
        s["queue"] = [t for t in s["queue"] if t["seen"] + ttl >= now]
        return gone

    def _log_expired(self, now: float, gone: list) -> None:
        for t in gone:
            self._log(now, "expire", ticket=t["ticket"], holder=t["holder"])

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
            gone = self._expire(s, now, ttl)
            c = s["claim"]
            if c is not None and c["id"] == ticket:
                c["renewed"] = now
                self._write(s)
                self._log_expired(now, gone)
                self._log(now, "claim", id=ticket, holder=c["holder"], again=True)
                return self._answer(s, now, "granted")
            old = next((t for t in s["queue"] if t["ticket"] == ticket), None)
            s["queue"] = [t for t in s["queue"] if t["ticket"] != ticket]
            s["queue"].append({"ticket": ticket, "holder": holder, "purpose": purpose,
                               "enqueued": old["enqueued"] if old else now, "seen": now})
            head = self._queue(s)[0]["ticket"]
            stale = c is not None and now > c["renewed"] + c["lease"]
            if head == ticket and (c is None or stale):
                broken = c
                s["claim"] = {"id": ticket, "holder": holder, "purpose": purpose, "since": now,
                              "renewed": now, "lease": lease, "installs": []}
                s["queue"] = [t for t in s["queue"] if t["ticket"] != ticket]
                self._write(s)
                self._log_expired(now, gone)
                if stale:
                    self._log(now, "break", id=broken["id"], holder=broken["holder"], by=ticket,
                              reason="lease lapsed")
                self._log(now, "claim", id=ticket, holder=holder, purpose=purpose)
                return self._answer(s, now, "granted")
            self._write(s)
            self._log_expired(now, gone)
            return self._answer(s, now, "waiting", ticket)

    def renew(self, now: Optional[float], id: str, lease: Optional[float] = None) -> dict:
        _check_id("id", id)
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
        _check_id("id", id)
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
        _check_id("ticket", ticket)
        with self._locked():
            now = time.time() if now is None else now
            s = self._read()
            gone = self._expire(s, now, ttl)
            s["queue"] = [t for t in s["queue"] if t["ticket"] != ticket]
            c = s["claim"]
            released = False
            # an untouched grant whose answer never reached the client (a renewed claim is in use: keep it)
            if c is not None and c["id"] == ticket and c["renewed"] == c["since"]:
                s["claim"] = None
                released = True
            self._write(s)
            self._log_expired(now, gone)
            if released:
                self._log(now, "release", id=ticket, holder=c["holder"], by="leave")
            return self._answer(s, now, "left")

    def status(self, now: Optional[float], ttl: float = TICKET_TTL) -> dict:
        with self._locked():
            now = time.time() if now is None else now
            s = self._read()
            gone = self._expire(s, now, ttl)
            self._write(s)
            self._log_expired(now, gone)
            return self._answer(s, now, "status")


class _Lock:
    """flock(LOCK_EX) on a file, held for one state update."""

    def __init__(self, path: Path):
        self.path = path

    def __enter__(self):
        self.fd = os.open(self.path, os.O_RDWR | os.O_CREAT | os.O_NOFOLLOW, 0o600)
        try:
            fcntl.flock(self.fd, fcntl.LOCK_EX)
        except BaseException:
            os.close(self.fd)
            raise
        return self

    def __exit__(self, *exc):
        os.close(self.fd)  # closing releases the flock
        return False


# -- command line ----------------------------------------------------------------------------------------------

class Usage(Exception):
    pass


def parse_dur(text: str, allow_forever: bool = False) -> Optional[float]:
    """Seconds for 30s / 10m / 2h / a bare number; None for 'forever' where allowed."""
    if allow_forever and text == "forever":
        return None
    m = re.fullmatch(r"(\d+(?:\.\d+)?)([smh]?)", text)
    if not m:
        raise Usage("bad duration %r (use 30s, 10m, 2h%s)" % (text, ", or forever" if allow_forever else ""))
    return float(m.group(1)) * {"": 1, "s": 1, "m": 60, "h": 3600}[m.group(2)]


def parse_lease(text: str) -> float:
    sec = parse_dur(text)
    if not sec > 0:
        raise Usage("bad --lease %r (must be greater than 0)" % text)
    return sec


def _clean(text: str) -> str:
    """Text from a claimant, safe to print: control characters (an ANSI escape) become '?'."""
    return re.sub(r"[\x00-\x1f\x7f]", "?", text)


def fmt(sec: Optional[float]) -> str:
    if sec and 0 < sec < 1:
        return "%gs" % round(sec, 2)
    sec = int(round(sec or 0))
    if sec < 60:
        return "%ds" % sec
    if sec < 3600:
        m, r = divmod(sec, 60)
        return "%dm%02ds" % (m, r) if r else "%dm" % m
    h, r = divmod(sec, 3600)
    return "%dh%02dm" % (h, r // 60) if r // 60 else "%dh" % h


def _self_argv() -> list:
    """How this tool is run from a shell, so printed command lines can be pasted as they are."""
    return [sys.executable, str(Path(__file__).resolve())]


def _line(argv: list) -> str:
    return " ".join(shlex.quote(a) for a in argv)


def default_holder() -> str:
    h = os.environ.get("TTYAK_ACCOUNTS_HOLDER")
    if h:
        return h
    try:
        user = getpass.getuser()
    except Exception:
        user = "user"
    return "%s@%s" % (user, socket.gethostname())


def _poll() -> float:
    return float(os.environ.get("TTYAK_ACCOUNTS_POLL") or 5)


def _ledger() -> tuple:
    d = data_dir()
    return d, load_ledger(d)["accounts"]


def _account(args: argparse.Namespace) -> tuple:
    """(Claims, ledger entry) for args.account; an unknown name lists the known ones."""
    d, accts = _ledger()
    if args.account not in accts:
        raise Usage("unknown account %r; known accounts: %s" % (args.account, ", ".join(sorted(accts)) or "(none)"))
    return Claims(d, args.account), accts[args.account]


def _state_word(c: Optional[dict]) -> str:
    return "free" if c is None else "stale" if c["stale"] else "held"


def cmd_list(args: argparse.Namespace) -> int:
    d, accts = _ledger()
    rows = []
    for name in sorted(accts):
        a = accts[name]
        c = Claims(d, name).status(None)["claim"]
        if c is None:
            st = "free"
        else:
            st = "held by %s for %s" % (_clean(c["holder"]), _clean(c["purpose"])) + (" (stale)" if c["stale"] else "")
        rows.append((name, a["profile"], a["role"], st, a["rules"]))
    w = [max([len(r[i]) for r in rows] + [1]) for i in range(3)]
    for r in rows:
        print("  ".join(r[i].ljust(w[i]) for i in range(3)) + "  " + r[3])
        for rule in r[4]:
            print("    rule: " + rule)
    return 0


_SHOWN_CLAIM = {"holder", "purpose", "since", "renewed", "lease", "held_for", "since_renewal", "stale"}
_SHOWN_TICKET = {"holder", "purpose", "waited"}


def _hide_keys(ans: dict) -> dict:
    """The claim id and the tickets are the keys to a claim: never shown. Neither are the recorded installs."""
    if ans.get("claim"):
        ans["claim"] = {k: v for k, v in ans["claim"].items() if k in _SHOWN_CLAIM}
    ans["queue"] = [{k: v for k, v in t.items() if k in _SHOWN_TICKET} for t in ans.get("queue", [])]
    return ans


def cmd_status(args: argparse.Namespace) -> int:
    d, accts = _ledger()
    if args.account and args.account not in accts:
        raise Usage("unknown account %r; known accounts: %s" % (args.account, ", ".join(sorted(accts)) or "(none)"))
    names = [args.account] if args.account else sorted(accts)
    answers = {n: _hide_keys(Claims(d, n).status(None)) for n in names}
    if args.json:
        print(json.dumps(answers, indent=2, sort_keys=True))
        return 0
    rows = [("ACCOUNT", "STATE", "HOLDER", "PURPOSE", "HELD", "RENEWED", "QUEUE")]
    for n in names:
        a = answers[n]
        c = a["claim"]
        rows.append((n, _state_word(c), _clean(c["holder"]) if c else "-", _clean(c["purpose"]) if c else "-",
                     fmt(c["held_for"]) if c else "-",
                     (fmt(c["since_renewal"]) + " ago" + (" (stale)" if c["stale"] else "")) if c else "-",
                     str(len(a["queue"]))))
    widths = [max(len(r[i]) for r in rows) for i in range(len(rows[0]) - 1)]
    for r in rows:
        print("  ".join(r[i].ljust(widths[i]) for i in range(len(widths))) + "  " + r[-1])
    return 0


def _status_lines(answer: dict, ticket: str) -> list:
    out = []
    c = answer.get("claim")
    if c:
        out.append("holder: %s" % _clean(c["holder"]))
        out.append("purpose: %s" % _clean(c["purpose"]))
        out.append("held for: %s" % fmt(c["held_for"]))
        out.append("since last renewal: %s%s" % (fmt(c["since_renewal"]), " (stale)" if c.get("stale") else ""))
    else:
        out.append("no live claim")
    if answer.get("position"):
        out.append("your position: %d of %d" % (answer["position"], len(answer["queue"])))
    mine = [t for t in answer.get("queue", []) if t["ticket"] == ticket]
    if mine:
        out.append("waited: %s" % fmt(mine[0]["waited"]))
    return out


def cmd_claim(args: argparse.Namespace) -> int:
    wait = parse_dur(args.wait, allow_forever=True)
    report = parse_dur(args.report_every) if args.report_every else None
    lease = parse_lease(args.lease) if args.lease else float(LEASE)
    ticket = args.ticket if args.ticket is not None else uuid.uuid4().hex[:12]
    if not NAME_RE.fullmatch(ticket):
        raise Usage("bad --ticket %r (letters, digits, - and _, at most 64)" % ticket)
    claims, entry = _account(args)
    holder = args.holder or default_holder()
    start = time.monotonic()

    def left() -> str:
        # the wait limit counts from the first claim, so a resume carries what is left of it
        return "forever" if wait is None else "%.2fs" % max(0.0, wait - (time.monotonic() - start))

    def leave() -> None:
        """Best effort: take our ticket (and a grant whose answer we never saw) off the queue."""
        try:
            claims.leave(None, ticket)
        except (OSError, LedgerError, ValueError, KeyboardInterrupt):
            pass

    try:
        while True:
            try:
                ans = claims.claim(None, ticket, holder, args.purpose, lease)
            except (OSError, LedgerError, ValueError):
                leave()
                raise
            if ans["state"] == "granted":
                c = ans["claim"]
                print("claimed %s id=%s lease=%s" % (args.account, c["id"], fmt(c["lease"])))
                print("profile: %s" % entry["profile"])
                print("release: " + _line(_self_argv() + ["release", args.account, c["id"]]))
                return 0
            elapsed = time.monotonic() - start
            limit = wait is not None and elapsed >= wait
            if limit or (report is not None and elapsed >= report):
                for line in _status_lines(ans, ticket):
                    print(line)
                if limit:
                    leave()
                    return 3
                argv = _self_argv() + ["claim", args.account, "--ticket", ticket,
                                           "--purpose=" + args.purpose]
                for flag, val in (("--holder", args.holder), ("--wait", left()),
                                  ("--report-every", args.report_every), ("--lease", args.lease)):
                    if val and not (flag == "--wait" and val == "forever"):
                        argv += [flag + "=" + val] if flag == "--holder" else [flag, val]
                print("resume: " + _line(argv))
                return 10
            nxt = wait if report is None else (report if wait is None else min(wait, report))
            time.sleep(max(0.0, min(_poll(), nxt - elapsed)) if nxt is not None else _poll())
    except KeyboardInterrupt:
        print("accounts: interrupted", file=sys.stderr)
        leave()
        return 130


def _not_holder(args: argparse.Namespace, ans: dict) -> int:
    print("accounts: %s: no live claim with that id (not the holder, or it lapsed)" % args.account, file=sys.stderr)
    return 6


def cmd_renew(args: argparse.Namespace) -> int:
    lease = parse_lease(args.lease) if args.lease else None
    claims, _ = _account(args)
    ans = claims.renew(None, args.id, lease)
    if ans["state"] != "renewed":
        return _not_holder(args, ans)
    print("renewed %s id=%s" % (args.account, args.id))
    return 0


def cmd_release(args: argparse.Namespace) -> int:
    if args.force:
        if not args.reason:
            raise Usage("--force needs --reason")
    elif not args.id:
        raise Usage("release needs an ID (or --force --reason R)")
    else:
        _check_id("id", args.id)
    claims, _ = _account(args)
    if args.force:
        ans = claims.force_release(None, args.by or default_holder(), args.reason)
        print("no claim on %s" % args.account if ans["state"] == "no_claim"
              else "released %s (forced)" % args.account)
        return 0
    ans = claims.release(None, args.id)
    if ans["state"] != "released":
        return _not_holder(args, ans)
    print("released %s" % args.account)
    return 0


def build_parser() -> argparse.ArgumentParser:
    p = argparse.ArgumentParser(prog="accounts.py", description="ttyak's Slack test accounts and their claims")
    sub = p.add_subparsers(dest="command", required=True)
    sub.add_parser("list", help="the accounts, their rules and claim state").set_defaults(func=cmd_list)
    q = sub.add_parser("status", help="who holds which account")
    q.add_argument("account", nargs="?")
    q.add_argument("--json", action="store_true")
    q.set_defaults(func=cmd_status)
    q = sub.add_parser("claim", help="claim an account, waiting in the queue")
    q.add_argument("account")
    q.add_argument("--purpose", required=True)
    q.add_argument("--holder")
    q.add_argument("--wait", default="forever", metavar="DUR|forever")
    q.add_argument("--report-every", metavar="DUR")
    q.add_argument("--lease", metavar="DUR")
    q.add_argument("--ticket")
    q.set_defaults(func=cmd_claim)
    q = sub.add_parser("renew", help="renew a claim")
    q.add_argument("account")
    q.add_argument("id")
    q.add_argument("--lease", metavar="DUR")
    q.set_defaults(func=cmd_renew)
    q = sub.add_parser("release", help="release a claim (or --force it)")
    q.add_argument("account")
    q.add_argument("id", nargs="?")
    q.add_argument("--force", action="store_true")
    q.add_argument("--reason")
    q.add_argument("--by")
    q.set_defaults(func=cmd_release)
    return p


def main(argv: Optional[list] = None) -> int:
    args = build_parser().parse_args(argv)
    try:
        return args.func(args)
    except (Usage, LedgerError, ValueError, OSError) as e:
        print("accounts: %s" % e, file=sys.stderr)
        return 2
    except KeyboardInterrupt:
        print("accounts: interrupted", file=sys.stderr)
        return 130


if __name__ == "__main__":
    sys.exit(main())
