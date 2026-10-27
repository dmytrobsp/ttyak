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
