import re
import sys
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).parent))
import shots  # noqa: E402
from ansi2html import CH, CW, parse  # noqa: E402

SCREEN = ("┌ acme-ops-infra ─┐┌ chat ──────┐\n"
          "│ # general       ││ hi there   │\n"
          "│ acme-op…        ││ acme-ops-i │\n")


class BlurRuns(unittest.TestCase):
    def test_the_name_and_its_cut_forms(self):
        s = parse(SCREEN, 34, 3)
        self.assertEqual(shots.blur_runs(s, ["acme-ops-infra"]), [(0, 2, 16), (2, 2, 10)])

    def test_a_prefix_without_the_ellipsis_is_left(self):
        # "acme-ops-i" on row 2 is cut by the pane, not by ttyak: only "…" marks a cut name
        s = parse(SCREEN, 34, 3)
        self.assertNotIn(2, [r for r, c0, _ in shots.blur_runs(s, ["acme-ops-infra"]) if c0 > 20])

    def test_wide_characters_count_two_cells(self):
        s = parse("🚀 acme\n", 10, 1)
        self.assertEqual(shots.blur_runs(s, ["acme"]), [(0, 3, 7)])

    def test_nothing_to_blur(self):
        self.assertEqual(shots.blur_runs(parse(SCREEN, 34, 3), ["zeta"]), [])


class BuildPage(unittest.TestCase):
    CFG = dict(cols=34, rows=3, title="ttyak", blur=True)

    def test_blur_required_but_missing_refuses(self):
        with self.assertRaisesRegex(ValueError, "nothing to blur"):
            shots.build_page(SCREEN, self.CFG, blur=["zeta"])

    def test_one_blur_box_per_run_over_its_cells(self):
        html = shots.build_page(SCREEN, self.CFG, blur=["acme-ops-infra"])
        boxes = re.findall(r'class="blur" style="left:([\d.]+)px;top:([\d.]+)px;width:([\d.]+)px;height:([\d.]+)px', html)
        self.assertEqual(boxes, [(str(2 * CW), "0", str(14 * CW), str(CH)), (str(2 * CW), str(2 * CH), str(8 * CW), str(CH))])
        self.assertIn("backdrop-filter", html)

    def test_rename_happens_before_the_blur_is_found(self):
        html = shots.build_page(SCREEN.replace("acme-ops-infra", "real-name-here"), self.CFG,
                                rename={"real-name-here": "acme-ops-infra"}, blur=["acme-ops-infra"])
        self.assertNotIn("real-name-here", html)
        self.assertEqual(html.count('class="blur"'), 2)  # row 0, and its cut form on row 2

    def test_fixes_go_into_cells_and_refuse_a_blank_one(self):
        cfg = dict(cols=34, rows=3, title="ttyak")
        html = shots.build_page(SCREEN, cfg, fixes={(1, 4): "Z"})
        self.assertIn("# Zeneral", html)
        with self.assertRaisesRegex(ValueError, "no character"):
            shots.build_page(SCREEN, cfg, fixes={(1, 30): "Z"})

    def test_a_fixes_capture_without_fixes_refuses(self):
        with self.assertRaisesRegex(ValueError, "fixes"):
            shots.build_page(SCREEN, dict(cols=34, rows=3, title="t", needs_fixes=True))

    def test_pictures_must_match(self):
        with self.assertRaisesRegex(ValueError, "pictures"):
            shots.build_page(SCREEN, dict(cols=34, rows=3, title="t", pictures=("pattern",)))

    def test_the_test_pattern_goes_over_the_picture_cells(self):
        ph = "\U0010EEEE"
        ansi = "ab\x1b[38;5;1m" + ph * 3 + "\x1b[0m\n" + "ab\x1b[38;5;1m" + ph * 3 + "\x1b[0m\n"
        html = shots.build_page(ansi, dict(cols=8, rows=2, title="t", pictures=("pattern",)))
        self.assertIn(f'class="pic" style="left:{2 * CW}px;top:0px;width:{3 * CW}px;height:{2 * CH}px"', html)
        self.assertEqual(html.count("<rect"), 1 + 7 + 2)  # the screen's background, 7 bars, the band and the square


class Helpers(unittest.TestCase):
    def test_svg_of_a_site_fragment(self):
        frag = '<div class="term capture"><div class="dots">x</div><div class="screen"><div class="grid">' \
               '<svg class="cells" viewBox="0 0 8 18"><rect/></svg></div></div></div>\n'
        self.assertEqual(shots.svg_of_fragment(frag), '<svg class="cells" viewBox="0 0 8 18"><rect/></svg>')
        with self.assertRaises(ValueError):
            shots.svg_of_fragment("<div></div>")

    def test_frame_size(self):
        w, h = shots.frame_size(136, 34)
        self.assertEqual((w, h), (136 * CW + 2 * shots.PAD, 34 * CH + 2 * shots.PAD + shots.BAR))

    def test_chrome_command_names_only_its_own_folder(self):
        cmd = shots.chrome_command(Path("/s/p.html"), Path("/s/p.png"), (100, 50), Path("/s/prof"), chrome=None)
        self.assertEqual(cmd[:3], ["flatpak", "run", "--filesystem=/s"])
        self.assertIn("--headless=new", cmd)
        self.assertIn("--window-size=100,50", cmd)
        self.assertIn("--screenshot=/s/p.png", cmd)
        self.assertEqual(cmd[-1], "file:///s/p.html")
        self.assertEqual(shots.chrome_command(Path("/s/p.html"), Path("/s/p.png"), (1, 1), Path("/s/prof"),
                                              chrome="/x/chrome")[0], "/x/chrome")


if __name__ == "__main__":
    unittest.main()
