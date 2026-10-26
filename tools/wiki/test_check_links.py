"""Tests for check_links.py: heading slugs, anchors, relative and absolute links, the command line."""
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import check_links  # noqa: E402

SCRIPT = Path(__file__).resolve().parent / "check_links.py"
WIKI = "https://github.com/dmytrobsp/ttyak/wiki/"


class Tree:
    """A temp folder with docs/wiki and optional README / site files."""

    def __init__(self, test, wiki, readme=None, site=None):
        tmp = tempfile.TemporaryDirectory()
        test.addCleanup(tmp.cleanup)
        self.root = Path(tmp.name)
        self.wiki = self.root / "wiki"
        self.wiki.mkdir()
        for name, text in wiki.items():
            (self.wiki / (name + ".md")).write_text(text, encoding="utf-8")
        self.readme = self.root / "README.md" if readme is not None else None
        if readme is not None:
            self.readme.write_text(readme, encoding="utf-8")
        self.site = self.root / "index.html" if site is not None else None
        if site is not None:
            self.site.write_text(site, encoding="utf-8")


class CheckLinks(unittest.TestCase):
    def test_slug(self):
        self.assertEqual(check_links.slug("If it doesn't work"), "if-it-doesnt-work")
        self.assertEqual(check_links.slug("Status for bars & scripts"), "status-for-bars--scripts")
        self.assertEqual(check_links.slug("F6 → Keys"), "f6--keys")

    def test_anchors_skip_code_and_number_duplicates(self):
        text = "# Top\n```\n# not\n```\n## Linux\ntext\n## Linux\n## Linux\n<a id=\"mine\"></a>\n"
        self.assertEqual(check_links.anchors(text), {"top", "linux", "linux-1", "linux-2", "mine"})

    def test_relative_wiki_links(self):
        t = Tree(self, {"Home": "# Home\nsee [o](Other#x) and [p](Other)\n", "Other": "# Other\n## x\n"})
        self.assertEqual(check_links.check(t.wiki, None, []), [])
        t = Tree(self, {"Home": "# Home\nfine\nsee [o](Other#y) and [p](Missing)\n", "Other": "# Other\n## x\n"})
        bad = check_links.check(t.wiki, None, [])
        self.assertEqual(len(bad), 2)
        self.assertTrue(bad[0].endswith("Home.md:3: Other#y"), bad[0])
        self.assertTrue(bad[1].endswith("Home.md:3: Missing"), bad[1])

    def test_own_anchors_and_code_are_handled(self):
        t = Tree(self, {"Home": "# Home\n## Part\n[a](#part) [b](#nope)\n`[c](Gone)`\n```\n[d](Gone)\n```\n"})
        bad = check_links.check(t.wiki, None, [])
        self.assertEqual(len(bad), 1)
        self.assertTrue(bad[0].endswith("Home.md:3: #nope"), bad[0])

    def test_absolute_wiki_and_readme_links(self):
        t = Tree(self, {"Install": "# Install\n## Update\n"},
                 readme="# ttyak\n## Verify\n[u](%sInstall#update) [bad](%sInstall#gone)\n" % (WIKI, WIKI),
                 site='<a href="https://github.com/dmytrobsp/ttyak#verify">v</a> '
                      '<a href="https://github.com/dmytrobsp/ttyak#nope">n</a> <a href="#x">x</a><b id="x"></b>')
        bad = check_links.check(t.wiki, t.readme, [t.site])
        self.assertEqual(len(bad), 2, bad)
        self.assertTrue(bad[0].endswith("README.md:3: %sInstall#gone" % WIKI), bad[0])
        self.assertTrue(bad[1].endswith("index.html:1: https://github.com/dmytrobsp/ttyak#nope"), bad[1])

    def test_ignores_other_urls(self):
        t = Tree(self, {"Home": "# Home\n[a](https://example.com/x#y) [b](mailto:a@b.c) [c](img/pic.png)\n"})
        self.assertEqual(check_links.check(t.wiki, None, []), [])

    def test_relative_file_links_in_other_markdown(self):
        t = Tree(self, {"Home": "# Home\n"})
        (t.root / "docs" / "agents").mkdir(parents=True)
        (t.root / "tools").mkdir()
        (t.root / "tools" / "README.md").write_text("# Tools\n## The VM\n", encoding="utf-8")
        (t.root / "docs" / "RELEASING.md").write_text("# Releasing\n## Tag\n", encoding="utf-8")
        page = t.root / "docs" / "agents" / "workflow.md"
        page.write_text("# Workflow\n"
                        "[ok](../RELEASING.md) [ok](../RELEASING.md#tag) [ok](../../tools/README.md#the-vm)\n"
                        "[dir](../../tools/) [ok](<../RELEASING.md>) [web](https://example.com/x.md#y)\n"
                        "[gone](../GONE.md) [anchor](../RELEASING.md#nope) [img](../images/none.png)\n"
                        "`[code](../GONE.md)`\n", encoding="utf-8")
        bad = check_links.check(t.wiki, None, [page])
        self.assertEqual(len(bad), 3, bad)
        self.assertTrue(bad[0].endswith("workflow.md:4: ../GONE.md"), bad[0])
        self.assertTrue(bad[1].endswith("workflow.md:4: ../RELEASING.md#nope"), bad[1])
        self.assertTrue(bad[2].endswith("workflow.md:4: ../images/none.png"), bad[2])

    def test_relative_links_with_a_query_or_encoded_characters(self):
        t = Tree(self, {"Home": "# Home\n"})
        (t.root / "a b.md").write_text("# A\n## Part\n", encoding="utf-8")
        page = t.root / "page.md"
        page.write_text("# P\n[q](a%20b.md?plain=1#part) [e](a%20b.md) [bad](a%20c.md?x=1)\n", encoding="utf-8")
        bad = check_links.check(t.wiki, None, [page])
        self.assertEqual(len(bad), 1, bad)
        self.assertTrue(bad[0].endswith("page.md:2: a%20c.md?x=1"), bad[0])

    def test_readme_and_html_relative_links_are_not_followed(self):
        # The wiki's publish exports only the wiki, the README and the site: their own relative files are not there
        # (test_readme.py checks the README's).
        t = Tree(self, {"Home": "# Home\n"}, readme="# R\n[c](CONTRIBUTING.md)\n", site='<a href="img/x.png">x</a>')
        self.assertEqual(check_links.check(t.wiki, t.readme, [t.site]), [])

    def test_cli_default_set_takes_the_agent_docs(self):
        t = Tree(self, {"Home": "# Home\n"}, readme="# R\n")
        (t.root / "docs" / "wiki").mkdir(parents=True)
        (t.root / "docs" / "agents").mkdir()
        (t.root / "AGENTS.md").write_text("# Agents\n[w](docs/agents/workflow.md)\n", encoding="utf-8")
        (t.root / "docs" / "agents" / "workflow.md").write_text("# W\n[gone](../GONE.md)\n", encoding="utf-8")
        for d in ("distribution", "protocol"):
            (t.root / "docs" / d).mkdir()
            (t.root / "docs" / d / "x.md").write_text("# X\n[gone](GONE-%s.md)\n" % d, encoding="utf-8")
        r = subprocess.run([sys.executable, str(SCRIPT)], capture_output=True, text=True, cwd=t.root)
        self.assertEqual(r.returncode, 1, r.stdout + r.stderr)
        self.assertIn("workflow.md:2: ../GONE.md", r.stdout)
        self.assertIn("x.md:2: GONE-distribution.md", r.stdout)
        self.assertIn("x.md:2: GONE-protocol.md", r.stdout)
        self.assertNotIn("AGENTS.md", r.stdout)

    def test_cli_exit_code(self):
        t = Tree(self, {"Home": "# Home\n[a](Home)\n"}, readme="# R\n")
        run = lambda: subprocess.run([sys.executable, str(SCRIPT), "--wiki", str(t.wiki), "--readme", str(t.readme),
                                      str(t.root / "none.html")], capture_output=True, text=True)
        self.assertEqual(run().returncode, 0)
        (t.wiki / "Home.md").write_text("# Home\n[a](Gone)\n", encoding="utf-8")
        r = run()
        self.assertEqual(r.returncode, 1)
        self.assertIn("Gone", r.stdout)


if __name__ == "__main__":
    unittest.main()
