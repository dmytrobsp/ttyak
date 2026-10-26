"""Checks of the test-machine scripts in tools/vm (they run on Windows; nothing here runs them):

- every script is plain ASCII: Windows PowerShell 5.1 reads a file without a byte-order mark in the system's code page;
- every script parses (PowerShell's own parser): with `pwsh` on PATH (GitHub's Ubuntu runners have it), else in a
  podman container (not in CI unless TTYAK_PODMAN_TESTS=1), else skipped — except in CI, where it fails."""
import glob
import os
import shutil
import subprocess
import tempfile
import unittest

HERE = os.path.dirname(os.path.abspath(__file__))
IMAGE = "mcr.microsoft.com/powershell:7.5-ubuntu-24.04"
PARSE = r"""
$bad = 0
foreach ($f in Get-ChildItem (Join-Path $args[0] '*.ps1')) {
  $t = $null; $e = $null
  [System.Management.Automation.Language.Parser]::ParseFile($f.FullName, [ref]$t, [ref]$e) | Out-Null
  foreach ($x in $e) { $bad++; "$($f.Name):$($x.Extent.StartLineNumber): $($x.Message)" }
}
"errors: $bad"
"""


def scripts(folder=HERE):
    return sorted(glob.glob(os.path.join(folder, "*.ps1")))


def non_ascii(folder=HERE):
    """[(file name, line number)] of every line with a byte outside ASCII."""
    out = []
    for path in scripts(folder):
        with open(path, "rb") as f:
            for n, line in enumerate(f, 1):
                if any(b > 0x7F for b in line):
                    out.append((os.path.basename(path), n))
    return out


def parse_script(folder):
    """PARSE as a script block called with `folder` (pwsh -Command passes no further arguments to $args)."""
    return "& {%s} '%s'" % (PARSE, folder.replace("'", "''"))


def parse_command(folder, env=None):
    """The command that parses every script in `folder`, or None when there is no PowerShell to run."""
    env = os.environ if env is None else env
    if shutil.which("pwsh"):
        return ["pwsh", "-NoProfile", "-Command", parse_script(folder)]
    if shutil.which("podman") and (not env.get("CI") or env.get("TTYAK_PODMAN_TESTS") == "1"):
        return ["podman", "run", "--rm", "-v", folder + ":/mnt:ro,Z", IMAGE, "pwsh", "-NoProfile", "-Command",
                parse_script("/mnt")]
    return None


def without_powershell(env=None):
    """What the parse check does with no PowerShell to run: "fail" in CI (env CI set), where the runner is expected to
    have pwsh, so a runner-image change can't switch the check off unnoticed; "skip" elsewhere."""
    env = os.environ if env is None else env
    return "fail" if env.get("CI") else "skip"


def parse_errors(folder=HERE):
    """PowerShell's parse errors for the scripts in `folder` ('' when there are none); None without PowerShell."""
    # a copy, so that the container never relabels the repository
    d = tempfile.mkdtemp(prefix="ttyak-vm-parse-")
    try:
        for path in scripts(folder):
            shutil.copy(path, d)
        cmd = parse_command(d)
        if cmd is None:
            return None
        p = subprocess.run(cmd, capture_output=True, text=True, timeout=600)
        if p.returncode != 0 or "errors: " not in p.stdout:
            raise AssertionError("PowerShell did not run: " + p.stdout + p.stderr)
        return "" if "errors: 0" in p.stdout else p.stdout
    finally:
        shutil.rmtree(d)


class VmScripts(unittest.TestCase):
    def test_there_are_scripts(self):
        self.assertIn("insession.ps1", [os.path.basename(p) for p in scripts()])

    def test_plain_ascii(self):
        self.assertEqual(non_ascii(), [])

    def test_they_parse(self):
        errors = parse_errors()
        if errors is None:
            why = "needs pwsh, or podman (in CI only with TTYAK_PODMAN_TESTS=1)"
            if without_powershell() == "fail":
                self.fail("no PowerShell in CI: " + why)
            self.skipTest(why)
        self.assertEqual(errors, "")

    def test_ci_without_powershell_fails(self):
        # a runner image without pwsh must not turn the parse check off unnoticed
        self.assertEqual(without_powershell({"CI": "true"}), "fail")
        self.assertEqual(without_powershell({}), "skip")

    def test_podman_only_when_asked_in_ci(self):
        if shutil.which("pwsh") or not shutil.which("podman"):
            self.skipTest("only without pwsh and with podman")
        self.assertIsNone(parse_command("/x", {"CI": "true"}))
        self.assertIsNotNone(parse_command("/x", {"CI": "true", "TTYAK_PODMAN_TESTS": "1"}))


if __name__ == "__main__":
    unittest.main()
