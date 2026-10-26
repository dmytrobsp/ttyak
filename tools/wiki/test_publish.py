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
        git(self.repo, "config", "user.name", "Repo Person")
        git(self.repo, "config", "user.email", "repo@example.invalid")
        self.global_cfg = self.tmp / "global.gitconfig"
        self.global_cfg.write_text("[user]\n\tname = Global Person\n\temail = global@example.invalid\n", encoding="utf-8")
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

    def run_publish(self, *args, extra_env=None, cwd=None):
        env = {k: v for k, v in os.environ.items() if not k.startswith(("GIT_AUTHOR", "GIT_COMMITTER"))}
        env.update({"TTYAK_WIKI_URL": str(self.bare), "TTYAK_SITE_FORBIDDEN": str(self.forbidden),
                    "TMPDIR": str(self.tmp), "GIT_CONFIG_GLOBAL": str(self.global_cfg), "GIT_CONFIG_NOSYSTEM": "1",
                    "HOME": str(self.tmp), **(extra_env or {})})
        return subprocess.run(["bash", str(SCRIPT), *args], cwd=cwd or self.repo, capture_output=True, text=True, env=env)

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

    def test_commit_uses_repo_identity(self):
        r = self.run_publish("--push")
        self.assertEqual(r.returncode, 0, r.stderr)
        fmt = subprocess.run(["git", "--git-dir", str(self.bare), "log", "-1", "--format=%an|%ae|%cn|%ce"],
                             capture_output=True, text=True).stdout.strip()
        self.assertEqual(fmt, "Repo Person|repo@example.invalid|Repo Person|repo@example.invalid")

    def test_prints_identity_and_url_before_diff(self):
        r = self.run_publish()
        self.assertEqual(r.returncode, 0, r.stderr)
        out = r.stdout
        self.assertIn("Repo Person <repo@example.invalid>", out)
        self.assertIn(str(self.bare), out)
        self.assertLess(out.index("repo@example.invalid"), out.index("diff --git"))
        self.assertLess(out.index(str(self.bare)), out.index("diff --git"))

    def test_refuses_when_only_global_identity(self):
        git(self.repo, "config", "--unset", "user.name")
        git(self.repo, "config", "--unset", "user.email")
        r = self.run_publish("--push")
        self.assertNotEqual(r.returncode, 0)
        self.assertEqual(self.wiki_log(), "")
        self.assertIn("identity", r.stderr)

    def test_caller_env_identity_does_not_win(self):
        other = {"GIT_COMMITTER_NAME": "Env Person", "GIT_COMMITTER_EMAIL": "env@example.invalid",
                 "GIT_AUTHOR_NAME": "Env Person", "GIT_AUTHOR_EMAIL": "env@example.invalid"}
        r = self.run_publish("--push", extra_env=other)
        self.assertEqual(r.returncode, 0, r.stderr)
        fmt = subprocess.run(["git", "--git-dir", str(self.bare), "log", "-1", "--format=%an|%ae|%cn|%ce"],
                             capture_output=True, text=True).stdout.strip()
        self.assertEqual(fmt, "Repo Person|repo@example.invalid|Repo Person|repo@example.invalid")

    def test_clone_gets_repo_credential_entries_in_order(self):
        key = "credential.https://example.invalid.helper"
        git(self.repo, "config", "--local", "--add", key, "")
        git(self.repo, "config", "--local", "--add", key, "!echo a b")
        wd = self.tmp / "clone"
        r = self.run_publish("--wiki-dir", str(wd))
        self.assertEqual(r.returncode, 0, r.stderr)
        self.assertEqual(git(wd, "config", "--local", "--get-all", key), "\n!echo a b\n")
        # a reused clone does not accumulate duplicates
        r = self.run_publish("--wiki-dir", str(wd))
        self.assertEqual(r.returncode, 0, r.stderr)
        self.assertEqual(git(wd, "config", "--local", "--get-all", key), "\n!echo a b\n")

    def test_clone_without_repo_credentials_gets_none(self):
        wd = self.tmp / "clone"
        r = self.run_publish("--wiki-dir", str(wd))
        self.assertEqual(r.returncode, 0, r.stderr)
        got = subprocess.run(["git", "config", "--local", "--get-regexp", "^credential\\."], cwd=wd,
                             capture_output=True, text=True)
        self.assertEqual(got.stdout, "")

    def assert_repo_untouched(self, head, r):
        self.assertNotEqual(r.returncode, 0, r.stdout)
        self.assertIn("refusing", r.stderr)
        self.assertEqual(git(self.repo, "rev-parse", "HEAD"), head)
        self.assertEqual((self.repo / "docs" / "wiki" / "Home.md").read_text(), "# Home\nuncommitted\n")
        self.assertEqual(self.wiki_log(), "")

    def test_refuses_wiki_dir_dot(self):
        # `--wiki-dir .` from the repo must not mirror into the repo itself, even when its origin is the wiki URL
        git(self.repo, "remote", "add", "origin", str(self.bare))
        head = git(self.repo, "rev-parse", "HEAD")
        self.write("Home", "# Home\nuncommitted\n")
        r = self.run_publish("--wiki-dir", ".", "--push")
        self.assert_repo_untouched(head, r)

    def test_refuses_wiki_dir_inside_repo(self):
        head = git(self.repo, "rev-parse", "HEAD")
        self.write("Home", "# Home\nuncommitted\n")
        r = self.run_publish("--wiki-dir", str(self.repo / "sub"))
        self.assert_repo_untouched(head, r)
        self.assertFalse((self.repo / "sub").exists())

    def test_relative_wiki_dir_is_from_the_callers_folder(self):
        # from docs/, `../clone` is repo/clone (inside the repo: refused), not a folder beside the repo
        head = git(self.repo, "rev-parse", "HEAD")
        self.write("Home", "# Home\nuncommitted\n")
        r = self.run_publish("--wiki-dir", "../clone", cwd=self.repo / "docs")
        self.assert_repo_untouched(head, r)
        self.assertFalse((self.tmp / "clone").exists())
        self.assertFalse((self.repo / "clone").exists())

    def test_refuses_wiki_dir_holding_the_repo(self):
        head = git(self.repo, "rev-parse", "HEAD")
        self.write("Home", "# Home\nuncommitted\n")
        r = self.run_publish("--wiki-dir", str(self.tmp))
        self.assert_repo_untouched(head, r)

    def test_refuses_reused_clone_with_another_origin(self):
        other_bare = self.tmp / "other.git"
        other = self.tmp / "other"
        git(self.tmp, "init", "-q", "--bare", str(other_bare))
        git(self.tmp, "clone", "-q", str(other_bare), str(other))
        (other / "keep.txt").write_text("keep\n", encoding="utf-8")
        git(other, "add", "-A")
        git(other, "commit", "-q", "-m", "other")
        git(other, "push", "-q", "origin", "HEAD:master")
        head = git(other, "rev-parse", "HEAD")
        r = self.run_publish("--wiki-dir", str(other), "--push")
        self.assertNotEqual(r.returncode, 0, r.stdout)
        self.assertIn("refusing", r.stderr)
        self.assertIn("origin", r.stderr)
        self.assertEqual(git(other, "rev-parse", "HEAD"), head)
        self.assertEqual((other / "keep.txt").read_text(), "keep\n")
        self.assertEqual(self.wiki_log(), "")
        r = subprocess.run(["git", "--git-dir", str(other_bare), "log", "--format=%s", "master"],
                           capture_output=True, text=True)
        self.assertEqual(r.stdout, "other\n")

    def test_reused_clone_publishes_and_pushes_to_the_wiki(self):
        wd = self.tmp / "clone"
        r = self.run_publish("--wiki-dir", str(wd), "--push")
        self.assertEqual(r.returncode, 0, r.stderr)
        r = self.run_publish("--ref", "master", "--wiki-dir", str(wd), "--push")
        self.assertEqual(r.returncode, 0, r.stderr)
        self.assertIn("new", self.wiki_file("Home.md"))
        self.assertTrue(self.wiki_log().startswith("wiki: docs/wiki at master ("), self.wiki_log())

    def test_same_size_edit_with_equal_mtimes_is_published(self):
        # the exported files carry their commit's time; two commits in one second and an edit of the same size must
        # still differ for git (a stat-only compare, as on a coarse file system, sees none)
        env = {**os.environ, **GIT_ENV, "GIT_AUTHOR_DATE": "2020-01-01T00:00:00", "GIT_COMMITTER_DATE": "2020-01-01T00:00:00"}
        for text, tag in (("# Home\nold\n", "same-a"), ("# Home\nnew\n", "same-b")):
            self.write("Home", text)
            subprocess.run(["git", "commit", "-q", "-am", tag], cwd=self.repo, env=env, check=True)
            git(self.repo, "tag", tag)
        wd = self.tmp / "clone"
        r = self.run_publish("--ref", "same-a", "--wiki-dir", str(wd), "--push")
        self.assertEqual(r.returncode, 0, r.stderr)
        git(wd, "config", "core.checkStat", "minimal")
        git(wd, "config", "core.trustctime", "false")
        r = self.run_publish("--ref", "same-b", "--wiki-dir", str(wd), "--push")
        self.assertEqual(r.returncode, 0, r.stderr)
        self.assertIn("new", self.wiki_file("Home.md"))

    def test_dry_run_then_push_with_same_wiki_dir_pushes(self):
        wd = self.tmp / "clone"
        r = self.run_publish("--wiki-dir", str(wd))
        self.assertEqual(r.returncode, 0, r.stderr)
        self.assertEqual(self.wiki_log(), "")
        r = self.run_publish("--wiki-dir", str(wd), "--push")
        self.assertEqual(r.returncode, 0, r.stderr)
        self.assertTrue(self.wiki_log().startswith("wiki: docs/wiki at v1.0.0 ("), self.wiki_log())
        self.assertIn("old", self.wiki_file("Home.md"))

    def seeded_clone(self):
        wd = self.tmp / "clone"
        self.assertEqual(self.run_publish("--wiki-dir", str(wd), "--push").returncode, 0)
        return wd

    def test_reused_clone_leftover_untracked_file_is_not_published(self):
        wd = self.seeded_clone()
        (wd / "Secret.md").write_text("# Secret\n", encoding="utf-8")
        r = self.run_publish("--ref", "master", "--wiki-dir", str(wd), "--push")
        self.assertEqual(r.returncode, 0, r.stderr)
        self.assertIsNone(self.wiki_file("Secret.md"))
        self.assertFalse((wd / "Secret.md").exists())

    def test_reused_clone_local_commit_is_not_published(self):
        wd = self.seeded_clone()
        (wd / "x.md").write_text("# x\n", encoding="utf-8")
        git(wd, "add", "-A")
        git(wd, "commit", "-q", "-m", "local")
        r = self.run_publish("--ref", "master", "--wiki-dir", str(wd), "--push")
        self.assertEqual(r.returncode, 0, r.stderr)
        self.assertIsNone(self.wiki_file("x.md"))
        self.assertNotIn("local", self.wiki_log())

    def test_reused_clone_dirty_edit_is_not_published(self):
        wd = self.seeded_clone()
        (wd / "Home.md").write_text("# Home\ndirty\n", encoding="utf-8")
        r = self.run_publish("--wiki-dir", str(wd), "--push")
        self.assertEqual(r.returncode, 0, r.stderr)
        self.assertNotIn("dirty", self.wiki_file("Home.md"))
        self.assertNotIn("dirty", (wd / "Home.md").read_text())

    def test_reused_clone_with_another_work_tree_is_refused(self):
        wd = self.seeded_clone()
        other = self.tmp / "other-tree"
        other.mkdir()
        (other / "keep.txt").write_text("keep\n", encoding="utf-8")
        git(wd, "config", "core.worktree", str(other))
        r = self.run_publish("--ref", "master", "--wiki-dir", str(wd), "--push")
        self.assertNotEqual(r.returncode, 0, r.stdout)
        self.assertIn("refusing", r.stderr)
        self.assertEqual((other / "keep.txt").read_text(), "keep\n")
        self.assertEqual(sorted(p.name for p in other.iterdir()), ["keep.txt"])
        self.assertIn("old", self.wiki_file("Home.md"))

    def test_git_environment_of_the_caller_has_no_effect(self):
        other = self.tmp / "other-tree"
        other.mkdir()
        (other / "keep.txt").write_text("keep\n", encoding="utf-8")
        wd = self.seeded_clone()
        r = self.run_publish("--ref", "master", "--wiki-dir", str(wd), "--push",
                             extra_env={"GIT_WORK_TREE": str(other), "GIT_DIR": str(self.repo / ".git")})
        self.assertEqual(r.returncode, 0, r.stderr)
        self.assertEqual((other / "keep.txt").read_text(), "keep\n")
        self.assertIn("new", self.wiki_file("Home.md"))

    def test_an_ignore_rule_does_not_drop_a_page(self):
        (self.repo / "docs" / "wiki" / ".gitignore").write_text("Ignored.md\n", encoding="utf-8")
        self.write("Ignored", "# Ignored\n")
        git(self.repo, "add", "-f", "-A")
        git(self.repo, "commit", "-q", "-m", "ignored")
        r = self.run_publish("--ref", "master", "--push")
        self.assertEqual(r.returncode, 0, r.stderr)
        self.assertEqual(self.wiki_file("Ignored.md"), "# Ignored\n")

    def test_same_dir_spelled_otherwise_is_refused(self):
        link = self.tmp / "link"
        link.symlink_to(self.tmp)
        spelled = link / "repo" / "sub"
        r = subprocess.run(["bash", "-c", 'source "$1"; wiki_dir_clash "$2" "$3"', "x", str(SCRIPT), str(self.repo), str(spelled)],
                           capture_output=True, text=True)
        self.assertEqual(r.stdout.strip(), "inside", r.stderr)
        r = subprocess.run(["bash", "-c", 'source "$1"; wiki_dir_clash "$2" "$3"', "x", str(SCRIPT), str(self.repo), str(link)],
                           capture_output=True, text=True)
        self.assertEqual(r.stdout.strip(), "holds", r.stderr)
        r = subprocess.run(["bash", "-c", 'source "$1"; wiki_dir_clash "$2" "$3"', "x", str(SCRIPT), str(self.repo), str(link / "beside")],
                           capture_output=True, text=True)
        self.assertEqual(r.stdout.strip(), "", r.stderr)

    def test_default_clone_is_said_to_be_deleted(self):
        r = self.run_publish()
        self.assertEqual(r.returncode, 0, r.stderr)
        self.assertIn("deleted on exit", r.stdout)
        self.assertIn("--wiki-dir", r.stdout)

    def test_refuses_without_identity(self):
        git(self.repo, "config", "--unset", "user.name")
        git(self.repo, "config", "--unset", "user.email")
        self.global_cfg.write_text("", encoding="utf-8")
        r = self.run_publish("--push")
        self.assertNotEqual(r.returncode, 0)
        self.assertEqual(self.wiki_log(), "")
        self.assertIn("identity", r.stderr)


class WikiUrl(unittest.TestCase):
    def derive(self, origin):
        r = subprocess.run(["bash", "-c", 'source "$1"; wiki_url_from_origin "$2"', "x", str(SCRIPT), origin],
                           capture_output=True, text=True)
        return r.returncode, r.stdout.strip()

    def test_derivations(self):
        cases = {
            "https://dmytrobsp@github.com/dmytrobsp/ttyak.git": "https://dmytrobsp@github.com/dmytrobsp/ttyak.wiki.git",
            "https://github.com/dmytrobsp/ttyak": "https://github.com/dmytrobsp/ttyak.wiki.git",
            "git@github.com:dmytrobsp/ttyak.git": "git@github.com:dmytrobsp/ttyak.wiki.git",
            "ssh://git@github.com/dmytrobsp/ttyak.git": "ssh://git@github.com/dmytrobsp/ttyak.wiki.git",
        }
        for origin, want in cases.items():
            self.assertEqual(self.derive(origin), (0, want), origin)

    def test_unrecognised_origin_fails(self):
        self.assertNotEqual(self.derive("https://example.com/other/repo.git")[0], 0)


if __name__ == "__main__":
    unittest.main()
