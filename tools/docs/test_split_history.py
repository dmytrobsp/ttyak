"""split_history: PROGRESS.md's status rows and bug log become an archive by era, and --check proves no text changed."""
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import split_history  # noqa: E402

LONG = "word " * 60
FIXTURE = f"""# ttyak — progress

## Status (branch `master`)

| Area | State | Verified | Commit |
|---|---|---|---|
| Sidebar: channels and DMs, {LONG}end | done | LIVE | 284da42 |
| TY0 — runbooks. Accounts for `a | b` and more | done | UNIT (3) | 111aaaa, 222bbbb |
| AT1 — Task 1 of 0.12.0: lighter CI. {LONG}| done | LIVE: ok; NOT: x | abc1234 |
| AT2 — odd row with `unbalanced backtick | done | 5 cells | here | extra | f00 |

Tests: `cargo test` → 10 passed.

## Next (in order)

- nothing

## Bug log

- The first bug: {LONG}end.
- The second bug is short.

- Third, after a blank line.
"""


def run(*args):
    return subprocess.run([sys.executable, str(Path(split_history.__file__)), *args], capture_output=True, text=True)


class SplitHistory(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.addCleanup(self.tmp.cleanup)
        self.dir = Path(self.tmp.name)
        self.progress = self.dir / "PROGRESS.md"
        self.progress.write_text(FIXTURE, encoding="utf-8")
        self.out = self.dir / "history"

    def test_writes_one_file_per_era_and_the_bug_log(self):
        r = run(str(self.progress), str(self.out))
        self.assertEqual(r.returncode, 0, r.stderr)
        self.assertEqual(sorted(p.name for p in self.out.iterdir()),
                         ["0.1-0.9.md", "0.10-0.11.md", "0.12.md", "bug-log.md"])
        early = (self.out / "0.1-0.9.md").read_text(encoding="utf-8")
        self.assertIn("### Sidebar:", early)
        self.assertIn("- **Commit:** 284da42", early)
        mid = (self.out / "0.10-0.11.md").read_text(encoding="utf-8")
        self.assertIn("### TY0 — runbooks.", mid)
        self.assertIn("`a | b`", mid)
        late = (self.out / "0.12.md").read_text(encoding="utf-8")
        self.assertIn("### AT1 — Task 1 of 0.12.0: lighter CI.", late)
        self.assertIn("### AT2 — odd row", late)
        self.assertIn("Tests: `cargo test`", late)
        for p in self.out.iterdir():
            for line in p.read_text(encoding="utf-8").splitlines():
                self.assertLessEqual(len(line), 120, line[:40])

    def test_check_passes_on_a_fresh_archive(self):
        run(str(self.progress), str(self.out))
        r = run(str(self.progress), str(self.out), "--check")
        self.assertEqual(r.returncode, 0, r.stdout + r.stderr)

    def test_check_catches_one_changed_word(self):
        run(str(self.progress), str(self.out))
        f = self.out / "0.12.md"
        f.write_text(f.read_text(encoding="utf-8").replace("lighter", "heavier"), encoding="utf-8")
        r = run(str(self.progress), str(self.out), "--check")
        self.assertNotEqual(r.returncode, 0)

    def test_check_catches_a_changed_bug_log_and_a_missing_row(self):
        run(str(self.progress), str(self.out))
        b = self.out / "bug-log.md"
        b.write_text(b.read_text(encoding="utf-8").replace("second", "2nd"), encoding="utf-8")
        self.assertNotEqual(run(str(self.progress), str(self.out), "--check").returncode, 0)
        run(str(self.progress), str(self.out))
        m = self.out / "0.10-0.11.md"
        m.write_text("# gone\n", encoding="utf-8")
        self.assertNotEqual(run(str(self.progress), str(self.out), "--check").returncode, 0)

    def test_whitespace_only_changes_pass(self):
        run(str(self.progress), str(self.out))
        f = self.out / "0.1-0.9.md"
        f.write_text(f.read_text(encoding="utf-8").replace("word word", "word\n  word"), encoding="utf-8")
        self.assertEqual(run(str(self.progress), str(self.out), "--check").returncode, 0)


if __name__ == "__main__":
    unittest.main()
