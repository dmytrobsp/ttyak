#!/usr/bin/env python3
"""Split PROGRESS.md's `## Status` rows and `## Bug log` into an archive by era.

    split_history.py PROGRESS.md docs/history/            write the archive
    split_history.py PROGRESS.md docs/history/ --check   prove it: the archive's text, whitespace normalised, equals
                                                          the old rows' text and the bug log; exit 1 on any difference

Each status row becomes a `### <heading>` section. The heading is the first sentence of the row's first cell (for most
rows `<id> — <title>`), the rest of that cell is the body, and the State / Verified / Commit cells follow as bullets.
Rows that do not split into four cells are kept whole (heading and body, no bullets). Text that sits between rows
(like the final `Tests:` line) becomes a note section in the era where it stood. Nothing is reworded; only line breaks
change.
"""
import re
import sys
import textwrap
from pathlib import Path

WIDTH = 120
# (file, title, first row of the era: a prefix of the row's first cell; None = from the start)
ERAS = [
    ("0.1-0.9.md", "History 0.1–0.9 — slacktui and klack", None),
    ("0.10-0.11.md", "History 0.10–0.11 — the rename to ttyak and distribution", "TY0 — "),
    ("0.12.md", "History 0.12 — attention, the README/wiki split, 0.12.2", "AT1 — "),
]
NOTE = "Note under the table"
LABELS = [("State", 1), ("Verified", 2), ("Commit", 3)]


def norm(s):
    return " ".join(s.split())


def wrap(text, first="", rest=""):
    return textwrap.fill(norm(text), WIDTH, initial_indent=first, subsequent_indent=rest,
                         break_long_words=False, break_on_hyphens=False)


def split_cells(line):
    """Cells of a table row, ignoring `|` inside backticks and `\\|`."""
    s, cells, cur, bt, i = line[1:], [], "", False, 0
    while i < len(s):
        c = s[i]
        if c == "\\" and s[i + 1:i + 2] == "|":
            cur += "\\|"
            i += 2
            continue
        if c == "`":
            bt = not bt
        if c == "|" and not bt:
            cells.append(cur)
            cur = ""
        else:
            cur += c
        i += 1
    cells.append(cur)
    return cells


def balanced(s):
    return s.count("`") % 2 == 0 and s.count('"') % 2 == 0 and s.count("(") == s.count(")")


def heading_split(text):
    """(heading, body): the first sentence of the text, cut where the backticks are balanced."""
    text = norm(text)
    limit = 100
    for stops in (r"[.;] ", r": "):  # a full stop is a better end for a heading than a colon
        for m in re.finditer(stops, text):
            end = m.start() + 1
            if end > limit:
                break
            if balanced(text[:end]):
                return text[:end], text[end:].strip()
    if len(text) <= limit:
        return text, ""
    end = text.rfind(" ", 0, limit)
    while end > 0 and not balanced(text[:end]):
        end = text.rfind(" ", 0, end)
    if end <= 0:
        return text, ""
    return text[:end], text[end:].strip()


def parse_progress(path):
    lines = Path(path).read_text(encoding="utf-8").split("\n")

    def section(name):
        start = next(i for i, l in enumerate(lines) if l.startswith(name))
        end = next((i for i in range(start + 1, len(lines)) if lines[i].startswith("## ")), len(lines))
        return lines[start + 1:end]

    items, note = [], []  # items: ("row", line) | ("note", text)

    def flush():
        if note:
            items.append(("note", "\n".join(note)))
            note.clear()

    for l in section("## Status"):
        if l.startswith("|---") or l.startswith("| Area |"):
            continue
        if l.startswith("| "):
            flush()
            items.append(("row", l))
        elif l.strip():
            note.append(l)
        else:
            flush()
    flush()
    return items, bug_items(section("## Bug log"))


def bug_items(lines):
    out, cur = [], None
    for l in lines:
        if not l.strip():
            cur = None
        elif l.startswith("- ") or cur is None:
            cur = [l]
            out.append(cur)
        else:
            cur.append(l)
    return [" ".join(x) for x in out]


