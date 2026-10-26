"""Tests for tools/launch.sh: it never starts ttyak without a profile it was given."""
import os
import shutil
import subprocess
import tempfile
import unittest

SCRIPT = os.path.join(os.path.dirname(os.path.abspath(__file__)), "launch.sh")
SHELLS = [s for s in (shutil.which("dash"), shutil.which("bash")) if s]


class Launch(unittest.TestCase):
    def setUp(self):
        self.dir = tempfile.mkdtemp(prefix="launch-test-")
        self.addCleanup(shutil.rmtree, self.dir)
        self.log = os.path.join(self.dir, "calls")
        bin_dir = os.path.join(self.dir, "bin")
        os.mkdir(bin_dir)
        for tool in ("hyprctl", "kitty", "jq"):
            path = os.path.join(bin_dir, tool)
            with open(path, "w") as f:
                f.write("#!/bin/sh\necho %s \"$@\" >> '%s'\nexit 0\n" % (tool, self.log))
            os.chmod(path, 0o755)
        self.env = {"PATH": bin_dir + os.pathsep + os.environ["PATH"], "HOME": self.dir}

    def run_script(self, shell, *args, **env):
        return subprocess.run([shell, SCRIPT, *args], capture_output=True, text=True, timeout=30,
                              env={**self.env, **env})

    def calls(self):
        return open(self.log).read() if os.path.exists(self.log) else ""

    def test_refuses_without_a_profile(self):
        for shell in SHELLS:
            with self.subTest(shell=shell):
                r = self.run_script(shell)
                self.assertEqual(r.returncode, 2, r.stderr)
                self.assertIn("usage", r.stderr)
                self.assertIn("AGENTS.local.md", r.stderr)
                self.assertEqual(self.calls(), "", "nothing may run without a profile")

    def test_refuses_an_empty_profile_from_the_environment(self):
        for shell in SHELLS:
            with self.subTest(shell=shell):
                r = self.run_script(shell, TTYAK_TEST_PROFILE="")
                self.assertEqual(r.returncode, 2, r.stderr)
                self.assertEqual(self.calls(), "")

    def test_refuses_a_profile_that_looks_like_an_option(self):
        for shell in SHELLS:
            with self.subTest(shell=shell):
                r = self.run_script(shell, "--help")
                self.assertEqual(r.returncode, 2, r.stderr)
                self.assertEqual(self.calls(), "")


if __name__ == "__main__":
    unittest.main()
