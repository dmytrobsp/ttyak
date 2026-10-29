"""ttyak's milestone helper: the git part of running a milestone with several implementers at once.

A milestone NAME has a branch `milestone/NAME` (from master), a gate worktree `<root>/NAME/_gate` on it, and one
branch `m/NAME/UNIT` with its own worktree `<root>/NAME/UNIT` per unit of work. The root is $TTYAK_WORKTREES if set,
else the folder `<repository name>-worktrees` next to the repository, so the repository's own checks never walk it.

Commands:
  start NAME                       branch milestone/NAME from master, and the gate worktree <root>/NAME/_gate
  add NAME UNIT [--floor-gb N]     branch m/NAME/UNIT and worktree <root>/NAME/UNIT from the milestone tip
  rm NAME UNIT [--delete-branch]   remove that worktree with its build output (the branch only if merged)
  end NAME                         remove the gate worktree once no unit worktree is left
  ls [NAME]                        the worktrees with their size in GB, then the free GB
  gate NAME UNIT                   merge --no-ff the unit into the clean gate; prints `old SHA`, then `new SHA`
  ungate NAME                      reset the clean gate to origin/milestone/NAME (a red gate, before its push)
Global option: --repo PATH (the repository to work on; default: the one this script lives in).
Exit codes: 0 done, 2 refused (bad name, exists, disk floor, not our worktree, dirty gate, no or diverged
origin/milestone/NAME), 3 merge conflict.

Removal is checked: `rm` and `end` delete only the exact path `<root>/NAME/UNIT` (not a symlink, resolving under the
root) that is a registered worktree of this repository, and refuse when it holds uncommitted work (tracked, staged or
untracked files that are not ignored; ignored build output such as target/ is fine).

`ungate` reads the remote-tracking ref as it is (no fetch), refuses unless it is an ancestor of the local
`milestone/NAME`, then runs `git reset --hard` to it in the gate worktree and lists the commits it dropped.

It never pushes or fetches, never deletes a remote branch, never deletes a branch except `rm --delete-branch`, and
then only one that is merged into `milestone/NAME`, and never runs tests.
"""
from __future__ import annotations

import argparse
import os
import re
import shutil
import subprocess
import sys
from pathlib import Path
from typing import Optional

FLOOR_GB = 40  # free GB on the root's filesystem below which `add` refuses
GATE = "_gate"
NAME_RE = re.compile(r"[a-z0-9][a-z0-9.-]{0,40}")


class Refused(Exception):
    """An action the helper will not take. The message says why."""


def check_name(s: str) -> str:
    """A milestone or unit name: lower-case letters, digits, dots and dashes, 1 to 41 characters, no `..`."""
    if not NAME_RE.fullmatch(s) or ".." in s or s.endswith((".", ".lock")):
        raise Refused(f"bad name {s!r}: use a-z, 0-9, . and -, start with a letter or digit, at most 41 characters, "
                      "not ending in . or .lock")
    return s


def git(repo: Path, *args: str, cwd: Optional[Path] = None, check: bool = True) -> subprocess.CompletedProcess:
    """Run git in `cwd` (default: the repository); a failure is a Refused carrying git's message."""
    r = subprocess.run(["git", "-C", str(cwd or repo), *args], capture_output=True, text=True)
    if check and r.returncode != 0:
        raise Refused((r.stderr.strip() or r.stdout.strip() or f"git {args[0]} failed"))
    return r


def find_repo(start: Path) -> Path:
    """The main repository (not a linked worktree) that contains `start`."""
    r = subprocess.run(["git", "-C", str(start), "rev-parse", "--path-format=absolute", "--git-common-dir"],
                       capture_output=True, text=True)
    if r.returncode != 0:
        raise Refused(f"{start} is not in a git repository")
    common = Path(r.stdout.strip())
    if common.name != ".git":
        raise Refused(f"{start}: cannot find the working copy of a bare repository")
    return common.parent


