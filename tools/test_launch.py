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
                f.write("#!/bin/sh\necho %s \"$@\" >> '%s'\n" % (tool, self.log))
                if tool == "kitty":
                    f.write("env | grep '^TTYAK_\\|^HOME=\\|^XDG_' | sort >> '%s'\n" % self.log)
                f.write("exit 0\n")
            os.chmod(path, 0o755)
        self.env = {"PATH": bin_dir + os.pathsep + os.environ["PATH"], "HOME": self.dir}

    def fake_home(self, profile="test-a"):
        home = os.path.join(self.dir, "fake")
        os.makedirs(os.path.join(home, ".config", "ttyak", profile))
        with open(os.path.join(home, ".config", "ttyak", profile, "session.json"), "w") as f:
            f.write("{}")
        return home

    def run_script(self, shell, *args, **env):
        return subprocess.run([shell, SCRIPT, *args], capture_output=True, text=True, timeout=30,
                              env={**self.env, **env})

    def calls(self):
        if not os.path.exists(self.log):
            return ""
        with open(self.log) as f:
            return f.read()

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

    def test_refuses_without_a_fake_home(self):
        for shell in SHELLS:
            for env in ({}, {"TTYAK_FAKE_HOME": ""}):
                with self.subTest(shell=shell, env=env):
                    r = self.run_script(shell, "test-a", **env)
                    self.assertEqual(r.returncode, 2, r.stderr)
                    self.assertIn("TTYAK_FAKE_HOME", r.stderr)
                    self.assertIn("accounts.py", r.stderr)
                    self.assertIn("install", r.stderr)
                    self.assertEqual(self.calls(), "")

    def test_refuses_a_fake_home_that_is_not_a_directory(self):
        for shell in SHELLS:
            with self.subTest(shell=shell):
                r = self.run_script(shell, "test-a", TTYAK_FAKE_HOME=os.path.join(self.dir, "nope"))
                self.assertEqual(r.returncode, 2, r.stderr)
                self.assertEqual(self.calls(), "")

    def test_refuses_a_fake_home_without_the_profiles_session(self):
        home = self.fake_home("test-b")
        for shell in SHELLS:
            with self.subTest(shell=shell):
                r = self.run_script(shell, "test-a", TTYAK_FAKE_HOME=home)
                self.assertEqual(r.returncode, 2, r.stderr)
                self.assertIn("session.json", r.stderr)
                self.assertIn("install", r.stderr)
                self.assertEqual(self.calls(), "")

    def test_the_command_runs_ttyak_in_the_fake_home(self):
        home = self.fake_home()
        for shell in SHELLS:
            with self.subTest(shell=shell):
                if os.path.exists(self.log):
                    os.remove(self.log)
                r = self.run_script(shell, "test-a", TTYAK_FAKE_HOME=home)
                self.assertEqual(r.returncode, 0, r.stderr)
                calls = self.calls()
                # The environment of the window carries the fake home and the profile ...
                self.assertIn("TTYAK_FAKE_HOME=" + home, calls)
                self.assertIn("TTYAK_LAUNCH_PROFILE=test-a", calls)
                # ... and the command takes them from it; the path is never written into the command.
                cmd = calls.split("kitty --class", 1)[1].split("sh -c ", 1)[1].split("exec sleep 600")[0]
                for var in ("HOME", "XDG_CONFIG_HOME", "XDG_DATA_HOME", "XDG_CACHE_HOME"):
                    self.assertIn(var + "=", cmd)
                self.assertIn("$TTYAK_FAKE_HOME", cmd)
                self.assertIn("/.config", cmd)
                self.assertIn("/.local/share", cmd)
                self.assertIn("/.cache", cmd)
                self.assertNotIn(home, cmd)
                self.assertNotIn(self.dir, cmd)
                self.assertRegex(cmd, r"\./target/debug/ttyak --profile")


if __name__ == "__main__":
    unittest.main()
