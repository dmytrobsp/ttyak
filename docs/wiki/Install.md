## Install

One line installs ttyak, or your package manager does. (0.10.0 and older: only the zips on the
[releases page](https://github.com/dmytrobsp/ttyak/releases).) What your terminal and system need is on
[Requirements](Requirements).

### The install script

Linux and macOS:

```sh
curl -fsSL https://ttyak.sh/install.sh | sh
```

Windows (PowerShell):

```powershell
irm https://ttyak.sh/install.ps1 | iex
```

The script takes the latest release's zip for your system, checks it against the release's `SHA256SUMS` and puts
`ttyak` in `~/.local/bin` (Windows: `%LOCALAPPDATA%\Programs\ttyak`). When that folder is not on your PATH yet, it
puts it there: a marked line in your shell's startup file (sh, dash, ksh, bash, zsh, fish; for csh, nushell, elvish,
xonsh and other shells it prints the line to add instead) or, on Windows, an entry in your user PATH. Open a new terminal
afterwards. Then `ttyak auth login`, or plain `ttyak`, signs you in.

Run it as yourself: it needs no sudo, and under sudo it stops unless you give it a folder with `--dir`.

`--help` (`-Help`) lists its flags: another version (`--version X.Y.Z`), another folder (`--dir PATH`), PATH left
alone (`--no-modify-path`), no questions (`--yes`). Through the pipe they go like this:

```sh
curl -fsSL https://ttyak.sh/install.sh | sh -s -- --version 0.12.0 --dir ~/bin
```

```powershell
& ([scriptblock]::Create((irm https://ttyak.sh/install.ps1))) -Version 0.12.0 -Dir C:\tools\ttyak
```

The environment variables `TTYAK_VERSION`, `TTYAK_DIR`, `TTYAK_NO_MODIFY_PATH=1` and `TTYAK_YES=1` do the same (the
flags win). Running the script again over an installed copy updates it in place.

The macOS builds are signed and notarized by Apple. `ttyak.exe` is **not code-signed** yet (a later release will be),
so SmartScreen may warn the first time you run it: **More info → Run anyway**.

### Package managers

| Package manager | |
|---|---|
| Homebrew (macOS, Linux) | `brew install dmytrobsp/ttyak/ttyak` |
| Scoop | `scoop bucket add ttyak https://github.com/dmytrobsp/scoop-ttyak`, then `scoop install ttyak` |
| winget | `winget install dmytrobsp.ttyak` |
| Chocolatey | `choco install ttyak` |
| Debian, Ubuntu | the release's `.deb`: `sudo apt install ./ttyak_<version>_amd64.deb` (arm: `_arm64.deb`) |
| Fedora, RHEL | the release's `.rpm`: `sudo dnf install ./ttyak-<version>-1.x86_64.rpm` (arm: `.aarch64.rpm`) |
| Arch | not in the AUR yet (its sign-up is closed for now): unzip the release's `ttyak-<version>-recipes.zip` and run `makepkg -si` in its `aur` folder |

winget and Chocolatey review each new version before they list it: a version shows up there a few days after the
release. winget has not listed ttyak yet (its first version is waiting for review); until it does,
`winget install dmytrobsp.ttyak` finds nothing, so use the install script or Scoop.

Homebrew on Linux uses the system's ALSA library: `libasound2` (`libasound2t64` on Ubuntu 24.04, Debian 13 and later) on
Debian/Ubuntu, `alsa-lib` on Fedora/Arch. Homebrew's own alsa-lib is only a fallback for systems without one, and it needs
glibc 2.38 or newer.

### By hand

Each release has one zip per system with `ttyak` (`ttyak.exe`) and the two licences (0.12.1 and older: the README
too) at its top. Unzip it and put `ttyak` somewhere on your PATH. To check a download yourself, see
[Verifying downloads](Verifying-downloads). (Releases before 0.11.0 are not signed: on macOS run
`xattr -d com.apple.quarantine ttyak` once, or allow it in System Settings → Privacy & Security.)

| Download | Runs on | Needs |
|---|---|---|
| `x86_64-linux.zip` | glibc 2.34 or newer (Ubuntu 22.04, Debian 12, Fedora 35, RHEL 9 and later) | `libasound.so.2` (ALSA), `libstdc++` — on every desktop Linux |
| `aarch64-linux.zip` | 64-bit Arm Linux with glibc 2.34 or newer (as above; Raspberry Pi OS 12) | as above |
| `aarch64-macos.zip` | Apple Silicon Macs, macOS 11 or newer | nothing (system frameworks only) |
| `x86_64-macos.zip` | Intel Macs, macOS 11 or newer | as above |
| `x86_64-windows.zip` | 64-bit Windows 10 / 11 | nothing (Windows system DLLs only, no MinGW runtime) |

The Intel Mac build is made and signed the same way as the Apple Silicon one, but it has not been run on an Intel Mac
yet.

Optional everywhere: `ffmpeg` / `ffprobe` on PATH for video, cameras and screen sharing; a terminal with graphics for
real pictures (see [Requirements](Requirements)). To build it yourself instead, see
[Building from source](Building-from-source).

## Update

ttyak says once when a new version is out (a desktop notification when they are on, and `⬆ <version>` in the status
line), and
F6 → Updates shows how this copy was installed and what to run to update it (its "To update it" row; Enter copies it):

- the install script: while a newer version is known, F6 → Updates has an **Update** row that opens the update screen;
  **Update now** there downloads the new version, checks it against the release's `SHA256SUMS` and puts it in place
  (start ttyak again: it never restarts by itself). Or run the one-liner again. A copy in a folder ttyak cannot write
  shows the one-liner for that folder instead.
- `brew upgrade dmytrobsp/ttyak/ttyak` · `scoop update ttyak` · `winget upgrade dmytrobsp.ttyak` ·
  `choco upgrade ttyak` (winget and Chocolatey list a new version a few days after the release)
- `.deb`, `.rpm`: install the new release's package as above; Arch: `makepkg -si` with the new release's recipes zip
  (F6 → Updates shows the release page to download from)
- by hand: the new zip's `ttyak` over the old one

## Uninstall

- the install script: `ttyak uninstall` removes the program, its `install.json` and what put its folder on PATH (the
  marked line in your shell's startup file; on Windows the user PATH entry), and asks whether to keep your settings,
  sign-ins and drafts. For scripts: `ttyak uninstall --yes` deletes the data too, `ttyak uninstall --yes --keep-data`
  keeps it. A startup file that is a link, or lies in a linked folder, or is not writable, is left alone: ttyak shows
  the lines to remove by hand. The install script's `--uninstall` (`-Uninstall`) runs the same command.
- `brew uninstall ttyak` · `winget uninstall dmytrobsp.ttyak` · `sudo apt remove ttyak` · `sudo dnf remove ttyak` ·
  Arch: `sudo pacman -Rns ttyak-bin` — these leave your ttyak data (settings, sign-ins, drafts, caches) in your home
  folder: run `ttyak purge` first to delete it
- `scoop uninstall ttyak` and `choco uninstall ttyak` delete your ttyak data too, as `ttyak purge` does (an update
  keeps it)
- by hand: `ttyak purge` (if you want your data gone), then delete `ttyak`

`ttyak purge` lists what it will delete and asks first (`--yes` for scripts), and refuses while a ttyak runs. Files you
saved from messages stay, and Slack is not signed out.

## If it doesn't work

- [Troubleshooting](Troubleshooting)