def root(repo: Optional[Path] = None) -> Path:
    """$TTYAK_WORKTREES if set, else <parent of the repository>/<repository folder name>-worktrees."""
    env = os.environ.get("TTYAK_WORKTREES")
    if env:
        rt = Path(env).expanduser()
        if not rt.is_absolute():
            raise Refused(f"TTYAK_WORKTREES must be an absolute path, got {env!r}")
    else:
        repo = repo or find_repo(Path(__file__).resolve().parent)
        rt = repo.parent / f"{repo.name}-worktrees"
    if repo is not None and (rt.resolve() == repo.resolve() or repo.resolve() in rt.resolve().parents):
        raise Refused(f"the worktrees folder {rt} lies inside the repository {repo}")
    return rt


def worktrees(repo: Path) -> list[dict]:
    """`git worktree list --porcelain` as dicts: path (resolved), branch (short name or ''), raw path."""
    out = git(repo, "worktree", "list", "--porcelain").stdout
    items, cur = [], None
    for line in out.splitlines():
        if line.startswith("worktree "):
            cur = {"raw": line[9:], "path": Path(line[9:]).resolve(), "branch": ""}
            items.append(cur)
        elif line.startswith("branch ") and cur is not None:
            cur["branch"] = line[7:].removeprefix("refs/heads/")
    return items


def unit_path(rt: Path, name: str, unit: str) -> Path:
    return rt / name / unit


def check_worktree(repo: Path, rt: Path, name: str, unit: str) -> dict:
    """The `worktrees()` entry for `<root>/NAME/UNIT` if that is exactly a registered worktree of `repo`, else
    Refused. Callers hand git the entry's own path (`raw`), not the path they computed."""
    p = unit_path(rt, name, unit)
    if p.is_symlink():
        raise Refused(f"{p} is a symlink; not touching it")
    if not p.is_dir():
        raise Refused(f"{p} is not a folder")
    if p.resolve() != rt.resolve() / name / unit:
        raise Refused(f"{p} does not resolve to the expected path under {rt}")
    for w in worktrees(repo):
        if w["path"] == p.resolve():
            return w
    raise Refused(f"{p} is not a registered worktree of this repository; not touching it")


def free_gb(rt: Path) -> float:
    p = rt
    while not p.exists() and p != p.parent:
        p = p.parent
    return shutil.disk_usage(p).free / 1e9


def dir_gb(p: Path) -> float:
    total = 0
    for base, dirs, files in os.walk(p, followlinks=False):
        for f in files + dirs:
            try:
                total += os.lstat(os.path.join(base, f)).st_size
            except OSError:
                pass
    return total / 1e9


def branch_exists(repo: Path, branch: str) -> bool:
    return git(repo, "show-ref", "--verify", "--quiet", f"refs/heads/{branch}", check=False).returncode == 0


def cmd_start(repo: Path, rt: Path, a) -> int:
    path, branch = unit_path(rt, a.name, GATE), f"milestone/{a.name}"
    if branch_exists(repo, branch):
        raise Refused(f"branch {branch} exists")
    if os.path.lexists(path):
        raise Refused(f"{path} exists")
    path.parent.mkdir(parents=True, exist_ok=True)
    git(repo, "worktree", "add", "-q", "-b", branch, str(path), "master")
    print(path)
    return 0


def cmd_add(repo: Path, rt: Path, a) -> int:
    path, branch, base = unit_path(rt, a.name, a.unit), f"m/{a.name}/{a.unit}", f"milestone/{a.name}"
    floor = FLOOR_GB if a.floor_gb is None else a.floor_gb
    free = free_gb(rt)
    if free < floor:
        raise Refused(f"only {free:.1f} GB free on the worktrees' filesystem, the floor is {floor} GB")
    if not branch_exists(repo, base):
        raise Refused(f"branch {base} does not exist; start the milestone first")
    if branch_exists(repo, branch):
        raise Refused(f"branch {branch} exists")
    if os.path.lexists(path):
        raise Refused(f"{path} exists")
    path.parent.mkdir(parents=True, exist_ok=True)
    git(repo, "worktree", "add", "-q", "-b", branch, str(path), base)
    print(path)
    return 0