def row_cells(line):
    """The four cells of a regular row, or None; and the whole inner text."""
    cells = split_cells(line)
    inner = line.strip()[1:]
    if inner.endswith("|"):
        inner = inner[:-1]
    if len(cells) == 5 and cells[4] == "" and line.rstrip().endswith("|"):
        return [c.strip() for c in cells[:4]], inner.strip()
    return None, inner.strip()


def render_row(line):
    cells, inner = row_cells(line)
    heading, body = heading_split(cells[0] if cells else inner)
    out = ["### " + heading, ""]
    if body:
        out += [wrap(body), ""]
    if cells:
        for label, n in LABELS:
            out.append(wrap(cells[n], f"- **{label}:** ", "  ").rstrip())
    elif out[-1] == "":
        out.pop()
    return "\n".join(out).rstrip()


def build(progress):
    items, bugs = parse_progress(progress)
    files = {name: [f"# {title}"] for name, title, _ in ERAS}
    idx = 0
    for kind, text in items:
        if kind == "row":
            cells, inner = row_cells(text)
            first = cells[0] if cells else inner
            for j, (_, _, start) in enumerate(ERAS):
                if start and first.startswith(start):
                    idx = j
            files[ERAS[idx][0]].append(render_row(text))
        else:
            files[ERAS[idx][0]].append(f"### {NOTE}\n\n" + wrap(text))
    out = {n: "\n\n".join(parts) + "\n" for n, parts in files.items()}
    bl = ["# Bug log"]
    for b in bugs:
        if b.startswith("- "):
            bl.append(wrap(b[2:], "- ", "  "))
        else:
            bl.append(wrap(b))
    out["bug-log.md"] = "\n\n".join(bl) + "\n"
    return out


def parse_archive_section(block):
    """(heading, body text, {label: text}) of one `### ` section."""
    lines = block.split("\n")
    heading = lines[0]
    body, bullets, cur = [], {}, None
    for l in lines[1:]:
        m = re.match(r"- \*\*(State|Verified|Commit):\*\*(.*)$", l)
        if m:
            cur = m.group(1)
            bullets[cur] = m.group(2)
        elif cur and l.startswith("  "):
            bullets[cur] += " " + l
        else:
            body.append(l)
    return heading, " ".join(body), bullets


def archive_sections(dirpath):
    secs = []
    for name, _, _ in ERAS:
        p = Path(dirpath) / name
        text = p.read_text(encoding="utf-8") if p.exists() else ""
        for block in re.split(r"(?m)^(?=### )", text)[1:]:
            secs.append(parse_archive_section(block.rstrip("\n")))
    return secs


def check(progress, dirpath):
    items, bugs = parse_progress(progress)
    secs = archive_sections(dirpath)
    problems = []
    if len(secs) != len(items):
        problems.append(f"{len(items)} rows/notes in PROGRESS, {len(secs)} sections in the archive")
    for n, ((kind, text), (heading, body, bullets)) in enumerate(zip(items, secs), 1):
        h = norm(heading[4:] + " " + body)
        if kind == "note":
            ok = heading == "### " + NOTE and norm(body) == norm(text)
        else:
            cells, inner = row_cells(text)
            if cells:
                ok = (norm(h) == norm(cells[0]) and [norm(bullets.get(k, "")) for k, _ in LABELS] ==
                      [norm(cells[i]) for _, i in LABELS])
            else:
                ok = norm(h) == norm(inner) and not bullets
        if not ok:
            problems.append(f"item {n} differs: {norm(text)[:60]!r}")
    bl = Path(dirpath) / "bug-log.md"
    got = bl.read_text(encoding="utf-8").split("\n", 1)[1] if bl.exists() else ""
    if norm(got) != norm(" ".join(bugs)):
        problems.append("the bug log differs")
    for p in problems:
        print("MISMATCH:", p)
    return 1 if problems else 0


def main(argv):
    args = [a for a in argv if not a.startswith("--")]
    if len(args) != 2:
        print(__doc__)
        return 2
    progress, dirpath = args
    if "--check" in argv:
        rc = check(progress, dirpath)
        if rc == 0:
            print("ok: the archive holds exactly the text of the status rows and the bug log")
        return rc
    Path(dirpath).mkdir(parents=True, exist_ok=True)
    for name, text in build(progress).items():
        (Path(dirpath) / name).write_text(text, encoding="utf-8")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
