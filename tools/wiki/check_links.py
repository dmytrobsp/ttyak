"""Check the links of the wiki pages, the README and other files: page names, heading anchors, #anchors in a file,
and relative file links (with their anchors) in other Markdown files."""
import argparse
import glob
import re
import sys
from pathlib import Path
from urllib.parse import unquote

REPO = "https://github.com/dmytrobsp/ttyak"
_FENCE = re.compile(r"^\s*(```|~~~)")
_HEADING = re.compile(r"^ {0,3}(#{1,6})[ \t]+(.*?)(?:[ \t]+#+)?[ \t]*$")
_EXPLICIT = re.compile(r"""<a\s[^>]*?\b(?:id|name)\s*=\s*["']([^"']+)["']""", re.I)
_HTML_ID = re.compile(r"""\bid\s*=\s*["']([^"']+)["']""", re.I)
_LINK = re.compile(r"""\]\(\s*<?([^)\s>]+)|\bhref\s*=\s*["']([^"']+)["']""", re.I)
_CODE = re.compile(r"`[^`]*`")
_ABS_WIKI = re.compile(re.escape(REPO) + r"/wiki/([^\s)#\"'<>]+)(?:#([^\s)\"'<>]*))?")
_ABS_README = re.compile(re.escape(REPO) + r"/?#([^\s)\"'<>]*)")
_SCHEME = re.compile(r"^[a-zA-Z][a-zA-Z0-9+.-]*:")
# What `check_links.py` checks without arguments besides the wiki and the README (globs, from the repo root).
DEFAULT_EXTRA = ["site/index.html", "AGENTS.md", "CONTRIBUTING.md", "docs/*.md", "docs/agents/*.md",
                 "docs/history/README.md", "docs/superpowers/ledgers/README.md", "tools/README.md", "tools/*/README.md"]


def slug(heading):
    """GitHub's heading slug: lowercase, keep letters, digits, spaces, - and _, spaces become -."""
    s = re.sub(r"\[([^\]]*)\]\([^)]*\)", r"\1", heading)
    s = re.sub(r"<[^>]*>", "", s).replace("`", "").lower()
    return "".join(c for c in s if c.isalnum() or c in " -_").replace(" ", "-")


def _lines_outside_fences(text):
    fenced = False
    for n, line in enumerate(text.splitlines(), 1):
        if _FENCE.match(line):
            fenced = not fenced
            continue
        if not fenced:
            yield n, line


def anchors(md_text):
    """The anchors of a Markdown text: heading slugs (duplicates x, x-1, x-2) and explicit <a id/name>."""
    found, seen = set(), {}
    for _, line in _lines_outside_fences(md_text):
        m = _HEADING.match(line)
        if m:
            base = slug(m.group(2))
            k = seen.get(base, 0)
            seen[base] = k + 1
            found.add(base if k == 0 else "%s-%d" % (base, k))
        found.update(_EXPLICIT.findall(line))
    return found


def _anchors_of(path):
    text = path.read_text(encoding="utf-8")
    if path.suffix.lower() in (".html", ".htm"):
        return set(_HTML_ID.findall(text))
    return anchors(text)


def _links(path):
    """(line number, target) for every link in a file; code is skipped in Markdown."""
    text = path.read_text(encoding="utf-8")
    if path.suffix.lower() in (".html", ".htm"):
        lines = list(enumerate(text.splitlines(), 1))
    else:
        lines = [(n, _CODE.sub("", line)) for n, line in _lines_outside_fences(text)]
    for n, line in lines:
        got = []
        for m in _LINK.finditer(line):
            got.append(m.group(1) or m.group(2))
        for t in got:
            yield n, t
        for m in list(_ABS_WIKI.finditer(line)) + list(_ABS_README.finditer(line)):
            if not any(m.group(0) in t for t in got):
                yield n, m.group(0)


def check(wiki, readme, extra):
    """One 'file:line: target' line per broken link."""
    wiki = Path(wiki)
    pages = {p.stem: p for p in sorted(wiki.glob("*.md"))} if wiki.is_dir() else {}
    cache = {}

    def anchor_set(p):
        if p not in cache:
            cache[p] = {a.lower() for a in _anchors_of(p)}
        return cache[p]

    files = list(pages.values()) + ([Path(readme)] if readme and Path(readme).is_file() else [])
    files += [Path(e) for e in extra if Path(e).is_file()]
    readme_path = Path(readme) if readme and Path(readme).is_file() else None
    bad = []
    for f in files:
        in_wiki = f in pages.values()
        # Relative file links are followed in Markdown outside the wiki, but not in the README: the wiki's publish
        # exports only the wiki, the README and the site (test_readme.py checks the README's own files).
        follow_files = not in_wiki and f != readme_path and f.suffix.lower() == ".md"
        for n, target in _links(f):
            ok = True
            m = _ABS_WIKI.match(target)
            r = _ABS_README.match(target)
            if m:
                page, frag = m.group(1).rstrip("/"), m.group(2)
                ok = page in pages and (not frag or frag.lower() in anchor_set(pages[page]))
            elif r:
                ok = readme_path is None or r.group(1).lower() in anchor_set(readme_path)
            elif target.startswith("#"):
                ok = target[1:].lower() in anchor_set(f)
            elif follow_files and not _SCHEME.match(target) and not target.startswith("/"):
                path, _, frag = target.partition("#")
                dest = f.parent / unquote(path.partition("?")[0])
                ok = dest.exists() and not (frag and dest.is_file() and dest.suffix.lower() in (".md", ".html", ".htm")
                                            and frag.lower() not in anchor_set(dest))
            elif in_wiki and not re.match(r"^[a-zA-Z][a-zA-Z0-9+.-]*:|^/|^\.", target):
                page, _, frag = target.partition("#")
                if "." in page.rsplit("/", 1)[-1]:
                    continue  # a file (image), not a page
                ok = page in pages and (not frag or frag.lower() in anchor_set(pages[page]))
            if not ok:
                bad.append("%s:%d: %s" % (f, n, target))
    return bad


def main(argv=None):
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument("--wiki", default="docs/wiki")
    ap.add_argument("--readme", default="README.md")
    ap.add_argument("extra", nargs="*", default=None)
    a = ap.parse_args(argv)
    extra = a.extra if a.extra else sorted({p for g in DEFAULT_EXTRA for p in glob.glob(g)})
    bad = check(Path(a.wiki), Path(a.readme), [Path(e) for e in extra])
    for line in bad:
        print(line)
    return 1 if bad else 0


if __name__ == "__main__":
    sys.exit(main())
