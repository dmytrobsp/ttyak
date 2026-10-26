#!/usr/bin/env python3
"""Builds the README's screenshots, docs/images/<name>.png, from raw kitty captures (RW8).

    python3 tools/site/shots.py RAW_DIR docs/images --blur NAME [--blur NAME …] [--fixes FILE] [--scratch DIR]

RAW_DIR holds main.ansi and huddle.ansi (`kitty @ get-text --extent screen --ansi`); the sign-in guide is the site's
own capture (site/captures/signin.html). The screen is drawn on its cell grid by ansi2html.py, the real names replaced
with ~/.config/ttyak-site/rename.json (outside the repo, never printed or committed), single cells overwritten from
--fixes ({"huddle": {"row,col": "letter"}}: the tiles' letters drawn from real names), and every cell run showing a
--blur text (the made-up workspace name, also cut short with "…") is covered by a blur. Headless Chrome (flatpak
com.google.Chrome, or $CHROME) renders that page to a PNG at 2× — a file, never the screen. See docs/images/README.md.
"""
import base64
import html
import json
import os
import re
import shutil
import subprocess
import sys
import tempfile
from pathlib import Path

sys.path.insert(0, str(Path(__file__).parent))
from ansi2html import CH, CW, KITTY, convert, render_svg  # noqa: E402

ROOT = Path(__file__).resolve().parents[2]
FONTS = ROOT / "site/fonts"
RENAME = Path.home() / ".config/ttyak-site/rename.json"
SHOTS = {
    "main": dict(cols=136, rows=34, title="ttyak", blur=True),
    # the tiles show the first letters of the real names: not built without its fixes
    "huddle": dict(cols=136, rows=34, title="ttyak — huddle", blur=True, pictures=("pattern",), needs_fixes=True),
    "signin": dict(cols=120, rows=29, title="ttyak auth login", fragment="site/captures/signin.html"),
}
PAD, BAR = 10, 26       # the screen's padding and the title bar, in CSS px (one cell is CW × CH px)
SCALE = 2               # device pixels per CSS px
BLUR = 5                # the blur's radius, CSS px: letters go, the word's length stays
BARS = [[235, 235, 235], [235, 235, 16], [16, 235, 235], [16, 235, 16], [235, 16, 235], [235, 16, 16], [16, 16, 235]]


def blur_runs(screen, texts):
    """[(row, first cell, end cell)] of every place a text shows: whole, or cut short by ttyak (a prefix of at least
    three characters followed by "…")."""
    alts = []
    for t in sorted({t for t in texts if t}, key=len, reverse=True):
        alts.append(re.escape(t))
        if len(t) > 3:
            alts.append(re.escape(t[:3]) + "".join(f"(?:{re.escape(ch)}" for ch in t[3:-1]) + ")?" * (len(t) - 4) + "…")
    if not alts:
        return []
    pat = re.compile("|".join(alts))
    out = []
    for r, row in enumerate(screen.cells):
        at = [i for i, c in enumerate(row) if c[2]]
        text = "".join(row[i][0][:1] or " " for i in at)
        for m in pat.finditer(text):
            last = at[m.end() - 1]
            out.append((r, at[m.start()], last + row[last][2]))
    return out


def pattern_svg():
    """One frame of the synthetic shared screen `ctl huddle-media --share-test` sends (crates/huddle/src/video.rs
    `test_pattern`): colour bars over the top two thirds, a white square on a dark band below. It fills the
    picture's cells, as kitty draws the picture (their shape is within a few per cent of 16:9)."""
    w, h, n = 640, 360, len(BARS)
    band, side = h * 2 // 3, (h - h * 2 // 3) // 2
    bars = "".join(f'<rect x="{i * w // n}" width="{(i + 1) * w // n - i * w // n}" height="{band}" '
                   f'fill="#{r:02x}{g:02x}{b:02x}"/>' for i, (r, g, b) in enumerate(BARS))
    return (f'<svg viewBox="0 0 {w} {h}" preserveAspectRatio="none">{bars}'
            f'<rect y="{band}" width="{w}" height="{h - band}" fill="#282828"/>'
            f'<rect x="200" y="{band + (h - band - side) // 2}" width="{side}" height="{side}" fill="#ebebeb"/></svg>')