def remove_worktree(repo: Path, rt: Path, name: str, unit: str) -> None:
    """Remove `<root>/NAME/UNIT` with its ignored build output, after the checks; a missing folder is pruned."""
    p = unit_path(rt, name, unit)
    if not os.path.lexists(p):
        git(repo, "worktree", "prune")
        print(f"nothing at {p}; pruned", file=sys.stderr)
        return
    w = check_worktree(repo, rt, name, unit)
    dirty = git(Path(w["raw"]), "status", "--porcelain").stdout.rstrip()
    if dirty:
        raise Refused(f"{p} has uncommitted work (ignored build output is fine); commit or discard it first:\n{dirty}")
    w = check_worktree(repo, rt, name, unit)  # again, right before the removal
    git(repo, "worktree", "remove", "--force", w["raw"])
    # the same checked path again, if ignored build output survived
    if os.path.lexists(p) and not p.is_symlink() and p.resolve() == rt.resolve() / name / unit:
        shutil.rmtree(p)
    git(repo, "worktree", "prune")


def cmd_rm(repo: Path, rt: Path, a) -> int:
    remove_worktree(repo, rt, a.name, a.unit)
    if a.delete_branch:
        branch, base = f"m/{a.name}/{a.unit}", f"milestone/{a.name}"
        if not branch_exists(repo, branch):
            print(f"branch {branch} does not exist", file=sys.stderr)
            return 2
        # merged means into the milestone branch (`git branch -d` would compare with master)
        if any(w["branch"] == branch for w in worktrees(repo)):
            print(f"{branch} is checked out in a worktree; branch kept", file=sys.stderr)
            return 2
        sha = git(repo, "rev-parse", f"refs/heads/{branch}").stdout.strip()
        if git(repo, "merge-base", "--is-ancestor", sha, base, check=False).returncode != 0:
            print(f"{branch} is not merged into {base}; branch kept", file=sys.stderr)
            return 2
        # delete only the commit just checked: a commit that landed since makes this fail
        r = git(repo, "update-ref", "-d", f"refs/heads/{branch}", sha, check=False)
        if r.returncode != 0:
            print(f"{branch} changed while it was checked; branch kept ({r.stderr.strip()})", file=sys.stderr)
            return 2
        # what `git branch -D` also did: drop the branch's config section (absent is fine)
        git(repo, "config", "--remove-section", f"branch.{branch}", check=False)
    return 0


def cmd_end(repo: Path, rt: Path, a) -> int:
    d = rt / a.name
    left = sorted(c.name for c in d.iterdir() if c.name != GATE) if d.is_dir() else []
    left += [w["path"].name for w in worktrees(repo)
             if w["path"].parent == rt.resolve() / a.name and w["path"].name not in left + [GATE]]
    if left:
        raise Refused(f"unit worktrees left: {', '.join(left)}")
    remove_worktree(repo, rt, a.name, GATE)
    if d.is_dir() and not d.is_symlink():
        try:
            d.rmdir()
        except OSError:
            pass
    return 0


def cmd_ls(repo: Path, rt: Path, a) -> int:
    base = rt.resolve()
    for w in worktrees(repo):
        p = w["path"]
        if p.parent.parent != base or (a.name and p.parent.name != a.name):
            continue
        size = f"{dir_gb(p):.2f}" if p.is_dir() else "missing"
        print(p.parent.name, p.name, w["branch"] or "-", size, p)
    print(f"free {free_gb(rt):.1f}")
    return 0


def clean_gate(repo: Path, rt: Path, name: str) -> Path:
    """The gate worktree's path, if it is our registered worktree, on `milestone/NAME` and clean; else Refused."""
    w = check_worktree(repo, rt, name, GATE)
    gate = Path(w["raw"])
    if w["branch"] != f"milestone/{name}":
        raise Refused(f"the gate worktree {gate} is on {w['branch'] or 'no branch'}, not milestone/{name}")
    if git(gate, "status", "--porcelain").stdout.strip():
        raise Refused(f"the gate worktree {gate} is not clean")
    return gate


