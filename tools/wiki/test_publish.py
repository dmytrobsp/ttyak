"""Tests for publish.sh against a local bare repo standing in for the wiki (the real wiki is never contacted)."""
import os
import subprocess
import tempfile
import unittest
from pathlib import Path

SCRIPT = Path(__file__).resolve().parent / "publish.sh"
GIT_ENV = {"GIT_AUTHOR_NAME": "t", "GIT_AUTHOR_EMAIL": "t@example.invalid",
           "GIT_COMMITTER_NAME": "t", "GIT_COMMITTER_EMAIL": "t@example.invalid"}


def git(cwd, *args):
    return subprocess.run(["git", *args], cwd=cwd, check=True, capture_output=True, text=True,
                          env={**os.environ, **GIT_ENV}).stdout


class Publish(unittest.TestCase):
    def setUp(self):
        tmp = tempfile.TemporaryDirectory()
        self.addCleanup(tmp.cleanup)
        self.tmp = Path(tmp.name)
        self.repo = self.tmp / "repo"
        self.bare = self.tmp / "wiki.git"
        self.forbidden = self.tmp / "forbidden.txt"
        self.forbidden.write_text("# fake list\nzzz-never-written-qq\n", encoding="utf-8")
        git(self.tmp, "init", "-q", "--bare", str(self.bare))
        git(self.tmp, "init", "-q", "-b", "master", str(self.repo))
        (self.repo / "docs" / "wiki").mkdir(parents=True)
        self.write("Home", "# Home\nold\n")
        self.write("Gone", "# Gone\nwill be deleted\n")
        self.commit("old")
        git(self.repo, "tag", "v1.0.0")
        self.write("Home", "# Home\nnew\n")
        (self.repo / "docs" / "wiki" / "Gone.md").unlink()
        self.commit("new")

    def write(self, name, text):
        (self.repo / "docs" / "wiki" / (name + ".md")).write_text(text, encoding="utf-8")

    def commit(self, msg):
        git(self.repo, "add", "-A")
        git(self.repo, "commit", "-q", "-m", msg)

    def run_publish(self, *args):
        env = {**os.environ, **GIT_ENV, "TTYAK_WIKI_URL": str(self.bare), "TTYAK_SITE_FORBIDDEN": str(self.forbidden),
               "TMPDIR": str(self.tmp)}
        return subprocess.run(["bash", str(SCRIPT), *args], cwd=self.repo, capture_output=True, text=True, env=env)

    def wiki_log(self):
        r = subprocess.run(["git", "--git-dir", str(self.bare), "log", "--format=%s"], capture_output=True, text=True)
        return r.stdout if r.returncode == 0 else ""

    def wiki_file(self, name):
        r = subprocess.run(["git", "--git-dir", str(self.bare), "show", "HEAD:" + name], capture_output=True, text=True)
        return r.stdout if r.returncode == 0 else None

    def test_default_ref_is_latest_tag(self):
        wd = self.tmp / "clone"
        r = self.run_publish("--wiki-dir", str(wd))
        self.assertEqual(r.returncode, 0, r.stderr)
        self.assertIn("v1.0.0", r.stdout.splitlines()[0])
        self.assertIn("old", (wd / "Home.md").read_text())

    def test_ref_master(self):
        wd = self.tmp / "clone"
        r = self.run_publish("--ref", "master", "--wiki-dir", str(wd))
        self.assertEqual(r.returncode, 0, r.stderr)
        self.assertIn("master", r.stdout.splitlines()[0])
        self.assertIn("new", (wd / "Home.md").read_text())

    def test_no_push_without_flag(self):
        r = self.run_publish("--wiki-dir", str(self.tmp / "clone"))
        self.assertEqual(r.returncode, 0, r.stderr)
        self.assertEqual(self.wiki_log(), "")
        self.assertIn("wiki: docs/wiki at v1.0.0", git(self.tmp / "clone", "log", "-1", "--format=%s"))

    def test_push(self):
        r = self.run_publish("--push")
        self.assertEqual(r.returncode, 0, r.stderr)
        self.assertTrue(self.wiki_log().startswith("wiki: docs/wiki at v1.0.0 ("), self.wiki_log())
        self.assertIn("old", self.wiki_file("Home.md"))

    def test_deleted_page_is_removed(self):
        self.assertEqual(self.run_publish("--push").returncode, 0)
        self.assertIsNotNone(self.wiki_file("Gone.md"))
        r = self.run_publish("--ref", "master", "--push")
        self.assertEqual(r.returncode, 0, r.stderr)
        self.assertIsNone(self.wiki_file("Gone.md"))
        self.assertIn("new", self.wiki_file("Home.md"))

    def test_refuses_on_broken_link(self):
        self.write("Home", "# Home\n[x](Missing)\n")
        self.commit("broken")
        r = self.run_publish("--ref", "master", "--push")
        self.assertNotEqual(r.returncode, 0)
        self.assertEqual(self.wiki_log(), "")

    def test_refuses_on_forbidden_string(self):
        self.write("Home", "# Home\nzzz-never-written-qq\n")
        self.commit("leak")
        r = self.run_publish("--ref", "master", "--push")
        self.assertNotEqual(r.returncode, 0)
        self.assertEqual(self.wiki_log(), "")
        self.assertNotIn("zzz-never-written-qq", r.stdout + r.stderr)


if __name__ == "__main__":
    unittest.main()
