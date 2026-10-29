from __future__ import annotations

import contextlib
import io
import os
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path
from unittest import mock

REPO = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(REPO))

from tools.milestone import milestone  # noqa: E402


def git(repo: Path, *args: str) -> str:
    r = subprocess.run(["git", "-C", str(repo), *args], capture_output=True, text=True, check=True)
    return r.stdout.strip()


def make_repo(parent: Path, name: str = "repo") -> Path:
    """A temp repository with a repo-local identity and one commit on master."""
    repo = parent / name
    repo.mkdir()
    git(repo, "init", "-q", "-b", "master")
    git(repo, "config", "user.name", "test")
    git(repo, "config", "user.email", "test@example.invalid")
    (repo / "f.txt").write_text("base\n")
    (repo / ".gitignore").write_text("target/\n")
    git(repo, "add", "f.txt", ".gitignore")
    git(repo, "commit", "-q", "-m", "base")
    return repo


def commit_file(wt: Path, name: str, text: str, msg: str = "change") -> None:
    (wt / name).write_text(text)
    git(wt, "add", name)
    git(wt, "commit", "-q", "-m", msg)


class MilestoneTest(unittest.TestCase):
    def setUp(self):
        self._tmp = tempfile.TemporaryDirectory()
        self.addCleanup(self._tmp.cleanup)
        self.tmp = Path(self._tmp.name).resolve()
        self.repo = make_repo(self.tmp)
        self.root = self.tmp / "wt"
        env = mock.patch.dict(os.environ, {"TTYAK_WORKTREES": str(self.root)})
        env.start()
        self.addCleanup(env.stop)

    def run_cli(self, *args: str):
        out, err = io.StringIO(), io.StringIO()
        with contextlib.redirect_stdout(out), contextlib.redirect_stderr(err):
            code = milestone.main(["--repo", str(self.repo), *args])
        return code, out.getvalue(), err.getvalue()

    def start(self, name: str = "trial") -> Path:
        code, out, _ = self.run_cli("start", name)
        self.assertEqual(code, 0)
        return Path(out.strip())

    def add(self, unit: str, name: str = "trial") -> Path:
        code, out, _ = self.run_cli("add", name, unit, "--floor-gb", "0")
        self.assertEqual(code, 0)
        return Path(out.strip())

    def branches(self) -> list[str]:
        return git(self.repo, "branch", "--format=%(refname:short)").split()

    def test_start_makes_branch_and_gate(self):
        gate = self.start()
        self.assertEqual(gate, self.root / "trial" / "_gate")
        self.assertEqual(git(self.repo, "rev-parse", "milestone/trial"), git(self.repo, "rev-parse", "master"))
        self.assertTrue(gate.is_dir())
        self.assertEqual(git(gate, "branch", "--show-current"), "milestone/trial")
        code, _, _ = self.run_cli("start", "trial")
        self.assertEqual(code, 2)

    def test_add_makes_unit_from_milestone_tip(self):
        gate = self.start()
        commit_file(gate, "g.txt", "g\n")
        code, out, _ = self.run_cli("add", "trial", "fix-1", "--floor-gb", "0")
        self.assertEqual(code, 0)
        self.assertEqual(Path(out.strip()), self.root / "trial" / "fix-1")
        self.assertEqual(git(self.repo, "rev-parse", "m/trial/fix-1"), git(self.repo, "rev-parse", "milestone/trial"))
        self.assertTrue((self.root / "trial" / "fix-1" / "g.txt").is_file())

    def test_add_refuses_below_floor(self):
        self.start()
        code, _, _ = self.run_cli("add", "trial", "fix-1", "--floor-gb", str(10 ** 9))
        self.assertEqual(code, 2)
        self.assertNotIn("m/trial/fix-1", self.branches())
        self.assertFalse((self.root / "trial" / "fix-1").exists())

    def test_bad_names_refused(self):
        bad = ["a/b", "..", "a b", "-x", "Fix", "", "a" * 42, "a.lock", "b."]
        for name in bad:
            for argv in (["start", name], ["add", "trial", name, "--floor-gb", "0"], ["rm", name, "u"],
                         ["end", name], ["gate", "trial", name]):
                with self.subTest(name=name, argv=argv), mock.patch("subprocess.run") as run:
                    code, _, _ = self.run_cli(*argv)
                    self.assertEqual(code, 2)
                    run.assert_not_called()

    def test_rm_removes_worktree_and_target(self):
        self.start()
        wt = self.add("fix-1")
        (wt / "target" / "debug").mkdir(parents=True)
        (wt / "target" / "debug" / "x").write_text("x")
        code, _, _ = self.run_cli("rm", "trial", "fix-1")
        self.assertEqual(code, 0)
        self.assertFalse(wt.exists())
        self.assertNotIn(str(wt), git(self.repo, "worktree", "list", "--porcelain"))
        self.assertIn("m/trial/fix-1", self.branches())
        # a merged branch (no new commits) is deleted
        wt = self.add("fix-2")
        code, _, _ = self.run_cli("rm", "trial", "fix-2", "--delete-branch")
        self.assertEqual(code, 0)
        self.assertNotIn("m/trial/fix-2", self.branches())
        # an unmerged one is kept, exit non-zero
        wt = self.add("fix-3")
        commit_file(wt, "u.txt", "u\n")
        code, _, err = self.run_cli("rm", "trial", "fix-3", "--delete-branch")
        self.assertEqual(code, 2)
        self.assertTrue(err.strip())
        self.assertFalse(wt.exists())
        self.assertIn("m/trial/fix-3", self.branches())

    def test_rm_refuses_foreign_paths(self):
        self.start()
        target = self.root / "trial" / "fix-1"
        # a plain folder
        target.mkdir()
        (target / "keep.txt").write_text("k")
        code, _, _ = self.run_cli("rm", "trial", "fix-1")
        self.assertEqual(code, 2)
        self.assertTrue((target / "keep.txt").is_file())
        (target / "keep.txt").unlink()
        target.rmdir()
        # a symlink to another folder
        other = self.tmp / "other"
        other.mkdir()
        (other / "keep.txt").write_text("k")
        target.symlink_to(other)
        code, _, _ = self.run_cli("rm", "trial", "fix-1")
        self.assertEqual(code, 2)
        self.assertTrue((other / "keep.txt").is_file())
        self.assertTrue(target.is_symlink())
        target.unlink()
        # a worktree of a second repository
        repo2 = make_repo(self.tmp, "repo2")
        git(repo2, "worktree", "add", "-q", "-b", "x", str(target))
        code, _, _ = self.run_cli("rm", "trial", "fix-1")
        self.assertEqual(code, 2)
        self.assertTrue((target / "f.txt").is_file())
        # the gate by its reserved name
        code, _, _ = self.run_cli("rm", "trial", "_gate")
        self.assertEqual(code, 2)
        self.assertTrue((self.root / "trial" / "_gate").is_dir())

    def test_rm_refuses_tracked_changes(self):
        self.start()
        wt = self.add("fix-1")
        (wt / "f.txt").write_text("changed\n")
        code, _, _ = self.run_cli("rm", "trial", "fix-1")
        self.assertEqual(code, 2)
        self.assertTrue(wt.is_dir())

    def test_rm_prunes_missing_folder(self):
        self.start()
        wt = self.add("fix-1")
        # remove the folder behind git's back
        for p in sorted(wt.rglob("*"), key=lambda p: len(p.parts), reverse=True):
            p.unlink() if (p.is_file() or p.is_symlink()) else p.rmdir()
        wt.rmdir()
        self.assertIn(str(wt), git(self.repo, "worktree", "list", "--porcelain"))
        code, _, _ = self.run_cli("rm", "trial", "fix-1")
        self.assertEqual(code, 0)
        self.assertNotIn(str(wt), git(self.repo, "worktree", "list", "--porcelain"))

    def test_ls_lists_sizes_and_missing(self):
        self.start()
        a = self.add("a-1")
        b = self.add("b-1")
        for p in sorted(b.rglob("*"), key=lambda p: len(p.parts), reverse=True):
            p.unlink() if (p.is_file() or p.is_symlink()) else p.rmdir()
        b.rmdir()
        code, out, _ = self.run_cli("ls")
        self.assertEqual(code, 0)
        lines = out.strip().splitlines()
        self.assertTrue(lines[-1].startswith("free "))
        by_unit = {ln.split()[1]: ln.split() for ln in lines[:-1]}
        self.assertEqual(set(by_unit), {"_gate", "a-1", "b-1"})
        self.assertEqual(by_unit["a-1"][0:3], ["trial", "a-1", "m/trial/a-1"])
        self.assertEqual(by_unit["a-1"][4], str(a))
        float(by_unit["a-1"][3])
        self.assertEqual(by_unit["b-1"][3], "missing")
        code, out, _ = self.run_cli("ls", "other")
        self.assertEqual(code, 0)
        self.assertEqual(len(out.strip().splitlines()), 1)

    def test_gate_merges_no_ff(self):
        self.start()
        wt = self.add("fix-1")
        commit_file(wt, "u.txt", "u\n")
        old = git(self.repo, "rev-parse", "milestone/trial")
        code, out, _ = self.run_cli("gate", "trial", "fix-1")
        self.assertEqual(code, 0)
        new = git(self.repo, "rev-parse", "milestone/trial")
        self.assertEqual(out.strip(), new)
        self.assertEqual(git(self.repo, "rev-list", "--parents", "-n1", new).split()[1], old)
        self.assertEqual(len(git(self.repo, "rev-list", "--parents", "-n1", new).split()), 3)

    def test_gate_conflict_aborts(self):
        gate = self.start()
        wt = self.add("fix-1")
        commit_file(wt, "f.txt", "unit\n")
        commit_file(gate, "f.txt", "milestone\n")
        old = git(self.repo, "rev-parse", "milestone/trial")
        code, out, _ = self.run_cli("gate", "trial", "fix-1")
        self.assertEqual(code, 3)
        self.assertIn("f.txt", out)
        self.assertEqual(git(gate, "status", "--porcelain"), "")
        self.assertEqual(git(gate, "rev-parse", "HEAD"), old)

    def test_gate_refuses_dirty(self):
        gate = self.start()
        wt = self.add("fix-1")
        commit_file(wt, "u.txt", "u\n")
        old = git(self.repo, "rev-parse", "milestone/trial")
        (gate / "stray.txt").write_text("s")
        code, _, _ = self.run_cli("gate", "trial", "fix-1")
        self.assertEqual(code, 2)
        (gate / "stray.txt").unlink()
        (gate / "f.txt").write_text("dirty\n")
        code, _, _ = self.run_cli("gate", "trial", "fix-1")
        self.assertEqual(code, 2)
        self.assertEqual(git(self.repo, "rev-parse", "milestone/trial"), old)

    def test_end_removes_gate_only_when_units_gone(self):
        gate = self.start()
        self.add("fix-1")
        code, _, _ = self.run_cli("end", "trial")
        self.assertEqual(code, 2)
        self.assertTrue(gate.is_dir())
        self.assertEqual(self.run_cli("rm", "trial", "fix-1")[0], 0)
        code, _, _ = self.run_cli("end", "trial")
        self.assertEqual(code, 0)
        self.assertFalse(gate.exists())
        self.assertIn("milestone/trial", self.branches())

    def test_delete_branch_after_gate(self):
        self.start()
        wt = self.add("fix-1")
        commit_file(wt, "u.txt", "u\n")
        self.assertEqual(self.run_cli("gate", "trial", "fix-1")[0], 0)
        code, _, _ = self.run_cli("rm", "trial", "fix-1", "--delete-branch")
        self.assertEqual(code, 0)
        self.assertNotIn("m/trial/fix-1", self.branches())
        wt = self.add("fix-2")
        commit_file(wt, "v.txt", "v\n")
        code, _, err = self.run_cli("rm", "trial", "fix-2", "--delete-branch")
        self.assertEqual(code, 2)
        self.assertIn("m/trial/fix-2 is not merged into milestone/trial; branch kept", err)
        self.assertIn("m/trial/fix-2", self.branches())

    def test_root_must_be_absolute_and_outside_repo(self):
        with mock.patch.dict(os.environ, {"TTYAK_WORKTREES": "rel/wt"}):
            with self.assertRaises(milestone.Refused):
                milestone.root()
            self.assertEqual(self.run_cli("start", "trial")[0], 2)
        inside = self.repo / "wt"
        with mock.patch.dict(os.environ, {"TTYAK_WORKTREES": str(inside)}):
            with self.assertRaises(milestone.Refused):
                milestone.root(self.repo)
            self.assertEqual(self.run_cli("start", "trial")[0], 2)
        self.assertFalse(inside.exists())
        self.assertNotIn("milestone/trial", self.branches())

    def test_rm_refuses_untracked_and_staged(self):
        self.start()
        wt = self.add("fix-1")
        (wt / "new.txt").write_text("n")
        code, _, err = self.run_cli("rm", "trial", "fix-1")
        self.assertEqual(code, 2)
        self.assertIn("new.txt", err)
        self.assertTrue((wt / "new.txt").is_file())
        git(wt, "add", "new.txt")
        self.assertEqual(self.run_cli("rm", "trial", "fix-1")[0], 2)
        self.assertTrue((wt / "new.txt").is_file())
        git(wt, "reset", "-q", "new.txt")
        (wt / "new.txt").unlink()
        (wt / "target").mkdir()
        (wt / "target" / "x").write_text("x")
        self.assertEqual(self.run_cli("rm", "trial", "fix-1")[0], 0)
        self.assertFalse(wt.exists())

    def test_gate_refuses_wrong_branch(self):
        gate = self.start()
        wt = self.add("fix-1")
        commit_file(wt, "u.txt", "u\n")
        git(gate, "checkout", "-q", "-b", "other")
        code, _, err = self.run_cli("gate", "trial", "fix-1")
        self.assertEqual(code, 2)
        self.assertIn("milestone/trial", err)
        self.assertEqual(git(gate, "rev-parse", "HEAD"), git(self.repo, "rev-parse", "master"))

    def test_gate_reports_failed_abort(self):
        gate = self.start()
        wt = self.add("fix-1")
        commit_file(wt, "f.txt", "unit\n")
        commit_file(gate, "f.txt", "milestone\n")
        orig = milestone.git

        def fake(repo, *args, **kw):
            if args[:2] == ("merge", "--abort"):
                return subprocess.CompletedProcess(args, 1, "", "boom")
            return orig(repo, *args, **kw)

        with mock.patch.object(milestone, "git", fake):
            code, _, err = self.run_cli("gate", "trial", "fix-1")
        self.assertEqual(code, 2)
        self.assertIn("needs a hand", err)

    def test_rm_nothing_there_says_so(self):
        self.start()
        code, _, err = self.run_cli("rm", "trial", "typo")
        self.assertEqual(code, 0)
        self.assertIn(f"nothing at {self.root / 'trial' / 'typo'}; pruned", err)

    def test_repo_option_accepts_a_worktree(self):
        gate = self.start()
        out, err = io.StringIO(), io.StringIO()
        with contextlib.redirect_stdout(out), contextlib.redirect_stderr(err):
            code = milestone.main(["--repo", str(gate), "add", "trial", "fix-1", "--floor-gb", "0"])
        self.assertEqual(code, 0)
        self.assertEqual(Path(out.getvalue().strip()), self.root / "trial" / "fix-1")
        self.assertIn("m/trial/fix-1", self.branches())

    def test_rm_refuses_symlinked_name_folder(self):
        self.start()
        real = self.tmp / "real"
        real.mkdir()
        git(self.repo, "worktree", "add", "-q", "-b", "m/trial/fix-1", str(real / "fix-1"), "milestone/trial")
        (self.root / "other").mkdir()
        (self.root / "link").symlink_to(real)
        code, _, _ = self.run_cli("rm", "link", "fix-1")
        self.assertEqual(code, 2)
        self.assertTrue((real / "fix-1" / "f.txt").is_file())

    def test_start_refuses_existing_folder_without_branch(self):
        (self.root / "trial" / "_gate").mkdir(parents=True)
        code, _, _ = self.run_cli("start", "trial")
        self.assertEqual(code, 2)
        self.assertNotIn("milestone/trial", self.branches())

    def test_check_name_and_root(self):
        self.assertEqual(milestone.check_name("a.b-1"), "a.b-1")
        self.assertEqual(milestone.check_name("a" * 41), "a" * 41)
        with self.assertRaises(milestone.Refused):
            milestone.check_name("a\n")
        with self.assertRaises(milestone.Refused):
            milestone.check_name("_gate")
        self.assertEqual(milestone.root(), self.root)
        with mock.patch.dict(os.environ):
            del os.environ["TTYAK_WORKTREES"]
            self.assertEqual(milestone.root(self.repo), self.tmp / "repo-worktrees")


if __name__ == "__main__":
    unittest.main()
