To build: a recent stable Rust toolchain (edition 2024; [rustup](https://rustup.rs)), a C/C++ compiler, `pkg-config`,
**cmake** (the bundled Opus codec is built with it) and the **ALSA headers** on Linux. On a fresh system that is:

| System | Packages |
|---|---|
| Debian 13 / Ubuntu 24.04 | `apt install build-essential pkg-config cmake libasound2-dev git curl` |
| Fedora | `dnf install gcc gcc-c++ make pkgconf-pkg-config cmake alsa-lib-devel git` |
| Arch | `pacman -S base-devel cmake alsa-lib git` |
| macOS | Xcode command-line tools, then `brew install cmake` |

The Linux lines are checked with a clean checkout in a fresh container: `tools/container-build.sh debian|ubuntu|fedora|arch`
(needs podman) installs those packages (plus curl, and on Debian/Ubuntu CA certificates, for rustup) and rustup's
stable toolchain, then builds and tests. On macOS, CI builds
and tests on macOS 14 (Apple Silicon); the list above on a fresh Mac is not tried yet.

## Build and run

```sh
cargo build --release
./target/release/ttyak    # the first time: a short guide signs you in, then ttyak opens
```

For working on ttyak (a debug build, the tests, logs), see
[CONTRIBUTING.md](https://github.com/dmytrobsp/ttyak/blob/master/CONTRIBUTING.md).

## What it needs to run

`libasound.so.2` (ALSA's library, on every desktop Linux) and the C++ runtime. Optional: **ffmpeg / ffprobe on PATH**
for video playback, cameras and (outside Wayland) screen sharing; a D-Bus notification service for desktop
notifications; a terminal with graphics for real images (see [Requirements](Requirements); others get half-block
images). Without a sound card huddles still work (silently, with a note in the call bar); without system CA
certificates ttyak uses the Mozilla roots it carries.

## Windows from Linux

`rustup target add x86_64-pc-windows-gnu`, `apt install mingw-w64` (or only its x86_64 part, as CI does:
`gcc-mingw-w64-x86-64 g++-mingw-w64-x86-64`), then `cargo build --release --target x86_64-pc-windows-gnu`. The result
is one `ttyak.exe` that needs no MinGW DLLs (`crates/ttyak/build.rs` links OpenH264's C++ runtime statically,
`.cargo/config.toml` the C runtime). A native Windows build (MSVC) is not tried yet.

## If it doesn't work

- [Troubleshooting](Troubleshooting)
