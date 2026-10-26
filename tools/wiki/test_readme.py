"""The README is the short front door: few lines, a fixed outline, a wiki link in each section, no broken link."""
import re
import sys
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import check_links  # noqa: E402

ROOT = Path(__file__).resolve().parents[2]
README = ROOT / "README.md"
WIKI = "https://github.com/dmytrobsp/ttyak/wiki/"

OUTLINE = [
    "## What ttyak is",
    "## Signing in",
    "## Install",
    "### Requirements",
    "### Verify",
    "### Update",
    "### Uninstall",
    "## Build and contribute",
    "## Project layout",
    "## Licence",
]


def _text():
    return README.read_text(encoding="utf-8")


def _headings(text):
    """The ## and ### headings outside code fences, in order."""
    return [line.rstrip() for _, line in check_links._lines_outside_fences(text) if re.match(r"^#{2,3} ", line)]


def _sections(text):
    """{'## heading': its text up to the next ## heading} (### subsections belong to their ## section)."""
    out, cur = {}, None
    for _, line in check_links._lines_outside_fences(text):
        if re.match(r"^## ", line):
            cur = line.rstrip()
            out[cur] = ""
        elif cur:
            out[cur] += line + "\n"
    return out


class Readme(unittest.TestCase):
    def test_short(self):
        n = len(_text().splitlines())
        self.assertLess(n, 200, "the README has %d lines; the detail belongs in the wiki" % n)

    def test_section_order(self):
        self.assertEqual(_headings(_text()), OUTLINE)

    def test_shipped_anchors(self):
        # The 0.12.1 binary, the site and the package manifests link to these.
        self.assertTrue({"requirements", "verify", "licence"} <= check_links.anchors(_text()))

    def test_every_section_links_the_wiki(self):
        for heading, body in _sections(_text()).items():
            if heading == "## Licence":
                continue
            with self.subTest(heading):
                self.assertIn(WIKI, body)

    def test_links_resolve(self):
        bad = check_links.check(ROOT / "docs" / "wiki", README, [ROOT / "site" / "index.html"])
        self.assertEqual(bad, [])

    def test_local_files_exist(self):
        # Relative links and images (screenshots, CONTRIBUTING.md, the licences) point at files in the repo.
        targets = list(check_links._links(README))
        for n, line in enumerate(_text().splitlines(), 1):
            targets += [(n, src) for src in re.findall(r"""\bsrc\s*=\s*["']([^"']+)["']""", line)]
        for n, target in targets:
            if re.match(r"^[a-zA-Z][a-zA-Z0-9+.-]*:|^#", target):
                continue
            with self.subTest(line=n, target=target):
                self.assertTrue((ROOT / target.partition("#")[0]).exists(), "README.md:%d: %s" % (n, target))


if __name__ == "__main__":
    unittest.main()
