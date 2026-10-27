# Linux instances Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** An agent can get a throwaway Linux desktop in a rootless podman container — sway first, GNOME if
gnome-shell runs headless — run ttyak in it with a claimed account's session, watch its windows, notifications and
sound there, and have it gone afterwards; nothing appears on the maintainer's screen.

**Architecture:** The machine-level claim tool (a private repository, named in `AGENTS.local.md`) gains `instance`
commands. An instance is a container from a desktop image built once (`images/<desktop>/`): an entrypoint starts a
session bus, a headless compositor with one invisible output, a notification daemon and PipeWire with a null sink,
then reports ready. Creating one takes a **slot**: a claim counted against the desktop's free RAM (a reserve kept for
the maintainer), so instances queue like boxes do. Instances are labelled with holder, purpose and slot; one whose
slot claim is gone or stale is removed by `instance reap` (also run before every `instance new`). ttyak's live-testing
recipe then runs ttyak inside an instance, with the account's session `install`ed into a fake home mounted there.

**Tech Stack:** Python 3.9+ stdlib (the claim tool), podman (rootless, cgroups v2), Arch Linux images with sway,
mako, kitty, PipeWire/WirePlumber, grim; GNOME's gnome-shell/mutter for the second image.

**Spec:** [docs/superpowers/specs/2026-10-27-test-machines-design.md](../specs/2026-10-27-test-machines-design.md)
— Part 3 (Linux, common rules, capacity, SSD) and the spike of 2026-10-27 recorded there.

## Global Constraints

- Shell rules: bash, never fish (`NAME=value`); `set -euo pipefail`; no recursive `rm`/`chmod`/`chown`/`mv`/`cp` on a
  variable path (literal, or `${VAR:?}` after a check); the tool never `shutil.rmtree`s; containers are removed with
  `podman rm -f <name>` by the name it created.
- The claim tool's rules hold (its README; Python 3.9 floor, stdlib only, tests pristine under `-W error`, its own
  git identity, implementers never push, nothing private in ttyak's tracked files — the tool is named only as "the
  claim tool named in `AGENTS.local.md`").
