# Ledger — plan docs/superpowers/plans/2026-10-27-linux-instances.md

Design: docs/superpowers/specs/2026-10-27-test-machines-design.md (Part 3, the spike of 2026-10-27). Start: ttyak
1d08267, the claim tool 0406eff. Tasks: 1 the sway image, 2 slots, 3 instance lifecycle, 4 GNOME (spike
first), 5 ttyak's live checks in an instance, final review. Tasks 1–4 are in the claim tool's private repository.

## Pre-flight scan

| Rows | Produces → consumes | Found |
|---|---|---|
| 1 ↔ 3 | `run_args`, the ready/failed file contract → `instance new`/`exec` | agree |
| 2 ↔ 3 | slot claims and ids → every `instance` command | agree |
| 1 ↔ 4 | the entrypoint contract → the GNOME image | agree; Task 4 reuses it |
| 3 ↔ 5 | the `instance` commands → ttyak's recipe | agree |
| 1, 3 (self) | opt-in real-podman tests beside the unit tests | agree; the claim tool has no CI |
| 5 (self) | step 1 docs, step 2 live | ruling 1 |

- Ruling: Task 5 step 2 (live, with the Slack accounts) is the controller's; implementers never run ttyak against
  Slack — costs time if wrong.
- The maintainer's decisions (2026-10-27): Hyprland goes to the VM plan; containers now (sway, then GNOME); the plan
  approved.

## Events

- Task 1: dispatched.
- Task 1: 703261a (image 1.38 GB, ready in 1.2 s, idle instance ~93 MiB). Review: not approved — the podman argument
  list not pinned by the test; `<tool>-failed` unreadable (the failing entrypoint stopped the container); podman's exit
  codes passed through; the null sink loadable twice; no pipewire-alsa (ttyak's sound would not reach the sink); 5
  nits → fix round 1 with all of them, plus a run with another host uid.
- Task 1: fix round 1/5 (all 5 + nits addressed; 78d0db5; 141 unit, 12 opt-in podman tests; image 1.37 GB; ttyak
  plays through cpal callback streams, so aplay's drain hang on the null sink is not on its path). The uid check found a
  real bug; its first fix only covered the simulated case → fix round 2: always `--user=UID:GID` (real uid and gid),
  and the entrypoint honours SIGTERM (stops took 10 s).
- Task 1: fix round 2/5 (2 addressed; 3be9fe4). Complete (0406eff..3be9fe4 in the claim tool, review clean). UNIT: 144;
  opt-in podman: 15 (ready in ~1 s, null sink, ALSA reaching it, failure readable, another uid, stop < 3 s).
- Task 2: dispatched (claim tool base 3be9fe4).
- Task 2: fc20d0e; review: not approved — virsh inside the pool mutex (a hung libvirtd, or 3+ stopped VMs passing the
  mutex's stale age → double grants); over-grant in a burst (MemAvailable falls only once a container runs); the
  concurrency test modelled instant start; ids unchecked in the library; a malformed pool file → raw errors → fix
  round 1 with all of them (pending slots charged until `started`). The desktop user is in the libvirt group (no
  polkit prompt from virsh).
- Task 2: fix round 1/5 (5 addressed; 5e482bf; no test lost — all 18 earlier names present, 5 new); new: MemAvailable
  read before the mutex reopens the burst race through `started()` + 2 nits → fix round 2.
- Task 2: fix round 2/5 (3 addressed; 7797c59). Complete (3be9fe4..7797c59, review clean). UNIT: the suite re-run by
  the controller: 168 tests OK (5 opt-in skipped). A 126-column line in cli.py goes into Task 3's brief.
- Task 3: dispatched (claim tool base 7797c59).
- Task 3: b387706 (implementer; 199 unit, 2 opt-in real-podman tests; no container left). Outside the workflow: the
  implementer did not work test-first (tests and code in one pass) — the review is asked to check by mutation that
  the key guards' tests would fail without them.
- Task 3: review: not approved — a failed start could remove an unlabelled container the tool did not create; the
  real-folder rule ignored the password-database home (a fake HOME could mount the real ttyak folder); a busy pool
  lock leaked the container; `reap` could print a claim id; the relative-path test proved nothing; `//etc` and paths
  under /proc, /sys, /dev, /run accepted as mount targets; a README gap; 6 nits → fix round 1 with all of them,
  test-first. Mutation checks: the claim check, reap's label/live checks and the failed-start cleanup are proven.
- Task 3: fix round 1/5 (7 + nits addressed, test-first: 8 RED before; ac5be70). Complete (7797c59..ac5be70, review
  clean; 8 mutations on a copy each fail their tests). UNIT: 206; opt-in podman: 2 (end to end, failed start).
- Task 3: carried to the final fix wave (not dropped): the slot is not renewed during `podman run` itself (a lease
  under ~3 min could lapse while the image starts).
- Task 4: dispatched (GNOME spike, bounded to an hour; claim tool base ac5be70).
- Task 4: GNOME works (16eb002: headless gnome-shell 51.1, a private stand-in system bus, an image extension for
  windows and notifications; image 2.73 GB, idle ~305 MB). Review: not approved — the failed-start test was empty;
  screenshots refused though the extension can take them; an unknown desktop label fell back to sway tools; a brittle
  test line; image size → fix round 1 with all of them. Ruling: keep noto-fonts in the gnome image (text renders
  the same as on sway) — costs ~100 MB if wrong.
- Task 4: fix round 1/5 (fc4c897): the real failed-start test, screenshots on gnome through the extension, strict
  desktop dispatch, the brittle assert; beyond: SYS_NICE left off for gnome (screenshots came out empty with it) and a
  narrower private-bus policy. The slimming saved nothing (the stack dominates) → a last small round: README honesty
  and wrap, the extension closing its stream on error. Ruling: accept the 2.73 GB gnome image (built once; dropping
  noto-fonts or mesa's llvm would trade rendering parity for ~10%) — costs disk if wrong.
- Task 4: fix round 2/5 (451a93b). Complete (ac5be70..451a93b, review clean). UNIT: 219; opt-in real gnome: 3
  (notifications, a focused window, screenshots, failed start). Dropped (nit): the Containerfile's redundant doc/locale
  `rm` (pacman.conf already NoExtracts them; the README says so).