def cmd_gate(repo: Path, rt: Path, a) -> int:
    gate, branch = clean_gate(repo, rt, a.name), f"m/{a.name}/{a.unit}"
    if not branch_exists(repo, branch):
        raise Refused(f"branch {branch} does not exist")
    old = git(gate, "rev-parse", "HEAD").stdout.strip()
    r = git(gate, "merge", "--no-ff", "--no-edit", branch, check=False)
    if r.returncode != 0:
        files = git(gate, "diff", "--name-only", "--diff-filter=U", check=False).stdout.split()
        if git(gate, "rev-parse", "-q", "--verify", "MERGE_HEAD", check=False).returncode != 0:
            # no merge in progress (e.g. unrelated histories): nothing to abort, show git's own message
            raise Refused(r.stderr.strip() or r.stdout.strip() or "merge failed")
        aborted = git(gate, "merge", "--abort", check=False)
        if (aborted.returncode != 0 or git(gate, "rev-parse", "HEAD").stdout.strip() != old
                or git(gate, "status", "--porcelain").stdout.strip()):
            raise Refused(f"the merge failed and the gate {gate} is not back at {old[:10]} and clean: "
                          f"it needs a hand ({aborted.stderr.strip()})")
        if not files:
            raise Refused(r.stderr.strip() or r.stdout.strip() or "merge failed")
        print("conflict in:")
        for f in files:
            print(f)
        return 3
    print(f"old {old}")
    print(f"new {git(gate, 'rev-parse', 'HEAD').stdout.strip()}")
    return 0


def cmd_ungate(repo: Path, rt: Path, a) -> int:
    gate, remote = clean_gate(repo, rt, a.name), f"refs/remotes/origin/milestone/{a.name}"
    r = git(gate, "rev-parse", "-q", "--verify", f"{remote}^{{commit}}", check=False)
    if r.returncode != 0:
        raise Refused(f"there is no origin/milestone/{a.name} (the remote-tracking ref; nothing is fetched): "
                      "push the milestone branch at `start`")
    tip, local = r.stdout.strip(), git(gate, "rev-parse", "HEAD").stdout.strip()
    if tip == local:
        print(f"nothing to undo: milestone/{a.name} is at origin/milestone/{a.name} ({tip})")
        return 0
    if git(gate, "merge-base", "--is-ancestor", tip, local, check=False).returncode != 0:
        raise Refused(f"origin/milestone/{a.name} ({tip[:10]}) is not an ancestor of milestone/{a.name} "
                      f"({local[:10]}): not resetting; it needs a hand")
    dropped = git(gate, "log", "--oneline", f"{tip}..{local}").stdout.rstrip()
    git(gate, "reset", "-q", "--hard", tip)
    print("dropped:")
    print(dropped)
    print(f"new {git(gate, 'rev-parse', 'HEAD').stdout.strip()}")
    return 0


def build_parser() -> argparse.ArgumentParser:
    ap = argparse.ArgumentParser(prog="milestone.py", description=__doc__,
                                 formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("--repo", type=Path, help="the repository to work on (default: the one this script is in)")
    sub = ap.add_subparsers(dest="cmd", required=True)
    sub.add_parser("start").add_argument("name")
    p = sub.add_parser("add")
    p.add_argument("name")
    p.add_argument("unit")
    p.add_argument("--floor-gb", type=float, default=None, help=f"minimum free GB (default {FLOOR_GB})")
    p = sub.add_parser("rm")
    p.add_argument("name")
    p.add_argument("unit")
    p.add_argument("--delete-branch", action="store_true")
    sub.add_parser("end").add_argument("name")
    sub.add_parser("ls").add_argument("name", nargs="?")
    p = sub.add_parser("gate")
    p.add_argument("name")
    p.add_argument("unit")
    sub.add_parser("ungate").add_argument("name")
    return ap


COMMANDS = {"start": cmd_start, "add": cmd_add, "rm": cmd_rm, "end": cmd_end, "ls": cmd_ls, "gate": cmd_gate,
            "ungate": cmd_ungate}


def main(argv: list[str]) -> int:
    try:
        a = build_parser().parse_args(argv)
    except SystemExit as e:
        return 2 if e.code else 0
    try:
        for key in ("name", "unit"):
            if getattr(a, key, None) is not None:
                check_name(getattr(a, key))
        repo = find_repo(a.repo or Path(__file__).resolve().parent)
        return COMMANDS[a.cmd](repo, root(repo), a)
    except Refused as e:
        print(f"refused: {e}", file=sys.stderr)
        return 2


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