- **Nothing on the maintainer's screen; no capture of the real microphone, camera or screen.** An instance has no
  access to the host's Wayland/X11 sockets, PipeWire, D-Bus session, GPU or devices; it runs `--userns=keep-id`
  (files in mounted folders stay the desktop user's), `--cap-add=SYS_NICE` (sway needs it), no other added caps.
- **SSD**: an image is built once and rebuilt only on purpose (`instance image build <desktop>`); an instance writes
  to tmpfs (`--tmpfs` for `/tmp`, the runtime dir and the user's home); the only host folders mounted are the ones
  the caller names (a fake home, read-write; a binary or folder, read-only); nothing is compiled inside.
- **Capacity**: an instance needs `instance_gb` (default 1.5) of RAM; a slot is granted only if the desktop's
  `MemAvailable` minus `instance_gb` stays above the reserve: `reserve_gb` (default 16) plus the RAM of each libvirt VM
  in the box list that is defined but not running (it may start at any time). Values live in the box list's config.
- **Secrets**: sessions reach an instance only through a fake home that `accounts.py install` filled (ttyak) and that
  the caller mounts; the claim tool never reads or copies session files.
- Exit codes as the claim tool's: 0, 2, 3, 5 (podman or the image unavailable), 6 (no live slot claim), 10, 130.

## Review Focus

1. **A leaked instance**: an agent killed after `instance new` — the container must be reaped once its slot claim
   lapses (Task 3, `test_reap_removes_instances_of_lapsed_slots`), and never a container the tool did not create
   (`test_reap_ignores_unlabelled_containers`).
2. **The maintainer's screen or sound**: no host display, audio or D-Bus socket reaches the container (Task 1,
   `test_run_args_have_no_host_sockets`, checked on the exact podman argument list).
3. **Memory pressure**: two agents asking at once when one slot is left — exactly one gets it (Task 2,
   `test_concurrent_slot_claims_respect_capacity`).
4. **A mounted path that escapes**: `--home`/`--mount` given a symlink, a relative path or a path inside the
   maintainer's real ttyak folders — refused (Task 3, `test_mount_paths_checked`).
5. **Ready that never comes**: the entrypoint fails (no sway) — `instance new` removes the container, releases the
   slot and exits 5 with the entrypoint's last log lines (Task 3, `test_failed_start_cleans_up`).

---

### Task 1: The sway desktop image

**Files (claim tool repository):** Create `images/sway/Containerfile`, `images/sway/entrypoint.sh`,
`images/sway/sway.conf`, `<tool>/images.py`, `tests/test_images.py`; modify `<tool>/cli.py`, `README.md`.

**Interfaces:**
- `instance image build DESKTOP` builds `localhost/<tool>-DESKTOP:YYYYMMDD` and tags `:latest`
  (`podman build images/DESKTOP`); `instance image ls`.
- `images.run_args(desktop, name, labels, home, mounts) -> list[str]` — the full `podman run -d` argument list:
  `--name`, `--label <tool>.*`, `--userns=keep-id`, `--cap-add=SYS_NICE`, `--tmpfs` for `/tmp`,
  `/run/user/<uid>` and the user's home unless `home` is given, `-v HOME:/home/user:rw` when given, each mount
  `-v SRC:DST:ro`, `--network=slirp4netns` (Slack needs the network), the image. Nothing else.
- The image: Arch base; packages sway, kitty, mako, libnotify, dbus, pipewire, wireplumber, pipewire-pulse, libpulse,
  grim, jq, alsa-lib (ttyak links libasound), noto fonts; user `user` with the desktop user's uid via keep-id.
  `entrypoint.sh`: `dbus-launch`, sway headless (`WLR_BACKENDS=headless WLR_LIBINPUT_NO_DEVICES=1
  WLR_RENDERER=pixman`, output 1280x800, `exec mako`), pipewire + wireplumber + pipewire-pulse with a null sink as
  the default output, then writes `/run/user/<uid>/<tool>-ready` holding `WAYLAND_DISPLAY`, `SWAYSOCK`,
  `DBUS_SESSION_BUS_ADDRESS`, `PULSE_SERVER` as `KEY=VALUE` lines; on any failure it writes `<tool>-failed` with
  the reason and exits.

- [ ] **Step 1: Failing tests** (`tests/test_images.py`, no podman needed): `test_run_args_have_no_host_sockets` (no
  `/run/user/<host uid>`, no `WAYLAND_DISPLAY`/`DISPLAY`/`XDG_RUNTIME_DIR`/`DBUS`/`PULSE` from the host env, no
  `/dev`, no `--privileged`, no `--net=host`), `test_run_args_mounts_ro_and_home_rw`,
  `test_run_args_tmpfs_without_home`.
- [ ] **Step 2:** Run — FAIL. **Step 3:** Implement. **Step 4:** PASS; commit.
- [ ] **Step 5 (real podman, opt-in test `<TOOL>_PODMAN=1`):** `test_sway_image_comes_up` — build once, run an
  instance, read `<tool>-ready` within 30 s, `swaymsg -t get_outputs` lists one output, `notify-send` reaches
  `makoctl list`, `pactl get-default-sink` is the null sink, `grim` writes a PNG; the container is removed after.
  Record the image size and the instance's RSS (`podman stats --no-stream`) in the report.

### Task 2: Slots — capacity claims counted from free RAM

**Files:** Create `<tool>/slots.py`, `tests/test_slots.py`; modify `<tool>/config.py` (`instances` section:
`instance_gb`, `reserve_gb`), `README.md`.

**Interfaces:**
- The slots live in the desktop's own claim state (`~/.<tool>/slots/`), one claim state per slot `1..max_slots`
  (`max_slots` default 8) plus one shared FIFO queue; the same claim rules as boxes (lease, ticket TTL, stale break by
  the head waiter, report-exit 10, resume with `--ticket`).
- `slots.allowed(now_meminfo: dict, vms: list) -> int` — how many slots may be held now: the free RAM rule above,
  counting instances already running as using `instance_gb` each.
- `slots.claim(ticket, holder, purpose, lease, ttl) -> answer` — under one lock: expire tickets, the head of the
  queue takes the lowest free slot number if `held < allowed`, else waits; answers carry the slot number.
- `<tool> slot status` (held slots, queue, the RAM figures behind `allowed`).

- [ ] **Step 1: Failing tests**: `test_allowed_from_memavailable_and_reserve`, `test_stopped_vm_counts_in_reserve`,
  `test_head_takes_lowest_free_slot`, `test_no_slot_when_memory_short_then_granted_after_release`,
  `test_concurrent_slot_claims_respect_capacity` (8 processes, allowed = 2: exactly 2 granted, distinct slots),
  `test_stale_slot_broken_by_head`.
- [ ] **Step 2:** Run — FAIL. **Step 3:** Implement (reuse the box claim code where it fits; one lock for the whole
  slot state). **Step 4:** PASS; commit.

### Task 3: Instance lifecycle

**Files:** Create `<tool>/instances.py`, `tests/test_instances.py`, `tests/fake_podman.py`; modify
`<tool>/cli.py`, `README.md`.

**Interfaces** (all take the slot claim id `ID` that `instance new` prints; without a live claim → exit 6):
- `instance new --desktop sway --purpose P [--holder H] [--home DIR] [--mount SRC:DST]… [--wait …]
  [--report-every …] [--lease …]` — reap first; claim a slot; `podman run -d` with `run_args`; wait for
  `<tool>-ready` (60 s) or `<tool>-failed`; prints `instance NAME id=ID slot=N` and the `rm:` line. On failure:
  `podman rm -f`, release the slot, exit 5 with the entrypoint's reason. `--home` and each mount SRC must be
  absolute, existing, not symlinks, and not inside the maintainer's real ttyak folders (same rule as the account tool).
- `instance exec NAME ID [--tty] -- CMD…` — `podman exec` as `user` with the ready file's environment; renews the
  slot claim while it runs (like `run`).
- `instance shot NAME ID OUT.png`, `instance notifications NAME ID [--json]` (`makoctl list`), `instance
  record NAME ID SECONDS OUT.wav` (`pw-record` from the null sink's monitor), `instance windows NAME ID`
  (`swaymsg -t get_tree` → class, title, focused).
- `instance rm NAME ID` (remove, release); `instance ls` (labels, age, slot, holder, purpose; no ids); `instance
  reap` — removes every container labelled `<tool>.instance=1` whose slot claim is released, stale or held by a
  different id; never touches an unlabelled container.

- [ ] **Step 1: Failing tests** (with `fake_podman.py` recording calls and simulating ready/failed):
  `test_new_claims_slot_runs_and_waits_ready`, `test_failed_start_cleans_up`, `test_exec_uses_ready_env_and_renews`,
  `test_exec_without_claim_exit_6`, `test_rm_removes_and_releases`, `test_reap_removes_instances_of_lapsed_slots`,
  `test_reap_ignores_unlabelled_containers`, `test_mount_paths_checked`, `test_ls_hides_ids`.
- [ ] **Step 2:** Run — FAIL. **Step 3:** Implement. **Step 4:** PASS; commit.
- [ ] **Step 5 (real podman, opt-in):** `test_instance_end_to_end` — new, exec `notify-send`, `notifications` shows
  it, `windows` shows a kitty started by exec, `shot` writes a PNG, `record` writes a WAV, `rm`; `ls` empty after.

### Task 4: GNOME, if gnome-shell runs headless

**Files:** `images/gnome/…` like Task 1.

- [ ] **Step 1 (spike, bounded to an hour):** an Arch image with gnome-shell; `gnome-shell --headless --wayland
  --virtual-monitor 1280x800` (or `--nested`-free equivalents) under the same entrypoint contract; a `notify-send`
  must reach GNOME's notification list (`gdbus call --session --dest org.gnome.Shell …` or the
  `org.freedesktop.Notifications` server's presence) and a window must take focus. If it does not work in an hour,
  record why in the ledger and stop this task (NOT, with the reason).
- [ ] **Step 2 (if it works):** `images/gnome/`, the desktop name `gnome` accepted by `instance new`, the opt-in
  end-to-end test for it (`notifications` reads GNOME's list), README.

### Task 5: ttyak's live checks run in an instance

**Files (ttyak):** `docs/agents/live-testing.md` (a new "A Linux instance" section; "Test windows" points to it as the
default for Linux checks; the hidden desktop workspace stays for what an instance cannot do: Hyprland, the
maintainer's own desktop), `AGENTS.local.md` (local: the instance commands).

- [ ] **Step 1 (docs, implementer):** the recipe: claim the account; `install` into a fake home on the desktop;
  `instance new --desktop sway --home "$FAKE" --mount "$REPO/target/debug/ttyak:/opt/ttyak/ttyak"`;
  `instance exec … -- kitty --class ttyak-test -e /opt/ttyak/ttyak --profile <A>` (HOME is the mounted fake home
  inside); drive it with `kitty @` through `instance exec`; observe with `notifications`, `windows`, `shot`,
  `record`; end with `instance rm`, then the account `release`. Public wording; links and privacy checks.
- [ ] **Step 2 (controller, live):** A in a sway instance, B posting from the desktop with its own claimed fake home
  (`ctl post`): the notification appears in the instance's `notifications`; a mention raises/focuses ttyak's window
  per `windows`; ttyak's sound (if its settings allow it in the fake home) shows in a `record`; nothing appeared on
  the maintainer's screen; `instance ls` empty and both accounts free after. LIVE/UNIT/NOT in the ledger.
- [ ] **Step 3:** Final whole-plan review (both repositories); every real finding fixed; the ledger's last entry;
  `docs/PROGRESS.md` rewritten.