def frame_size(cols, rows):
    return cols * CW + 2 * PAD, rows * CH + 2 * PAD + BAR


def _font(weight):
    data = (FONTS / f"space-mono-{weight}.woff2").read_bytes()
    return (f"@font-face{{font-family:'Space Mono';font-weight:{weight};"
            f"src:url(data:font/woff2;base64,{base64.b64encode(data).decode()}) format('woff2')}}")


def page(svg, cols, rows, title, blurs=(), pictures=()):
    """The page Chrome renders: a terminal window (a title bar over the screen) the exact size of the PNG."""
    w, h = frame_size(cols, rows)
    px = lambda v: ("%.2f" % v).rstrip("0").rstrip(".")
    over = "".join(f'<div class="pic" style="left:{px(p["c"] * CW)}px;top:{px(p["r"] * CH)}px;'
                   f'width:{px(p["w"] * CW)}px;height:{px(p["h"] * CH)}px">{pattern_svg()}</div>'
                   for kind, p in pictures)
    over += "".join(f'<div class="blur" style="left:{px(c0 * CW)}px;top:{px(r * CH)}px;width:{px((c1 - c0) * CW)}px;'
                    f'height:{px(CH)}px"></div>' for r, c0, c1 in blurs)
    if all((FONTS / f"space-mono-{w}.woff2").is_file() for w in (400, 700)):
        fonts = _font(400) + _font(700)
    else:
        fonts = ""
        print(f"warning: Space Mono not found in {FONTS}: the picture falls back to another monospace font",
              file=sys.stderr)
    return f"""<!doctype html><html><head><meta charset="utf-8"><style>{fonts}
html,body{{margin:0;background:transparent}}
.win{{width:{w - 2}px;height:{h - 2}px;border:1px solid #45475a;border-radius:10px;overflow:hidden;background:{KITTY["bg"]}}}
.bar{{height:{BAR - 1}px;line-height:{BAR - 1}px;padding:0 12px;border-bottom:1px solid #313244;background:#181825;
font:12px 'Space Mono',monospace;color:#7f849c;white-space:pre}}
.bar b{{font-weight:400;letter-spacing:2px}}
.screen{{position:relative;margin:{PAD - 1}px {PAD - 1}px}}
.screen svg.cells{{display:block}}
.pic{{position:absolute;background:#000;overflow:hidden}}.pic svg{{display:block;width:100%;height:100%}}
.blur{{position:absolute;backdrop-filter:blur({BLUR}px);-webkit-backdrop-filter:blur({BLUR}px)}}
</style></head><body><div class="win"><div class="bar"><b>● ● ●</b>  {html.escape(title, quote=False)}</div>
<div class="screen">{svg}{over}</div></div></body></html>
"""


def build_page(ansi, cfg, rename=None, fixes=None, blur=()):
    """One capture's page. `fixes`: {(row, col): letter} put into single cells after renaming; each must replace one
    letter (a tile's initial) with one letter, so a fix that lands elsewhere (the layout moved) is refused."""
    if cfg.get("needs_fixes") and not fixes:
        raise ValueError("this capture needs its cell fixes (--fixes) and has none")
    _, screen = convert(ansi, rename, cfg["cols"], cfg["rows"])
    for (r, c), ch in (fixes or {}).items():
        if not (0 <= r < screen.rows and 0 <= c < screen.cols) or not screen.cells[r][c][0].strip():
            raise ValueError(f"a fix at {r},{c} has no character to replace (the layout moved?)")
        cell = screen.cells[r][c]
        if not (len(cell[0]) == 1 and cell[0].isalpha() and cell[2] == 1):
            raise ValueError(f"a fix at {r},{c} does not land on a single letter (the layout moved?)")
        if not (isinstance(ch, str) and len(ch) == 1 and ch.isalpha()):
            raise ValueError(f"a fix at {r},{c} must put a single letter")
        cell[0] = ch
    kinds = cfg.get("pictures", ())
    if len(screen.pictures) != len(kinds):
        raise ValueError(f"{len(screen.pictures)} pictures on the screen, {len(kinds)} expected")
    runs = blur_runs(screen, blur)
    if cfg.get("blur") and not runs:
        raise ValueError("nothing to blur: the workspace name was not found on the screen")
    return page(render_svg(screen), cfg["cols"], cfg["rows"], cfg["title"], runs, list(zip(kinds, screen.pictures)))


