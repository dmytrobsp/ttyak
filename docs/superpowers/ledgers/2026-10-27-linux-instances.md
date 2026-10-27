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