def svg_of_fragment(fragment):
    """The <svg> of a site capture fragment (site/captures/<name>.html)."""
    m = re.search(r"<svg\b.*?</svg>", fragment, re.S)
    if not m:
        raise ValueError("no <svg> in the fragment")
    return m.group(0)


def chrome_command(html_path, png_path, size, profile, chrome=None):
    flags = ["--headless=new", f"--user-data-dir={profile}", "--no-first-run", "--no-default-browser-check",
             "--hide-scrollbars", "--mute-audio", "--disable-extensions", "--password-store=basic",
             f"--force-device-scale-factor={SCALE}", "--default-background-color=00000000",
             f"--window-size={size[0]},{size[1]}", f"--screenshot={png_path}", f"file://{html_path}"]
    if chrome:
        return [chrome, *flags]
    return ["flatpak", "run", f"--filesystem={html_path.parent}", "com.google.Chrome", *flags]


def shoot(page_html, png, size, scratch=None, chrome=None):
    """Renders the page to `png` in a fresh folder (its own browser profile), deleted after."""
    work = Path(tempfile.mkdtemp(prefix="shots-", dir=scratch))
    try:
        (work / "page.html").write_text(page_html, encoding="utf-8")
        cmd = chrome_command(work / "page.html", work / "page.png", size, work / "prof", chrome)
        subprocess.run(cmd, check=True, timeout=120, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
        shutil.copyfile(work / "page.png", png)
    finally:
        shutil.rmtree(work, ignore_errors=True)


def main(argv, shoot=shoot, rename_file=None):
    """Draws every shot; a capture missing from RAW_DIR is named ("skipped …") and makes the exit code 1."""
    args, blur, fixes_path, scratch = [], [], None, None
    usage = "usage: " + __doc__.strip().splitlines()[2].strip()
    i = 0
    while i < len(argv):
        a = argv[i]
        if a in ("--blur", "--fixes", "--scratch"):
            if i + 1 >= len(argv) or argv[i + 1].startswith("--"):
                print(f"{usage}\n{a} needs a value", file=sys.stderr)
                return 2
            v = argv[i + 1]
            if a == "--blur":
                blur.append(v)
            elif a == "--fixes":
                fixes_path = Path(v)
            else:
                scratch = v
            i += 2
            continue
        args.append(a)
        i += 1
    if len(args) != 2 or not blur:
        print(usage, file=sys.stderr)
        return 2
    raw, out = Path(args[0]), Path(args[1])
    out.mkdir(parents=True, exist_ok=True)
    rename = json.loads(Path(rename_file or RENAME).read_text())
    fixes = json.loads(fixes_path.read_text()) if fixes_path else {}
    chrome = os.environ.get("CHROME")
    skipped = []
    for name, cfg in SHOTS.items():
        if "fragment" in cfg:
            svg = svg_of_fragment((ROOT / cfg["fragment"]).read_text(encoding="utf-8"))
            html_page = page(svg, cfg["cols"], cfg["rows"], cfg["title"])
        else:
            src = raw / f"{name}.ansi"
            if not src.is_file():
                print(f"skipped {name}: no {src}", file=sys.stderr)
                skipped.append(name)
                continue
            f = {tuple(map(int, k.split(","))): v for k, v in fixes.get(name, {}).items()}
            html_page = build_page(src.read_text(encoding="utf-8"), cfg, rename, f, blur)
        shoot(html_page, out / f"{name}.png", frame_size(cfg["cols"], cfg["rows"]), scratch, chrome)
        print(f"{name}: written")
    return 1 if skipped else 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
